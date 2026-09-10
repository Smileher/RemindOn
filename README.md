# RemindOn

轻量桌面提醒工具，基于 Tauri 2、Vue 3、TypeScript 和 Rust，面向 Windows 与 macOS。无需账号或服务器，数据保存在本地。

## 功能

- **事件提醒**：单次、每天、每周、每月提醒。
- **休息提醒**：自定义间隔和文案，支持稍后提醒。
- **定时操作**：自动关机、锁定、重启，执行前有 60 秒可取消倒计时。
- **通知方式**：系统通知或置顶软件通知，提供三种软件通知样式。
- **个性设置**：中英文切换，深色、浅色、跟随系统主题及四种强调色。
- **桌面集成**：托盘常驻、开机自启、本地数据导入与导出。
- **应用更新**：安装版启动后自动检查更新，也可在“关于”页手动检查，确认后下载安装并重启；便携版提供手动下载入口。

## 开发运行

准备 Node.js 24 LTS、pnpm 11 和 Rust stable，并安装对应平台的 [Tauri 开发依赖](https://v2.tauri.app/start/prerequisites/)：

- Windows：MSVC 工具链、Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载、WebView2。
- macOS：Xcode Command Line Tools。

```sh
pnpm install
pnpm tauri:dev
```

`pnpm tauri:dev` 会启动前端开发服务器和桌面窗口。单独运行 `pnpm dev` 仅启动网页界面，无法使用托盘、通知等原生能力。

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

# macOS：在 macOS 主机上构建 DMG
pnpm tauri:build --bundles app,dmg
```

安装包输出到 `src-tauri/target/release/bundle/`。Windows 可执行文件为 `src-tauri/target/release/remindon.exe`，也可单独分发使用。macOS 构建需在对应平台验证。

## 使用说明

- 关闭主窗口会隐藏到托盘并继续提醒；完全退出请使用托盘菜单中的“退出”。
- 软件休息通知显示期间暂停下一轮计时，关闭后重新计时；系统通知发出后直接开始下一轮。
- 定时操作到点后显示倒计时，关闭通知可取消。因休眠等原因错过计划时间超过 1 分钟时，会跳过本次操作。

配置自动保存为 `remindon.json`，安装版、便携版和开发版共用同一数据目录：

- Windows：`%APPDATA%\com.remindon.app\remindon.json`
- macOS：`~/Library/Application Support/com.remindon.app/remindon.json`

可在应用内导入、导出数据。读取到损坏或不兼容的数据文件时，程序会先备份原文件，再生成默认配置。

## 发布更新

更新文件托管在 [GitHub Releases](https://github.com/Smileher/RemindOn/releases)，无需自建服务器。客户端读取最新正式版本的 `latest.json`，使用内置公钥校验更新包签名。检查与下载需要能够访问 GitHub；自动检查失败时保持安静，手动检查失败时会显示错误并提供下载入口。

- Windows 自动更新仅支持 NSIS 安装版：当前程序目录必须与注册表中的安装目录一致，并包含卸载程序。单独复制的 `remindon.exe` 需要手动替换。
- macOS 支持 Intel 和 Apple Silicon。请先将应用复制到 `/Applications` 或 `~/Applications`；直接从 DMG 或其他位置运行时提供手动下载入口。
- 开发模式不检查更新。更新只在用户确认后安装；安装和重启期间无法发送提醒，请先完成编辑或等待定时操作结束。
- 0.5 及更早版本需要先手动安装一次含更新功能的版本。提醒数据仍保存在原应用数据目录。

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
2. 执行上述检查，提交代码并推送版本标签，例如 `v0.6.0`。
3. Release 工作流创建草稿，并行构建 Windows x64、macOS Apple Silicon、macOS Intel 安装包及签名，另提供 Windows 便携可执行文件。
4. 所有构建成功后统一生成含三个平台的 `latest.json`，再公开发布。失败时保留草稿，不向客户端发布不完整的更新。

如需重跑失败的发布，可重新运行工作流，或在 Actions 中选择对应版本标签手动运行。已公开的版本不可覆盖，应递增版本号重新发布。macOS 构建使用 ad-hoc 签名，未配置 Apple Developer ID 公证；更新签名与操作系统代码签名是不同机制。

## 项目结构

```text
src/             Vue 界面、组件、样式与中英文文案
src-tauri/src/   Rust 调度器、数据存储、通知及系统集成
src-tauri/       Tauri 配置、权限声明与应用图标
```

作者：ChenHe
