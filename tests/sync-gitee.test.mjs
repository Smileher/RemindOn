import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import https from 'node:https'
import { EventEmitter } from 'node:events'
import childProcess from 'node:child_process'
import { createGiteeManifest, syncGiteeRelease } from '../scripts/sync-gitee-release.mjs'

const version = '0.8.0'
const names = [
  `RemindOn_${version}_x64-setup.exe`,
  `RemindOn_${version}_arm64-setup.exe`,
  `RemindOn_${version}_x64_portable.exe`,
  `RemindOn_${version}_arm64_portable.exe`,
  'RemindOn_aarch64.app.tar.gz',
  `RemindOn_${version}_aarch64.dmg`,
  `RemindOn_${version}_x64_portable.zip`,
  `RemindOn_${version}_arm64_portable.zip`,
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
  let transientDownload = false
  let lostManifestResponse = false
  const fetchImpl = async (url, options = {}) => {
    if (new URL(url).pathname.startsWith('/smileher/RemindOn/releases/download/')) {
      const name = new URL(url).pathname.split('/').at(-1)
      if (transientDownload) {
        transientDownload = false
        return new Response('unavailable', { status: 503 })
      }
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
      if (file.name === 'latest.json' && lostManifestResponse) {
        lostManifestResponse = false
        throw new Error('connection reset after upload completed')
      }
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
  transientDownload = true
  lostManifestResponse = true
  await syncGiteeRelease(options)
  assert.deepEqual([...uploaded].sort(), [...selectedNames, 'latest.json', 'latest.json', 'latest.json'].sort())
  assert.equal(existingAssets.length, selectedNames.length + 1)
  remoteBytes.set(selectedNames[0], Buffer.from('partial attachment'))
  await syncGiteeRelease(options)
  assert.equal(Buffer.from(remoteBytes.get(selectedNames[0])).toString('utf8'), selectedNames[0])
  assert.equal(existingAssets.filter((asset) => asset.name === selectedNames[0]).length, 1)
  corruptDownload = true
  // 并发下所有资产同时校验失败，错误按数量聚合；具体原因仍逐条打印。
  await assert.rejects(syncGiteeRelease(options), (error) => {
    assert.match(error.message, new RegExp(`of ${selectedNames.length} Gitee assets failed to mirror`))
    return true
  })
  assert.equal(existingAssets.length, selectedNames.length + 1)
})
}

for (const failure of ['response error', 'aborted response', 'deadline', 'curl fallback']) {
test(`HTTPS upload recovers a completed attachment after ${failure}`, { timeout: 2000 }, async (context) => {
  const directory = await mkdtemp(join(tmpdir(), 'remindon-gitee-'))
  context.after(() => rm(directory, { recursive: true, force: true }))
  for (const name of names) await writeFile(join(directory, name), name)
  const assets = names.map((name, id) => ({ id, name, browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/${name}` }))
  let manifestBytes
  let uploadCount = 0
  context.mock.timers.enable({ apis: ['setTimeout'] })
  context.mock.method(globalThis, 'fetch', async (url) => {
    const parsed = new URL(url)
    if (parsed.pathname.includes('/releases/download/')) {
      const name = parsed.pathname.split('/').at(-1)
      return new Response(name === 'latest.json' ? manifestBytes : name)
    }
    if (parsed.pathname.endsWith('/attach_files')) return Response.json(assets)
    if (parsed.pathname.endsWith('/releases')) return Response.json([{ id: 42, tag_name: `v${version}` }])
    return Response.json({ default_branch: 'master' })
  })
  context.mock.method(https, 'request', (_url, _options, callback) => {
    uploadCount += 1
    const request = new EventEmitter()
    request.destroy = (error) => request.emit('error', error)
    request.end = (body) => {
      if (failure === 'curl fallback') {
        queueMicrotask(() => request.emit('error', new Error('socket hang up')))
        return
      }
      const text = body.toString('utf8')
      manifestBytes = Buffer.from(text.split('Content-Type: application/octet-stream\r\n\r\n')[1].split('\r\n--')[0], 'utf8')
      assets.push({ id: 100, name: 'latest.json', browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/latest.json` })
      queueMicrotask(() => {
        if (failure === 'deadline') {
          context.mock.timers.tick(8 * 60 * 1000)
          return
        }
        const response = new EventEmitter()
        callback(response)
        if (failure === 'response error') response.emit('error', new Error('connection reset'))
        else response.emit('aborted')
      })
    }
    return request
  })
  let curlCount = 0
  context.mock.method(childProcess, 'execFile', (command, args, options, callback) => {
    curlCount += 1
    assert.equal(command, 'curl')
    assert.ok(args.includes('--fail-with-body'))
    assert.ok(!args.some((arg) => arg.includes('test-token')))
    assert.equal(options.timeout, 8 * 60 * 1000 + 5000)
    const child = { stdin: new EventEmitter() }
    child.stdin.end = (body) => {
      const text = body.toString('utf8')
      assert.ok(text.includes('test-token'))
      manifestBytes = Buffer.from(text.split('Content-Type: application/octet-stream\r\n\r\n')[1].split('\r\n--')[0], 'utf8')
      const asset = { id: 100, name: 'latest.json', browser_download_url: `https://gitee.com/smileher/RemindOn/releases/download/v${version}/latest.json` }
      assets.push(asset)
      callback(null, JSON.stringify(asset))
    }
    return child
  })
  if (failure === 'curl fallback') {
    context.mock.method(console, 'warn', () => queueMicrotask(() => context.mock.timers.tick(5000)))
  }
  const result = await syncGiteeRelease({
    tag: `v${version}`, githubRelease: { tag_name: `v${version}`, assets: names.map((name) => ({ name })) },
    githubManifest: manifest(), assetDir: directory, token: 'test-token',
  })
  assert.equal(result.assetCount, names.length)
  assert.equal(uploadCount, 1)
  assert.equal(curlCount, failure === 'curl fallback' ? 1 : 0)
  assert.equal(assets.filter((asset) => asset.name === 'latest.json').length, 1)
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
