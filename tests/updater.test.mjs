import assert from 'node:assert/strict'
import { beforeEach, mock, test } from 'node:test'

const invoke = mock.fn()
const openUrl = mock.fn()
const listen = mock.fn()
const unlistenStatus = mock.fn()
const unlistenProgress = mock.fn()
let statusListener
let progressListener

mock.module('@tauri-apps/api/core', { namedExports: { invoke } })
mock.module('@tauri-apps/api/event', { namedExports: { listen } })
mock.module('@tauri-apps/plugin-opener', { namedExports: { openUrl } })
const { useUpdater } = await import('../src/composables/useUpdater.ts')

const idleState = { phase: 'idle', version: null, error: null }

beforeEach(() => {
  for (const fn of [invoke, openUrl, listen, unlistenStatus, unlistenProgress]) fn.mock.resetCalls()
  statusListener = undefined
  progressListener = undefined
  invoke.mock.mockImplementation(async (command) => {
    if (command === 'get_update_mode') return 'installed'
    if (command === 'get_update_status') return idleState
    if (command === 'get_update_progress') return null
    if (command === 'check_for_updates') return idleState
  })
  openUrl.mock.mockImplementation(async () => {})
  listen.mock.mockImplementation(async (event, listener) => {
    if (event === 'update-status') {
      statusListener = listener
      return unlistenStatus
    }
    progressListener = listener
    return unlistenProgress
  })
})

test('loads Rust update status and progress without contacting the updater plugin', async () => {
  const updater = useUpdater()
  await updater.loadStatus()
  assert.equal(updater.mode.value, 'installed')
  assert.equal(updater.status.value, 'upToDate')
  assert.equal(invoke.mock.calls.filter((call) => call.arguments[0] === 'check_for_updates').length, 0)
  assert.equal(listen.mock.callCount(), 2)
})

test('manual checks call the Rust service and expose an available version', async () => {
  invoke.mock.mockImplementation(async (command, args) => {
    if (command === 'get_update_mode') return 'portable'
    if (command === 'check_for_updates') {
      assert.deepEqual(args, { force: true })
      return { phase: 'available', version: '0.8.0', error: null }
    }
    if (command === 'get_update_progress') return null
  })
  const updater = useUpdater()
  await updater.checkForUpdates()
  assert.equal(updater.mode.value, 'portable')
  assert.equal(updater.status.value, 'available')
  assert.equal(updater.newVersion.value, '0.8.0')
})

test('background checks use the same Rust command without showing failures', async () => {
  invoke.mock.mockImplementation(async (command, args) => {
    if (command === 'get_update_mode') return 'installed'
    if (command === 'check_for_updates') {
      assert.deepEqual(args, { force: false })
      throw new Error('offline')
    }
  })
  const updater = useUpdater()
  await updater.checkForUpdates(true)
  assert.equal(updater.status.value, 'idle')
  assert.equal(updater.errorMessage.value, '')
})

test('manual failures show an error and can be retried', async () => {
  let failed = true
  invoke.mock.mockImplementation(async (command) => {
    if (command === 'get_update_mode') return 'installed'
    if (command === 'check_for_updates') {
      if (failed) throw new Error('offline')
      return idleState
    }
    if (command === 'get_update_progress') return null
  })
  const updater = useUpdater()
  await updater.checkForUpdates()
  assert.equal(updater.status.value, 'error')
  assert.equal(updater.errorMessage.value, 'update-failed')
  failed = false
  await updater.checkForUpdates()
  assert.equal(updater.status.value, 'upToDate')
  assert.equal(updater.errorMessage.value, '')
})

test('paused automatic updates remain manually retryable', async () => {
  invoke.mock.mockImplementation(async (command, args) => {
    if (command === 'get_update_mode') return 'installed'
    if (command === 'get_update_status') return { phase: 'error', version: '1.3.2', error: 'automatic-update-paused' }
    if (command === 'get_update_progress') return null
    if (command === 'check_for_updates') {
      assert.deepEqual(args, { force: true })
      return idleState
    }
  })
  const updater = useUpdater()
  await updater.loadStatus()
  assert.equal(updater.errorMessage.value, 'automatic-update-paused')
  assert.equal(updater.newVersion.value, '1.3.2')
  assert.equal(updater.busy.value, false)
  await updater.checkForUpdates()
  assert.equal(updater.status.value, 'upToDate')
  assert.equal(updater.errorMessage.value, '')
})

test('development and unsupported copies do not invoke the Rust check', async () => {
  for (const mode of ['development', 'unsupported']) {
    invoke.mock.mockImplementation(async (command) => command === 'get_update_mode' ? mode : undefined)
    const updater = useUpdater()
    await updater.checkForUpdates()
    assert.equal(updater.mode.value, mode)
    assert.equal(updater.status.value, 'idle')
  }
  assert.equal(invoke.mock.calls.filter((call) => call.arguments[0] === 'check_for_updates').length, 0)
})

test('Rust events update an already-open window while a download is running', async () => {
  const updater = useUpdater()
  await updater.loadStatus()
  statusListener({ payload: { phase: 'downloading', version: '0.8.0', error: null } })
  progressListener({ payload: { downloadedBytes: 40, totalBytes: 100, percentage: 40 } })
  assert.equal(updater.status.value, 'downloading')
  assert.equal(updater.newVersion.value, '0.8.0')
  assert.equal(updater.progress.value, 40)
})

test('dispose removes Rust event listeners so a destroyed window stays detached', async () => {
  const updater = useUpdater()
  await updater.loadStatus()
  updater.dispose()
  await Promise.resolve()
  assert.equal(unlistenStatus.mock.callCount(), 1)
  assert.equal(unlistenProgress.mock.callCount(), 1)
  statusListener({ payload: { phase: 'available', version: '0.8.0', error: null } })
  assert.equal(updater.newVersion.value, '')
})

test('manual downloads open the localized website and expose open errors', async () => {
  const updater = useUpdater()
  await updater.openReleases()
  assert.deepEqual(openUrl.mock.calls[0].arguments, ['https://smileher.github.io/RemindOn/#download'])
  await updater.openReleases('en')
  assert.deepEqual(openUrl.mock.calls[1].arguments, ['https://smileher.github.io/RemindOn/en/#download'])
  openUrl.mock.mockImplementation(async () => { throw new Error('browser unavailable') })
  await updater.openReleases()
  assert.equal(updater.errorMessage.value, 'update-failed')
})

test('website links follow the language and Store updates launch the native app', async () => {
  const updater = useUpdater()
  await updater.openAuthorPage('en')
  assert.deepEqual(openUrl.mock.calls[0].arguments, ['https://smileher.github.io/RemindOn/en/'])
  await updater.openStoreUpdates()
  assert.deepEqual(openUrl.mock.calls[1].arguments, ['ms-windows-store://pdp/?productid=9P9K31N2CJBW'])
  openUrl.mock.mockImplementation(async () => { throw new Error('Store unavailable') })
  await updater.openStoreUpdates()
  assert.equal(updater.errorMessage.value, 'store-open-failed')
})
