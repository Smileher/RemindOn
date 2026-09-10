import { locales } from './content.mjs'

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
  windows: '<path d="m3 5 8-1v7H3Zm10-1 8-1v8h-8ZM3 13h8v7l-8-1Zm10 0h8v8l-8-1Z"/>',
  apple: '<path d="M15 3c0 2-1.5 3-3 3 0-2 1-3 3-3Zm-3 5c-2-2-7-1-7 4 0 4 3 9 5 9 1 0 1-1 2-1s2 1 3 1c2 0 4-3 4-5-3-1-4-5-1-7-2-2-4-2-6-1Z"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  system: '<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/>',
  light: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>',
  dark: '<path d="M20 14A9 9 0 0 1 10 3a9 9 0 1 0 10 11Z"/>',
}

function icon(name, className = '') {
  return `<svg class="icon ${className}" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`
}

export function renderPage(language, release, screenshots) {
  const t = locales[language]
  const e = escapeHtml
  const url = `${origin}${basePath}${t.path}`
  const logo = `${basePath}assets/remindon.svg`
  const screenshot = (theme) => `overview-${language}-${theme}.webp`
  const hasScreenshots = ['light', 'dark'].every((theme) => screenshots.includes(screenshot(theme)))
  const productImage = hasScreenshots
    ? `<picture><source media="(prefers-color-scheme: dark)" srcset="${basePath}assets/screenshots/${screenshot('dark')}"><img class="product-image" src="${basePath}assets/screenshots/${screenshot('light')}" alt="${e(t.screenshotAlt)}" width="1560" height="1080" fetchpriority="high" decoding="async"></picture>`
    : `<div class="screenshot-placeholder"><div class="placeholder-inner"><img src="${logo}" width="64" height="64" alt=""><strong>${e(t.placeholder)}</strong><span>${e(t.placeholderDetail)}</span></div></div>`
  const releaseDate = new Intl.DateTimeFormat(t.lang, { dateStyle: 'long', timeZone: 'UTC' }).format(new Date(release.publishedAt))

  return `<!doctype html>
<html lang="${t.lang}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <meta name="theme-color" content="#f7f9f5" media="(prefers-color-scheme: light)">
  <meta name="theme-color" content="#141b18" media="(prefers-color-scheme: dark)">
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
  <header class="site-header">
    <div class="container header-inner">
      <a class="brand" href="${basePath}${t.path}" aria-label="RemindOn"><img src="${logo}" width="36" height="36" alt=""><span>RemindOn<span class="brand-dot">.</span></span></a>
      <nav class="main-nav" aria-label="${e(t.navigation)}">${['features', 'download', 'faq'].map((id, i) => `<a href="#${id}">${e(t.nav[i])}</a>`).join('')}</nav>
      <div class="theme-controls" role="group" aria-label="${e(t.theme)}" hidden>${['system', 'light', 'dark'].map((theme, i) => `<button type="button" data-theme-value="${theme}" aria-label="${e(t.themes[i])}" title="${e(t.themes[i])}" aria-pressed="${theme === 'system'}">${icon(theme)}</button>`).join('')}</div>
      <nav class="language-nav" aria-label="${e(t.language)}"><a href="${basePath}" lang="zh-CN" hreflang="zh-CN"${language === 'zh' ? ' aria-current="page"' : ''}>中文</a><span aria-hidden="true">/</span><a href="${basePath}en/" lang="en" hreflang="en"${language === 'en' ? ' aria-current="page"' : ''}>EN</a></nav>
    </div>
  </header>
  <main id="main">
    <section class="hero container" aria-labelledby="hero-title">
      <p class="eyebrow hero-eyebrow"><span class="status-dot" aria-hidden="true"></span>${e(t.eyebrow)}</p>
      <h1 id="hero-title">${e(t.headline[0])}<br><span>${e(t.headline[1])}</span></h1>
      <p class="hero-intro">${e(t.intro)}</p>
      <p class="hero-description">${e(t.introDetail)}</p>
      <div class="hero-actions"><a class="button button-primary" href="#download">${icon('download')}${e(t.download)}</a><a class="button button-secondary" href="${repository}">${e(t.github)}${icon('arrow')}</a></div>
      <p class="platform-note">${icon('check')}${e(t.platforms)}</p>
      <figure class="product-preview">${productImage}<figcaption><span class="preview-dot" aria-hidden="true"></span>${e(t.preview)}<span class="preview-wordmark" aria-hidden="true">REMINDON</span></figcaption></figure>
    </section>
    <section class="features section container" id="features" aria-labelledby="features-title">
      <div class="section-heading"><p class="eyebrow">${e(t.featuresLabel)}</p><h2 id="features-title">${e(t.featuresTitle)}</h2><p>${e(t.featuresIntro)}</p></div>
      <div class="feature-grid">${t.features.map((feature, i) => `<article class="feature-card"><div class="feature-top"><span class="feature-icon">${icon(feature.icon)}</span><span class="feature-number" aria-hidden="true">0${i + 1}</span></div><h3>${e(feature.title)}</h3><p>${e(feature.text)}</p><div class="feature-tags">${feature.tags.map((tag) => `<span>${e(tag)}</span>`).join('')}</div></article>`).join('')}</div>
      <div class="details-heading"><h3>${e(t.detailsTitle)}</h3><span aria-hidden="true">REMINDON / EVERYDAY ESSENTIALS</span></div>
      <div class="details-grid">${t.details.map((detail) => `<article class="detail-item">${icon(detail.icon)}<div><h4>${e(detail.title)}</h4><p>${e(detail.text)}</p></div></article>`).join('')}</div>
    </section>
    <section class="download-section section" id="download" aria-labelledby="download-title">
      <div class="container"><div class="download-heading"><div class="section-heading"><p class="eyebrow">${e(t.downloadLabel)}</p><h2 id="download-title">${e(t.downloadTitle)}</h2><p>${e(t.downloadIntro)}</p></div><div class="release-meta"><a class="version-badge" href="${e(release.url)}"><span class="status-dot" aria-hidden="true"></span>v${e(release.version)}${icon('arrow')}</a><span>${e(t.published)} <time datetime="${e(release.publishedAt)}">${e(releaseDate)}</time></span></div></div>
      <div class="download-grid">${t.packages.map((pkg) => `<article class="download-card"><div class="package-top">${icon(pkg.icon)}${pkg.badge ? `<span class="package-badge">${e(pkg.badge)}</span>` : ''}</div><h3>${e(pkg.title)}</h3><span class="package-arch">${e(pkg.arch)}</span><p>${e(pkg.text)}</p><a class="button ${pkg.key === 'windows' ? 'button-primary' : 'button-secondary'}" href="${e(release.downloads[pkg.key].url)}">${icon('download')}${e(pkg.action)}</a><span class="package-size">${(release.downloads[pkg.key].size / 1024 / 1024).toFixed(1)} MiB</span></article>`).join('')}</div>
      <div class="download-bottom"><p>${e(t.downloadNote)}</p><a class="text-link" href="${repository}/releases">${e(t.allReleases)}${icon('arrow')}</a></div></div>
    </section>
    <section class="faq-section section container" id="faq" aria-labelledby="faq-title"><div class="section-heading"><p class="eyebrow">${e(t.faqLabel)}</p><h2 id="faq-title">${e(t.faqTitle)}</h2><p>${e(t.faqIntro)}</p></div><div class="faq-list">${t.faqs.map((faq, i) => `<details${i === 0 ? ' open' : ''}><summary>${e(faq.question)}${icon('plus')}</summary><div class="faq-answer">${faq.paragraphs.map((p) => `<p>${e(p)}</p>`).join('')}${faq.paths ? `<dl>${faq.paths.map(([name, path]) => `<dt>${e(name)}</dt><dd><code>${e(path)}</code></dd>`).join('')}</dl>` : ''}</div></details>`).join('')}</div></section>
  </main>
  <footer class="site-footer"><div class="container"><div class="footer-top"><div><a class="brand" href="${basePath}${t.path}"><img src="${logo}" width="32" height="32" alt=""><span>RemindOn<span class="brand-dot">.</span></span></a><p>${e(t.footerTagline)}</p></div><nav aria-label="${e(t.source)}"><a href="${repository}">${e(t.source)}</a><a href="${repository}/issues">${e(t.issues)}</a><a href="${repository}/releases">${e(t.releaseNotes)}</a></nav></div><div class="footer-bottom"><span>© ${new Date(release.publishedAt).getUTCFullYear()} RemindOn</span><span>${e(t.author)}</span></div></div></footer>
</body>
</html>
`
}
