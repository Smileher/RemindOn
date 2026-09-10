# RemindOn 宣传官网

官网由 Node.js 24 内置能力生成，无需安装额外依赖。中文路径为 `/RemindOn/`，英文路径为 `/RemindOn/en/`。所有样式、图标和截图由本站提供。主题默认跟随系统，也可在导航栏选择浅色或深色；选择保存在浏览器本地，跨语言页面共用，并同步切换软件截图。禁用 JavaScript 时仍可跟随系统主题、切换语言、展开 FAQ 和下载软件。

## 本地预览

在项目根目录运行：

```sh
node site/preview.mjs
```

打开 <http://127.0.0.1:4173/RemindOn/>。预览命令先查询 GitHub 最新正式 Release，再构建至已被仓库忽略的 `site/dist/`，然后启动仅监听本机的服务器。需要能够访问 `api.github.com`；遇到匿名 API 限流时可通过环境变量 `GH_TOKEN` 提供只读令牌，不要将令牌写入源码。

修改页面、样式或放入截图后，用 `Ctrl+C` 停止并重新运行命令，再刷新浏览器。不需要重启桌面软件。

本地预览允许缺失截图并显示占位。截图位置和操作步骤见 [截图说明](assets/screenshots/README.md)。

## 验证与正式构建

```sh
node --test site/tests/*.test.mjs
node site/build.mjs
```

正式构建要求四张 WebP 截图齐全。源码和网站均只保留无损 WebP，每张约 37–40 KB。可单独运行 `node site/build.mjs --preview` 生成带占位的本地预览产物；Pages 工作流始终使用正式构建。

版本号、发布日期、文件大小及下载地址由 GitHub API 的最新正式 Release 一次性提供，不从 `package.json` 回退，不在访客浏览器中调用 API。仅使用 Windows x64 NSIS 安装包、Windows x64 便携 EXE 和 Apple Silicon DMG；签名文件和应用更新专用压缩包不会作为下载按钮。

接口失败、版本数据异常、必要附件缺失或截图不完整都会使正式构建失败，不上传新 Pages 产物，不替换线上页面。

## 发布

发布前先检查页面与下载信息：

1. 在 GitHub 仓库 **Settings → Pages → Build and deployment → Source** 中选择 **GitHub Actions**。
2. 将已确认的变更提交并推送至 `master`，或手动运行 **Pages** 工作流。
3. 确认部署成功，访问 <https://smileher.github.io/RemindOn/> 及英文页面，核对截图、链接和安装包。

网站或引用的品牌图标变更推送至 `master` 时会重新部署。现有 **Release** 工作流成功完成后，Pages 通过 `workflow_run` 触发并从 `master` 读取网站代码，重新查询最新正式版本。该方式兼容 Release 使用 `GITHUB_TOKEN` 发布的现状，无需调整现有发布工作流或添加签名密钥。

如果 Pages 构建失败，原页面继续提供上一次成功部署的版本；修复后可手动重跑 **Pages**。预发布和草稿不会被当作最新版使用。

## 维护

- 中英文文案：`content.mjs`；共享页面结构：`template.mjs`；深浅主题及响应式样式：`style.css`。
- 基础路径与正式域名定义在 `template.mjs`。首版固定使用 GitHub 项目 Pages 地址，不配置自定义域名。
- 构建时复制现有应用 Logo 和 PNG 图标，无需维护另一套品牌资源。
- 发布前检查 375px、768px、1440px 宽度，中英文与两种系统主题，以及四张截图是否清晰一致。
