# RemindOn 在 macOS 上的开发指南

这份文档写给熟悉 Windows 开发、但刚开始接触 Mac 的你。核心差异集中在**路径、命令、产物格式**三点，工具链本身（Node / pnpm / Rust / VS Code）两边完全一致。

## 一、已经装好的环境

| 工具 | 版本 | 说明 |
| --- | --- | --- |
| macOS | 27.0 | Apple Silicon（arm64） |
| Xcode Command Line Tools | clang 21.0.0 | 编译 Rust 原生依赖必需，已就绪 |
| Homebrew | 6.0.17 | `/opt/homebrew/bin/brew` |
| Node.js | 22.22.2 | 由 WorkBuddy 托管 |
| pnpm | 11.28.3 | 前端包管理器，项目锁文件要求 pnpm 11 |
| Rust | 1.99.0 stable | `~/.cargo/bin`，已加入 PATH |

> Rust 装在 `~/.cargo/bin`，这是**用户级**目录，不在 Homebrew 也不在系统 PATH 里。你自己新开终端时如果提示 `cargo: command not found`，执行下面这行补上：
>
> ```zsh
> echo 'source "$HOME/.cargo/env"' >> ~/.zshrc
> ```

## 二、打开项目和运行

### 用 VS Code（推荐，和你 Windows 上的习惯一致）

1. 打开 VS Code → `文件` → `打开文件夹` → 选 `/Users/smileher/Desktop/Remindon`
   - 注意要选**项目根目录**，即同时包含 `package.json` 和 `src-tauri/` 的那一层，不要选到子目录。
2. 安装推荐扩展：打开项目后 VS Code 右下角会提示，点安装全部即可。
   - Vue (Official)、rust-analyzer、CodeLLDB、Tauri
3. 新建终端（`Ctrl+` 反引号，或 `终端` → `新建终端`），确认三个命令都在：
   ```zsh
   node --version
   pnpm --version
   cargo --version
   ```
4. `F5` 启动，顶部下拉框选 **RemindOn: 开发运行（热更新）**

### 用命令行

```zsh
cd /Users/smileher/Desktop/Remindon
pnpm tauri:dev
```

启动后会弹出桌面应用窗口。前端改代码自动热更新；`Ctrl+C` 停止。

**只跑网页界面**用 `pnpm dev`，浏览器打开 `http://localhost:1420`。但托盘、通知、开机自启这些原生能力会失效，只适合调界面。

## 三、命令速查

```zsh
# 依赖安装（换机器或删了 node_modules 时才需要）
pnpm install

# 前端类型检查 + 构建
pnpm build

# 前端测试
pnpm test

# Rust 编译检查
cargo check --manifest-path src-tauri/Cargo.toml

# Rust 单元测试
cargo test --manifest-path src-tauri/Cargo.toml

# 桌面应用开发运行（热更新）
pnpm tauri:dev

# 打包：macOS 上产出 .app 和 DMG
pnpm tauri:build --bundles app,dmg
```

`pnpm tauri:build` 需要先配置更新签名环境变量（`TAURI_SIGNING_PRIVATE_KEY` 等），**本地想快速看 .app 出来的话可以加 `--no-bundle` 跳过打包**，或者直接跑：

```zsh
pnpm tauri:build --debug --no-bundle
```

## 四、macOS 与 Windows 的关键差异

### 1. 可执行文件没有扩展名

| | 路径 |
| --- | --- |
| Windows | `src-tauri/target/debug/remindon.exe` |
| macOS | `src-tauri/target/debug/remindon` |

`file` 命令可以确认产物架构：

```zsh
file src-tauri/target/debug/remindon
# Mach-O 64-bit executable arm64
```

### 2. 安装包格式不同

- Windows：NSIS 安装包（`.exe`）
- macOS：DMG 磁盘镜像，或者未签名的 `.app`（`target/release/bundle/macos/`）

macOS 分发到别人机器时，未签名应用会被 Gatekeeper 拦，需要 Apple 开发者证书签名和公证（notarization）。自己本机开发测试不受影响。

### 3. 数据文件位置

配置文件 `remindon.json` 在：

- Windows：`%APPDATA%\com.remindon.app\remindon.json`
- macOS：`~/Library/Application Support/com.remindon.app/remindon.json`

用 Finder 打开这个目录：`Cmd + Shift + G`，粘贴 `~/Library/Application Support/`，回车。

### 4. 沙箱权限弹窗

macOS 首次触发系统能力时会弹窗询问，需要同意：

| 操作 | 需要授予 |
| --- | --- |
| 发送系统通知 | `系统设置` → `通知` → 允许 RemindOn |
| 开机自启 | `系统设置` → `通用` → `登录项`，勾选 RemindOn |
| 自动关机 / 重启 / 锁屏 | `系统设置` → `隐私与安全性` → `自动化`，允许 RemindOn 控制 System Events |
| 导入导出配置文件 | 首次会弹文件选择框，选一个目录即可 |

自动化权限那条最容易漏，不授权的话定时关机功能会静默失败。

## 五、常见问题

**`cargo: command not found`**

```zsh
source "$HOME/.cargo/env"
```

装进 `~/.zshrc` 可以永久生效。

**Vite 报 `SAFE_DELETE_BULK_CONFIRM_REQUIRED`**

这是 AI 编程工具环境里的批量删除保护拦截了 Vite 清空 `dist` 目录的动作，不是项目问题。手动执行 `rm -rf dist` 后重跑构建即可。你自己用普通终端不会遇到。

**`pnpm install` 报 symlink 错误**

用扁平化模式重装，绕开符号链接：

```zsh
pnpm install --node-linker=hoisted
```

**单实例冲突，旧窗口接管新窗口**

项目启用了单实例，各版本共用数据。启动前先从托盘彻底退出正在运行的版本，否则旧进程会拦住新进程。

**Rust 首次编译很慢**

Tauri 依赖树很大，第一次 `cargo build` 要编译几百个 crate，5~10 分钟正常。编译产物缓存在 `src-tauri/target/`，之后增量编译就快了。**不要删这个目录**，除非你要完全重新编译。

**开发时端口 1420 被占用**

`vite.config.ts` 里设了 `strictPort: true`，端口被占会直接报错而不是自动换端口。找出占用进程：

```zsh
lsof -ti:1420 | xargs kill
```

## 六、测试结果（2026-10-02，Apple Silicon）

| 检查项 | 命令 | 结果 |
| --- | --- | --- |
| 前端类型检查 + 构建 | `pnpm build` | 通过，1832 个模块，产物约 141 KB JS |
| 前端单元测试 | `pnpm test` | 46 / 46 通过 |
| Rust 编译 | `cargo build` | 通过，无警告 |
| Rust 单元测试 | `cargo test` | 27 / 27 通过 |

平台适配方面，`src-tauri/src/lib.rs` 里的 macOS 分支是原生实现，不存在 Windows 硬移植的问题：

- 锁屏走系统的 `CGSession -suspend`
- 关机和重启走 `osascript` 调用 System Events
- 全屏处理对 macOS 单独写了分支（第 572、828、849 行附近）

## 七、目录结构

```
src/             Vue 界面、组件、样式与中英文文案
src-tauri/src/   Rust 调度器、数据存储、通知及系统集成
src-tauri/       Tauri 配置、权限声明与应用图标
site/            宣传官网静态站
tests/           前端测试
scripts/         构建与发布辅助脚本
```

官网本地预览：

```zsh
node site/preview.mjs
# 打开 http://127.0.0.1:4173/RemindOn/
```
