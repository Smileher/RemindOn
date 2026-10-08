# RemindOn Project Guide

RemindOn runs on Windows x64/ARM64 and Apple Silicon macOS. This guide covers development, application behavior, packaging, and releases. For downloads, see the [website](https://smileher.github.io/RemindOn/en/).

## Development and debugging

Use Node.js 24 LTS, pnpm 11, Rust stable, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs MSVC, the Visual Studio Build Tools Desktop development with C++ workload, and WebView2. macOS needs Xcode Command Line Tools.

```sh
pnpm install
pnpm tauri:dev
```

`pnpm tauri:dev` starts the frontend server and desktop window with hot reload. `pnpm dev` starts only the web interface; tray, notifications, and other native features are unavailable. Quit any installed or portable copy from its tray menu first: all channels share reminder data and the single-instance lock, so an existing process can take over a development launch.

Open the repository root in VS Code. Confirm `node --version`, `pnpm --version`, and `cargo --version` work in a new terminal; restart VS Code after installing tools. Install Vue (Official), rust-analyzer, and CodeLLDB, including its platform component.

The existing Run and Debug entries are:

- **RemindOn: 开发运行（热更新）**: F5 runs `pnpm tauri:dev`, without attaching a Rust debugger. On Windows, Ctrl+Shift+I opens frontend developer tools; Ctrl+C stops the terminal process.
- **RemindOn: Rust 断点调试（Debug EXE）**: builds a debug executable with embedded frontend assets and launches CodeLLDB for Rust breakpoints. It has no hot reload; stop and restart after edits.

Ctrl+Shift+B builds the debug executable using `tauri build --debug --no-bundle`. On Windows it is `src-tauri/target/debug/remindon.exe`. A `tauri dev` build can occupy the same path but needs the dev server; the last build determines its behavior. Tasks are also available under Terminal > Run Task. The NPM Scripts view may need to be opened through View > Open View; it uses pnpm and does not require a Tauri extension.

VS Code includes portable, NSIS, macOS APP/DMG, Store upload, and Store local-test packaging tasks. Run Mac tasks on a Mac and Store tasks on Windows with the Windows SDK. Local installer tasks use `--no-sign`; formal updater releases still require the signing secrets below. Tasks do not upload or publish packages.

## Checks and packaging

```sh
pnpm build
pnpm test
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml

# These release packages require updater signing environment variables.
# Windows NSIS installer
pnpm tauri:build --bundles nsis

# Windows portable ZIP (default x64; add -Architecture arm64 for ARM64)
pwsh -NoProfile -File ./scripts/build-portable.ps1

# Apple Silicon Mac: application and DMG
pnpm tauri:build --bundles app,dmg
```

Packages are written to `src-tauri/target/release/bundle/` for local builds. Portable output is `src-tauri/target/portable/<version>/`, containing a versioned ZIP and its SHA-256 file; the matching EXE is inside the ZIP. ARM64 Windows builds require the corresponding Rust target, MSVC ARM64 C++ tools, and LLVM/Clang. Mac packaging and operation must be verified on macOS.

Preview the website with `node site/preview.mjs` at <http://127.0.0.1:4173/RemindOn/>. Its interface preview is HTML/CSS, not a bitmap screenshot. See the [Website Guide](../site/README.md) for build and deployment details.

## Application behavior

- Closing the main window hides it to the tray; reminders continue. Use the tray's Quit command to exit. Launching another copy shows the existing window.
- Launch at login defaults to off, and minimize to tray defaults to on. Initial setup shows the main window. Subsequent background starts or hiding the main window send a system notification regardless of the reminder notification setting.
- Launch at login reflects whether the current program owns the startup entry. Enabling it in an installer or portable build replaces the ordinary startup target. Disable it in the old channel before switching between Store and ordinary builds. Startup checks the real system state and does not restore an entry disabled by the system. Reset and import do not change autostart ownership. On macOS, move the application to Applications before enabling autostart; DMG and App Translocation paths are rejected.
- Defaults are blue accent, system theme, full-screen always-on-top popups, and background fade-in. Popup settings are shared by break and scheduled reminders, and preview works without a background image. The bundled transparent image is copied to the configuration folder once when no image exists. Removing it keeps popups image-free across restarts; resetting settings restores the image and its lower-right placement. Existing custom images are preserved.
- Eight accent colors are available. Before opening settings in an older build, select blue, mint, violet, or amber; older builds cannot read the four new accent values.
- Initial schedules are disabled: lunch and lock at 11:50 daily; finish work and lock at 17:30 Monday through Thursday; weekend and shutdown at 17:30 Friday. Deleted presets are not recreated.
- While any break popup remains open, including a test popup, the next break timer pauses. Leaving it alone or minimizing it does not resume the timer.
- Snooze postpones only the current break. Finishing or closing the popup starts a new interval from that action. For example, a one-minute interval with a four-hour snooze reminds again after four hours, then returns to one-minute intervals after the break.
- Reminders queue by scheduled time rather than replacing an open popup. Simultaneous scheduled reminders sort by ID, with break reminders last. Closing or finishing shows the next queued reminder.
- A scheduled computer action starts a cancellable 60-second countdown. Closing the popup cancels that occurrence without deleting a recurring schedule. Action reminders cannot snooze. If sleep or queuing makes an action more than one minute late, that occurrence is skipped. Test actions require another confirmation after the countdown.
- Editing, disabling, or deleting a schedule cancels its queued reminders and active action countdown.

Settings are saved automatically in `remindon.json`. Store, installer, portable, and development builds share the same directory:

- Windows: `%USERPROFILE%\.remindon\remindon.json` since v1.2.1; old test directories are not migrated.
- macOS: `~/Library/Application Support/com.remindon.app/remindon.json`.

Import and export are available in Settings. Successful import closes active popups, clears the queue, cancels timers and pending computer actions, clears pause/snooze state, and restarts timing using the imported configuration. The current configuration version is 6 and older versions are not migrated. A damaged or incompatible stored file is backed up before defaults are generated; importing another version reports an error. Export does not transfer machine-specific autostart ownership.

## Updates and release assets

[GitHub Releases](https://github.com/Smileher/RemindOn/releases) hosts updates, mirrored to [Gitee](https://gitee.com/smileher/RemindOn/releases). The ordinary builds read `latest.json` and use the embedded public key to validate signed update packages. Automatic checks run at most once every 24 hours and fail quietly; manual failures display an error and a download entry. Development and Store builds do not use this updater.

- Windows NSIS builds install updates in the app. Portable builds download a versioned ZIP to Downloads, validate SHA-256, and require the user to quit the old program, extract, and replace it manually. They do not automatically extract, replace, or launch a new EXE. Select the matching architecture; ARM64 installers cannot install on x64 devices.
- macOS copies in `/Applications` or `~/Applications` update through the signed `.app.tar.gz` archive, replace the app, and restart automatically. The DMG contains the same application for manual installation. Copies running from the DMG or another location download a new DMG to Downloads instead.
- If a GitHub connection fails or makes no progress, About shows a five-second countdown and an immediate switch to Gitee. Gitee retains the version, signatures, and hashes while changing download URLs.
- Downloads first use a `.part` file, validate SHA-256, then rename to the final name. They do not overwrite the running program or a different same-name file.
- Updates are user-confirmed. Reminders are unavailable during installation and restart; finish editing or wait for scheduled actions first. Version 0.6 portable builds need a first manual upgrade to 0.7; later versions can download from About, keeping the same data directory.

The public assets are two Windows installers, two Windows portable ZIPs, an Apple Silicon DMG, the macOS update archive, and `latest.json`. GitHub adds two source archives. Separate `.sig` and `.sha256` files are used inside Actions to generate the manifest, then removed from the draft before publication. Signatures and checksums remain embedded in the manifest; client verification is unchanged.

## Releasing a version

Set Actions secrets `TAURI_SIGNING_PRIVATE_KEY` (the complete private key file) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The key must match the public key in `src-tauri/tauri.conf.json`. Back up both outside the repository; do not commit them or regenerate keys for each release.

For a local signed build, set the same environment variables explicitly. For example, using a key directory outside the repository in PowerShell:

```powershell
$signingDir = Join-Path $env:USERPROFILE '.codex/secrets/remindon'
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -LiteralPath (Join-Path $signingDir 'updater.key') -Raw -Encoding UTF8
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -LiteralPath (Join-Path $signingDir 'updater-password.txt') -Raw -Encoding UTF8
pnpm tauri:build --bundles nsis
```

1. Update the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and the UI version fallback; update the Cargo lockfile.
2. Add `docs/releases/X.Y.Z.md` with 2-5 concise English bullets describing user-visible changes, normally within 120 words. Include compatibility or data changes when needed. Do not list test counts, CI internals, or packaging steps. Commit messages remain Chinese.
3. Run the checks, commit, and push a matching tag such as `v1.2.3`. Actions verifies the versions and requires a nonempty release-notes file before building. No local downloads or manual manifest assembly are needed.
4. The workflow creates or reuses a draft and applies the versioned notes. Windows x64, Windows ARM64, and Apple Silicon Mac build in parallel. Windows portable EXEs are published only inside ZIPs.
5. After all builds succeed, Actions generates and uploads the complete manifest, removes independent signature/checksum attachments, then publishes the release. A failed step leaves the release as a draft. Full reruns rebuild the temporary signature/checksum attachments; a publish-only retry after partial cleanup must rerun all jobs.
6. Successful Release runs trigger independent Gitee synchronization and Pages deployment. Gitee failures do not block GitHub Release or Pages. No source repository synchronization is performed to Gitee.

Retry failed releases with a full workflow rerun, or manually run Release on the matching version tag. Never overwrite a published version; increment the version for a new release. This repository presentation update does not itself require a version bump or changes to historical releases.

Mac builds use ad-hoc signing and are not notarized with Apple Developer ID. Updater signatures and OS code signing are different mechanisms. Users should download only from the official release, then allow opening in System Settings > Privacy & Security if macOS blocks it.

### Gitee mirror

Set the `GITEE_TOKEN` Actions secret to a private token with release and attachment write permission for `smileher/RemindOn`. Never commit the token. Initialize a default branch in the Gitee repository first; an empty repository can start with a minimal README.

Normal releases automatically mirror six binary assets and a rewritten `latest.json`. For failures, run **Retry Gitee release sync** with the published tag. Select `master` for retries to use current fixes. Enable `legacy_x64` only for historical releases such as v0.8.0 that predate ARM64; normal releases require all architectures.

The script anonymously downloads mirrored binaries and compares SHA-256 before publishing the mirror manifest, then verifies that manifest. Upload limits are three minutes idle and eight minutes total; API queries have a one-minute limit. The script budget is 25 minutes and the workflow limit is 30 minutes. Queries, uploads, and downloads use bounded retries. Lost upload responses trigger remote verification, and corrupt attachments are replaced. Automatic and manual sync for the same tag are mutually exclusive. Source archive entries are not binary attachments; verify six binaries plus the manifest.

### Microsoft Store MSIX

Store packages compile the Tauri EXE, then use Windows SDK `MakeAppx.exe` to produce MSIX or an x64/ARM64 `.msixbundle`. Tauri CLI does not directly build an MSIX bundle. A single-architecture MSIX can also update through the Store; a bundle includes multiple architecture packages for one upload. Store builds do not read GitHub/Gitee manifests and are updated by Microsoft Store.

Configure these repository secrets to exactly match Partner Center:

- `MSIX_IDENTITY_NAME`: `54317Smileher.RemindOn`
- `MSIX_PUBLISHER`: `CN=426E8CF5-3861-440D-B400-CDB0323C5FD4`
- `MSIX_PUBLISHER_DISPLAY_NAME`: `Smileher`

Optional `MSIX_PFX_BASE64` and `MSIX_PFX_PASSWORD` are only for signed local tests and must be supplied together. Store uploads do not require purchasing a signing certificate; the Store signs published packages. Do not commit certificates.

The Store workflow runs on matching tags, builds a four-part version such as `1.2.3.0`, and produces the `remindon-msix-store` artifact. Extract its `.msixbundle` and manually upload to Partner Center; the workflow does not call the Store API. Manual runs must also use the application version plus `.0`. Ordinary releases and Store versions can run in parallel; Store users are not replaced by ordinary installers.

On Windows, use the VS Code Store upload task or `./scripts/build-store.ps1`. Requirements are Node.js, pnpm, Rust's x64 Windows target, MSVC C++ tools, and Windows SDK `MakeAppx.exe`. Default output is an unsigned x64 `.msix` in `src-tauri/target/store/<version>/upload/`. For both architectures, install the ARM64 Rust target, ARM64 MSVC C++ tools, and LLVM/Clang, then use `-AllArchitectures` or the dual-architecture task. Continue serving both architectures in future Store updates, with a version greater than the published Store package.

The local-test task or `./scripts/build-store.ps1 -LocalTest` also creates a certificate in the current user's certificate store. On first use, it exports `src-tauri/target/store-certificate/RemindOn-local-test.cer`. Install that certificate into Local Machine > Trusted People (administrator permission required), then rerun. Signed test packages are in `src-tauri/target/store/<version>/local-test/`; `-AllArchitectures -LocalTest` produces a signed bundle.

The private key stays in the current user's certificate store, and temporary PFX files are deleted. Test packages share the Store package identity. To install a local test over the same version, export data first and uninstall the existing Store package. Never upload local-test packages or distribute test certificates. Real Store installation, upgrades, ARM64, and launch-at-login behavior still need device validation.

To replace the logo, edit `src/assets/remindon.svg`, then run the VS Code icon task or `pnpm tauri icon src/assets/remindon.svg` and commit the generated icons. Builds use these icons; the MSIX packaging script generates its additional sizes automatically.

## Repository settings and structure

The intended GitHub About description is:

> A lightweight break reminder app with customizable full-screen popups and scheduled reminders for Windows and macOS.

The intended Website field is <https://smileher.github.io/RemindOn/en/>. These are GitHub repository settings, not values automatically applied by a README or workflow. Review local changes before authorizing pushes, deployment, or writing these settings.

```text
src/             Vue interface, components, styles, and bilingual copy
src-tauri/src/   Rust scheduler, storage, notifications, and system integration
src-tauri/       Tauri configuration, permissions, and icons
site/            Bilingual static website with an HTML/CSS app preview
scripts/         Packaging and release utilities
docs/releases/   Versioned English release notes
```
