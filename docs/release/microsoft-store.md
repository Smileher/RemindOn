# RemindOn 首次发布 Microsoft Store 操作指南

本指南适用于 Tauri 2 的 Windows 桌面程序，通过官方 Windows SDK 打包为 MSIX，再上传 Microsoft Store。Tauri 使用 Rust 而非 .NET，不影响 MSIX 打包。MSIX 是应用文件、包身份、图标和 Manifest 的容器，不要求应用使用某种编程语言。

## 1. 先理解本项目的产物

| 产物 | 用途 | 上传到哪里 |
| --- | --- | --- |
| `RemindOn_0.9.0_x64-setup.exe` | 普通 NSIS 安装版 | GitHub / Gitee Release |
| `RemindOn_0.9.0_x64_portable.exe` | 普通便携版 | GitHub / Gitee Release |
| `RemindOn_1.8.0.0_x64.msix` | 商店版 x64 单架构包 | 可单独上传 Partner Center |
| `RemindOn_1.8.0.0_arm64.msix` | 商店版 ARM64 单架构包 | 可单独上传 Partner Center |
| `RemindOn_1.8.0.0_bundle.msixbundle` | 含 x64、ARM64 的商店版集合 | **推荐上传这一文件** |

商店版构建时设置 `REMINDON_STORE_BUILD=1`，不显示普通版的 GitHub/Gitee 更新入口，不进行其自动检查。微软商店负责该渠道的安装和更新。

**不要上传普通 NSIS EXE、便携 EXE、GitHub 的 `latest.json` 或 Actions artifact 的 ZIP 到 MSIX 包页面。** ZIP 下载后先解压，上传里面的 `.msixbundle`。本项目不生成 `.msixupload`；对于这里的桌面应用，可以直接上传 `.msixbundle`。

当前实现状态：已有手动 Actions 构建及 PowerShell 打包脚本；尚未接入自动提交 Partner Center。使用本机现有 EXE 和测试身份通过了 x64 MakeAppx 打包校验，这仅验证打包脚本与 Manifest，不是可提交的正式商店版。真实商店身份构建、ARM64 构建、安装、升级和商店审核仍需实际验证。

## 2. 在 Partner Center 确认产品类型与身份

1. 登录 <https://partner.microsoft.com/dashboard>，进入管理 Windows 应用的区域，找到已创建的 RemindOn 产品。
2. 确认这个产品支持上传 MSIX/Appx 包。如果页面只让你填写 EXE/MSI 安装器下载 URL，说明当前是另一种分发流程；先确认是否能迁移到 MSIX 产品，不要把 MSIX 的身份直接套用到那个流程。
3. 进入产品概览中的“产品管理 / Product management”，打开“产品标识 / Product identity”或“查看应用标识详细信息 / View app identity details”。入口名称可能随网页语言和版本变化。
4. 记录下面三个值，**完整复制，不改大小写、空格或标点**。

| Partner Center 字段 | GitHub Variable |
| --- | --- |
| `Package/Identity/Name` | `MSIX_IDENTITY_NAME` |
| `Package/Identity/Publisher` | `MSIX_PUBLISHER` |
| `Package/Properties/PublisherDisplayName` | `MSIX_PUBLISHER_DISPLAY_NAME` |

`Publisher` 通常是 `CN=...`，不是开发者昵称。`Identity Name` 不是商店展示名，也不是 Rust 的包名。不要把它们改成自己猜测的 `com.remindon.app`。该 Tauri identifier 和 Store 包身份可以分别存在。

这三个值不是私钥，可以提供以便检查配置；PFX 密码、客户端密钥及 Gitee token 不要发到聊天中。

## 3. 配置 GitHub 构建变量

1. 打开 <https://github.com/Smileher/RemindOn>。
2. 点击 `Settings → Secrets and variables → Actions`。
3. 切换到 **Secrets** 页签。
4. 点击 `New repository secret`，分别添加上一节的三个 Secret。当前工作流从 Secrets 读取这些值，名称必须完全一致。

仅为上传 Microsoft Store 构建时，**不用配置** `MSIX_PFX_BASE64` 和 `MSIX_PFX_PASSWORD`。商店会在审核通过后重新签名，不要求你购买 CA 代码签名证书。

两个 PFX Secret 仅用于可选的本地签名测试。配置时必须同时存在；证书 Subject 必须与 `MSIX_PUBLISHER` 完全匹配，测试设备还必须信任该证书。Tauri updater 私钥不适用于 MSIX 签名，不要混用。

## 4. 选择商店包版本号

微软文档对 Windows 商店包版本的规则为：

- 四段数字，每段不超过 `65535`。
- 第一段不能为 `0`。
- 第四段保留给商店，自己构建时必须是 `0`。
- 给现有用户发布更新时，新包版本应高于他们已有的对应架构包。

因此之前的 `0.8.0.0` 示例应改正。本指南采用 **`1.8.0.0`** 演示首次上传，前提是当前产品没有更高的已有版本。后续可以用 `1.8.1.0`、`1.9.0.0`；这只是示例，不是流水线自动映射规则。

应用自身的产品版本仍然可以是 `0.9.0`。普通版的 `v0.9.0` tag 和商店的 `1.8.0.0` MSIX 包版本用途不同。手动商店工作流的 version 输入只设置 MSIX 版本，不会修改 `package.json`、Cargo 或应用内显示的版本。

## 5. 推荐：在 GitHub Actions 一键构建

首次发布使用这个方式即可，不需要在本机安装额外打包工具。

1. 确认代码已经推送到 GitHub 的 `master`，其中包含 `.github/workflows/store.yml`。
2. 打开仓库 `Actions`。
3. 左侧选择 **Microsoft Store MSIX**。
4. 点击右侧 `Run workflow`。
5. Branch 选准备提交的代码所在分支，通常是 `master`。
6. version 输入准备使用的商店包版本，例如 `1.8.0.0`。
7. 点击 `Run workflow`，打开新出现的运行记录。
8. 等待 `package (x64)`、`package (arm64)` 和 `bundle` 三部分全部成功。
9. 在运行详情底部的 **Artifacts** 中下载 **`remindon-msix-store`**。
10. 解压下载的 ZIP，得到 **`RemindOn_1.8.0.0_bundle.msixbundle`**，这就是后续上传的文件。

工作流会完成：安装 Node/pnpm/Rust、构建商店专用 EXE、分别打包两种架构、生成 MSIXBundle、上传 Actions artifact。它不创建普通版 Release，也不自动提交商店审核。

如果某个 job 失败，先打开红色步骤查看日志。没有成功的 bundle artifact 就不要继续提交。ARM64 交叉编译还需要 runner 的 ARM64 MSVC C++ 工具链，真实构建失败时应根据日志补齐工具链。

## 6. 本地构建方式与文件位置

相关脚本位于项目根目录下：

- `scripts/package-msix.ps1`：将一个架构的 EXE、图标和 Manifest 打成 `.msix`。
- `scripts/bundle-msix.ps1`：将两个架构的 `.msix` 合成 `.msixbundle`。
- `.github/workflows/store.yml`：完成以上步骤的云端入口。

**打包脚本本身不编译应用。** 本地完整过程是先构建 EXE，再执行脚本。准备：Node.js、pnpm、Rust、Visual Studio C++ Build Tools、Windows SDK；构建 ARM64 还需对应 MSVC ARM64 编译器和库。

下面在项目根目录的 PowerShell 执行。三项身份值必须先填写成 Partner Center 的真实值；示例占位符不能直接提交。

```powershell
$identityName = '替换为真实 Package/Identity/Name'
$publisher = '替换为完整 Package/Identity/Publisher'
$publisherDisplayName = '替换为真实 PublisherDisplayName'
$storeVersion = '1.8.0.0'

pnpm install --frozen-lockfile
rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
```

构建 x64。环境变量在本次构建后恢复，避免后续把普通版误构建为商店版：

```powershell
$previousStoreBuild = $env:REMINDON_STORE_BUILD
try {
  $env:REMINDON_STORE_BUILD = '1'
  pnpm tauri build --target x86_64-pc-windows-msvc --no-bundle --ci -- --locked
  if ($LASTEXITCODE -ne 0) { throw 'x64 构建失败' }
} finally {
  $env:REMINDON_STORE_BUILD = $previousStoreBuild
}

./scripts/package-msix.ps1 `
  -Architecture x64 `
  -Version $storeVersion `
  -IdentityName $identityName `
  -Publisher $publisher `
  -PublisherDisplayName $publisherDisplayName `
  -SourceExecutable 'src-tauri/target/x86_64-pc-windows-msvc/release/remindon.exe' `
  -IconPath 'src-tauri/icons/128x128.png' `
  -OutputDirectory 'store-input'
```

构建 ARM64：

```powershell
$previousStoreBuild = $env:REMINDON_STORE_BUILD
try {
  $env:REMINDON_STORE_BUILD = '1'
  pnpm tauri build --target aarch64-pc-windows-msvc --no-bundle --ci -- --locked
  if ($LASTEXITCODE -ne 0) { throw 'ARM64 构建失败' }
} finally {
  $env:REMINDON_STORE_BUILD = $previousStoreBuild
}

./scripts/package-msix.ps1 `
  -Architecture arm64 `
  -Version $storeVersion `
  -IdentityName $identityName `
  -Publisher $publisher `
  -PublisherDisplayName $publisherDisplayName `
  -SourceExecutable 'src-tauri/target/aarch64-pc-windows-msvc/release/remindon.exe' `
  -IconPath 'src-tauri/icons/128x128.png' `
  -OutputDirectory 'store-input'

./scripts/bundle-msix.ps1 `
  -Version $storeVersion `
  -InputDirectory 'store-input' `
  -OutputDirectory 'store-output'
```

最终文件在 **`store-output/RemindOn_1.8.0.0_bundle.msixbundle`**。`store-input` 应只保留本次同一身份、同一版本的两份 MSIX；脚本发现不是恰好两份会失败。

Windows SDK 工具由脚本自动查找。打包时 MakeAppx 会执行 Manifest 校验；不要加 `/nv` 跳过校验。未签名的商店上传包不能直接双击安装，应另做签名测试或使用商店测试分发。

## 7. 在 Partner Center 创建首次提交

回到 RemindOn 产品概览，点击“开始提交 / Start your submission”或“创建新提交 / Create a new submission”。如果已经存在草稿，打开该草稿。

页面通常分成下面几个部分。首次都需要完成，后续更新大部分内容可以沿用。网页必填标记和当前规则优先于示例。

### 7.1 定价与可用性 / Pricing and availability

- 价格：免费软件选择 **Free**。
- 市场：选择希望发布的国家和地区，至少包含实际目标市场。
- 受众：公开发布选择 **Public audience**；若先进行受控验证，可以使用适用的私人受众或测试分发功能。
- 可发现性：公开版本选择可在 Store 中发现。
- 发布时间：首次建议审核通过后尽快发布；需要自行确认上线时，可以设置发布保留。

### 7.2 属性 / Properties

- 类别：本应用可以考虑 **Productivity / 效率**，以最终实际用途为准。
- 子类别：如页面要求，选最接近提醒、时间管理的现有选项。
- 网站：可以填写项目官网 `https://smileher.github.io/RemindOn/`。
- 支持地址：可以填写项目 Issues 地址 `https://github.com/Smileher/RemindOn/issues`，也可以使用自己的支持邮箱。
- 隐私政策：填写本项目公开的隐私政策页面：<https://github.com/Smileher/RemindOn/blob/master/docs/privacy-policy.md>。该页面包含中文和英文版本。提交前确认链接在未登录 GitHub 的浏览器中也能打开。
- 系统要求：说明 Windows 10 1809 或更高版本、x64 或 ARM64、需要 Microsoft Edge WebView2 Runtime。MSIX 当前不负责下载 WebView2；尤其要在干净 Windows 10 环境验证。

隐私政策应描述实际版本的数据行为，例如提醒设置保存于本地、导入导出由用户操作，以及是否存在联网、统计或广告。应与真正提交的包和第三方组件一致。

### 7.3 年龄分级 / Age ratings

完成 IARC 问卷。按应用真实内容回答，不要只因为它是效率工具就跳过问题。暴力、赌博、用户生成内容、购买行为等都按实际情况填写。系统会生成分级结果。

### 7.4 程序包 / Packages

1. 上传解压得到的 **`RemindOn_1.8.0.0_bundle.msixbundle`**。
2. 等待网页验证完成。
3. 核对包身份、发布者、版本和架构；应看到 x64、ARM64。
4. 核对目标设备族为 Windows Desktop。
5. 保存，并完成该页面其他必填配置。单个包显示 `Validated` 不代表整个 Packages 页面已经完成。

常见错误：

| 网页错误 | 检查内容 |
| --- | --- |
| Identity Name 不匹配 | `MSIX_IDENTITY_NAME` 是否来自当前产品 |
| Publisher 不匹配 | 是否完整复制 `CN=...`，包括大小写与标点 |
| PublisherDisplayName 不匹配 | 第三个 Variable 是否来自当前开发者身份 |
| 版本无效或不能更新 | 第一段是否非零、第四段是否为零、是否高于已发布版本 |
| 不支持的包类型 | 产品是否采用 MSIX 流程，是否错误上传了 ZIP/EXE |
| 缺少架构 | 两个 package job 是否都成功，是否上传了正确 bundle |

### 7.5 商店描述 / Store listings

至少完成一种支持语言。当前包声明中文简体和英语，可以先完成简体中文，再补英语。

- 应用名称：使用已经为该产品保留的名称。
- 描述：准确介绍提醒、休息间隔、托盘和本地设置等真实功能。
- 本版更新说明：例如“首次发布 Windows 商店版，支持 x64 和 ARM64”。
- 截图：至少一张真实应用截图，建议提供事件提醒、休息提醒、设置和提醒弹窗。尺寸与格式按网页提示准备；不要上传含私人提醒内容的截图。
- 商店图标：按网页要求上传合适尺寸的图片；MSIX 内的 50px StoreLogo 不等于网页宣传图标。
- 关键词、支持和联系信息：按实际情况填写。

中文描述参考，提交前确认其对应实际包：

> RemindOn 是一款轻量桌面提醒工具，支持事件提醒、定时休息和定时操作。应用可常驻系统托盘，支持中英文、主题设置以及提醒配置的导入和导出。提醒设置保存在本地，无需创建应用账号。

### 7.6 提交选项 / Submission options

本项目 Manifest 使用 `runFullTrust`，这是把 Tauri 桌面程序作为普通用户进程运行所需的受限能力。出现能力用途说明时，可以填写以下说明，发布前按实测结果修正：

> RemindOn is a Tauri desktop reminder application. It requires runFullTrust to run its native desktop process, maintain its system tray icon, schedule reminders, display desktop windows and notifications, and execute user-configured lock, shutdown, or restart actions with a cancellable countdown. It runs with the current user's privileges and does not request administrator elevation.

审核备注可说明：无需登录；如何创建短时间提醒；关闭主窗口会进入托盘；从托盘退出；普通版的内置更新入口在商店版已禁用。不要在审核备注中放证书、token 或 API 密钥。

如选择“审核通过后自动发布”，审核成功后上线。如选择保留发布，则审核通过后还需手动点发布。

### 7.7 提交审核

确认所有必填部分完成，再点击 **Submit for certification / 提交认证**。等待状态从预处理、认证到发布。审核失败时根据具体报告处理；包网页校验通过不等于认证通过。

## 8. 发布前必须实际验证的桌面行为

当前尚未对真实身份 MSIX 做完下面的测试，打包校验不能代替它们：

- 在 x64 和 ARM64 设备上安装并打开应用。
- 干净系统的 WebView2 运行时是否齐备。
- 从开始菜单启动、主窗口显示、托盘菜单、关闭后常驻和完全退出。
- 提醒弹窗、系统通知、通知点击以及用户配置的定时操作。
- 商店版关于页没有普通更新按钮，启动不请求 GitHub/Gitee 更新。
- 提醒设置保存、重启后恢复、导入导出及 MSIX 文件系统虚拟化对数据位置的影响。
- 与普通版同时安装时的单实例行为。
- 从旧 MSIX 升级到同一身份的新 MSIX，确认数据保留。
- 使用 Windows App Certification Kit 检查实际包。

特别注意开机自启：现有插件使用普通桌面应用的机制，当前 MSIX Manifest 没有声明 `StartupTask`。不能据此承诺商店版开机自启可用；如实测失败，应单独确认采用 MSIX 对应机制的修改范围，不要通过提升权限来绕过。

本地安装测试通常需要同一身份的已签名包以及可信的签名证书，安装后可以用 `Get-AppxPackage` 检查身份。开发者自签名测试证书不应要求普通用户安装；正式用户从 Microsoft Store 获取由微软签名的包。

## 9. 后续更新与自动提交

首次成功发布后，后续手动更新只需：构建递增版本的 bundle、创建新提交、替换 Packages、填写更新说明、提交审核。Store 用户继续由 Store 更新，不会被 GitHub 普通版覆盖。

要自动提交，后续再接入官方 Microsoft Store submission API，需要：已完成的首次提交和年龄分级、Entra 租户、具有 Partner Center Manager 权限的 Entra 应用、Tenant ID、Client ID、Client Secret 和 Store application ID。

这些 API 凭据与 MSIX 的 Package Identity Name/Publisher 不是同一组值。当前工作流没有调用该 API，单纯设置这些 Secret 不会启用自动提交。自动提交仍然需要微软审核，不能绕过。

## 10. 官方参考

- MSIX 提交页面及必填项：<https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission>
- 包类型、身份、签名与版本规则：<https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements>
- 自动提交 API 前置条件：<https://learn.microsoft.com/en-us/windows/uwp/monetize/create-and-manage-submissions-using-windows-store-services>
- 自动提交 API 流程：<https://learn.microsoft.com/en-us/windows/uwp/monetize/manage-app-submissions>
