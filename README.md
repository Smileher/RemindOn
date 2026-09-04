# RemindOn

RemindOn 是一个使用 Tauri 2、Vue 3、TypeScript 和 Rust 编写的轻量桌面提醒工具，当前目标平台为 Windows 和 macOS。

它不使用服务器、登录系统或数据库。提醒数据保存在本地 JSON 文件中。

标语：轻量定时提醒。作者：ChenHe（SimileHe）。

## 当前功能

- 单次、每天、每周、每月、工作日和休息日提醒
- 固定间隔休息提醒和自定义提醒文字
- 系统通知或居中置顶的软件通知
- 自动关机、自动锁定和自动重启，执行前提供 60 秒可取消倒计时
- 系统托盘常驻、双击打开、托盘跳转关于页和开机自动运行
- 深色、浅色、跟随系统主题及四种强调色
- 简体中文和 English 动态切换
- 本地数据导入、导出和异常文件备份

## 给 WPF/.NET 开发者的快速理解

可以先用下面的方式理解项目分工：

| WPF/.NET 概念 | RemindOn 中对应的部分 |
| --- | --- |
| Window / UserControl | `src/App.vue`、`src/components/*.vue` |
| XAML 样式、资源 | `src/style.css` 和组件中的 `<style>`（本项目样式集中在 `src/style.css`） |
| ViewModel 状态 | Vue `ref`、`reactive`、`computed` |
| C# 服务或宿主层 | `src-tauri/src/lib.rs` |
| C# 调用原生能力 | 前端 `invoke(...)` 调用 Rust `#[tauri::command]` |
| 事件聚合器、消息总线 | Rust `app.emit(...)` 和前端 `listen(...)` |
| `.csproj`、发布配置 | `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` |

Tauri 应用由两部分组成：

1. Vue/TypeScript 负责界面、表单和交互。
2. Rust 负责文件、托盘、调度器、窗口控制、通知和系统集成。

前端和 Rust 不是直接共享内存，而是通过异步命令和事件通信。前端调用 Rust 命令时，参数名必须和 Rust 命令的参数名一致。

## 环境准备

### Windows

需要以下工具：

- Node.js，建议使用当前 LTS 版本
- pnpm
- Rust stable MSVC 工具链
- Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载
- WebView2 Runtime（现代 Windows 通常已经安装）

检查是否安装成功：

```powershell
node --version
pnpm --version
rustc --version
cargo --version
rustup show active-toolchain
```

本机当前使用的是：

```text
rustc 1.98.0
cargo 1.98.0
stable-x86_64-pc-windows-msvc
```

如果没有 Rust，可以先安装 rustup，然后安装 MSVC 工具链：

```powershell
winget install Rustlang.Rustup
rustup toolchain install stable --profile minimal
rustup default stable-msvc
```

安装后重新打开终端，使 PATH 生效。Rust 的 `rustup` 可以类比 .NET 的 SDK 管理工具，`cargo` 可以类比 NuGet/MSBuild 的项目构建入口。

### macOS

需要 Node.js、pnpm、Rust、Xcode Command Line Tools 和 Xcode 的 macOS 构建环境。macOS 的 DMG 必须在 macOS 主机或 CI 上生成，Windows 不能直接生成可运行的 macOS 应用。

## 第一次运行

在项目根目录执行：

```powershell
pnpm install
pnpm tauri:dev
```

`tauri:dev` 会自动完成两件事：

- 启动 Vite 开发服务器，默认端口为 `1420`
- 编译并启动 Tauri 原生窗口

修改 Vue、TypeScript 或 CSS 后，界面通常会热更新。修改 Rust 后，Tauri 会重新编译 Rust 部分，第一次重新编译可能需要较长时间。

只运行下面的命令时，只会启动浏览器前端：

```powershell
pnpm dev
```

这种模式适合调整静态界面，但 `invoke`、托盘、通知和文件对话框等 Tauri 原生功能不能正常工作。调试完整应用时使用 `pnpm tauri:dev`。

## 常用命令

```powershell
# 只检查前端类型并构建 dist
pnpm build

# 检查 Rust 代码，不生成安装包
cargo check --manifest-path src-tauri/Cargo.toml

# 运行 Rust 单元测试
cargo test --manifest-path src-tauri/Cargo.toml

# 构建 Windows release 程序和 NSIS 安装包
pnpm tauri:build
```

Windows 构建结果通常在：

```text
src-tauri/target/release/remindon.exe
src-tauri/target/release/bundle/nsis/RemindOn_0.3.1_x64-setup.exe
```

当前项目的便携版 ZIP 是由 release 单文件程序压缩得到的。构建完成后，可以在 PowerShell 中执行：

```powershell
$zip = 'src-tauri/target/release/bundle/RemindOn_0.3.1_x64_portable.zip'
Compress-Archive -Path 'src-tauri/target/release/remindon.exe' -DestinationPath $zip -Force
```

安装版负责安装和卸载入口；便携版只是解压后直接运行。两者的程序逻辑相同，配置都保存在系统应用数据目录，而不是程序所在目录。

## 目录结构

```text
RemindOn/
├─ index.html                 # Vite HTML 入口
├─ package.json               # 前端依赖和 pnpm 命令
├─ pnpm-lock.yaml             # 前端依赖锁定版本
├─ vite.config.ts             # Vite 开发服务器和构建配置
├─ tsconfig.json              # TypeScript 配置
├─ src/                       # Vue 前端
│  ├─ main.ts                 # 创建 Vue 应用
│  ├─ App.vue                 # 主窗口页面、提醒列表、设置页面
│  ├─ i18n.ts                 # 中英文界面文字
│  ├─ types.ts                # 前端数据类型和默认配置
│  ├─ style.css               # 全局界面样式
│  ├─ assets/                 # 软件 Logo 和赞赏码等资源
│  └─ components/
│     └─ ReminderPopup.vue    # 到期软件通知窗口
└─ src-tauri/                 # Tauri/Rust 原生层
   ├─ src/
   │  ├─ main.rs              # Rust 程序入口
   │  └─ lib.rs               # 数据、调度器、托盘和命令实现
   ├─ capabilities/
   │  └─ default.json         # 窗口允许使用的插件权限
   ├─ icons/                  # 应用图标
   ├─ Cargo.toml              # Rust 依赖和 release 优化选项
   ├─ Cargo.lock              # Rust 依赖锁定版本
   ├─ build.rs                # Tauri 构建脚本
   └─ tauri.conf.json         # 窗口、开发服务器和安装包配置
```

`dist/` 和 `src-tauri/target/` 是构建产物，不需要手工修改，也已经被 `.gitignore` 忽略。

## 最常见的修改位置

### 修改页面和交互

- 主页面、提醒表单、列表、设置页：`src/App.vue`
- 软件通知布局和按钮：`src/components/ReminderPopup.vue`
- 颜色、间距、窗口内布局：`src/style.css`
- 前端数据类型：`src/types.ts`

例如，增加一个设置项时，通常需要同时修改：

1. `src/types.ts` 中的 `AppSettings` 和 `defaultData()`。
2. `src/App.vue` 中的设置控件和保存逻辑。
3. `src-tauri/src/lib.rs` 中的 Rust `AppSettings`，确保 JSON 字段能够反序列化。
4. `validate_and_normalize()` 中的默认值或边界校验（如果该设置需要校验）。

### 修改提醒时间规则

调度器在 `src-tauri/src/lib.rs` 中：

- `parse_datetime()`：解析单次提醒的 ISO 8601 时间。
- `parse_time()`：解析每日提醒的 `HH:MM` 时间。
- `next_daily()`：计算每日提醒或关机提醒的下一次时间。
- `next_recurring()`：计算每天、每周、每月、工作日和休息日的下一次时间。
- `validate_and_normalize()`：启动、保存和导入时校验数据。
- `process_due()`：每秒检查一次到期提醒并推进下一次时间。
- `spawn_scheduler()`：启动后台调度线程。

调度器的时间计算应该放在 Rust 侧，前端只负责编辑和展示。修改时间规则后，至少运行：

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

### 增加一个 Rust 原生命令

在 `src-tauri/src/lib.rs` 中定义命令：

```rust
#[tauri::command]
fn example_command(value: String) -> Result<String, String> {
    Ok(format!("收到：{value}"))
}
```

然后把命令加入 `invoke_handler`：

```rust
tauri::generate_handler![
    load_data,
    save_data,
    example_command
]
```

前端调用：

```ts
const result = await invoke<string>('example_command', { value: '测试' })
```

如果命令没有加入 `generate_handler!`，前端会收到“找不到命令”错误。新增插件能力时，还要检查 `src-tauri/capabilities/default.json` 是否授予了对应权限。

### 修改托盘、窗口和系统能力

- 托盘菜单：`tray_menu()`、`setup_tray()` 和 `.on_menu_event(...)`
- 关闭主窗口隐藏到托盘：`RunEvent::WindowEvent` 中的 `CloseRequested`
- 主窗口和提醒窗口尺寸、置顶、是否显示任务栏：`src-tauri/tauri.conf.json`
- 开机启动：前端 `updateAutostart()` 和 Rust 的 `tauri_plugin_autostart`
- 系统通知：Rust `dispatch_trigger()` 和 `tauri_plugin_notification`
- 软件通知：`src/components/ReminderPopup.vue` 和 Rust `dispatch_trigger()`
- 通知方式、主题、语言和强调色：`src/App.vue`、`src/i18n.ts`、`src/components/ReminderPopup.vue` 和 `src/types.ts`
- 权限声明：`src-tauri/capabilities/default.json`

Tauri 配置中有两个窗口：

- `main`：主设置和提醒管理窗口
- `reminder`：隐藏创建、到期时显示的置顶提醒窗口

两个窗口共用同一个 Vue 入口，`window.location.hash === '#/reminder'` 时显示提醒弹窗页面。

## 本地 JSON 数据

配置文件名为 `remindon.json`，由 Rust 的 `app_config_dir()` 决定目录。安装版、便携版和开发版都使用相同的应用标识 `com.remindon.app`，因此不会把设置写进 EXE 所在目录。

常见位置：

```text
Windows: C:\Users\<用户名>\AppData\Roaming\com.remindon.app\remindon.json
macOS:   ~/Library/Application Support/com.remindon.app/remindon.json
```

当前结构：

```json
{
  "version": 3,
  "settings": {
    "language": "zh-CN",
    "autostart": false,
    "minimizeToTray": true,
    "popupAlwaysOnTop": true,
    "restEnabled": false,
    "restIntervalMinutes": 45,
    "restMessage": "休息时间到了，该休息一下了。",
    "notificationMode": "popup",
    "notificationStyle": "standard",
    "theme": "dark",
    "accentColor": "mint",
    "shutdownReminderEnabled": false,
    "powerAction": "shutdown",
    "shutdownReminderTime": "23:30",
    "shutdownReminderMessage": "即将自动关闭电脑。"
  },
  "reminders": [
    {
      "id": "example-id",
      "title": "提交周报",
      "type": "once",
      "triggerAt": "2026-09-03T18:00:00+08:00",
      "time": null,
      "weekdays": [],
      "monthDays": [],
      "enabled": true,
      "nextTriggerAt": "2026-09-03T18:00:00+08:00"
    }
  ]
}
```

提醒类型目前为：

- `once`：使用 `triggerAt`，单次触发后自动禁用。
- `daily`：使用本地时间 `time`，例如 `09:30`。
- `weekly`：使用 `weekdays` 保存一个或多个星期，1 代表周一，7 代表周日。
- `monthly`：使用 `monthDays` 保存一个或多个日期，当月不存在的日期会跳过。
- `workday`：周一至周五提醒。
- `weekend`：周六和周日提醒。
- `interval`：类型已保留在数据模型中，当前界面中的休息提醒由 `settings.restEnabled` 和 `restIntervalMinutes` 管理。

界面中的通知方式可以设置为 `system`（系统通知）或 `popup`（软件通知），默认为 `popup`。软件通知是独立的带标题栏窗口，居中显示，可选简洁、标准或醒目样式。参数设置页的“测试通知”会先保存当前选项，再使用当前通知方式和样式发送测试。

独立的定时操作页可设置三种操作：`shutdown`（自动关机）、`lock`（自动锁定）和 `restart`（自动重启）。到点后会强制显示软件通知，倒计时 60 秒后执行，期间关闭窗口可取消，也可立即执行。如果因休眠等原因错过计划时间超过 1 分钟，操作会跳过，不会补执行。Windows 使用系统自带的 `shutdown.exe` 和 `rundll32.exe`；macOS 使用系统 AppleScript 和 `CGSession`，首次执行时可能要求系统权限。

当前的工作日和休息日按周一至周五、周六至周日计算，不包含中国法定节假日调休数据。

系统通知由 Rust 后端直接调用 Tauri 通知插件，并使用 RemindOn 的应用标识，不通过 PowerShell 脚本发送。软件通知由隐藏的 `reminder` 窗口显示；后端会先居中、置顶并显示窗口，再定向发送提醒内容。

添加、修改、删除、设置变更和导入都会立即保存。保存失败时界面会恢复原设置并显示错误；数据文件损坏或结构不符合当前版本时，程序会先把原文件改名为类似 `remindon.json.corrupt.时间戳` 的备份，再创建默认配置。当前开发阶段不迁移早期版本的数据结构。

## 调试方法

### 调试 Vue/TypeScript

使用：

```powershell
pnpm tauri:dev
```

在 Tauri 开发窗口中打开开发者工具（通常可使用 `F12` 或 `Ctrl+Shift+I`），查看 Console、Network 和元素样式。前端临时日志可以使用 `console.log()`，修改完成后再清理无用日志。

### 调试 Rust

Rust 的标准输出会出现在启动 `pnpm tauri:dev` 的终端中，可以临时使用：

```rust
println!("当前提醒数量：{}", data.reminders.len());
dbg!(&next_trigger);
```

修改 Rust 后可以先快速检查：

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

调度器每秒运行一次，测试时间规则时可以临时把提醒时间设置为当前时间后几十秒。不要把测试代码或过短的生产间隔留在正式版本中。

### 断点调试

前端可以直接使用浏览器开发者工具断点。Rust 断点调试需要配置 Rust 调试器和 IDE；如果只是修改业务规则，先用 `cargo test` 覆盖纯函数通常更快。当前单元测试位于 `src-tauri/src/lib.rs` 底部。

## 常见问题

### `cargo` 或 `rustc` 不是命令

重新打开终端；如果仍然找不到，确认 rustup 已加入 PATH，并执行：

```powershell
rustup default stable-msvc
```

Windows 编译失败且提示 linker 或 MSVC 缺失时，安装 Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载。

### 前端能打开，但提醒、托盘或导入导出报错

通常是运行了 `pnpm dev` 而不是 `pnpm tauri:dev`。这些功能必须在 Tauri 原生窗口中运行。

### 端口 1420 被占用

结束占用端口的旧 Vite/Node 进程后重试。端口定义同时出现在 `vite.config.ts` 的 `server.port` 和 `src-tauri/tauri.conf.json` 的 `devUrl`，修改时需要保持一致。

### 修改了 Rust 命令但前端提示命令不存在

检查三个地方：命令函数上的 `#[tauri::command]`、`tauri::generate_handler![...]` 注册项、前端 `invoke()` 使用的命令名和参数名。

### 修改源码后，release 目录里的程序没有变化

`src-tauri/target/release/remindon.exe` 是上一次构建生成的成品，不会随着源码保存自动更新。开发时运行 `pnpm tauri:dev`；需要新的正式程序时重新运行 `pnpm tauri:build`。

### 重启后看起来设置丢失

先确认运行的是刚构建的 `0.3.1`，并完全退出托盘中的旧实例。安装版、便携版和开发版共享上面列出的标准配置文件；如果同时保留不同开发版本，旧程序可能用旧数据结构重写同一个文件。新版保存失败时会在页面显示原因，不会只改变界面而不提示。

### 想清空所有本地数据

先退出 RemindOn，再备份并删除系统应用数据目录中的 `remindon.json`。下次启动会自动生成默认配置。删除前请确认路径只指向 RemindOn 的配置文件。

## 推荐开发流程

1. 先修改 `src/App.vue`、组件或 CSS，使用 `pnpm tauri:dev` 看界面效果。
2. 如果需要系统能力，再修改 `src-tauri/src/lib.rs` 或 Tauri 插件配置。
3. 前后端数据模型变更时，同时检查 `src/types.ts`、Rust 结构体和 JSON 导入校验。
4. 运行 `pnpm build`、`cargo check` 和 `cargo test`。
5. 最后运行 `pnpm tauri:build`，在 Windows 验证安装包；macOS 构建放到 macOS 主机或 CI 验证。

项目刻意保持本地、轻量和功能简单。新增功能时，优先沿用现有 Vue、Tauri 命令和 Rust 调度器结构，不要为了一个页面引入数据库、服务器或复杂状态管理库。
