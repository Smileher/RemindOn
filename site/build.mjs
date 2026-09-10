import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { locales } from './content.mjs'
import { basePath, origin, renderPage, repository } from './template.mjs'

const siteDir = dirname(fileURLToPath(import.meta.url))
export const outputDir = join(siteDir, 'dist')
export const screenshotNames = ['overview-zh-dark.webp', 'overview-zh-light.webp', 'overview-en-dark.webp', 'overview-en-light.webp']
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
    portable: `RemindOn_${version}_x64_portable.exe`,
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

export async function checkScreenshots(directory, preview = false) {
  const available = []
  for (const name of screenshotNames) {
    try {
      const bytes = await readFile(join(directory, name))
      if (bytes.length < 20 || bytes.toString('ascii', 0, 4) !== 'RIFF' || bytes.toString('ascii', 8, 12) !== 'WEBP' || bytes.readUInt32LE(4) !== bytes.length - 8) {
        throw new Error(`Screenshot is not a valid WebP: ${name}`)
      }
      available.push(name)
    } catch (error) {
      if (error.code !== 'ENOENT') throw error
    }
  }
  // Only local preview may use placeholders; production must have all four images.
  const missing = screenshotNames.filter((name) => !available.includes(name))
  if (missing.length && !preview) throw new Error(`Add screenshots before deployment: ${missing.join(', ')}`)
  return available
}

export async function buildSite({ preview = false } = {}) {
  const screenshotDir = join(siteDir, 'assets', 'screenshots')
  const screenshots = await checkScreenshots(screenshotDir, preview)
  const release = await fetchLatestRelease()
  await mkdir(join(outputDir, 'assets', 'screenshots'), { recursive: true })
  await copyFile(join(siteDir, '..', 'src', 'assets', 'remindon.svg'), join(outputDir, 'assets', 'remindon.svg'))
  await copyFile(join(siteDir, '..', 'src-tauri', 'icons', 'icon.png'), join(outputDir, 'assets', 'share-icon.png'))
  await copyFile(join(siteDir, 'style.css'), join(outputDir, 'assets', 'style.css'))
  await copyFile(join(siteDir, 'theme.js'), join(outputDir, 'assets', 'theme.js'))
  for (const name of screenshots) await copyFile(join(screenshotDir, name), join(outputDir, 'assets', 'screenshots', name))
  for (const [language, { path }] of Object.entries(locales)) {
    await mkdir(join(outputDir, path), { recursive: true })
    await writeFile(join(outputDir, path, 'index.html'), renderPage(language, release, screenshots), 'utf8')
  }
  const urls = Object.values(locales).map(({ path }) => `${origin}${basePath}${path}`)
  await writeFile(join(outputDir, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls.map((url) => `<url><loc>${url}</loc></url>`).join('')}</urlset>\n`, 'utf8')
  await writeFile(join(outputDir, '.nojekyll'), '', 'utf8')
  console.log(`Built RemindOn v${release.version}: ${outputDir}${preview ? ' (local preview; placeholders allowed)' : ''}`)
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const args = process.argv.slice(2)
  if (args.some((arg) => arg !== '--preview')) throw new Error('Usage: node site/build.mjs [--preview]')
  await buildSite({ preview: args.includes('--preview') })
}
