import { readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

// Publish one complete manifest after all platform builds finish, avoiding concurrent overwrites.
export function createUpdaterManifest(version, release, signatures) {
  if (release.tag_name !== `v${version}`) throw new Error('Release tag does not match the application version')
  const packages = {
    'windows-x86_64': `RemindOn_${version}_x64-setup.exe`,
    'darwin-aarch64': 'RemindOn_aarch64.app.tar.gz',
  }
  const platforms = {}
  for (const [platform, name] of Object.entries(packages)) {
    const asset = release.assets.find((asset) => asset.name === name)
    const signature = signatures[`${name}.sig`]?.trim()
    if (!asset?.browser_download_url || !signature) {
      throw new Error(`Missing signed update package for ${platform}: ${name}`)
    }
    platforms[platform] = { url: asset.browser_download_url, signature }
  }
  return {
    version,
    notes: release.body ?? '',
    pub_date: release.published_at ?? new Date().toISOString(),
    platforms,
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [version, releasePath, signatureDir, outputPath] = process.argv.slice(2)
  if (!version || !releasePath || !signatureDir || !outputPath) {
    throw new Error('Usage: create-updater-manifest.mjs <version> <release.json> <signature-dir> <output.json>')
  }
  const release = JSON.parse(await readFile(releasePath, 'utf8'))
  const signatures = {}
  for (const asset of release.assets.filter((asset) => asset.name.endsWith('.sig'))) {
    signatures[asset.name] = await readFile(join(signatureDir, asset.name), 'utf8')
  }
  const manifest = createUpdaterManifest(version, release, signatures)
  await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
}
