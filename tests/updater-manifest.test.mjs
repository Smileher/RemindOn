import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createUpdaterManifest } from '../scripts/create-updater-manifest.mjs'

const names = ['RemindOn_0.6.0_x64-setup.exe', 'RemindOn_aarch64.app.tar.gz']
const release = {
  tag_name: 'v0.6.0',
  body: '更新说明',
  published_at: '2026-09-10T00:00:00Z',
  assets: names.map((name) => ({ name, browser_download_url: `https://github.com/Smileher/RemindOn/releases/download/v0.6.0/${name}` })),
}
const signatures = Object.fromEntries(names.map((name) => [`${name}.sig`, 'signed-package\n']))

test('one manifest contains the signed Windows and Apple Silicon packages', () => {
  const manifest = createUpdaterManifest('0.6.0', release, signatures)
  assert.deepEqual(Object.keys(manifest.platforms), ['windows-x86_64', 'darwin-aarch64'])
  assert.equal(manifest.version, '0.6.0')
  assert.equal(manifest.notes, '更新说明')
  assert.equal(manifest.pub_date, release.published_at)
  assert.equal(manifest.platforms['darwin-aarch64'].url, release.assets[1].browser_download_url)
  assert.equal(manifest.platforms['windows-x86_64'].signature, 'signed-package')
})

test('a missing platform, missing signature or mismatched version stops publication', () => {
  assert.throws(() => createUpdaterManifest('0.7.0', release, signatures), /version/)
  assert.throws(() => createUpdaterManifest('0.6.0', { ...release, assets: release.assets.slice(1) }, signatures), /windows-x86_64/)
  assert.throws(() => createUpdaterManifest('0.6.0', release, { ...signatures, [`${names[1]}.sig`]: '  ' }), /darwin-aarch64/)
})
