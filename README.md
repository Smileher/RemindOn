# RemindOn

轻量桌面提醒工具，基于 Tauri 2、Vue 3、TypeScript 和 Rust，面向 Windows 与 Apple Silicon Mac。无需账号或服务器，数据保存在本地。

## 宣传官网

宣传官网：<https://smileher.github.io/RemindOn/>。支持中英文，主题默认跟随系统，也可手动切换深浅，下载信息随正式版本发布自动更新。

在项目根目录运行 `node site/preview.mjs`，打开 <http://127.0.0.1:4173/RemindOn/> 进行本地预览。截图维护和部署步骤见 [官网说明](site/README.md)。

## 功能

- **事件提醒**：单次、每天、每周、每月提醒。
- **休息提醒**：自定义间隔和文案，支持稍后提醒。
- **定时操作**：自动关机、锁定、重启，执行前有 60 秒可取消倒计时。
- **通知方式**：始终显示置顶软件弹窗，可配置是否额外发送系统通知。
- **个性设置**：中英文切换，深色、浅色、跟随系统主题及四种强调色。
- **桌面集成**：托盘常驻、开机自启、本地数据导入与导出。
- **应用更新**：安装版可在应用内下载安装；便携版可将新版程序下载到系统“下载”目录。

## 开发运行

准备 Node.js 24 LTS、pnpm 11 和 Rust stable，并安装对应平台的 [Tauri 开发依赖](https://v2.tauri.app/start/prerequisites/)：

- Windows：MSVC 工具链、Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载、WebView2。
- macOS：Xcode Command Line Tools。

```sh
pnpm install
pnpm tauri:dev
```

`pnpm tauri:dev` 会启动前端开发服务器和桌面窗口。单独运行 `pnpm dev` 仅启动网页界面，无法使用托盘、通知等原生能力。

### 在 VS Code 中运行与调试

用“文件 → 打开文件夹”打开本项目根目录（同时包含 `package.json` 和 `src-tauri/` 的目录）。先确认 VS Code 新建终端内的 `node --version`、`pnpm --version`、`cargo --version` 均可执行；安装环境后应完全退出并重新打开 VS Code。

安装 Vue (Official)、rust-analyzer 和 CodeLLDB 扩展。在“运行和调试”（`Ctrl+Shift+D`）顶部下拉框中选择：

- **RemindOn: 开发运行（热更新）**：按 `F5` 执行 `pnpm tauri:dev`，适合日常查看效果；不附加 Rust 调试器。前端可在应用窗口按 `Ctrl+Shift+I` 打开开发者工具，终端按 `Ctrl+C` 停止运行。
- **RemindOn: Rust 断点调试（Debug EXE）**：按 `F5` 先构建包含前端资源的 Debug EXE，再用 CodeLLDB 启动，可在 `.rs` 文件中设置断点。此模式不使用热更新，修改后停止并重新按 `F5`。CodeLLDB 的平台组件需下载完成。

`Ctrl+Shift+B` 可单独构建 Debug EXE，输出为 `src-tauri/target/debug/remindon.exe`；该任务通过 `tauri build --debug --no-bundle` 嵌入前端资源，不需要 Vite 开发服务器，也不生成安装包。它与 `tauri dev` 生成的同路径 EXE 用途不同，以最后一次构建方式为准。

也可从“终端 → 运行任务”选择上述任务。资源管理器中的 NPM 脚本视图可能默认隐藏，可通过“查看 → 打开视图”搜索 `NPM` 后打开；脚本会使用 pnpm 执行。这些入口无需 Tauri 扩展识别项目。

启动前请从托盘退出正在运行的安装版或便携版：本项目启用了单实例，且各版本共用提醒数据，旧进程可能接管新进程的启动请求。

## 检查与构建

```sh
# 前端类型检查与构建
pnpm build
pnpm test

# Rust 检查与单元测试
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# 以下打包命令需要先配置更新签名环境变量，见“发布更新”
# Windows：构建 NSIS 安装包
pnpm tauri:build --bundles nsis

# macOS：在 Apple Silicon Mac 上构建 DMG
pnpm tauri:build --bundles app,dmg
```

安装包输出到 `src-tauri/target/release/bundle/`。Windows 可执行文件为 `src-tauri/target/release/remindon.exe`；发布时另生成版本化的便携程序 `RemindOn_0.8.0_x64_portable.exe`。macOS 构建需在对应平台验证。

## 使用说明

- 关闭主窗口会隐藏到托盘并继续提醒；完全退出请使用托盘菜单中的“退出”。
- 所有软件休息通知（包括“测试通知”）未关闭时，下一轮计时暂停，主界面显示“正在休息中”；不操作或最小化弹窗都不会恢复计时。
- “稍后提醒”只按所选时长延后本轮提醒，不修改提醒间隔；“完成休息”或关闭弹窗后，从操作时刻按设定间隔重新计时。例如间隔为 1 分钟，选择 4 小时并点击“稍后提醒”后，4 小时后再次提醒，完成休息后仍按 1 分钟计时。系统通知发出后直接开始下一轮。
- 定时操作到点后显示倒计时，关闭通知可取消。因休眠等原因错过计划时间超过 1 分钟时，会跳过本次操作。

配置自动保存为 `remindon.json`，安装版、便携版和开发版共用同一数据目录：

- Windows：`%APPDATA%\com.remindon.app\remindon.json`
- macOS：`~/Library/Application Support/com.remindon.app/remindon.json`

可在应用内导入、导出数据。导入成功后会关闭当前提醒弹窗、取消其计时和待执行操作，并清除暂停及稍后提醒状态，按导入配置重新计时。读取到损坏或不兼容的数据文件时，程序会先备份原文件，再生成默认配置。

## 发布更新

更新文件托管在 [GitHub Releases](https://github.com/Smileher/RemindOn/releases)，无需自建服务器。客户端读取最新正式版本的 `latest.json`，使用内置公钥校验更新包签名。检查与下载需要能够访问 GitHub；自动检查失败时保持安静，手动检查失败时会显示错误并提供下载入口。

- Windows NSIS 安装版可在应用内安装更新。便携版会把版本化 EXE 下载到系统“下载”目录；下载完成后可点击“打开所在位置”，退出旧程序后自行运行或替换为新版。
- macOS 仅发布 Apple Silicon 版本。位于 `/Applications` 或 `~/Applications` 的应用副本可自动更新；从 DMG 或其他位置运行时，会把最新版 DMG 下载到系统“下载”目录。
- 便携文件先写入 `.part` 临时文件，完成后校验 SHA-256，再改为正式文件；不会覆盖当前运行程序或同名的不同文件。
- 开发模式不检查更新。更新只在用户确认后安装；安装和重启期间无法发送提醒，请先完成编辑或等待定时操作结束。
- 0.6 便携版尚未包含应用内下载能力，需要首次手动升级到 0.7；此后可直接在“关于”页下载新版。提醒数据仍保存在原应用数据目录。

仓库的 Actions Secrets 需要配置 `TAURI_SIGNING_PRIVATE_KEY`（更新私钥文件的完整内容）和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（私钥密码）。它们必须与 `src-tauri/tauri.conf.json` 中的公钥对应。私钥和密码应保存在仓库外并备份，不能提交到 Git，也不要为每次发布重新生成密钥。

本地打包时将同名环境变量设置为私钥内容和密码。例如 PowerShell：

```powershell
$signingDir = Join-Path $env:USERPROFILE '.codex/secrets/remindon'
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -LiteralPath (Join-Path $signingDir 'updater.key') -Raw -Encoding UTF8
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -LiteralPath (Join-Path $signingDir 'updater-password.txt') -Raw -Encoding UTF8
pnpm tauri:build --bundles nsis
```

发布步骤：

1. 同步更新 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 和界面的版本回退值，并更新 Cargo 锁文件。
2. 执行上述检查，提交代码并推送版本标签，例如 `v0.8.0`。
3. Release 工作流创建草稿，并行构建 Windows x64、macOS Apple Silicon 安装包及签名，同时上传版本化 Windows 便携 EXE、macOS DMG 和对应 SHA-256。
4. 所有构建成功后统一生成包含安装包签名及便携下载信息的 `latest.json`，再公开发布。失败时保留草稿，不向客户端发布不完整的更新。

如需重跑失败的发布，可重新运行工作流，或在 Actions 中选择对应版本标签手动运行。已公开的版本不可覆盖，应递增版本号重新发布。macOS 构建使用 ad-hoc 签名，未配置 Apple Developer ID 公证；更新签名与操作系统代码签名是不同机制。

## 项目结构

```text
src/             Vue 界面、组件、样式与中英文文案
src-tauri/src/   Rust 调度器、数据存储、通知及系统集成
src-tauri/       Tauri 配置、权限声明与应用图标
```

作者：ChenHe
