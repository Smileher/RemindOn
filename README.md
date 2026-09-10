# RemindOn

轻量桌面提醒工具，基于 Tauri 2、Vue 3、TypeScript 和 Rust，面向 Windows 与 macOS。无需账号或服务器，数据保存在本地。

## 功能

- **事件提醒**：单次、每天、每周、每月提醒。
- **休息提醒**：自定义间隔和文案，支持稍后提醒。
- **定时操作**：自动关机、锁定、重启，执行前有 60 秒可取消倒计时。
- **通知方式**：系统通知或置顶软件通知，提供三种软件通知样式。
- **个性设置**：中英文切换，深色、浅色、跟随系统主题及四种强调色。
- **桌面集成**：托盘常驻、开机自启、本地数据导入与导出。

## 开发运行

准备 Node.js LTS、pnpm 和 Rust stable，并安装对应平台的 [Tauri 开发依赖](https://v2.tauri.app/start/prerequisites/)：

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

# Rust 检查与单元测试
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# Windows：构建 NSIS 安装包
pnpm tauri:build --bundles nsis

# macOS：在 macOS 主机上构建 DMG
pnpm tauri:build --bundles dmg
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

## 项目结构

```text
src/             Vue 界面、组件、样式与中英文文案
src-tauri/src/   Rust 调度器、数据存储、通知及系统集成
src-tauri/       Tauri 配置、权限声明与应用图标
```

作者：ChenHe
