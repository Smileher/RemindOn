import assert from 'node:assert/strict'
import test from 'node:test'
import { fetchLatestRelease, parseRelease } from '../build.mjs'
import { locales } from '../content.mjs'
import { renderPage, repository } from '../template.mjs'

function releaseFixture(version = '0.7.0') {
  const names = [`RemindOn_${version}_x64-setup.exe`, `RemindOn_${version}_arm64-setup.exe`, `RemindOn_${version}_x64_portable.exe`, `RemindOn_${version}_arm64_portable.exe`, `RemindOn_${version}_aarch64.dmg`, 'RemindOn_aarch64.app.tar.gz']
  return {
    tag_name: `v${version}`, draft: false, prerelease: false,
    published_at: '2026-09-10T07:18:05Z', html_url: `${repository}/releases/tag/v${version}`,
    assets: names.map((name) => ({ name, size: 3000000, browser_download_url: `${repository}/releases/download/v${version}/${name}` })),
  }
}

test('a new version updates GitHub downloads and the Store entry on both pages', () => {
  const release = parseRelease(releaseFixture('0.8.0'))
  assert.equal(release.version, '0.8.0')
  assert.deepEqual(Object.keys(release.downloads), ['windows', 'windowsArm64', 'portable', 'portableArm64', 'mac'])
  for (const language of Object.keys(locales)) {
    const html = renderPage(language, release)
    assert.match(html, /v0\.8\.0/)
    assert.doesNotMatch(html, /v0\.7\.0|app\.tar\.gz/)
    for (const asset of Object.values(release.downloads)) assert.ok(html.includes(`href="${asset.url}"`))
    assert.match(html, /apps\.microsoft\.com\/detail\/9P9K31N2CJBW/)
    assert.equal((html.match(/class="platform-card"/g) || []).length, 2)
    assert.ok(html.includes(`<html lang="${locales[language].lang}">`))
    assert.ok(html.includes(`href="https://smileher.github.io/RemindOn/${locales[language].path}"`))
    assert.ok(html.includes('href="/RemindOn/assets/style.css"'))
    assert.ok(html.includes('src="/RemindOn/assets/theme.js"'))
    assert.doesNotMatch(html, /undefined|screenshot-placeholder|\.webp/)
  }
})

test('the landing renders the glass structure and bilingual mockup copy', () => {
  const release = parseRelease(releaseFixture())
  for (const language of Object.keys(locales)) {
    const html = renderPage(language, release)
    assert.ok(html.includes('class="nav-inner"'))
    assert.ok(html.includes('class="mockup-frame"'))
    assert.ok(html.includes('class="gradient-text"'))
    assert.equal((html.match(/class="feature"/g) || []).length, 7)
    assert.equal((html.match(/<details class="faq-item"/g) || []).length, locales[language].faqs.length)
    const mockup = locales[language].mockup
    assert.ok(html.includes(mockup.title))
    assert.ok(html.includes(mockup.status))
    assert.ok(html.includes(mockup.message))
    assert.ok(html.includes(mockup.nav[0]))
    assert.ok(html.includes(mockup.settings))
    assert.ok(html.includes(mockup.about))
    assert.ok(html.includes(`RemindOn · v${release.version}`))
    for (const trust of locales[language].trust) assert.ok(html.includes(trust))
    assert.doesNotMatch(html, /undefined/)
  }
})

test('missing platform package fails instead of linking a signature or update archive', () => {
  for (const index of [0, 1, 2, 3, 4]) {
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
