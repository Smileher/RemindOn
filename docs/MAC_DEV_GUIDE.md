# RemindOn 在 macOS 上的开发指南

这份文档写给熟悉 Windows 开发、但刚开始接触 Mac 的你。核心差异集中在**路径、命令、产物格式**三点，工具链本身（Node / pnpm / Rust / VS Code）两边完全一致。

## 一、已经装好的环境

| 工具 | 版本 | 说明 |
| --- | --- | --- |
| macOS | 27.0 | Apple Silicon（arm64） |
| Xcode Command Line Tools | clang 21.0.0 | 编译 Rust 原生依赖必需，已就绪 |
| Homebrew | 6.0.17 | `/opt/homebrew/bin/brew` |
| Node.js | 24.19.0 | 官方版，装在 `/opt/homebrew/lib/nodejs/node-24`，与 CI 和 README 要求一致 |
| pnpm | 11.28.3 | 前端包管理器，项目锁文件要求 pnpm 11 |
| Rust | 1.99.0 stable | `~/.cargo/bin`，已加入 PATH |

> **Node 版本说明**：README 和 CI 都要求 24 LTS，本机已装 24.19.0 完全对齐。Node 22 也能跑（实测 46 个测试全过），但既然两台机器都是 24 就没必要降级。
>
> Node 24 相比 22 的实用改进：`crypto.randomUUIDv7()`（时间有序 ID）、`req.signal`（客户端断开时中止请求）、测试运行器支持随机顺序（能发现隐藏的测试依赖）。这个项目目前用不到，但升级没有代价。
>
> 想要 26 也行（`Temporal` 日期 API 默认开启、V8 14.6），但 26 目前是 Current 阶段，要到 2026 年 10 月底才进 LTS。个人工具项目没必要追 Current，等它进 LTS 再升更稳。
>
> 项目没有 `.nvmrc` 或 `volta` 配置，所以 Node 版本不会被自动约束——换设备时记得手动对齐。

> Rust 装在 `~/.cargo/bin`，这是**用户级**目录，不在 Homebrew 也不在系统 PATH 里。你自己新开终端时如果提示 `cargo: command not found`，执行下面这行补上：
>
> ```zsh
> echo 'source "$HOME/.cargo/env"' >> ~/.zshrc
> ```
>
> 这行和 Homebrew 的 PATH 已经帮你写进 `~/.zshrc` 了，正常新开终端就能直接用 `cargo`、`gh`、`node`、`pnpm`。

## 二、打开项目和运行

### 用 VS Code（推荐，和你 Windows 上的习惯一致）

1. 打开 VS Code → `文件` → `打开文件夹` → 选 `/Users/smileher/Desktop/Remindon`
   - 注意要选**项目根目录**，即同时包含 `package.json` 和 `src-tauri/` 的那一层，不要选到子目录。
2. 安装推荐扩展：打开项目后 VS Code 右下角会提示，点安装全部即可。
   - Vue (Official)、rust-analyzer、CodeLLDB、Tauri
3. 新建终端（`Ctrl+` 反引号，或 `终端` → `新建终端`），确认三个命令都在：
   ```zsh
   node --version    # v24.19.0
   pnpm --version    # 11.28.3
   cargo --version   # 1.99.0
   ```
4. `F5` 启动，顶部下拉框选 **RemindOn: 开发运行（热更新）**

> **报 `zsh: command not found: pnpm` 怎么办？**
>
> 这是 PATH 没配好。node 和 pnpm 装在 `/opt/homebrew/lib/nodejs/node-24/bin`，`/opt/homebrew/bin` 里有软链接指向它们。检查并修复：
>
> ```zsh
> grep -q 'homebrew/bin' ~/.zshrc || echo 'export PATH="/opt/homebrew/bin:$PATH"' >> ~/.zshrc
> source ~/.zshrc
> ```
>
> 如果 VS Code 之前就开着，PATH 不会自动刷新——**完全退出 VS Code 再重开**（`Cmd + Q`，不是关窗口）。仍然不行就重启终端，或在 VS Code 里执行 `命令面板 → 开发人员: 重新加载窗口`。
>
> 验证：`ls -la /opt/homebrew/bin/pnpm` 应该显示软链接存在。

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

# 只构建 .app（最快，约 2 分钟）
pnpm tauri:build --bundles app --no-sign --ci

# 构建 .app 和 DMG（DMG 需要 Finder 自动化权限，见下方说明）
pnpm tauri:build --bundles app,dmg --no-sign --ci
```

## 四、安装包格式全景对照

### Windows 侧

| 格式 | 定位 | 特点 | 本项目是否使用 |
| --- | --- | --- | --- |
| `.exe`（NSIS） | 安装版 | 有安装向导，可写注册表、创建快捷方式。`installMode: currentUser` 表示只装给当前用户，不需要管理员权限 | **是**，官网主推 |
| `.exe`（portable） | 便携版 | 就是一个 exe，双击即用，不写注册表。CI 里手工复制二进制并算 SHA256 拼出来的 | **是**，官网提供下载 |
| `.msi` | 安装版 | Windows Installer 包，支持静默安装、组策略批量部署到企业机器 | 否 |
| `.msix` | 商店包 | 单架构。只能覆盖一种 CPU，Intel 机器和 ARM 机器需要分别提交 | 是，双架构模式时用 |
| `.msixbundle` | 商店包 | 同时包含 x64 与 ARM64，微软商店上传用这个。用户在商店里看到的**一个条目**会自动装对架构 | **是**，商店发布推荐 |

`msi` 和 `exe` 的本质区别：前者是 Windows Installer 标准格式，支持企业批量部署；后者是自解压安装程序，用户体验更直观。个人工具项目用 `exe` 就够了，`msi` 主要给企业 IT 部门用。

### macOS 侧

| 格式 | 定位 | 特点 |
| --- | --- | --- |
| `.app` | 应用包 | 就是应用本体，双击即用，不需要安装。**作用等同 Windows 的便携版** |
| `.dmg` | 磁盘镜像 | 分发用。挂载后把 app 拖到"应用程序"文件夹，模拟 Windows 的安装向导体验 |
| 裸二进制 | 开发调试 | 没有图标和图标壳，只能命令行跑 |

所以你问的「`.app` 相当于 Win 的便携版吗」——**对，定位完全一致**。区别是 macOS 的 `.app` 就是标准应用格式，不存在"安装"这一步；而 Windows 便携版是"省略了安装步骤"。

`.dmg` 和 `.app` 的关系：`.app` 是内容，`.dmg` 是装 `.app` 的盒子。给用户分发用 `.dmg`，自己本机用直接跑 `.app` 就行。

### 关于「Windows 上能打出 dmg」

**打不出。** 容易误解的地方在于 `src-tauri/tauri.conf.json` 里写着：

```json
"targets": ["nsis", "dmg"]
```

这**不是**说任意平台都能出这两种包。Tauri 会自动跳过当前平台不支持的那个：
- Windows 上跑 → 跳过 `dmg`，只出 nsis
- Mac 上跑 → 跳过 `nsis`，只出 dmg

而且跳过时**不报错**。我在 Mac 上跑 `--bundles nsis` 时退出码是 0、日志显示成功，但 `bundle/nsis/` 目录压根不存在。

CI 里的 dmg 是在 `macos-latest` 这台 **Mac 机器**上打的，不是在 Windows 上。

## 五、Mac 上能构建哪些包（实测结论）

| 包类型 | 能否在 Mac 构建 | 产物位置 | 说明 |
| --- | --- | --- | --- |
| `.app` 应用包 | **可以** | `target/release/bundle/macos/RemindOn.app` | 直接双击即可运行 |
| `.dmg` 磁盘镜像 | 可以，需授权 | `target/release/bundle/dmg/RemindOn_<版本>_aarch64.dmg` | 首次会弹窗要 Finder 权限 |
| 免安装裸二进制 | **可以** | `target/release/remindon` | 命令行直接跑 |
| NSIS 安装包（`nsis`） | **不行** | 无 | NSIS 只能在 Windows 上生成，Mac 上会静默跳过、不报错但没有文件 |
| MSIX 商店包 | **不行** | 无 | 依赖 `makeappx.exe`、MSVC、Windows 证书存储 |
| 便携版 | 不适用 | 无 | 便携版是 Windows 概念（免安装 zip） |

**关键提醒**：`--bundles nsis` 在 Mac 上跑不会报错，但也不会产出任何文件——日志里只有 `Built application at: .../release/remindon`，没有 `Bundling ... .exe`。别以为成功了。

**DMG 首次打包要授权**：`create-dmg` 脚本需要调用 Finder 排版图标窗口。首次执行会弹出权限请求，选择允许。若失败报 `“Finder”遇到一个错误：发生权限违例 (-10004)`，到 `系统设置 → 隐私与安全性 → 自动化` 里勾选允许你的终端或 VS Code 控制 Finder，然后重试。

**分发给别人时的限制**：未签名的 `.app` 在别人 Mac 上会被 Gatekeeper 拦住，需要 Apple 开发者证书签名和公证。自己本机开发测试完全不受影响。

`pnpm tauri:build` 需要先配置更新签名环境变量（`TAURI_SIGNING_PRIVATE_KEY` 等），**本地想快速看 .app 出来的话可以加 `--no-bundle` 跳过打包**，或者直接跑：

```zsh
pnpm tauri:build --debug --no-bundle
```

## 六、macOS 与 Windows 的关键差异

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

### 4. 双设备协作（Windows + macOS）

两台机器连同一个 GitHub 仓库（`master` 分支），可以随意切换。macOS 侧提交身份为 `MacDevice <vchenhe@live.com>`，Windows 侧为 `chenhe`，在 GitHub 提交历史里能分辨来源。

**固定流程**：改代码前先 `git pull` 拿对方改动，改完 `git push` 推回去。

**推荐别名**（加到 `~/.zshrc`）：

```zsh
alias gs='git status'
alias gp='git push'
alias gl='git pull --rebase'
alias glog='git log --oneline --graph --decorate --all -20'
```

**两边都能做的**：前端开发、Rust 编译、调试断点、单元测试、macOS 侧打 DMG。

**只能在 Windows 做的**：MSIX 商店包、Microsoft Store 上传。原因是 `scripts/build-store.ps1` 依赖 `makeappx.exe`、MSVC 工具链、`Cert:\` 证书存储和 `vswhere.exe`，这些 Windows 独有，Mac 上无法替代。商店发布流程留在 Windows 上走。

> **其实不用在 Windows 上手工打包**：`store.yml` 工作流已经配好了——版本解析跑在 `ubuntu-latest`，打包签名跑在 `windows-latest`。打 `v*` 标签即可触发，或在 Actions 页面手动 `workflow_dispatch` 填版本号。`release.yml` 同样是三平台矩阵（windows x64、windows arm64、macos arm64），Mac 产物由 CI 自动构建。所以 Mac 上唯一需要做的，就是改代码和推标签。

**`scripts/*.ps1` 在 Mac 上不可运行**，这是预期行为，不是 bug。这两个脚本（`build-store.ps1`、`package-msix.ps1`）是 Windows 专用的 CI/本地打包入口。同目录下的 `create-updater-manifest.mjs` 和 `sync-gitee-release.mjs` 是跨平台的 Node 脚本，两边都能跑。

**关于 `Cargo.lock`**：实测两台设备各自 build 不会改写它，锁文件保持稳定，不需要特殊处理。只有在 Windows 侧新增了 Rust 依赖并提交后，Mac 侧 `cargo build` 才会自动更新本地副本；此时若出现冲突，以远程版本为准：
```zsh
git checkout -- src-tauri/Cargo.lock && git pull
```


### 5. 沙箱权限弹窗

macOS 首次触发系统能力时会弹窗询问，需要同意：

| 操作 | 需要授予 |
| --- | --- |
| 发送系统通知 | `系统设置` → `通知` → 允许 RemindOn |
| 开机自启 | `系统设置` → `通用` → `登录项`，勾选 RemindOn |
| 自动关机 / 重启 / 锁屏 | `系统设置` → `隐私与安全性` → `自动化`，允许 RemindOn 控制 System Events |
| 导入导出配置文件 | 首次会弹文件选择框，选一个目录即可 |

自动化权限那条最容易漏，不授权的话定时关机功能会静默失败。

## 七、常见问题

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

## 八、测试结果（2026-10-02，Apple Silicon）

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

2026-10-03 补充验证：合并 Windows 侧 4 个提交（含商店版图标资源、通知修复、新增 `src-tauri/src/notification.rs`）后，Mac 上重新编译与全部测试仍通过，说明双设备并行开发不会互相破坏。

## 九、目录结构

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

## 十、发布流程

推 `v*` 标签即可触发全部构建，不需要在任一平台手工打包：

```zsh
# 先同步、更新版本号、跑一遍检查
git pull
# 修改 package.json 与 src-tauri/tauri.conf.json 的 version，两处必须一致
pnpm test && cargo test --manifest-path src-tauri/Cargo.toml
git commit -am "chore: 提升版本至 1.2.0" && git push
git tag v1.2.0 && git push origin v1.2.0
```

标签推送后依次触发：

| 工作流 | 做什么 |
| --- | --- |
| `release.yml` | 在 Windows x64、Windows ARM64、macOS ARM64 三台机器上并行构建，产出 4 个安装包 + 便携版，合成 `latest.json` 更新清单，发布到 GitHub Releases |
| `store.yml` | 解析版本后在 `windows-latest` 打 MSIX 上传包 |
| `pages.yml` | 等待 Release 成功后运行 `site/build.mjs`，抓取最新 Release 的资产列表更新官网下载地址 |
| `sync-gitee.yml` | 同步到 Gitee 镜像 |

**官网更新是有校验的**：`site/build.mjs` 要求 Release 里必须存在以下文件，缺一个就构建失败：
```
RemindOn_<版本>_x64-setup.exe
RemindOn_<版本>_arm64-setup.exe
RemindOn_<版本>_x64_portable.exe
RemindOn_<版本>_arm64_portable.exe
RemindOn_<版本>_aarch64.dmg
```
所以 Windows 便携版和 Mac 的 dmg 都是必需产物，CI 已自动处理。
