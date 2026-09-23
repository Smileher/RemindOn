import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { defaultData } from '../src/types.ts'
import { translate } from '../src/i18n.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { compileScript, parse } = vueRequire('@vue/compiler-sfc')
const { descriptor } = parse(readFileSync(new URL('../src/components/ReminderPopup.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'reminder-popup-test' })
const scriptCode = ts.transpileModule(script.content, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const noop = () => {}
const settle = () => new Promise(setImmediate)
const restEvent = { id: '__rest__', title: '休息时间到了', isRest: true, isTest: false }
const powerEvent = { id: '__shutdown__', title: '锁定电脑', isShutdown: true, powerAction: 'lock', isTest: false }

async function mountPopup() {
  const data = defaultData()
  const listeners = new Map()
  const intervals = new Map()
  const calls = []
  const nativeHandlers = new Map()
  const invokeHandlers = new Map()
  let mounted
  let nextInterval = 1
  let clockNow = Date.now()
  let readData = async () => structuredClone(data)
  let confirmAction = async () => true
  const nativeWindow = {
    listen: async (name, callback) => { listeners.set(name, { target: 'reminder', callback }); return noop },
    onCloseRequested: async () => noop,
  }
  for (const name of ['hide', 'setAlwaysOnTop', 'center', 'show', 'setFocus']) {
    nativeWindow[name] = async () => {
      calls.push(name)
      await nativeHandlers.get(name)?.()
    }
  }
  const modules = {
    vue: { ...vue, onMounted: (callback) => { mounted = callback }, onUnmounted: noop },
    '@tauri-apps/api/app': {
      setTheme: async () => { calls.push('setTheme'); await nativeHandlers.get('setTheme')?.() },
    },
    '@tauri-apps/api/core': {
      invoke: async (command) => {
        calls.push(command)
        if (invokeHandlers.has(command)) return invokeHandlers.get(command)()
        if (command === 'load_data') return readData()
        if (['execute_power_action', 'dismiss_reminder', 'snooze_reminder'].includes(command)) return
        throw new Error(`Unexpected command: ${command}`)
      },
    },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => { listeners.set(name, { target: null, callback }); return noop },
    },
    '@tauri-apps/api/window': { getCurrentWindow: () => nativeWindow },
    '@tauri-apps/plugin-dialog': { confirm: () => confirmAction() },
    '@lucide/vue': new Proxy({}, { get: () => ({ render: noop }) }),
    '../assets/remindon.svg': { default: 'remindon.svg' },
    '../i18n': { translate },
    '../types': { defaultData },
    '../error': { logError: (_context, error) => { throw error } },
  }
  const exports = {}
  runInNewContext(scriptCode, {
    exports,
    require: (name) => {
      assert.ok(Object.hasOwn(modules, name), `Unexpected module: ${name}`)
      return modules[name]
    },
    window: {
      setInterval: (callback) => { const id = nextInterval++; intervals.set(id, callback); return id },
      clearInterval: (id) => intervals.delete(id),
    },
    document: { addEventListener: noop, removeEventListener: noop },
    localStorage: { getItem: () => null, setItem: noop },
    Date: class extends Date { static now() { return clockNow } },
    console,
  })
  const state = exports.default.setup({}, { expose: noop })
  await mounted()
  return {
    state, calls, intervals,
    setReadData: (callback) => { readData = callback },
    setNative: (name, callback) => nativeHandlers.set(name, callback),
    setInvoke: (name, callback) => invokeHandlers.set(name, callback),
    setConfirm: (callback) => { confirmAction = callback },
    emitTo: async (target, name, payload) => {
      const listener = listeners.get(name)
      if (listener && (listener.target === null || listener.target === target)) {
        listener.callback({ payload })
      }
      await settle()
    },
    advance: async (milliseconds) => {
      clockNow += milliseconds
      for (const callback of intervals.values()) callback()
      await settle()
    },
  }
}

test('system reminder events sent only to main never start popup handling', async () => {
  const popup = await mountPopup()
  const reads = popup.calls.filter((name) => name === 'load_data').length
  await popup.emitTo('main', 'reminder-triggered', restEvent)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.calls.filter((name) => name === 'load_data').length, reads)
  assert.equal(popup.calls.filter((name) => name === 'show').length, 0)
  assert.equal(popup.intervals.size, 0)
})

test('popup and main trigger emissions load the reminder content only once', async () => {
  const popup = await mountPopup()
  const reads = popup.calls.filter((name) => name === 'load_data').length
  await popup.emitTo('reminder', 'reminder-triggered', restEvent)
  await popup.emitTo('main', 'reminder-triggered', restEvent)
  assert.equal(popup.calls.filter((name) => name === 'load_data').length, reads + 1)
  assert.equal(popup.intervals.size, 1)
  assert.equal(popup.state.current.value.id, '__rest__')
})

test('import reset clears the current rest and its elapsed timer', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(restEvent)
  await popup.advance(28_000)
  assert.equal(popup.state.restElapsedSeconds.value, 28)
  await popup.emitTo('reminder', 'reminders-reset')
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.state.triggeredAt.value, null)
  assert.equal(popup.state.restElapsedSeconds.value, 0)
  assert.equal(popup.intervals.size, 0)
  assert.equal(popup.calls.at(-1), 'hide')
})

test('import reset cancels the pending automatic power countdown', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(powerEvent)
  assert.equal(popup.intervals.size, 1)
  await popup.emitTo('reminder', 'reminders-reset')
  await popup.advance(60_000)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.state.powerCountdown.value, 60)
  assert.equal(popup.intervals.size, 0)
  assert.ok(!popup.calls.includes('execute_power_action'))
})

test('dismiss, snooze, cancellation and reset prevent pending content from restarting old timers', async () => {
  for (const step of ['load_data', 'setTheme']) {
    for (const action of ['dismiss', 'snooze', 'cancelRest', 'resetReminders']) {
      for (const event of action === 'cancelRest' ? [restEvent] : [restEvent, powerEvent]) {
        const popup = await mountPopup()
        let resolveStep
        const pending = new Promise((resolve) => { resolveStep = resolve })
        if (step === 'load_data') popup.setReadData(() => pending)
        else popup.setNative(step, () => pending)
        const trigger = popup.state.handleTrigger(event)
        await settle()
        await popup.state[action](14_400)
        resolveStep(defaultData())
        await trigger
        const scenario = `${step}, ${action}, ${event.id}`
        assert.equal(popup.calls.filter((name) => name === 'show').length, 0, scenario)
        assert.equal(popup.intervals.size, 0, scenario)
        await popup.advance(60_000)
        assert.ok(!popup.calls.includes('execute_power_action'), scenario)
      }
    }
  }
})

test('a late dismiss or snooze response cannot hide a newer notification', async () => {
  for (const action of ['dismiss', 'snooze']) {
    const popup = await mountPopup()
    await popup.state.handleTrigger(restEvent)
    let resolveAction
    popup.setInvoke(`${action}_reminder`, () => new Promise((resolve) => { resolveAction = resolve }))
    const closing = popup.state[action](14_400)
    await settle()
    await popup.state.handleTrigger(powerEvent)
    const hides = popup.calls.filter((name) => name === 'hide').length
    resolveAction()
    await closing
    assert.equal(popup.calls.filter((name) => name === 'hide').length, hides, action)
    assert.equal(popup.state.current.value.id, powerEvent.id, action)
    assert.equal(popup.intervals.size, 1, action)
  }
})

test('an old power confirmation cannot execute after import reset', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger({ ...powerEvent, id: '__test_power__', isTest: true })
  let resolveConfirmation
  popup.setConfirm(() => new Promise((resolve) => { resolveConfirmation = resolve }))
  const executing = popup.state.executePowerAction()
  await popup.emitTo('reminder', 'reminders-reset')
  resolveConfirmation(true)
  await executing
  assert.ok(!popup.calls.includes('execute_power_action'))
})
