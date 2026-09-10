# 官网截图

本目录只保留四张无损 WebP 截图：

| 文件名 | 软件语言 | 软件主题 |
| --- | --- | --- |
| `overview-zh-dark.webp` | 简体中文 | 深色 |
| `overview-zh-light.webp` | 简体中文 | 浅色 |
| `overview-en-dark.webp` | English | 深色 |
| `overview-en-light.webp` | English | 浅色 |

当前四张均为 1560×1080，转换时已验证 WebP 与 PNG 解码后的像素完全一致，总体积减少约 61%。PNG 原图不保留。

## 如何截图

1. 打开 RemindOn，准备两三条不含个人信息的示例提醒，例如喝水、起身活动、每周整理；避免设置马上会触发的提醒。
2. 在“参数设置”中选择薄荷绿强调色，依次设置截图所需的语言和主题。英文截图的示例提醒内容也请使用英文，例如 Drink some water、Take a short break、Weekly reset；用户填写的内容不会自动翻译。
3. 回到“事件提醒”页面，关闭新增或编辑弹窗。四张保持相同窗口大小，当前网页图片尺寸为 1560×1080。
4. 使用系统截图工具框选完整软件窗口，包含标题栏，但不包含桌面、其他窗口或鼠标悬停提示。
5. 先保存临时 PNG，不添加背景、阴影或额外边框。无损转换为上表指定的 WebP 后，删除临时 PNG。

可使用 Windows 的 `Win + Shift + S` 框选窗口。截图后不要只改其他格式的扩展名为 `.png`，应保存实际 PNG 文件。

更新图片时，将 PNG 无损导出为同名 WebP，并保持 1560×1080 尺寸；若改变尺寸，需同步修改 `site/template.mjs` 中图片的 `width`、`height`。可用支持无损 WebP 的图片工具，或使用已安装的 `cwebp -lossless -m 6 input.png -o output.webp`，不要只更改扩展名。

放好图片后，从项目根目录重新运行 `node site/preview.mjs`，访问 <http://127.0.0.1:4173/RemindOn/>。页面会根据语言和网站深浅主题选择对应图片，支持跟随系统或手动切换。每种语言的两张 WebP 主题截图齐全后，才会替换该语言的占位区域。正式构建要求四张 WebP 齐全。
