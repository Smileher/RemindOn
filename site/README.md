# RemindOn 宣传官网

官网由 Node.js 24 内置能力生成，无需安装额外依赖。视觉采用"代码玻璃"风格：深浅两套主题基于 CSS 自定义属性切换，悬浮胶囊导航、玻璃拟态卡片、渐变强调色。Hero 区的应用界面由纯 HTML/CSS 绘制，布局与文案跟随软件最新界面（`src/components/AppSidebar.vue` 等），不使用任何位图截图。中文路径为 `/RemindOn/`，英文路径为 `/RemindOn/en/`。所有样式、图标由本站提供，不加载外部字体或脚本。主题默认跟随系统，也可在导航栏选择浅色或深色；选择保存在浏览器本地，跨语言页面共用。禁用 JavaScript 时仍可跟随系统主题、切换语言、展开 FAQ 和下载软件（渐显动效自动关闭）。

## 本地预览

在项目根目录运行：

```sh
node site/preview.mjs
```

打开 <http://127.0.0.1:4173/RemindOn/>。预览命令先查询 GitHub 最新正式 Release，再构建至已被仓库忽略的 `site/dist/`，然后启动仅监听本机的服务器。需要能够访问 `api.github.com`；遇到匿名 API 限流时可通过环境变量 `GH_TOKEN` 提供只读令牌，不要将令牌写入源码。

修改页面或样式后，用 `Ctrl+C` 停止并重新运行命令，再刷新浏览器。不需要重启桌面软件。

## 验证与正式构建

```sh
node --test site/tests/*.test.mjs
node site/build.mjs
```

版本号、发布日期、文件大小及 GitHub 下载地址由 GitHub API 的最新正式 Release 一次性提供，不从 `package.json` 回退，不在访客浏览器中调用 API。页面展示 Windows x64/ARM64 安装包与便携 ZIP、Apple Silicon DMG，并提供 Microsoft Store 官方渠道入口；便携下载必须提供 ZIP，签名文件和应用更新专用压缩包不会作为下载按钮。

接口失败、版本数据异常或必要附件缺失都会使正式构建失败，不上传新 Pages 产物，不替换线上页面。

## 发布

发布前先检查页面与下载信息：

1. 在 GitHub 仓库 **Settings → Pages → Build and deployment → Source** 中选择 **GitHub Actions**。
2. 将已确认的变更提交并推送至 `master`，或手动运行 **Pages** 工作流。
3. 确认部署成功，访问 <https://smileher.github.io/RemindOn/> 及英文页面，核对链接和安装包。

网站或引用的品牌图标变更推送至 `master` 时会重新部署。现有 **Release** 工作流成功完成后，Pages 通过 `workflow_run` 触发并从 `master` 读取网站代码，重新查询最新正式版本。该方式兼容 Release 使用 `GITHUB_TOKEN` 发布的现状，无需调整现有发布工作流或添加签名密钥。

如果 Pages 构建失败，原页面继续提供上一次成功部署的版本；修复后可手动重跑 **Pages**。预发布和草稿不会被当作最新版使用。

## 维护

- 中英文文案：`content.mjs`；共享页面结构：`template.mjs`；深浅主题及响应式样式：`style.css`。
- 功能区保留四项：休息提醒、个性化弹窗、定时提醒、桌面集成；锁屏、关机和重启并入定时提醒说明。下载区使用对称的平台主入口，下方提供 Windows 安装版与便携版。
- `downloads.js` 优先使用 Windows UA Client Hints，再检查明确的 ARM64 UA 标记，无法确定时默认 x64。手动切换同时控制安装版和便携版，优先于异步识别结果；无 JavaScript 时展示所有架构链接。
- `assets/store-{zh,en}-{dark,light}.svg` 来自微软官方 `https://get.microsoft.com/images/{zh-cn,en-us}%20{dark,light}.svg`，保持原始内容与比例。官网徽标跟随页面主题，项目 README 使用 `<picture>` 跟随系统主题；构建时复制到本地资源目录，无需访客访问外部徽标服务器。
- 软件界面调整后，同步更新 `content.mjs` 中各语言的 `mockup` 文案与 `template.mjs` 的 mockup 结构，使展示与真实界面保持一致。
- 文案强调“记得休息，早点收工。” / “Take breaks. Call it a day.”，采用温和但明确的语气；中英文使用自然表达，不强调过度工作或把系统操作作为独立核心功能。单纯发版无需更换界面截图。
- 基础路径与正式域名定义在 `template.mjs`。首版固定使用 GitHub 项目 Pages 地址，不配置自定义域名。
- 构建时复制现有应用 Logo 和 PNG 图标，无需维护另一套品牌资源。
- 发布前检查 375px、768px、1440px 宽度，以及中英文与两种系统主题。
