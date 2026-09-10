(() => {
  const storageKey = 'remindon-site-theme'
  const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
  let preference = 'system'
  try {
    const saved = localStorage.getItem(storageKey)
    if (['system', 'light', 'dark'].includes(saved)) preference = saved
  } catch {
    // Theme selection still works when browser storage is unavailable.
  }

  function applyTheme() {
    const dark = preference === 'dark' || (preference === 'system' && systemTheme.matches)
    document.documentElement.dataset.theme = dark ? 'dark' : 'light'
    const screenshot = document.querySelector('.product-preview source')
    if (screenshot) screenshot.media = dark ? 'all' : 'not all'
    document.querySelectorAll('[data-theme-value]').forEach((button) => {
      button.setAttribute('aria-pressed', String(button.dataset.themeValue === preference))
    })
    document.querySelectorAll('meta[name="theme-color"]').forEach((meta) => {
      meta.content = dark ? '#141b18' : '#f7f9f5'
    })
  }

  // Apply the stored palette before the stylesheet paints the first frame.
  applyTheme()
  systemTheme.addEventListener('change', applyTheme)
  document.addEventListener('DOMContentLoaded', () => {
    applyTheme()
    const controls = document.querySelector('.theme-controls')
    controls.hidden = false
    controls.addEventListener('click', (event) => {
      const button = event.target.closest('[data-theme-value]')
      if (!button) return
      preference = button.dataset.themeValue
      try {
        localStorage.setItem(storageKey, preference)
      } catch {
        // Keep the current selection for this page if it cannot be persisted.
      }
      applyTheme()
    })
  })
})()
