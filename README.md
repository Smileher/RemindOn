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

“终端 → 运行任务”还提供 Windows 便携版、Windows NSIS 安装版、macOS APP/DMG、Store 上传包和 Store 本机测试包构建任务。macOS 任务必须在 Mac 上运行；Store 任务必须在装有 Windows SDK 的 Windows 上运行。本地 Windows/macOS 安装版任务使用 `--no-sign`，适合测试；正式更新发布仍按下文配置更新签名密钥。构建任务不会自动上传或发布任何安装包。

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

# Windows：构建便携 EXE（不打包）
pnpm tauri:build --no-bundle

# macOS：在 Apple Silicon Mac 上构建 DMG
pnpm tauri:build --bundles app,dmg
```

安装包输出到 `src-tauri/target/release/bundle/`。Windows 可执行文件为 `src-tauri/target/release/remindon.exe`；发布时另生成版本化的便携程序，例如 `RemindOn_1.1.0_x64_portable.exe`。macOS 构建需在对应平台验证。

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

更新文件托管在 [GitHub Releases](https://github.com/Smileher/RemindOn/releases)，并自动同步到 [Gitee 镜像](https://gitee.com/smileher/RemindOn/releases)，无需自建服务器。客户端读取最新正式版本的 `latest.json`，使用内置公钥校验更新包签名。普通版启动后最多每 24 小时自动检查一次；检查失败保持安静，手动检查失败时会显示错误并提供下载入口。

- Windows NSIS 安装版可在应用内安装更新。便携版会把版本化 EXE 下载到系统“下载”目录；下载完成后可点击“打开所在位置”，退出旧程序后自行运行或替换为新版。
- Windows 同时发布 x64 和 ARM64 安装版、便携版。请根据系统架构选择对应文件，ARM64 版本不能安装到 x64 设备上。
- macOS 仅发布 Apple Silicon 版本。位于 `/Applications` 或 `~/Applications` 的应用副本可自动更新；从 DMG 或其他位置运行时，会把最新版 DMG 下载到系统“下载”目录。
- GitHub 下载连接失败或无进度时，关于页面会显示 5 秒倒计时，并允许立即切换到 Gitee；倒计时结束后自动切换。Gitee 清单与 GitHub 清单版本、签名和 SHA-256 相同，只替换下载地址。
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
2. 执行上述检查，提交代码并推送版本标签，例如 `v1.1.0`。
3. Release 工作流创建草稿，并行构建 Windows x64、Windows ARM64、macOS Apple Silicon 安装包及签名，同时上传版本化 Windows 便携 EXE、macOS DMG 和对应 SHA-256。
4. 所有构建成功后统一生成包含安装包签名及便携下载信息的 GitHub `latest.json`，再公开发布。失败时保留草稿，不向客户端发布不完整的更新。
5. 发布完成后，`sync-gitee` job 创建或复用同 tag 的 Gitee Release，上传 6 个二进制资产和 Gitee 版 `latest.json`。Gitee 仓库不接收源码同步。

如需重跑失败的发布，可重新运行工作流，或在 Actions 中选择对应版本标签手动运行。已公开的版本不可覆盖，应递增版本号重新发布。macOS 构建使用 ad-hoc 签名，未配置 Apple Developer ID 公证；更新签名与操作系统代码签名是不同机制。

### Gitee Actions 配置

在 GitHub 仓库打开 `Settings → Secrets and variables → Actions → New repository secret`，新增：

- `GITEE_TOKEN`：Gitee 个人设置中创建的、具有 `smileher/RemindOn` Release 和附件写入权限的私人令牌。

令牌只配置在 GitHub Secret 中，不提交到仓库，也不要写入 workflow 文件。Gitee 仓库必须先有默认分支；空仓库请先在 Gitee 页面创建一个最小 README。正常发布会自动同步，失败时可在 `Retry Gitee release sync` workflow 中输入已发布的 tag 重试。旧版本（例如尚无 ARM64 的 `v0.8.0`）重试时可启用 `legacy_x64`；正常发布仍要求全部架构资产。同步脚本会匿名下载镜像文件并比对 SHA-256，再上传和验证 Gitee 清单。

2026-09-29 实测：`GITEE_TOKEN` 能创建 Gitee Release，但 `v0.8.0` 和 `v0.9.0` 的同步曾因 Gitee 附件接口请求超时而失败，Release 页面显示的源码压缩包不代表安装包附件。同步脚本现将单次 Gitee 请求超时提高到 10 分钟，并输出上传阶段；重试成功后再以附件列表和匿名下载为准。此前失败记录见 [v0.8.0 测试](https://github.com/Smileher/RemindOn/actions/runs/36420353389)，[v0.9.0 测试](https://github.com/Smileher/RemindOn/actions/runs/36523413163)。

### Microsoft Store MSIX

Store 版本先由 Tauri 编译 EXE，再用 Windows SDK 的 `MakeAppx.exe` 生成 x64/ARM64 MSIX 与 `.msixbundle`；当前 Tauri CLI 没有直接生成 MSIX bundle 的选项。`.msix` 也可以由 Store 更新，bundle 只是把多个架构放在同一个上传文件中。Store 版本与 NSIS/便携版是独立渠道，不访问 GitHub 或 Gitee，更新由 Microsoft Store 管理。普通版仍保留应用内更新和 Gitee 回退。

首次构建前，在 GitHub 仓库配置以下值：

- Repository secrets：`MSIX_IDENTITY_NAME`、`MSIX_PUBLISHER`、`MSIX_PUBLISHER_DISPLAY_NAME`。它们必须与 Partner Center 现有产品的包身份、Publisher 和 PublisherDisplayName 完全一致。
- 可选 Repository secrets：`MSIX_PFX_BASE64`、`MSIX_PFX_PASSWORD`，仅用于本地签名测试，两项必须同时提供。商店上传不要求购买代码签名证书，由 Microsoft Store 在发布时重新签名；不要提交证书文件。

本产品的 Store 身份值为：`MSIX_IDENTITY_NAME=54317Smileher.RemindOn`、`MSIX_PUBLISHER=CN=426E8CF5-3861-440D-B400-CDB0323C5FD4`、`MSIX_PUBLISHER_DISPLAY_NAME=Smileher`。推送与版本文件一致的 `v1.1.0` 标签后，Release 工作流会构建并发布普通版、同步 Gitee；Microsoft Store workflow 同时使用 `1.1.0.0` 生成 x64/ARM64 `.msixbundle` artifact，不会调用 Store API。下载并解压 `remindon-msix-store` artifact 后，将其中的 `.msixbundle` 手动上传到 Partner Center。手动运行 Store workflow 时，输入版本必须是应用版本加 `.0`，例如 `1.1.0.0`。普通 GitHub/Gitee 版本和 Store 版本可以并行，但 Store 用户不会被普通版安装包覆盖。

在 Windows 本机可从 VS Code“终端 → 运行任务”选择“构建 Store 上传包”，或运行 `./scripts/build-store.ps1`。需要 Node.js、pnpm、Rust 的 x64 Windows target、MSVC C++ 工具链，以及 Windows SDK 的 `MakeAppx.exe`。默认生成可直接提交 Partner Center 的未签名 x64 `.msix`，位于 `src-tauri/target/store/<版本>/upload/`。如需与 GitHub 一样生成 x64/ARM64 `.msixbundle`，还须安装 Rust 的 ARM64 Windows target，以及 Visual Studio 的 ARM64 MSVC C++ 和 LLVM/Clang 组件，然后运行 VS Code“双架构上传包”任务或 `./scripts/build-store.ps1 -AllArchitectures`。单架构包也能由 Store 更新，但只覆盖所提交的架构；发布新版本时应继续照顾 ARM64 用户。新版本必须高于已发布的 Store 包版本。

选择“构建 Store 上传包和本机测试包”，或运行 `./scripts/build-store.ps1 -LocalTest`，还会在当前用户证书库生成一张本机测试证书。首次运行会导出 `src-tauri/target/store-certificate/RemindOn-local-test.cer` 并提示信任证书：双击该文件，选择“安装证书 → 当前用户 → 将所有证书放入下列存储”，分别安装到“受信任的根证书颁发机构”和“受信任人”；完成后重跑构建任务。随后输出签名的 `src-tauri/target/store/<版本>/local-test/RemindOn_<版本>_x64.msix`，可双击安装。`-AllArchitectures -LocalTest` 则输出签名的双架构 bundle。脚本只在当前用户证书库保存私钥，临时 PFX 用后删除；测试包与正式 Store 包使用相同包身份。同版本正式 Store 包已安装时，先在应用内导出数据，再卸载它，才能安装本机测试包。不要将本机测试包上传 Store，也不要分发测试证书。

更换 Logo 时，只需修改 `src/assets/remindon.svg`，然后在 VS Code 运行“更新应用图标”任务，或执行 `pnpm tauri icon src/assets/remindon.svg` 并提交生成的桌面图标。普通构建自动使用这些图标；MSIX 所需的无底板尺寸由打包脚本在每次构建时自动生成，无需手工替换。

真实 MSIX 的安装、升级、ARM64 和开机自启仍需验证。

## 项目结构

```text
src/             Vue 界面、组件、样式与中英文文案
src-tauri/src/   Rust 调度器、数据存储、通知及系统集成
src-tauri/       Tauri 配置、权限声明与应用图标
```

作者：Smileher
