import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { basename, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const api = 'https://gitee.com/api/v5/repos/smileher/RemindOn'
const requestTimeoutMs = 10 * 60 * 1000
const assetNames = (version, legacyX64) => [
  `RemindOn_${version}_x64-setup.exe`,
  ...(!legacyX64 ? [`RemindOn_${version}_arm64-setup.exe`] : []),
  `RemindOn_${version}_x64_portable.exe`,
  ...(!legacyX64 ? [`RemindOn_${version}_arm64_portable.exe`] : []),
  'RemindOn_aarch64.app.tar.gz',
  `RemindOn_${version}_aarch64.dmg`,
]

async function request(fetchImpl, token, path, { method = 'GET', body } = {}) {
  const url = new URL(`${api}${path}`)
  if (method === 'GET' || method === 'DELETE') url.searchParams.set('access_token', token)
  const response = await fetchImpl(url, { method, body, signal: AbortSignal.timeout(requestTimeoutMs) })
  if (!response.ok) throw new Error(`Gitee API ${method} ${path} returned HTTP ${response.status}`)
  return response.status === 204 ? null : response.json()
}

async function upload(fetchImpl, token, releaseId, name, bytes) {
  console.log(`Uploading Gitee asset: ${name} (${bytes.byteLength} bytes)`)
  const body = new FormData()
  body.set('access_token', token)
  body.set('file', new Blob([bytes], { type: 'application/octet-stream' }), name)
  const asset = await request(fetchImpl, token, `/releases/${releaseId}/attach_files`, { method: 'POST', body })
  if (asset?.name !== name || !asset.browser_download_url) throw new Error(`Gitee did not return a download URL for ${name}`)
  return asset
}

export function createGiteeManifest(githubManifest, giteeAssets) {
  const manifest = structuredClone(githubManifest)
  for (const group of [manifest.platforms, manifest.downloads]) {
    for (const entry of Object.values(group)) {
      const name = basename(new URL(entry.url).pathname)
      const url = giteeAssets.get(name)?.browser_download_url
      if (!url) throw new Error(`Gitee asset is missing: ${name}`)
      entry.url = url
    }
  }
  return manifest
}

export async function syncGiteeRelease({ tag, githubRelease, githubManifest, assetDir, token, legacyX64 = false, fetchImpl = fetch }) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag) || githubRelease.tag_name !== tag || githubManifest.version !== tag.slice(1)) {
    throw new Error('GitHub release, manifest and requested tag do not match')
  }
  if (!token) throw new Error('Configure GITEE_TOKEN in GitHub Actions secrets')
  // Only manual retries may mirror historical releases that predate ARM64 support.
  if (legacyX64 && (githubManifest.platforms?.['windows-aarch64'] || githubManifest.downloads?.['windows-aarch64-portable'])) {
    throw new Error('Legacy x64 mode cannot be used with an ARM64 update manifest')
  }
  const names = assetNames(githubManifest.version, legacyX64)
  for (const name of names) {
    if (!githubRelease.assets.some((asset) => asset.name === name)) throw new Error(`GitHub release is missing: ${name}`)
  }

  const repository = await request(fetchImpl, token, '')
  if (!repository.default_branch) throw new Error('Initialize the Gitee repository with a README before publishing releases')
  const releases = await request(fetchImpl, token, '/releases?per_page=100')
  let release = releases.find((item) => item.tag_name === tag)
  if (!release) {
    const body = new URLSearchParams({
      access_token: token,
      tag_name: tag,
      name: `RemindOn ${tag}`,
      body: githubRelease.body ?? '',
      target_commitish: repository.default_branch,
    })
    release = await request(fetchImpl, token, '/releases', { method: 'POST', body })
  }
  if (!Number.isSafeInteger(release?.id)) throw new Error('Gitee release ID is missing')

  const existing = await request(fetchImpl, token, `/releases/${release.id}/attach_files?per_page=100`)
  const assets = new Map(existing.map((asset) => [asset.name, asset]))
  for (const name of names) {
    const bytes = await readFile(join(assetDir, name))
    if (!assets.has(name)) {
      assets.set(name, await upload(fetchImpl, token, release.id, name, bytes))
    }
    const url = assets.get(name)?.browser_download_url
    if (!url || new URL(url).protocol !== 'https:') throw new Error(`Gitee download URL is missing or invalid: ${name}`)
    const downloaded = await fetchImpl(url, { signal: AbortSignal.timeout(requestTimeoutMs) })
    if (!downloaded.ok) throw new Error(`Gitee anonymous download failed for ${name}: HTTP ${downloaded.status}`)
    const expected = createHash('sha256').update(bytes).digest('hex')
    const actual = createHash('sha256').update(Buffer.from(await downloaded.arrayBuffer())).digest('hex')
    if (actual !== expected) throw new Error(`Gitee asset SHA256 does not match GitHub: ${name}`)
  }
  const manifest = createGiteeManifest(githubManifest, assets)
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
  const previousManifest = assets.get('latest.json')
  if (previousManifest) {
    await request(fetchImpl, token, `/releases/${release.id}/attach_files/${previousManifest.id}`, { method: 'DELETE' })
  }
  const uploadedManifest = await upload(fetchImpl, token, release.id, 'latest.json', manifestBytes)
  const downloadedManifest = await fetchImpl(uploadedManifest.browser_download_url, { signal: AbortSignal.timeout(requestTimeoutMs) })
  if (!downloadedManifest.ok || !manifestBytes.equals(Buffer.from(await downloadedManifest.arrayBuffer()))) {
    throw new Error('Gitee latest.json is not anonymously downloadable or does not match the generated manifest')
  }
  return { releaseId: release.id, assetCount: names.length }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [tag, releasePath, manifestPath, assetDir, option, ...extra] = process.argv.slice(2)
  if (!tag || !releasePath || !manifestPath || !assetDir || (option && option !== '--legacy-x64') || extra.length) {
    throw new Error('Usage: sync-gitee-release.mjs <tag> <release.json> <latest.json> <asset-dir> [--legacy-x64]')
  }
  const githubRelease = JSON.parse(await readFile(releasePath, 'utf8'))
  const githubManifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  const result = await syncGiteeRelease({ tag, githubRelease, githubManifest, assetDir, token: process.env.GITEE_TOKEN, legacyX64: option === '--legacy-x64' })
  console.log(`Synchronized Gitee release ${tag}: ${result.assetCount} binary assets and latest.json`)
}
