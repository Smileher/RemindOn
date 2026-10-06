import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { translate } from '../src/i18n.ts'
import { defaultData } from '../src/types.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { parse, compileScript } = vueRequire('@vue/compiler-sfc')
const { descriptor } = parse(readFileSync(new URL('../src/components/SettingsView.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'settings-view-test' })
const code = ts.transpileModule(script.content, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText

test('settings tabs follow navigation and numeric settings clamp before emitting', async () => {
  const exports = {}
  const modules = {
    vue: { ...vue, onMounted() {}, onUnmounted() {} },
    '@lucide/vue': {}, '../i18n': { translate }, './PopupBackground.vue': {},
  }
  runInNewContext(code, { exports, require: (name) => modules[name] })
  const props = vue.reactive({ initialTab: 'popup', settings: defaultData().settings, language: 'zh-CN' })
  const emitted = []
  const state = exports.default.setup(props, { expose() {}, emit: (...args) => emitted.push(args) })
  assert.equal(state.activeTab.value, 'popup')
  props.initialTab = 'general'
  await vue.nextTick()
  assert.equal(state.activeTab.value, 'general')
  state.activeTab.value = 'data'
  await vue.nextTick()
  assert.deepEqual(emitted.at(-1), ['update:initialTab', 'data'])
  state.updateImageValue('popupTitleSize', { target: { value: '999' } }, 20, 72)
  assert.deepEqual(emitted.at(-1), ['update:setting', 'popupTitleSize', 72])
  state.updateImageValue('popupOverlayOpacity', { target: { value: '-1' } }, 0, 100)
  assert.deepEqual(emitted.at(-1), ['update:setting', 'popupOverlayOpacity', 0])
  const count = emitted.length
  state.updateImageValue('popupTitleSize', { target: { value: 'NaN' } }, 20, 72)
  assert.equal(emitted.length, count)
})
