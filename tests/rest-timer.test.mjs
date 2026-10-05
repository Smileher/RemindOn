import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { renderToString } from 'vue/server-renderer'
import { defaultData, defaultReminders } from '../src/types.ts'
import { translate } from '../src/i18n.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { compileScript, compileTemplate, parse } = vueRequire('@vue/compiler-sfc')
// Run the component's actual handlers and template with only native bridges stubbed.
const { descriptor } = parse(readFileSync(new URL('../src/App.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'rest-timer-test' })
const template = compileTemplate({
  id: 'rest-timer-test',
  source: descriptor.template.content,
  filename: 'App.vue',
  compilerOptions: { bindingMetadata: script.bindings },
})
assert.deepEqual(template.errors, [])

function compile(source) {
  return ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText
}

const scriptCode = compile(script.content)
const templateCode = compile(template.code)
const noop = () => {}
const resting = { isResting: true, nextTriggerAt: null }

async function mountApp({ enabled = true, status = resting, nativeErrors = { autostartError: null, notificationError: null } } = {}) {
  const settingsData = defaultData()
  settingsData.settings.restEnabled = enabled
  settingsData.settings.restIntervalMinutes = 1
  const listeners = new Map()
  const windowListeners = new Map()
  const calls = []
  let mounted
  let focused
  let hideCount = 0
  let readStatus = async () => status
  let saveData = async (value) => structuredClone(value)
  let confirmReset = async () => false
  const invokeHandlers = new Map()
  const nativeHandlers = new Map()
  const errors = []
  function emitTo(target, name, payload) {
    const listener = listeners.get(name)
    if (listener && (listener.target === null || listener.target === target)) {
      return listener.callback({ payload })
    }
  }
  const modules = {
    vue: { ...vue, onMounted: (callback) => { mounted = callback }, onUnmounted: noop },
    '@tauri-apps/api/app': { getVersion: async () => '0.9.0', setTheme: async (theme) => nativeHandlers.get('setTheme')?.(theme) },
    '@tauri-apps/api/core': {
      invoke: async (command, args) => {
        calls.push(command)
        if (invokeHandlers.has(command)) return invokeHandlers.get(command)(args)
        if (command === 'load_data') return structuredClone(settingsData)
        if (command === 'get_native_errors') return nativeErrors
        if (command === 'save_data') return saveData(JSON.parse(JSON.stringify(args.data)), args)
        if (command === 'read_popup_image') return null
        if (command === 'clear_popup_image') return
        if (command === 'hide_idle_window') { hideCount += 1; return }
        if (command === 'get_rest_timer_status') {
          assert.ok(listeners.has('rest-timer-updated'), 'subscribe before reading initial status')
          return readStatus()
        }
        throw new Error(`Unexpected command: ${command}`)
      },
    },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => { listeners.set(name, { target: null, callback }); return noop },
    },
    '@tauri-apps/api/window': {
      getCurrentWindow: () => ({
        hide: async () => { hideCount += 1 },
        onFocusChanged: async (callback) => { focused = callback; return noop },
        listen: async (name, callback) => { listeners.set(name, { target: 'main', callback }); return noop },
      }),
    },
    '@tauri-apps/plugin-dialog': { ask: noop, confirm: () => confirmReset(), open: noop, save: noop },
    '@tauri-apps/plugin-autostart': { disable: async () => nativeHandlers.get('disable')?.(), enable: async () => nativeHandlers.get('enable')?.(), isEnabled: async () => false },
    '@tauri-apps/plugin-notification': { sendNotification: noop, onAction: async () => ({ unregister: async () => {} }), isPermissionGranted: async () => true, requestPermission: async () => 'granted' },
    '@lucide/vue': new Proxy({}, { get: () => ({ render: noop }) }),
    './components/ReminderPopup.vue': { default: { render: noop } },
    './components/AppSidebar.vue': { default: { render: noop } },
    './components/EventsView.vue': { default: { render: noop } },
    './components/RestView.vue': { default: { render: noop } },
    './components/SettingsView.vue': { default: { render: noop } },
    './components/AboutView.vue': { default: { render: noop } },
    './assets/remindon.svg': { default: 'remindon.svg' },
    './assets/donate.png': { default: 'donate.png' },
    './i18n': { translate },
    './types': { defaultData, defaultReminders },
    './error': { logError: (context, error) => { errors.push({ context, error }) } },
    './composables/useUpdater': {
      useUpdater: () => ({
        mode: vue.ref('development'), status: vue.ref('idle'), newVersion: vue.ref(''),
        progress: vue.ref(null), errorMessage: vue.ref(''), busy: vue.ref(false),
        checkForUpdates: async () => {}, installUpdate: noop, downloadPortableUpdate: noop,
        restartApp: noop, revealDownloadedUpdate: noop, openReleases: noop, dispose: noop,
      }),
    },
  }
  function load(code) {
    const exports = {}
    runInNewContext(code, {
      exports,
      require: (name) => {
        assert.ok(Object.hasOwn(modules, name), `Unexpected module: ${name}`)
        return modules[name]
      },
      window: {
        location: { hash: '' },
        setInterval: () => 1,
        clearInterval: noop,
        setTimeout: () => 1,
        clearTimeout: noop,
        addEventListener: (name, callback) => { windowListeners.set(name, callback) },
        removeEventListener: (name) => { windowListeners.delete(name) },
      },
      localStorage: { getItem: () => null, setItem: noop },
      document: { visibilityState: 'visible', hasFocus: () => true },
      console,
    })
    return exports
  }
  const component = load(scriptCode).default
  const state = component.setup({}, { expose: noop })
  const render = load(templateCode).render
  await mounted()
  return {
    state, calls, errors,
    setReadStatus: (callback) => { readStatus = callback },
    setSaveData: (callback) => { saveData = callback },
    setConfirm: (callback) => { confirmReset = callback },
    setInvoke: (command, callback) => { invokeHandlers.set(command, callback) },
    setNative: (name, callback) => { nativeHandlers.set(name, callback) },
    emit: (name, payload) => emitTo('main', name, payload),
    emitTo,
    focus: (isFocused) => focused({ payload: isFocused }),
    keydown: (event) => windowListeners.get('keydown')?.(event),
    getHideCount: () => hideCount,
    render: () => renderToString(vue.createSSRApp({
      setup: () => state, render, components: { BellRing: { render: noop } },
    })),
  }
}

test('Escape hides the main window to the tray', async () => {
  const app = await mountApp()
  app.keydown({ key: 'Enter', repeat: false })
  app.keydown({ key: 'Escape', repeat: true })
  assert.equal(app.getHideCount(), 0)
  app.keydown({ key: 'Escape', repeat: false })
  await new Promise(setImmediate)
  assert.equal(app.getHideCount(), 1)
})

test('serial saves keep newer edits and external popup mode changes', async () => {
  const app = await mountApp()
  const writes = []
  app.setSaveData((data, args) => new Promise((resolve) => writes.push({ data, args, resolve })))
  const first = app.state.updateSetting('popupOverlayOpacity', 10)
  await new Promise(setImmediate)
  const second = app.state.updateSetting('popupOverlayOpacity', 90)
  assert.equal(writes.length, 1)
  await app.emit('popup-fullscreen-updated', false)
  assert.equal(app.state.data.value.settings.popupOverlayOpacity, 90)
  writes[0].resolve(writes[0].data)
  await new Promise(setImmediate)
  assert.equal(writes.length, 2)
  assert.equal(writes[1].data.settings.popupOverlayOpacity, 90)
  assert.equal(writes[1].data.settings.popupFullscreen, false)
  assert.equal(writes[1].args.popupFullscreen, null)
  writes[1].resolve(writes[1].data)
  await Promise.all([first, second])
  assert.equal(app.state.data.value.settings.popupOverlayOpacity, 90)
  assert.equal(app.state.data.value.settings.popupFullscreen, false)
})

test('explicit main-window mode changes are included in the save', async () => {
  const app = await mountApp()
  let savedArgs
  app.setSaveData(async (data, args) => { savedArgs = args; return data })
  await app.state.updateSetting('popupFullscreen', false)
  assert.equal(savedArgs.popupFullscreen, false)
})

test('reset keeps English defaults, reminder entries and clears image-only changes', async () => {
  const app = await mountApp()
  app.state.data.value.settings = defaultData().settings
  app.state.data.value.settings.language = 'en'
  app.state.data.value.settings.restMessage = translate('en', 'rest.defaultMessage')
  app.state.data.value.reminders = [{ id: 'keep', title: 'keep reminder' }]
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  assert.equal(app.state.canResetSettings.value, true)
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(app.state.data.value.settings.language, 'en')
  assert.equal(app.state.data.value.settings.restMessage, translate('en', 'rest.defaultMessage'))
  assert.equal(app.state.data.value.reminders[0].id, 'keep')
  assert.equal(app.state.popupBackgroundPreview.value, '')
  assert.equal(app.state.canResetSettings.value, false)
})

test('failed reset restores settings, autostart and theme without clearing the image', async () => {
  const app = await mountApp()
  app.state.data.value.settings.autostart = true
  app.state.data.value.settings.theme = 'light'
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  const before = JSON.stringify(app.state.data.value.settings)
  const nativeCalls = []
  app.setNative('disable', async () => nativeCalls.push('disable'))
  app.setNative('enable', async () => nativeCalls.push('enable'))
  app.setNative('setTheme', async (theme) => nativeCalls.push(theme))
  app.setSaveData(async () => { throw new Error('disk full') })
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(JSON.stringify(app.state.data.value.settings), before)
  assert.ok(nativeCalls.includes('enable'))
  assert.equal(nativeCalls.at(-1), 'light')
  assert.equal(app.calls.includes('clear_popup_image'), false)
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,image')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.saveFailed'))
})

test('image removal failure keeps the saved defaults and reports the actual image state', async () => {
  const app = await mountApp()
  app.state.data.value.settings.theme = 'light'
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  app.setInvoke('clear_popup_image', async () => { throw new Error('permission denied') })
  app.setInvoke('read_popup_image', async () => 'data:image/png;base64,image')
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(app.state.data.value.settings.theme, 'system')
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,image')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.imageFailed'))
})

test('language changes translate untouched presets and preserve custom content', async () => {
  const app = await mountApp()
  app.state.data.value.reminders = defaultData().reminders
  app.state.data.value.reminders[1].title = '自定义下班提醒'
  await app.state.updateLanguage('en')
  assert.equal(app.state.data.value.reminders[0].title, translate('en', 'preset.lunch'))
  assert.equal(app.state.data.value.reminders[1].title, '自定义下班提醒')
  assert.equal(app.state.data.value.reminders[2].title, translate('en', 'preset.weekend'))
})

test('native startup failures display actual autostart status and retain background notification errors', async () => {
  const app = await mountApp({ nativeErrors: { autostartError: 'registration denied', notificationError: 'notifications disabled' } })
  assert.equal(app.state.data.value.settings.autostart, false)
  assert.equal(app.state.autostartError.value, translate('zh-CN', 'status.autostartFailed', { error: 'registration denied' }))
  assert.equal(app.state.notificationError.value, translate('zh-CN', 'status.notificationFailed', { error: 'notifications disabled' }))
})

test('reset synchronizes enabled-by-default autostart and system theme', async () => {
  const app = await mountApp()
  const nativeCalls = []
  app.setNative('enable', async () => nativeCalls.push('enable'))
  app.setNative('disable', async () => nativeCalls.push('disable'))
  app.setNative('setTheme', async (theme) => nativeCalls.push(theme))
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.deepEqual(nativeCalls, ['enable', null])
  assert.equal(app.state.data.value.settings.autostart, true)
})

test('test break notifications use backend rest state even with reminders disabled', async () => {
  const app = await mountApp({ enabled: false, status: { isResting: false, nextTriggerAt: null } })
  app.setReadStatus(async () => resting)
  await app.emit('reminder-triggered', { id: '__test_rest__', isRest: true, isTest: true })
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
  assert.equal(app.state.restProgress.value, 0)
  // 文案已下沉到 RestView，这里断言状态与文案来源保持一致，避免依赖界面文本。
  assert.equal(app.state.restStatusText.value, '正在休息中')
})

test('a slow status read cannot restore the countdown after a resting event', async () => {
  const app = await mountApp()
  let resolveStatus
  app.setReadStatus(() => new Promise((resolve) => { resolveStatus = resolve }))
  const refresh = app.state.refreshTimers()
  await app.emit('rest-timer-updated', resting)
  resolveStatus({ isResting: false, nextTriggerAt: '2026-09-23T16:00:00+08:00' })
  await refresh
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('temporary status failures keep the active break and do not restore a countdown', async () => {
  const app = await mountApp()
  app.setReadStatus(async () => { throw new Error('bridge unavailable') })
  await app.state.refreshTimers()
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('snooze and completion events apply their backend deadlines without stale reads replacing them', async () => {
  const app = await mountApp()
  const snoozedUntil = new Date(Date.now() + 4 * 60 * 60 * 1000).toISOString()
  let resolveStatus
  app.setReadStatus(() => new Promise((resolve) => { resolveStatus = resolve }))
  const refresh = app.state.refreshTimers()
  await app.emit('rest-timer-updated', { isResting: false, nextTriggerAt: snoozedUntil })
  resolveStatus(resting)
  await refresh
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, snoozedUntil)
  assert.equal(app.state.data.value.settings.restIntervalMinutes, 1)
  await app.emit('rest-timer-updated', resting)
  assert.equal(app.state.restIsActive.value, true)
  const completedUntil = new Date(Date.now() + 60 * 1000).toISOString()
  await app.emit('rest-timer-updated', { isResting: false, nextTriggerAt: completedUntil })
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, completedUntil)
})

test('newer refreshes take precedence over older reads', async () => {
  const app = await mountApp()
  let resolveOld
  app.setReadStatus(() => new Promise((resolve) => { resolveOld = resolve }))
  const oldRefresh = app.state.refreshTimers()
  app.setReadStatus(async () => resting)
  await app.state.refreshTimers()
  resolveOld({ isResting: false, nextTriggerAt: '2026-09-23T16:00:00+08:00' })
  await oldRefresh
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('window focus changes preserve the active backend break', async () => {
  const app = await mountApp()
  for (const focused of [false, true]) {
    const before = app.calls.length
    app.focus(focused)
    await new Promise(setImmediate)
    assert.equal(app.calls.length, before + 1)
    assert.equal(app.state.restIsActive.value, true)
    assert.equal(app.state.nextRestTrigger.value, null)
  }
})

test('a trigger follows backend countdown state for system notifications', async () => {
  const app = await mountApp()
  const nextTriggerAt = new Date(Date.now() + 60 * 1000).toISOString()
  app.setReadStatus(async () => ({ isResting: false, nextTriggerAt }))
  await app.emit('reminder-triggered', { id: '__rest__', isRest: true, isTest: false })
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, nextTriggerAt)
})

test('the main window ignores popup-targeted events and handles each scheduled reminder once', async () => {
  const nextTriggerAt = new Date(Date.now() + 60 * 1000).toISOString()
  const app = await mountApp({ status: { isResting: false, nextTriggerAt } })
  const callCount = app.calls.length
  const event = { id: '__rest__', isRest: true, isTest: false }
  app.setReadStatus(async () => resting)
  await app.emitTo('reminder', 'reminder-triggered', event)
  await app.emitTo('reminder', 'rest-timer-updated', resting)
  assert.equal(app.calls.length, callCount)
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, nextTriggerAt)

  await app.emitTo('main', 'reminder-triggered', event)
  assert.equal(app.calls.slice(callCount).filter((command) => command === 'load_data').length, 1)
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})
