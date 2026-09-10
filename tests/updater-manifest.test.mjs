import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createUpdaterManifest } from '../scripts/create-updater-manifest.mjs'

const names = [
  'RemindOn_0.7.0_x64-setup.exe',
  'RemindOn_aarch64.app.tar.gz',
  'RemindOn_0.7.0_x64_portable.exe',
  'RemindOn_0.7.0_aarch64.dmg',
]
const release = {
  tag_name: 'v0.7.0',
  body: '更新说明',
  published_at: '2026-09-10T00:00:00Z',
  assets: names.map((name) => ({ name, browser_download_url: `https://github.com/Smileher/RemindOn/releases/download/v0.7.0/${name}` })),
}
const signatures = Object.fromEntries(names.slice(0, 2).map((name) => [`${name}.sig`, 'signed-package\n']))
const checksums = Object.fromEntries(names.slice(2).map((name) => [`${name}.sha256`, `${'a'.repeat(64)}  ${name}\n`]))

test('one manifest contains the signed Windows and Apple Silicon packages', () => {
  const manifest = createUpdaterManifest('0.7.0', release, signatures, checksums)
  assert.deepEqual(Object.keys(manifest.platforms), ['windows-x86_64', 'darwin-aarch64'])
  assert.equal(manifest.version, '0.7.0')
  assert.equal(manifest.notes, '更新说明')
  assert.equal(manifest.pub_date, release.published_at)
  assert.equal(manifest.platforms['darwin-aarch64'].url, release.assets[1].browser_download_url)
  assert.equal(manifest.platforms['windows-x86_64'].signature, 'signed-package')
  assert.deepEqual(Object.keys(manifest.downloads), ['windows-x86_64-portable', 'darwin-aarch64-portable'])
  assert.equal(manifest.downloads['windows-x86_64-portable'].fileName, names[2])
  assert.equal(manifest.downloads['darwin-aarch64-portable'].sha256, 'a'.repeat(64))
})

test('a missing platform, missing signature or mismatched version stops publication', () => {
  assert.throws(() => createUpdaterManifest('0.8.0', release, signatures, checksums), /version/)
  assert.throws(() => createUpdaterManifest('0.7.0', { ...release, assets: release.assets.slice(1) }, signatures, checksums), /windows-x86_64/)
  assert.throws(() => createUpdaterManifest('0.7.0', release, { ...signatures, [`${names[1]}.sig`]: '  ' }, checksums), /darwin-aarch64/)
  assert.throws(() => createUpdaterManifest('0.7.0', release, signatures, { ...checksums, [`${names[2]}.sha256`]: 'invalid' }), /windows-x86_64-portable/)
})

test('draft asset URLs are converted to permanent version URLs before publication', () => {
  const draft = {
    ...release,
    assets: release.assets.map((asset) => ({
      ...asset,
      browser_download_url: asset.browser_download_url.replace('/v0.7.0/', '/untagged-draft/'),
    })),
  }
  const manifest = createUpdaterManifest('0.7.0', draft, signatures, checksums)
  assert.equal(manifest.platforms['windows-x86_64'].url, release.assets[0].browser_download_url)
  assert.equal(manifest.platforms['darwin-aarch64'].url, release.assets[1].browser_download_url)
  assert.equal(manifest.downloads['windows-x86_64-portable'].url, release.assets[2].browser_download_url)
  assert.equal(manifest.downloads['darwin-aarch64-portable'].url, release.assets[3].browser_download_url)
})
