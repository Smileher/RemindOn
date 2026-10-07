import assert from 'node:assert/strict'
import test from 'node:test'
import { detectArchitecture, setupDownloads } from '../downloads.js'

test('architecture detection uses Windows client hints, then explicit UA markers', async () => {
  for (const [browser, expected] of [
    [{ userAgent: 'Windows NT 10.0; Win64; x64' }, 'x64'],
    [{ userAgent: 'Windows NT 10.0; ARM64' }, 'arm64'],
    [{ userAgent: 'Windows NT 10.0; aarch64' }, 'arm64'],
    [{ userAgent: 'Macintosh; ARM64' }, 'x64'],
    [{ userAgent: 'unknown' }, 'x64'],
    [{ userAgentData: { platform: 'Windows', getHighEntropyValues: async () => ({ architecture: 'arm' }) } }, 'arm64'],
    [{ userAgent: 'Windows; ARM64', userAgentData: { getHighEntropyValues: async () => ({ architecture: 'x86' }) } }, 'x64'],
    [{ userAgent: 'Windows; ARM64', userAgentData: { getHighEntropyValues: async () => { throw new Error('denied') } } }, 'arm64'],
    [{ userAgent: 'Windows', userAgentData: { getHighEntropyValues: async () => ({}) } }, 'x64'],
  ]) assert.equal(await detectArchitecture(browser), expected)
})

function downloadFixture() {
  const buttons = ['x64', 'arm64'].map((architecture) => ({
    dataset: { architecture },
    setAttribute(name, value) { this[name] = value },
    addEventListener(name, listener) { this.click = listener },
  }))
  const packages = ['x64', 'arm64', 'x64', 'arm64'].map((packageArchitecture) => ({ dataset: { packageArchitecture }, hidden: false }))
  const controls = { hidden: true, querySelectorAll: () => buttons }
  const root = { querySelector: () => controls, querySelectorAll: () => packages }
  return { root, controls, buttons, packages }
}

test('automatic and manual selection switch both download types', async () => {
  const { root, controls, buttons, packages } = downloadFixture()
  assert.ok(packages.every((link) => !link.hidden))
  await setupDownloads(root, { userAgent: 'Windows; ARM64' })
  assert.equal(controls.hidden, false)
  assert.deepEqual(packages.map((link) => link.hidden), [true, false, true, false])
  assert.equal(buttons[1]['aria-pressed'], 'true')
  buttons[0].click()
  assert.deepEqual(packages.map((link) => link.hidden), [false, true, false, true])
  assert.equal(buttons[0]['aria-pressed'], 'true')
})

test('delayed detection cannot override a manual selection', async () => {
  const { root, buttons, packages } = downloadFixture()
  let resolveHints
  const pending = setupDownloads(root, { userAgentData: { platform: 'Windows', getHighEntropyValues: () => new Promise((resolve) => { resolveHints = resolve }) } })
  buttons[0].click()
  resolveHints({ architecture: 'arm' })
  await pending
  assert.deepEqual(packages.map((link) => link.hidden), [false, true, false, true])
})
