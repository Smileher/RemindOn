import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import https from 'node:https'
import { basename, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const api = 'https://gitee.com/api/v5/repos/smileher/RemindOn'
// Gitee 会掐断长时间的上传连接；超时过长只会让整个 job 挂到 GitHub 的6 小时上限。
const requestTimeoutMs = 3 * 60 * 1000
const apiTimeoutMs = 60 * 1000
// socket timeout 只检测空闲连接；上传持续有数据但迟迟不结束时仍会无限等待。
const uploadDeadlineMs = 8 * 60 * 1000
// 整轮镜像的总预算，防止个别资产反复重试把 workflow拖死。
const syncDeadlineMs = 25 * 60 * 1000
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
  // 只自动重试查询；写请求由上传恢复逻辑处理，避免重复创建 Release 或附件。
  const attempts = method === 'GET' ? 3 : 1
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    let retryable = true
    try {
      const response = await fetchImpl(url, { method, body, signal: AbortSignal.timeout(apiTimeoutMs) })
      if (!response.ok) {
        retryable = response.status === 408 || response.status === 429 || response.status >= 500
        throw new Error(`Gitee API ${method} ${path} returned HTTP ${response.status}`)
      }
      return response.status === 204 ? null : await response.json()
    } catch (error) {
      if (!retryable || attempt === attempts) throw error
      console.warn(`Retrying Gitee API ${method} ${path} (attempt ${attempt}/${attempts}): ${error.message}`)
      await new Promise((resolve) => setTimeout(resolve, attempt * 5000))
    }
  }
}

async function downloadWithRetry(fetchImpl, url, name) {
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      const response = await fetchImpl(url, { signal: AbortSignal.timeout(requestTimeoutMs) })
      if (!response.ok) throw new Error(`Gitee anonymous download failed for ${name}: HTTP ${response.status}`)
      return Buffer.from(await response.arrayBuffer())
    } catch (error) {
      if (attempt === 3) throw error
      console.warn(`Retrying Gitee download ${name} (attempt ${attempt}/3): ${error.message}`)
      await new Promise((resolve) => setTimeout(resolve, attempt * 5000))
    }
  }
}

async function upload(fetchImpl, token, releaseId, name, bytes) {
  console.log(`Uploading Gitee asset: ${name} (${bytes.byteLength} bytes)`)
  if (fetchImpl === fetch) {
    return uploadWithHttps(token, releaseId, name, bytes)
  }
  const body = new FormData()
  body.set('access_token', token)
  body.set('file', new Blob([bytes], { type: 'application/octet-stream' }), name)
  const asset = await request(fetchImpl, token, `/releases/${releaseId}/attach_files`, { method: 'POST', body })
  if (asset?.name !== name || !asset.browser_download_url) throw new Error(`Gitee did not return a download URL for ${name}`)
  return asset
}

async function uploadWithRetry(fetchImpl, token, releaseId, name, bytes) {
  let lastError
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      return await upload(fetchImpl, token, releaseId, name, bytes)
    } catch (error) {
      lastError = error
      // The upload may have completed before the connection was reset.
      try {
        const existing = await request(fetchImpl, token, `/releases/${releaseId}/attach_files?per_page=100`)
        const uploaded = existing.find((asset) => asset.name === name)
        if (uploaded) {
          const downloaded = await downloadWithRetry(fetchImpl, uploaded.browser_download_url, name)
          const actual = createHash('sha256').update(downloaded).digest('hex')
          const expected = createHash('sha256').update(bytes).digest('hex')
          if (actual === expected) return uploaded
          await request(fetchImpl, token, `/releases/${releaseId}/attach_files/${uploaded.id}`, { method: 'DELETE' })
        }
      } catch {
        // Keep the original upload error if the recovery lookup also fails.
      }
      if (attempt < 3) {
        console.warn(`Retrying Gitee asset ${name} after upload error (attempt ${attempt}/3): ${error.message}`)
        await new Promise((resolve) => setTimeout(resolve, attempt * 5000))
      }
    }
  }
  throw lastError
}

function uploadWithHttps(token, releaseId, name, bytes) {
  const boundary = `----RemindOn-${Date.now().toString(36)}`
  const prefix = Buffer.from(
    `--${boundary}\r\nContent-Disposition: form-data; name="access_token"\r\n\r\n${token}\r\n` +
      `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${name.replace(/"/g, '')}"\r\n` +
      'Content-Type: application/octet-stream\r\n\r\n',
    'utf8',
  )
  const suffix = Buffer.from(`\r\n--${boundary}--\r\n`, 'utf8')
  const body = Buffer.concat([prefix, Buffer.from(bytes), suffix])
  const url = new URL(`${api}/releases/${releaseId}/attach_files`)
  return new Promise((resolve, reject) => {
    let settled = false
    const finish = (error, asset) => {
      if (settled) return
      settled = true
      clearTimeout(deadline)
      if (error) reject(error)
      else resolve(asset)
    }
    const request = https.request(url, {
      method: 'POST',
      headers: {
        'Content-Type': `multipart/form-data; boundary=${boundary}`,
        'Content-Length': body.length,
      },
      timeout: requestTimeoutMs,
    }, (response) => {
      const chunks = []
      response.on('error', (error) => finish(error))
      response.on('aborted', () => finish(new Error(`Gitee upload response aborted: ${name}`)))
      response.on('data', (chunk) => chunks.push(chunk))
      response.on('end', () => {
        const text = Buffer.concat(chunks).toString('utf8')
        if (response.statusCode < 200 || response.statusCode >= 300) {
          finish(new Error(`Gitee API POST /releases/${releaseId}/attach_files returned HTTP ${response.statusCode}`))
          return
        }
        try {
          const asset = JSON.parse(text)
          if (asset?.name !== name || !asset.browser_download_url) throw new Error(`Gitee did not return a download URL for ${name}`)
          finish(null, asset)
        } catch (error) {
          finish(error)
        }
      })
    })
    request.on('timeout', () => request.destroy(new Error(`Gitee upload timed out: ${name}`)))
    request.on('error', (error) => finish(error))
    const deadline = setTimeout(() => {
      request.destroy(new Error(`Gitee upload exceeded ${uploadDeadlineMs / 60000} minute deadline: ${name}`))
    }, uploadDeadlineMs)
    request.end(body)
  })
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
  // 并发镜像：单个资产超时或失败不再阻塞其余资产，避免一个卡住整轮。
  const outcomes = await Promise.allSettled(names.map(async (name) => {
    const bytes = await readFile(join(assetDir, name))
    if (!assets.has(name)) {
      assets.set(name, await uploadWithRetry(fetchImpl, token, release.id, name, bytes))
    }
    const url = assets.get(name)?.browser_download_url
    if (!url || new URL(url).protocol !== 'https:') throw new Error(`Gitee download URL is missing or invalid: ${name}`)
    console.log(`Verifying Gitee asset: ${name}`)
    let downloaded = await downloadWithRetry(fetchImpl, url, name)
    const expected = createHash('sha256').update(bytes).digest('hex')
    let actual = createHash('sha256').update(downloaded).digest('hex')
    if (actual !== expected) {
      // 修复上次中断留下的同名附件；仍以 GitHub 文件的 SHA256 为准。
      console.warn(`Replacing corrupt Gitee asset: ${name}`)
      await request(fetchImpl, token, `/releases/${release.id}/attach_files/${assets.get(name).id}`, { method: 'DELETE' })
      assets.set(name, await uploadWithRetry(fetchImpl, token, release.id, name, bytes))
      downloaded = await downloadWithRetry(fetchImpl, assets.get(name).browser_download_url, name)
      actual = createHash('sha256').update(downloaded).digest('hex')
    }
    if (actual !== expected) throw new Error(`Gitee asset SHA256 does not match GitHub: ${name}`)
    console.log(`Verified Gitee asset: ${name}`)
    return name
  }))
  const failed = outcomes.filter((outcome) => outcome.status === 'rejected')
  for (const outcome of failed) console.error(`Gitee asset failed: ${outcome.reason.message}`)
  if (failed.length) throw new Error(`${failed.length} of ${names.length} Gitee assets failed to mirror`)
  const manifest = createGiteeManifest(githubManifest, assets)
  const manifestBytes = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
  const previousManifest = assets.get('latest.json')
  if (previousManifest) {
    await request(fetchImpl, token, `/releases/${release.id}/attach_files/${previousManifest.id}`, { method: 'DELETE' })
  }
  const uploadedManifest = await uploadWithRetry(fetchImpl, token, release.id, 'latest.json', manifestBytes)
  const downloadedManifest = await downloadWithRetry(fetchImpl, uploadedManifest.browser_download_url, 'latest.json')
  if (!manifestBytes.equals(downloadedManifest)) {
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
  // 整轮镜像设硬性上限：Gitee 不可用时尽快失败，让 workflow 报错而不是挂到 6 小时上限。
  const hardStop = setTimeout(() => {
    console.error(`Gitee mirror exceeded ${syncDeadlineMs} ms budget; aborting.`)
    process.exit(3)
  }, syncDeadlineMs + 60_000)
  hardStop.unref()
  const result = await syncGiteeRelease({ tag, githubRelease, githubManifest, assetDir, token: process.env.GITEE_TOKEN, legacyX64: option === '--legacy-x64' })
  clearTimeout(hardStop)
  console.log(`Synchronized Gitee release ${tag}: ${result.assetCount} binary assets and latest.json`)
}
