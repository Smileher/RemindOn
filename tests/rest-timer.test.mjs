import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { renderToString } from 'vue/server-renderer'
import { defaultData } from '../src/types.ts'
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

async function mountApp({ enabled = true, status = resting } = {}) {
  const settingsData = defaultData()
  settingsData.settings.restEnabled = enabled
  settingsData.settings.restIntervalMinutes = 1
  const listeners = new Map()
  const calls = []
  let mounted
  let focused
  let readStatus = async () => status
  const modules = {
    vue: { ...vue, onMounted: (callback) => { mounted = callback }, onUnmounted: noop },
    '@tauri-apps/api/app': { getVersion: async () => '0.8.0', setTheme: async () => {} },
    '@tauri-apps/api/core': {
      invoke: async (command) => {
        calls.push(command)
        if (command === 'load_data') return structuredClone(settingsData)
        if (command === 'get_rest_timer_status') {
          assert.ok(listeners.has('rest-timer-updated'), 'subscribe before reading initial status')
          return readStatus()
        }
        if (command === 'get_next_shutdown_trigger') return null
        throw new Error(`Unexpected command: ${command}`)
      },
    },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => { listeners.set(name, callback); return noop },
    },
    '@tauri-apps/api/window': {
      getCurrentWindow: () => ({ onFocusChanged: async (callback) => { focused = callback; return noop } }),
    },
    '@tauri-apps/plugin-dialog': { ask: noop, open: noop, save: noop },
    '@tauri-apps/plugin-autostart': { disable: noop, enable: noop, isEnabled: async () => false },
    '@lucide/vue': new Proxy({}, { get: () => ({ render: noop }) }),
    './components/ReminderPopup.vue': { default: { render: noop } },
    './assets/remindon.svg': { default: 'remindon.svg' },
    './assets/donate.png': { default: 'donate.png' },
    './i18n': { translate },
    './types': { defaultData },
    './error': { logError: (_context, error) => { throw error } },
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
      window: { location: { hash: '' }, setInterval: () => 1, clearInterval: noop },
      console,
    })
    return exports
  }
  const component = load(scriptCode).default
  const state = component.setup({}, { expose: noop })
  const render = load(templateCode).render
  await mounted()
  return {
    state, calls,
    setReadStatus: (callback) => { readStatus = callback },
    emit: (name, payload) => listeners.get(name)({ payload }),
    focus: (isFocused) => focused({ payload: isFocused }),
    render: () => renderToString(vue.createSSRApp({
      setup: () => state, render, components: { BellRing: { render: noop } },
    })),
  }
}

test('test break notifications use backend rest state even with reminders disabled', async () => {
  const app = await mountApp({ enabled: false, status: { isResting: false, nextTriggerAt: null } })
  app.setReadStatus(async () => resting)
  await app.emit('reminder-triggered', { id: '__test_rest__', isRest: true, isTest: true })
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
  assert.equal(app.state.restProgress.value, 0)
  assert.ok((await app.render()).includes('正在休息中'))
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
    assert.equal(app.calls.length, before + 2)
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
