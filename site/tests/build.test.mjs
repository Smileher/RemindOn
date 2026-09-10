import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { checkScreenshots, fetchLatestRelease, parseRelease, screenshotNames } from '../build.mjs'
import { locales } from '../content.mjs'
import { renderPage, repository } from '../template.mjs'

function releaseFixture(version = '0.7.0') {
  const names = [`RemindOn_${version}_x64-setup.exe`, `RemindOn_${version}_x64_portable.exe`, `RemindOn_${version}_aarch64.dmg`, 'RemindOn_aarch64.app.tar.gz']
  return {
    tag_name: `v${version}`, draft: false, prerelease: false,
    published_at: '2026-09-10T07:18:05Z', html_url: `${repository}/releases/tag/v${version}`,
    assets: names.map((name) => ({ name, size: 3000000, browser_download_url: `${repository}/releases/download/v${version}/${name}` })),
  }
}

test('a new version updates all three downloads and both pages together', () => {
  const release = parseRelease(releaseFixture('0.8.0'))
  assert.equal(release.version, '0.8.0')
  assert.deepEqual(Object.keys(release.downloads), ['windows', 'portable', 'mac'])
  for (const language of Object.keys(locales)) {
    const html = renderPage(language, release, screenshotNames)
    assert.match(html, /v0\.8\.0/)
    assert.doesNotMatch(html, /v0\.7\.0|app\.tar\.gz/)
    for (const asset of Object.values(release.downloads)) assert.ok(html.includes(`href="${asset.url}"`))
    assert.match(html, new RegExp(`overview-${language}-dark.webp`))
    assert.match(html, new RegExp(`overview-${language}-light.webp`))
    assert.ok(html.includes('media="(prefers-color-scheme: dark)"'))
    assert.ok(html.includes(`<html lang="${locales[language].lang}">`))
    assert.ok(html.includes(`href="https://smileher.github.io/RemindOn/${locales[language].path}"`))
    assert.ok(html.includes('href="/RemindOn/assets/style.css"'))
    assert.ok(html.includes('src="/RemindOn/assets/theme.js"'))
    assert.doesNotMatch(html, /undefined|screenshot-placeholder/)
  }
})

test('missing platform package fails instead of linking a signature or update archive', () => {
  for (const index of [0, 1, 2]) {
    const release = releaseFixture()
    release.assets.splice(index, 1)
    assert.throws(() => parseRelease(release), /Missing or invalid release asset/)
  }
})

test('drafts, prereleases, invalid versions and dates cannot be published', () => {
  for (const change of [{ draft: true }, { prerelease: true }, { tag_name: 'v0.8.0-beta.1' }, { published_at: null }, { published_at: 'invalid' }]) {
    assert.throws(() => parseRelease({ ...releaseFixture(), ...change }))
  }
})

test('rejects a mismatched download version or repository', () => {
  for (const badUrl of ['https://example.com/app.exe', `${repository}/releases/download/v0.6.0/RemindOn_0.6.0_x64-setup.exe`]) {
    const release = releaseFixture()
    release.assets[0].browser_download_url = badUrl
    assert.throws(() => parseRelease(release), /Missing or invalid release asset/)
  }
})

test('API errors and connection failures abort the build data fetch', async () => {
  for (const status of [403, 404, 429, 500]) {
    await assert.rejects(fetchLatestRelease(async () => ({ ok: false, status })), new RegExp(`HTTP ${status}`))
  }
  await assert.rejects(fetchLatestRelease(async () => { throw new Error('connection failed') }), /connection failed/)
  await assert.rejects(fetchLatestRelease(async () => ({ ok: true, json: async () => { throw new SyntaxError('invalid JSON') } })), /invalid JSON/)
})

test('one API response supplies the complete release snapshot', async () => {
  let requests = 0
  const release = await fetchLatestRelease(async (url) => {
    requests++
    assert.equal(url, 'https://api.github.com/repos/Smileher/RemindOn/releases/latest')
    return { ok: true, json: async () => releaseFixture('0.9.0') }
  })
  assert.equal(requests, 1)
  assert.equal(release.version, '0.9.0')
})

test('local preview shows a placeholder until both theme screenshots exist', () => {
  const release = parseRelease(releaseFixture())
  const html = renderPage('zh', release, ['overview-zh-light.webp'])
  assert.match(html, /软件界面截图待补充/)
  assert.doesNotMatch(html, /<picture>/)
  assert.match(html, /RemindOn_0\.7\.0_x64-setup\.exe/)
})

test('production requires all four WebP files, preview permits missing files', async (context) => {
  const directory = await mkdtemp(join(tmpdir(), 'remindon-site-screenshots-'))
  // mkdtemp returns a new, absolute task-owned directory; cleanup only that directory.
  context.after(() => rm(directory, { recursive: true, force: true }))
  assert.deepEqual(await checkScreenshots(directory, true), [])
  await assert.rejects(checkScreenshots(directory), /Add screenshots before deployment/)
  const webp = Buffer.from('UklGRhoAAABXRUJQVlA4TA0AAAAvAAAAEAcQERGIiP4HAA==', 'base64')
  for (const name of screenshotNames) await writeFile(join(directory, name), webp)
  assert.deepEqual(await checkScreenshots(directory), screenshotNames)
  await writeFile(join(directory, screenshotNames[0]), 'not a WebP', 'utf8')
  await assert.rejects(checkScreenshots(directory, true), /not a valid WebP/)
})
