# RemindOn

[Website](https://smileher.github.io/RemindOn/en/) · [中文官网](https://smileher.github.io/RemindOn/)

**Take breaks. Call it a day.**

A lightweight break reminder app with customizable full-screen popups and scheduled reminders for Windows and Apple Silicon Mac.

## Download

Windows: get RemindOn from the Microsoft Store, with updates managed by the Store.

<a href="https://apps.microsoft.com/detail/9P9K31N2CJBW"><picture><source media="(prefers-color-scheme: light)" srcset="site/assets/store-en-light.svg"><img src="site/assets/store-en-dark.svg" height="80" alt="Get RemindOn from Microsoft Store"></picture></a>

[Mac DMG](https://smileher.github.io/RemindOn/en/#download) (Apple Silicon) · [Windows installers and portable ZIPs](https://smileher.github.io/RemindOn/en/#windows-downloads) (x64 / ARM64) · [All releases](https://github.com/Smileher/RemindOn/releases)

## Features

- **Break reminders**: Custom intervals and messages, with snooze and full-screen or windowed popups.
- **Personalized popups**: Customize backgrounds, text, colors, and fade effects, with a built-in preview.
- **Scheduled reminders**: One-time, daily, weekly, or monthly reminders, optionally with lock, shutdown, or restart and a cancellable 60-second countdown.
- **Desktop integration**: System tray, launch at login, themes, English and Chinese, and data import/export.

## Development

Install Node.js 24 LTS, pnpm 11, Rust stable, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

```sh
pnpm install
pnpm tauri:dev

# Type checking, frontend build, and tests
pnpm build
pnpm test
```

`pnpm tauri:dev` runs the desktop app. `pnpm dev` runs only the web interface without native features. Quit any running copy from its tray menu before starting development.

See the [Project Guide](docs/PROJECT_GUIDE.md) for debugging, usage details, packaging, and releases, or the [Website Guide](site/README.md) for website maintenance.

Built with Tauri 2, Vue 3, TypeScript, and Rust. Maintained by [Smileher](https://github.com/Smileher).
