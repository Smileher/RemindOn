import { readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

function permanentAssetUrl(asset, release) {
  const downloadUrl = new URL(asset.browser_download_url)
  downloadUrl.pathname = downloadUrl.pathname.replace(
    /\/releases\/download\/[^/]+\//,
    `/releases/download/${encodeURIComponent(release.tag_name)}/`,
  )
  return downloadUrl.href
}

// Publish one complete manifest after all platform builds finish, avoiding concurrent overwrites.
export function createUpdaterManifest(version, release, signatures, checksums) {
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
    // Draft assets use an untagged URL that changes when the release is published.
    platforms[platform] = { url: permanentAssetUrl(asset, release), signature }
  }
  const portablePackages = {
    'windows-x86_64-portable': `RemindOn_${version}_x64_portable.exe`,
    'darwin-aarch64-portable': `RemindOn_${version}_aarch64.dmg`,
  }
  const downloads = {}
  for (const [platform, name] of Object.entries(portablePackages)) {
    const asset = release.assets.find((asset) => asset.name === name)
    const sha256 = checksums[`${name}.sha256`]?.trim().split(/\s+/)[0]?.toLowerCase()
    if (!asset?.browser_download_url || !/^[a-f0-9]{64}$/.test(sha256 ?? '')) {
      throw new Error(`Missing portable download or checksum for ${platform}: ${name}`)
    }
    downloads[platform] = {
      url: permanentAssetUrl(asset, release),
      fileName: name,
      sha256,
    }
  }
  return {
    version,
    notes: release.body ?? '',
    pub_date: release.published_at ?? new Date().toISOString(),
    platforms,
    downloads,
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [version, releasePath, signatureDir, checksumDir, outputPath] = process.argv.slice(2)
  if (!version || !releasePath || !signatureDir || !checksumDir || !outputPath) {
    throw new Error('Usage: create-updater-manifest.mjs <version> <release.json> <signature-dir> <checksum-dir> <output.json>')
  }
  const release = JSON.parse(await readFile(releasePath, 'utf8'))
  const signatures = {}
  for (const asset of release.assets.filter((asset) => asset.name.endsWith('.sig'))) {
    signatures[asset.name] = await readFile(join(signatureDir, asset.name), 'utf8')
  }
  const checksums = {}
  for (const asset of release.assets.filter((asset) => asset.name.endsWith('.sha256'))) {
    checksums[asset.name] = await readFile(join(checksumDir, asset.name), 'utf8')
  }
  const manifest = createUpdaterManifest(version, release, signatures, checksums)
  await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
}
