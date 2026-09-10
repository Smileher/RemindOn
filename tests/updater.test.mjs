import assert from 'node:assert/strict'
import { beforeEach, mock, test } from 'node:test'

const invoke = mock.fn()
const check = mock.fn()
const relaunch = mock.fn()
const openUrl = mock.fn()
const revealItemInDir = mock.fn()
const listen = mock.fn()
const unlistenProgress = mock.fn()
let progressListener
mock.module('@tauri-apps/api/core', { namedExports: { invoke } })
mock.module('@tauri-apps/api/event', { namedExports: { listen } })
mock.module('@tauri-apps/plugin-updater', { namedExports: { check } })
mock.module('@tauri-apps/plugin-process', { namedExports: { relaunch } })
mock.module('@tauri-apps/plugin-opener', { namedExports: { openUrl, revealItemInDir } })
const { useUpdater } = await import('../src/composables/useUpdater.ts')

beforeEach(() => {
  for (const fn of [invoke, check, relaunch, openUrl, revealItemInDir, listen, unlistenProgress]) fn.mock.resetCalls()
  progressListener = undefined
  invoke.mock.mockImplementation(async (command) => command === 'get_update_mode' ? 'installed' : undefined)
  check.mock.mockImplementation(async () => null)
  relaunch.mock.mockImplementation(async () => {})
  openUrl.mock.mockImplementation(async () => {})
  revealItemInDir.mock.mockImplementation(async () => {})
  listen.mock.mockImplementation(async (_event, listener) => {
    progressListener = listener
    return unlistenProgress
  })
})

test('portable copies check for updates but never run the native installer', async () => {
  const downloadAndInstall = mock.fn()
  invoke.mock.mockImplementation(async (command) => command === 'get_update_mode' ? 'portable' : undefined)
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close: async () => {}, downloadAndInstall }))
  const updater = useUpdater()
  await updater.checkForUpdates()
  await updater.installUpdate()
  assert.equal(updater.mode.value, 'portable')
  assert.equal(updater.status.value, 'available')
  assert.equal(check.mock.callCount(), 1)
  assert.equal(downloadAndInstall.mock.callCount(), 0)
  assert.equal(relaunch.mock.callCount(), 0)
})

test('development and unsupported copies never contact the update endpoint', async () => {
  for (const mode of ['development', 'unsupported']) {
    invoke.mock.mockImplementation(async () => mode)
    const updater = useUpdater()
    await updater.checkForUpdates()
    await updater.installUpdate()
    assert.equal(updater.mode.value, mode)
    assert.equal(updater.status.value, 'idle')
  }
  assert.equal(check.mock.callCount(), 0)
})

test('mode detection failures cannot enable native updates', async () => {
  invoke.mock.mockImplementation(async () => { throw new Error('mode unavailable') })
  const updater = useUpdater()
  await updater.checkForUpdates()
  assert.equal(check.mock.callCount(), 0)
  assert.equal(updater.status.value, 'error')
  assert.match(updater.errorMessage.value, /mode unavailable/)
})

test('concurrent checks are ignored, including during mode detection', async () => {
  let resolveMode
  invoke.mock.mockImplementation(() => new Promise((resolve) => { resolveMode = resolve }))
  const updater = useUpdater()
  const first = updater.checkForUpdates()
  await updater.checkForUpdates()
  resolveMode('installed')
  await first
  assert.equal(invoke.mock.callCount(), 1)
  assert.equal(check.mock.callCount(), 1)
  assert.equal(updater.status.value, 'upToDate')
})

test('automatic failures are quiet; manual failures show errors and can retry', async () => {
  check.mock.mockImplementation(async () => { throw new Error('offline') })
  const updater = useUpdater()
  await updater.checkForUpdates(true)
  assert.equal(updater.status.value, 'idle')
  assert.equal(updater.errorMessage.value, '')
  await updater.checkForUpdates()
  assert.equal(updater.status.value, 'error')
  assert.match(updater.errorMessage.value, /offline/)
  check.mock.mockImplementation(async () => null)
  await updater.checkForUpdates()
  assert.equal(updater.status.value, 'upToDate')
  assert.equal(updater.errorMessage.value, '')
})

test('rechecking releases old native resources and clears a stale update', async () => {
  const close = mock.fn(async () => {})
  const downloadAndInstall = mock.fn()
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close, downloadAndInstall }))
  const updater = useUpdater()
  await updater.checkForUpdates()
  assert.equal(downloadAndInstall.mock.callCount(), 0)
  assert.equal(updater.newVersion.value, '0.7.0')
  check.mock.mockImplementation(async () => null)
  await updater.checkForUpdates()
  assert.equal(close.mock.callCount(), 1)
  assert.equal(updater.newVersion.value, '')
  await updater.installUpdate()
  assert.equal(downloadAndInstall.mock.callCount(), 0)
})

test('a check finishing after unmount closes its native resource', async () => {
  let resolveCheck
  const close = mock.fn(async () => {})
  check.mock.mockImplementation(() => new Promise((resolve) => { resolveCheck = resolve }))
  const updater = useUpdater()
  const checking = updater.checkForUpdates()
  await Promise.resolve()
  updater.dispose()
  resolveCheck({ version: '0.7.0', close })
  await checking
  assert.equal(close.mock.callCount(), 1)
  assert.equal(updater.newVersion.value, '')
})

test('failed downloads never restart and can be retried without duplicate installation', async () => {
  let rejectDownload
  const downloadAndInstall = mock.fn(() => new Promise((_, reject) => { rejectDownload = reject }))
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close: async () => {}, downloadAndInstall }))
  const updater = useUpdater()
  await updater.checkForUpdates()
  const first = updater.installUpdate()
  await updater.installUpdate()
  await updater.checkForUpdates()
  assert.equal(downloadAndInstall.mock.callCount(), 1)
  assert.equal(check.mock.callCount(), 1)
  rejectDownload(new Error('invalid signature'))
  await first
  assert.equal(relaunch.mock.callCount(), 0)
  assert.equal(updater.status.value, 'error')
  downloadAndInstall.mock.mockImplementation(async (onEvent) => {
    onEvent({ event: 'Started', data: { contentLength: 100 } })
    onEvent({ event: 'Progress', data: { chunkLength: 130 } })
    assert.equal(updater.progress.value, 100)
    onEvent({ event: 'Finished' })
  })
  await updater.installUpdate()
  assert.equal(downloadAndInstall.mock.callCount(), 2)
  assert.equal(relaunch.mock.callCount(), 1)
})

test('unknown download lengths stay indeterminate and restart failure is retryable', async () => {
  const downloadAndInstall = mock.fn(async (onEvent) => {
    onEvent({ event: 'Started', data: {} })
    onEvent({ event: 'Progress', data: { chunkLength: 20 } })
  })
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close: async () => {}, downloadAndInstall }))
  relaunch.mock.mockImplementation(async () => { throw new Error('restart failed') })
  const updater = useUpdater()
  await updater.checkForUpdates()
  await updater.installUpdate()
  assert.equal(updater.progress.value, null)
  assert.equal(updater.status.value, 'ready')
  await updater.installUpdate()
  assert.equal(downloadAndInstall.mock.callCount(), 1)
  await updater.restartApp()
  assert.equal(relaunch.mock.callCount(), 2)
})

test('portable downloads report progress, never restart and reveal the completed file', async () => {
  const downloadAndInstall = mock.fn()
  const downloadedPath = 'C:\\Users\\Test\\Downloads\\RemindOn_0.7.0_x64_portable.exe'
  invoke.mock.mockImplementation(async (command, args) => {
    if (command === 'get_update_mode') return 'portable'
    if (command === 'download_portable_update') {
      assert.deepEqual(args, { expectedVersion: '0.7.0' })
      progressListener({ payload: { downloadedBytes: 40, totalBytes: 100, percentage: 40 } })
      return downloadedPath
    }
  })
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close: async () => {}, downloadAndInstall }))
  const updater = useUpdater()
  await updater.checkForUpdates()
  await updater.downloadPortableUpdate()
  assert.equal(updater.status.value, 'downloaded')
  assert.equal(updater.progress.value, 100)
  assert.equal(updater.downloadedPath.value, downloadedPath)
  assert.equal(downloadAndInstall.mock.callCount(), 0)
  assert.equal(relaunch.mock.callCount(), 0)
  assert.equal(unlistenProgress.mock.callCount(), 1)

  await updater.revealDownloadedUpdate()
  assert.deepEqual(revealItemInDir.mock.calls[0].arguments, [downloadedPath])
})

test('failed portable downloads can be retried and always remove the progress listener', async () => {
  let attempts = 0
  invoke.mock.mockImplementation(async (command) => {
    if (command === 'get_update_mode') return 'portable'
    if (command === 'download_portable_update') {
      attempts += 1
      if (attempts === 1) throw new Error('checksum mismatch')
      return 'C:\\Users\\Test\\Downloads\\RemindOn_0.7.0_x64_portable.exe'
    }
  })
  check.mock.mockImplementation(async () => ({ version: '0.7.0', close: async () => {} }))
  const updater = useUpdater()
  await updater.checkForUpdates()
  await updater.downloadPortableUpdate()
  assert.equal(updater.status.value, 'error')
  assert.match(updater.errorMessage.value, /checksum mismatch/)
  await updater.downloadPortableUpdate()
  assert.equal(updater.status.value, 'downloaded')
  assert.equal(unlistenProgress.mock.callCount(), 2)
})

test('manual downloads open only the project release page and expose open errors', async () => {
  const updater = useUpdater()
  await updater.openReleases()
  assert.deepEqual(openUrl.mock.calls[0].arguments, ['https://github.com/Smileher/RemindOn/releases/latest'])
  openUrl.mock.mockImplementation(async () => { throw new Error('browser unavailable') })
  await updater.openReleases()
  assert.match(updater.errorMessage.value, /browser unavailable/)
})
