// Architecture is a recommendation: browsers can hide it, so manual selection remains available.
export async function detectArchitecture(browser) {
  const ua = browser.userAgent || ''
  const windows = browser.userAgentData?.platform === 'Windows' || /Windows/i.test(ua)
  if (!windows) return 'x64'
  try {
    const hints = await browser.userAgentData?.getHighEntropyValues(['architecture'])
    if (/^(arm|arm64|aarch64)$/i.test(hints?.architecture || '')) return 'arm64'
    if (/^(x86|x64)$/i.test(hints?.architecture || '')) return 'x64'
  } catch {
    // Fall back to explicit UA markers when client hints are unavailable.
  }
  return /\b(ARM64|AARCH64)\b/i.test(ua) ? 'arm64' : 'x64'
}

// Keep all links available without JS, and give explicit choices priority over delayed hints.
export function setupDownloads(root, browser) {
  const controls = root.querySelector('.arch-controls')
  if (!controls) return
  const buttons = [...controls.querySelectorAll('[data-architecture]')]
  const packages = [...root.querySelectorAll('[data-package-architecture]')]
  let selectedManually = false
  function select(architecture) {
    buttons.forEach((button) => button.setAttribute('aria-pressed', String(button.dataset.architecture === architecture)))
    packages.forEach((link) => { link.hidden = link.dataset.packageArchitecture !== architecture })
  }
  buttons.forEach((button) => button.addEventListener('click', () => {
    selectedManually = true
    select(button.dataset.architecture)
  }))
  select('x64')
  controls.hidden = false
  return detectArchitecture(browser).then((architecture) => {
    if (!selectedManually) select(architecture)
  })
}

if (typeof document !== 'undefined') setupDownloads(document, navigator)
