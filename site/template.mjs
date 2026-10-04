import { languageLabels, locales, microsoftStoreUrl } from './content.mjs'

export const basePath = '/RemindOn/'
export const origin = 'https://smileher.github.io'
export const repository = 'https://github.com/Smileher/RemindOn'

export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character])
}

const paths = {
  arrow: '<path d="M7 17 17 7M7 7h10v10"/>',
  download: '<path d="M12 3v12m-5-5 5 5 5-5M5 16v5h14v-5"/>',
  calendar: '<rect x="3" y="5" width="18" height="16" rx="3"/><path d="M16 3v4M8 3v4M3 11h18m-13 5h2m4 0h2"/>',
  coffee: '<path d="M4 9h13v6a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5Zm13 0h2a3 3 0 1 1 0 6h-2M7 3v2m4-2v2m4-2v2M3 22h16"/>',
  power: '<path d="M12 2v10m-6-8a9 9 0 1 0 12 0"/>',
  bell: '<path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M9 21h6"/>',
  palette: '<path d="M12 3a9 9 0 1 0 0 18h1a2 2 0 0 0 1-4 1.5 1.5 0 0 1 1-3h3a3 3 0 0 0 3-3 9 9 0 0 0-9-8Z"/><path d="M7 10h.01M10 6.5h.01M15 7h.01M6.5 15h.01"/>',
  tray: '<path d="M4 4h16v16H4ZM4 14h4l2 3h4l2-3h4M12 7v6m-3-3 3 3 3-3"/>',
  folder: '<path d="M3 7V5a2 2 0 0 1 2-2h4l3 4h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><path d="M9 14h6m-3-3v6"/>',
  sliders: '<path d="M4 21v-7m0-4V3m8 18v-9m0-4V3m8 18v-5m0-4V3M1 14h6M9 8h6m2 8h6"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  windows: '<path d="m3 5 8-1v7H3Zm10-1 8-1v8h-8ZM3 13h8v7l-8-1Zm10 0h8v8l-8-1Z"/>',
  apple: '<path d="M15 3c0 2-1.5 3-3 3 0-2 1-3 3-3Zm-3 5c-2-2-7-1-7 4 0 4 3 9 5 9 1 0 1-1 2-1s2 1 3 1c2 0 4-3 4-5-3-1-4-5-1-7-2-2-4-2-6-1Z"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  system: '<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/>',
  light: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>',
  dark: '<path d="M20 14A9 9 0 0 1 10 3a9 9 0 1 0 10 11Z"/>',
}

function icon(name, className = '', size = 24) {
  return `<svg class="icon ${className}" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${paths[name]}</svg>`
}

const iconColors = ['violet', 'pink', 'cyan']

export function renderPage(language, release, screenshots) {
  const t = locales[language]
  const e = escapeHtml
  const windowsPackages = t.packages.filter((pkg) => pkg.icon === 'windows')
  const macPackage = t.packages.find((pkg) => pkg.key === 'mac')
  const url = `${origin}${basePath}${t.path}`
  const logo = `${basePath}assets/remindon.svg`
  const screenshot = (theme) => `overview-${language}-${theme}.webp`
  const hasScreenshots = ['light', 'dark'].every((theme) => screenshots.includes(screenshot(theme)))
  const productImage = hasScreenshots
    ? `<picture><source media="(prefers-color-scheme: dark)" srcset="${basePath}assets/screenshots/${screenshot('dark')}"><img class="product-image" src="${basePath}assets/screenshots/${screenshot('light')}" alt="${e(t.screenshotAlt)}" width="1560" height="1080" loading="lazy" decoding="async"></picture>`
    : `<div class="screenshot-placeholder"><div class="placeholder-inner"><img src="${logo}" width="64" height="64" alt=""><strong>${e(t.placeholder)}</strong><span>${e(t.placeholderDetail)}</span></div></div>`
  const releaseDate = new Intl.DateTimeFormat(t.lang, { dateStyle: 'long', timeZone: 'UTC' }).format(new Date(release.publishedAt))
  const allFeatures = [
    ...t.features.map((feature) => ({ ...feature, tags: feature.tags || [] })),
    ...t.details.map((detail) => ({ ...detail, tags: [] })),
  ]
  const trustRow = [t.platforms, ...t.trust].map((item) => `<span>${e(item)}</span>`).join('<span class="sep" aria-hidden="true"></span>')
  const mockup = t.mockup

  const mockupReminders = mockup.reminders.map((reminder, i) => `
                <div class="mu-reminder${i === 0 ? ' featured' : ''}">
                  <span class="mu-r-icon">${icon('calendar', '', 15)}</span>
                  <span class="mu-r-copy"><strong>${e(reminder.name)}</strong><span class="mono">${e(reminder.time)} · ${e(reminder.repeat)}</span></span>
                  <span class="mu-toggle${i < 2 ? ' on' : ''}" aria-hidden="true"></span>
                </div>`).join('')

  return `<!doctype html>
<html lang="${t.lang}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="dark light">
  <meta name="theme-color" content="#0A0B1A" media="(prefers-color-scheme: dark)">
  <meta name="theme-color" content="#F4F2FB" media="(prefers-color-scheme: light)">
  <title>${e(t.title)}</title>
  <meta name="description" content="${e(t.description)}">
  <link rel="canonical" href="${url}">
  <link rel="alternate" hreflang="zh-CN" href="${origin}${basePath}">
  <link rel="alternate" hreflang="en" href="${origin}${basePath}en/">
  <link rel="alternate" hreflang="x-default" href="${origin}${basePath}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="RemindOn">
  <meta property="og:title" content="${e(t.title)}">
  <meta property="og:description" content="${e(t.description)}">
  <meta property="og:url" content="${url}">
  <meta property="og:locale" content="${t.locale}">
  <meta property="og:locale:alternate" content="${language === 'zh' ? 'en_US' : 'zh_CN'}">
  <meta property="og:image" content="${origin}${basePath}assets/share-icon.png">
  <meta property="og:image:alt" content="RemindOn">
  <meta name="twitter:card" content="summary">
  <link rel="icon" href="${logo}" type="image/svg+xml">
  <script src="${basePath}assets/theme.js"></script>
  <link rel="stylesheet" href="${basePath}assets/style.css">
</head>
<body>
  <a class="skip-link" href="#main">${e(t.skip)}</a>
  <div class="bg" aria-hidden="true"></div>
  <nav class="nav" aria-label="${e(t.navigation)}">
    <div class="nav-inner">
      <a class="brand" href="${basePath}${t.path}" aria-label="RemindOn"><span class="brand-mark">${icon('bell', '', 14)}</span><span>RemindOn<span class="brand-dot">.</span></span></a>
      <div class="nav-links">${['features', 'download', 'faq'].map((id, i) => `<a href="#${id}">${e(t.nav[i])}</a>`).join('')}</div>
      <div class="nav-right">
        <div class="theme-controls" role="group" aria-label="${e(t.theme)}" hidden>${['system', 'light', 'dark'].map((theme, i) => `<button type="button" class="btn-icon" data-theme-value="${theme}" aria-label="${e(t.themes[i])}" title="${e(t.themes[i])}" aria-pressed="${theme === 'system'}">${icon(theme, '', 17)}</button>`).join('')}</div>
        <nav class="language-nav" aria-label="${e(t.language)}"><a href="${basePath}" lang="zh-CN" hreflang="zh-CN"${language === 'zh' ? ' aria-current="page"' : ''}>${e(languageLabels.zh)}</a><span aria-hidden="true">/</span><a href="${basePath}en/" lang="en" hreflang="en"${language === 'en' ? ' aria-current="page"' : ''}>${e(languageLabels.en)}</a></nav>
        <a class="btn btn-primary nav-cta" href="#download">${e(t.download)}${icon('arrow', 'arrow', 15)}</a>
      </div>
    </div>
  </nav>
  <main id="main">
    <header class="hero" id="top">
      <div class="wrap hero-inner reveal">
        <span class="eyebrow"><span class="pulse" aria-hidden="true"></span>${e(t.eyebrow)}</span>
        <h1 class="h-display hero-title"><span class="gradient-text">${e(t.headline[0])}</span><br><span class="gradient-text-cyan">${e(t.headline[1])}</span></h1>
        <p class="hero-sub">${e(t.intro)}</p>
        <div class="hero-cta"><a class="btn btn-primary" href="#download">${icon('download', '', 16)}${e(t.download)}</a><a class="btn btn-ghost" href="${repository}">${e(t.github)}${icon('arrow', 'arrow', 15)}</a></div>
        <div class="hero-trust mono">${trustRow}</div>
      </div>
      <div class="wrap">
        <div class="mockup">
          <div class="mockup-tilt">
            <div class="mockup-glow" aria-hidden="true"></div>
            <div class="mockup-floor" aria-hidden="true"></div>
            <div class="mockup-frame" role="img" aria-label="${e(t.mockupAria)}">
              <div class="mockup-bar" aria-hidden="true"><span class="dot"></span><span class="dot"></span><span class="dot"></span><span class="addr mono">RemindOn · v${e(release.version)}</span></div>
              <div class="mockup-ui" aria-hidden="true">
                <aside class="mu-side">
                  <div class="mu-ws"><span class="mu-ws-mark">${icon('bell', '', 12)}</span><span class="mu-ws-name">RemindOn</span></div>
                  <p class="mu-side-label">${e(mockup.sideLabel)}</p>
                  <ul class="mu-proj">
                    <li class="active">${icon('calendar', '', 14)}<span>${e(mockup.nav[0])}</span></li>
                    <li>${icon('coffee', '', 14)}<span>${e(mockup.nav[1])}</span></li>
                    <li>${icon('power', '', 14)}<span>${e(mockup.nav[2])}</span></li>
                  </ul>
                  <div class="mu-user"><span class="mu-avatar">${icon('sliders', '', 12)}</span><span class="mu-user-name">${e(mockup.settings)}</span></div>
                </aside>
                <div class="mu-main">
                  <div class="mu-head">
                    <div class="mu-head-copy"><p class="mu-eyebrow mono">${e(mockup.eyebrow)}</p><h4>${e(mockup.title)}</h4><p class="mu-sub">${e(mockup.subtitle)}</p></div>
                    <div class="mu-actions"><span class="mu-btn ghost">${e(mockup.test)}</span><span class="mu-btn grad">${icon('plus', '', 12)}${e(mockup.add)}</span></div>
                  </div>
                  <div class="mu-list">${mockupReminders}
                  </div>
                </div>
                <aside class="mu-panel">
                  <div class="mu-panel-head"><span class="mu-spark">${icon('bell', '', 11)}</span><span>${e(mockup.panelLabel)}</span><span class="mu-live mono">${e(mockup.live)}</span></div>
                  <div class="mu-toastcard">
                    <strong>${e(mockup.toastTitle)}</strong>
                    <p>${e(mockup.toastText)}</p>
                    <div class="mu-chips"><span class="chip">${e(mockup.snooze)}</span><span class="chip grad">${e(mockup.ok)}</span></div>
                  </div>
                  <div class="mu-count">
                    <div class="mu-count-head">${icon('clock', '', 13)}<span>${e(mockup.countdownLabel)}</span><span class="mono">${e(mockup.countdownTime)}</span></div>
                    <div class="mu-bar"><span class="mu-bar-fill"></span></div>
                    <p class="mono">${e(mockup.countdownTitle)} · ${e(mockup.countdownNote)}</p>
                  </div>
                </aside>
              </div>
            </div>
          </div>
        </div>
      </div>
    </header>
    <section class="features section" id="features" aria-labelledby="features-title">
      <div class="wrap">
        <div class="section-head"><p class="eyebrow">${e(t.featuresLabel)}</p><h2 id="features-title">${e(t.featuresTitle)}</h2><p>${e(t.featuresIntro)}</p></div>
        <div class="features-grid">${allFeatures.map((feature, i) => `<article class="feature" style="--i:${i % 3}"><span class="feature-icon ${iconColors[i % 3]}">${icon(feature.icon, '', 22)}</span><h3>${e(feature.title)}</h3><p>${e(feature.text)}</p>${feature.tags.length ? `<div class="feature-tags mono">${feature.tags.map((tag) => `<span class="chip">${e(tag)}</span>`).join('')}</div>` : ''}</article>`).join('')}</div>
      </div>
    </section>
    <section class="screens section" aria-labelledby="screens-title">
      <div class="wrap">
        <div class="section-head"><p class="eyebrow">${e(t.screensLabel)}</p><h2 id="screens-title">${e(t.screensTitle)}</h2><p>${e(t.screensIntro)}</p></div>
        <figure class="product-preview reveal">${productImage}<figcaption><span class="preview-dot" aria-hidden="true"></span>${e(t.preview)}<span class="preview-wordmark" aria-hidden="true">REMINDON</span></figcaption></figure>
      </div>
    </section>
    <section class="download-section section" id="download" aria-labelledby="download-title">
      <div class="wrap"><div class="download-heading"><div class="section-heading"><p class="eyebrow"><span class="pulse" aria-hidden="true"></span>${e(t.downloadLabel)}</p><h2 id="download-title">${e(t.downloadTitle)}</h2><p>${e(t.downloadIntro)}</p></div><div class="release-meta"><a class="version-badge" href="${e(release.url)}"><span class="pulse" aria-hidden="true"></span>v${e(release.version)}${icon('arrow', 'arrow', 14)}</a><span>${e(t.published)} <time datetime="${e(release.publishedAt)}">${e(releaseDate)}</time></span></div></div>
      <div class="download-grid">
        <article class="platform-card"><div class="platform-heading"><span class="platform-icon">${icon('windows', '', 22)}</span><div><h3>Windows</h3><p>${e(t.windowsIntro)}</p></div></div><div class="platform-options" aria-label="${e(t.chooseVersion)}">${windowsPackages.map((pkg) => { const external = Boolean(pkg.externalUrl); const download = external ? { url: pkg.externalUrl } : release.downloads[pkg.key]; return `<a class="platform-option${external ? ' store-option' : ''}" href="${e(download.url)}"><span class="platform-option-copy"><strong>${e(pkg.arch)}</strong><small>${e(pkg.text)}</small></span><span class="platform-option-action">${e(pkg.action)}${icon(external ? 'arrow' : 'download', 'arrow', 15)}</span></a>` }).join('')}</div></article>
        <article class="platform-card"><div class="platform-heading"><span class="platform-icon">${icon('apple', '', 22)}</span><div><h3>${e(macPackage.title)}</h3><p>${e(t.macIntro)}</p></div></div><div class="mac-package"><span class="package-badge mono">${e(macPackage.arch)}</span><strong>${e(macPackage.text)}</strong><a class="btn btn-primary" href="${e(release.downloads[macPackage.key].url)}">${icon('download', '', 16)}${e(macPackage.action)}</a><span class="package-size mono">${(release.downloads[macPackage.key].size / 1024 / 1024).toFixed(1)} MiB</span></div></article>
      </div>
      <div class="download-bottom"><p>${e(t.downloadNote)}</p><a class="text-link" href="${repository}/releases">${e(t.allReleases)}${icon('arrow', 'arrow', 15)}</a></div></div>
    </section>
    <section class="faq-section section" id="faq" aria-labelledby="faq-title">
      <div class="wrap">
        <div class="section-head"><p class="eyebrow">${e(t.faqLabel)}</p><h2 id="faq-title">${e(t.faqTitle)}</h2><p>${e(t.faqIntro)}</p></div>
        <div class="faq-list">${t.faqs.map((faq, i) => `<details class="faq-item"${i === 0 ? ' open' : ''}><summary class="faq-q"><span>${e(faq.question)}</span><span class="chev">${icon('plus', '', 16)}</span></summary><div class="faq-answer">${faq.paragraphs.map((p) => `<p>${e(p)}</p>`).join('')}${faq.paths ? `<dl>${faq.paths.map(([name, path]) => `<dt>${e(name)}</dt><dd><code>${e(path)}</code></dd>`).join('')}</dl>` : ''}</div></details>`).join('')}</div>
      </div>
    </section>
  </main>
  <footer class="site-footer">
    <div class="wrap">
      <div class="foot">
        <div class="foot-brand"><a class="brand" href="${basePath}${t.path}"><span class="brand-mark">${icon('bell', '', 13)}</span><span>RemindOn<span class="brand-dot">.</span></span></a><p>${e(t.footerTagline)}</p></div>
        <nav class="foot-col" aria-label="${e(t.footerProduct)}"><h4>${e(t.footerProduct)}</h4><a href="#features">${e(t.nav[0])}</a><a href="#download">${e(t.nav[1])}</a><a href="#faq">${e(t.nav[2])}</a></nav>
        <nav class="foot-col" aria-label="${e(t.footerProject)}"><h4>${e(t.footerProject)}</h4><a href="${repository}">${e(t.source)}</a><a href="${repository}/issues">${e(t.issues)}</a><a href="${repository}/releases">${e(t.releaseNotes)}</a></nav>
        <nav class="foot-col" aria-label="${e(t.footerChannels)}"><h4>${e(t.footerChannels)}</h4><a href="${e(microsoftStoreUrl)}">Microsoft Store</a><a href="${repository}/releases">${e(t.allReleases)}</a></nav>
      </div>
      <div class="foot-bottom">
        <span>© ${new Date(release.publishedAt).getUTCFullYear()} RemindOn</span>
        <span class="status"><span class="pulse" aria-hidden="true"></span>v${e(release.version)}</span>
        <span>${e(t.author)}</span>
      </div>
    </div>
  </footer>
</body>
</html>
`
}
