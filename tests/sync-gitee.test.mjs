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

test('Gitee sync creates missing assets and replaces latest.json on repeat', async (context) => {
  const directory = await mkdtemp(join(tmpdir(), 'remindon-gitee-'))
  context.after(() => rm(directory, { recursive: true, force: true }))
  for (const name of names) await writeFile(join(directory, name), name)

  let uploaded = []
  const fetchImpl = async (url, options = {}) => {
    const path = new URL(url).pathname.replace('/api/v5/repos/smileher/RemindOn', '') || ''
    if (options.method === 'POST' && path === '/releases') return Response.json({ id: 42 })
    if (options.method === 'POST' && path.endsWith('/attach_files')) {
      const file = options.body.get('file')
      uploaded.push(file.name)
      return Response.json({ id: uploaded.length, name: file.name, browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/${file.name}` })
    }
    if (options.method === 'DELETE') return new Response(null, { status: 204 })
    if (path === '') return Response.json({ default_branch: 'master' })
    if (path === '/releases') return Response.json([])
    if (path.endsWith('/attach_files')) return Response.json([])
    throw new Error(`unexpected ${options.method ?? 'GET'} ${path}`)
  }

  const result = await syncGiteeRelease({
    tag: `v${version}`,
    githubRelease: { tag_name: `v${version}`, body: '' , assets: names.map((name) => ({ name })) },
    githubManifest: manifest(),
    assetDir: directory,
    token: 'test-token',
    fetchImpl,
  })
  assert.equal(result.releaseId, 42)
  assert.deepEqual(uploaded, [...names, 'latest.json'])
})
