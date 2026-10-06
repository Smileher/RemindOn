import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { createGiteeManifest, syncGiteeRelease } from '../scripts/sync-gitee-release.mjs'

const version = '0.8.0'
const names = [
  `RemindOn_${version}_x64-setup.exe`,
  `RemindOn_${version}_arm64-setup.exe`,
  `RemindOn_${version}_x64_portable.exe`,
  `RemindOn_${version}_arm64_portable.exe`,
  'RemindOn_aarch64.app.tar.gz',
  `RemindOn_${version}_aarch64.dmg`,
]

function manifest() {
  return {
    version,
    platforms: {
      'windows-x86_64': { url: `https://github.com/Smileher/RemindOn/releases/download/v${version}/${names[0]}`, signature: 'sig' },
      'windows-aarch64': { url: `https://github.com/Smileher/RemindOn/releases/download/v${version}/${names[1]}`, signature: 'sig' },
      'darwin-aarch64': { url: `https://github.com/Smileher/RemindOn/releases/download/v${version}/${names[4]}`, signature: 'sig' },
    },
    downloads: Object.fromEntries([
      ['windows-x86_64-portable', names[2]],
      ['windows-aarch64-portable', names[3]],
      ['darwin-aarch64-portable', names[5]],
    ].map(([key, name]) => [key, { url: `https://github.com/Smileher/RemindOn/releases/download/v${version}/${name}`, fileName: name, sha256: 'a'.repeat(64) }])),
  }
}

test('Gitee manifest replaces every GitHub asset URL', () => {
  const assets = new Map(names.map((name) => [name, { browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/${name}` }]))
  const result = createGiteeManifest(manifest(), assets)
  for (const group of [result.platforms, result.downloads]) {
    for (const asset of Object.values(group)) assert.match(asset.url, /^https:\/\/gitee\.com\//)
  }
})

for (const legacyX64 of [false, true]) {
test(`Gitee ${legacyX64 ? 'legacy x64' : 'complete'} sync verifies anonymous downloads and does not duplicate assets`, async (context) => {
  const directory = await mkdtemp(join(tmpdir(), 'remindon-gitee-'))
  context.after(() => rm(directory, { recursive: true, force: true }))
  for (const name of names) await writeFile(join(directory, name), name)

  const selectedNames = legacyX64 ? names.filter((name) => !name.includes('_arm64')) : names
  const githubManifest = manifest()
  if (legacyX64) {
    delete githubManifest.platforms['windows-aarch64']
    delete githubManifest.downloads['windows-aarch64-portable']
  }
  const uploaded = []
  const remoteBytes = new Map()
  let existingAssets = []
  let releaseExists = false
  let corruptDownload = false
  const fetchImpl = async (url, options = {}) => {
    if (new URL(url).pathname.startsWith('/smileher/RemindOn/releases/download/')) {
      const name = new URL(url).pathname.split('/').at(-1)
      return new Response(corruptDownload ? 'invalid file' : remoteBytes.get(name))
    }
    const path = new URL(url).pathname.replace('/api/v5/repos/smileher/RemindOn', '') || ''
    if (options.method === 'POST' && path === '/releases') {
      releaseExists = true
      return Response.json({ id: 42 })
    }
    if (options.method === 'POST' && path.endsWith('/attach_files')) {
      const file = options.body.get('file')
      uploaded.push(file.name)
      remoteBytes.set(file.name, await file.arrayBuffer())
      const asset = { id: uploaded.length, name: file.name, browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/${file.name}` }
      existingAssets.push(asset)
      return Response.json(asset)
    }
    if (options.method === 'DELETE') {
      existingAssets = existingAssets.filter((asset) => asset.id !== Number(path.split('/').at(-1)))
      return new Response(null, { status: 204 })
    }
    if (path === '') return Response.json({ default_branch: 'master' })
    if (path === '/releases') return Response.json(releaseExists ? [{ id: 42, tag_name: `v${version}` }] : [])
    if (path.endsWith('/attach_files')) return Response.json(existingAssets)
    throw new Error(`unexpected ${options.method ?? 'GET'} ${path}`)
  }

  const options = {
    tag: `v${version}`,
    githubRelease: { tag_name: `v${version}`, body: '' , assets: selectedNames.map((name) => ({ name })) },
    githubManifest,
    legacyX64,
    assetDir: directory,
    token: 'test-token',
    fetchImpl,
  }
  const result = await syncGiteeRelease(options)
  assert.equal(result.releaseId, 42)
  // 资产并发上传，完成顺序不确定，按集合比较。
  assert.deepEqual([...uploaded].sort(), [...selectedNames, 'latest.json'].sort())
  await syncGiteeRelease(options)
  assert.deepEqual([...uploaded].sort(), [...selectedNames, 'latest.json', 'latest.json'].sort())
  assert.equal(existingAssets.length, selectedNames.length + 1)
  corruptDownload = true
  // 并发下所有资产同时校验失败，错误按数量聚合；具体原因仍逐条打印。
  await assert.rejects(syncGiteeRelease(options), (error) => {
    assert.match(error.message, new RegExp(`of ${selectedNames.length} Gitee assets failed to mirror`))
    return true
  })
  assert.equal(existingAssets.length, selectedNames.length + 1)
})
}

test('normal sync still rejects missing ARM64 assets and legacy mode rejects ARM64 manifests', async () => {
  const options = {
    tag: `v${version}`,
    githubRelease: { tag_name: `v${version}`, assets: names.filter((name) => !name.includes('_arm64')).map((name) => ({ name })) },
    githubManifest: manifest(),
    assetDir: 'unused',
    token: 'test-token',
    fetchImpl: () => { throw new Error('Validation must happen before network requests') },
  }
  await assert.rejects(syncGiteeRelease(options), /GitHub release is missing:.*arm64/)
  await assert.rejects(syncGiteeRelease({ ...options, legacyX64: true }), /cannot be used with an ARM64/)
})
