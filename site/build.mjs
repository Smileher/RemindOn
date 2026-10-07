import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { locales } from './content.mjs'
import { basePath, origin, renderPage, repository } from './template.mjs'

const siteDir = dirname(fileURLToPath(import.meta.url))
export const outputDir = join(siteDir, 'dist')
const latestReleaseApi = 'https://api.github.com/repos/Smileher/RemindOn/releases/latest'

// Read one release snapshot so the version and every download belong together.
export function parseRelease(release) {
  const version = /^v(\d+\.\d+\.\d+)$/.exec(release?.tag_name)?.[1]
  if (!version || release.draft !== false || release.prerelease !== false) {
    throw new Error('Expected a published stable release with a vX.Y.Z tag')
  }
  if (typeof release.published_at !== 'string' || !Number.isFinite(Date.parse(release.published_at))) {
    throw new Error('Release publication date is missing or invalid')
  }
  if (release.html_url !== `${repository}/releases/tag/${release.tag_name}` || !Array.isArray(release.assets)) {
    throw new Error('Release URL or assets are invalid')
  }
  const packages = {
    windows: `RemindOn_${version}_x64-setup.exe`,
    windowsArm64: `RemindOn_${version}_arm64-setup.exe`,
    portable: `RemindOn_${version}_x64_portable.zip`,
    portableArm64: `RemindOn_${version}_arm64_portable.zip`,
    mac: `RemindOn_${version}_aarch64.dmg`,
  }
  const downloads = {}
  for (const [platform, name] of Object.entries(packages)) {
    const matches = release.assets.filter((asset) => asset.name === name)
    const asset = matches[0]
    if (matches.length !== 1 || asset.browser_download_url !== `${repository}/releases/download/${release.tag_name}/${name}` || !Number.isSafeInteger(asset.size) || asset.size <= 0) {
      throw new Error(`Missing or invalid release asset: ${name}`)
    }
    downloads[platform] = { url: asset.browser_download_url, size: asset.size }
  }
  return { version, publishedAt: release.published_at, url: release.html_url, downloads }
}

export async function fetchLatestRelease(fetchImpl = fetch) {
  const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'RemindOn-Pages', 'X-GitHub-Api-Version': '2022-11-28' }
  if (process.env.GH_TOKEN) headers.Authorization = `Bearer ${process.env.GH_TOKEN}`
  const response = await fetchImpl(latestReleaseApi, { headers, signal: AbortSignal.timeout(20000) })
  if (!response.ok) throw new Error(`GitHub release request failed (HTTP ${response.status})`)
  return parseRelease(await response.json())
}

export async function buildSite() {
  const release = await fetchLatestRelease()
  await mkdir(join(outputDir, 'assets'), { recursive: true })
  await copyFile(join(siteDir, '..', 'src', 'assets', 'remindon.svg'), join(outputDir, 'assets', 'remindon.svg'))
  await copyFile(join(siteDir, '..', 'src-tauri', 'icons', 'icon.png'), join(outputDir, 'assets', 'share-icon.png'))
  await copyFile(join(siteDir, 'style.css'), join(outputDir, 'assets', 'style.css'))
  await copyFile(join(siteDir, 'theme.js'), join(outputDir, 'assets', 'theme.js'))
  for (const [language, { path }] of Object.entries(locales)) {
    await mkdir(join(outputDir, path), { recursive: true })
    await writeFile(join(outputDir, path, 'index.html'), renderPage(language, release), 'utf8')
  }
  const urls = Object.values(locales).map(({ path }) => `${origin}${basePath}${path}`)
  await writeFile(join(outputDir, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls.map((url) => `<url><loc>${url}</loc></url>`).join('')}</urlset>\n`, 'utf8')
  await writeFile(join(outputDir, '.nojekyll'), '', 'utf8')
  console.log(`Built RemindOn v${release.version}: ${outputDir}`)
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  await buildSite()
}
