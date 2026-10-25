# 桌面

Tauri 2。查词由 Tauri 的 Rust 部分（`src-tauri/`）直接调用 `zdict-core`，完全离线。界面用 Svelte 5 + TypeScript（Vite）写在 `src/`，只运行在 Tauri 的 WebView 里，不单独部署。

词条的 HTML 由 Rust 生成（`crates/zdict-html`，输入是核心的渲染树），界面只负责插入页面和处理点击，不在 TypeScript 里实现主题或排版规则。

阶段：`docs/roadmap.md` M3。

## 界面与 Rust 之间的命令

所有命令用 Tauri `invoke` 调用，参数和返回值是 JSON。字段名用 snake_case，与 `zdict-core` 的类型一致。

| 命令 | 参数 | 返回 |
| --- | --- | --- |
| `pack_info` | — | `{ id, title, entries, source_lang, target_lang }`；没有打开的包时报错 `no_pack` |
| `open_pack` | `{ path }` | 同 `pack_info`；成功后记住路径 |
| `suggest` | `{ text, limit }` | `Suggestion[]`：`{ form, kind, entry, headword, homograph, anchor }` |
| `correct` | `{ text, limit }` | `Correction[]`：`Suggestion` 的字段加 `distance` |
| `search` | `{ text, limit, column }`，`column` 为 `null`、`"text"`、`"example"` | `Hit[]`：`{ entry, headword, homograph, anchor, lang, score }` |
| `entry` | `{ key }` | `{ key, headword, homograph, html }` |
| `follow` | `{ current, link }`，`link` 为 `{ entry?, anchor?, anchor_type?, text }` | `{ kind: "entry", key, focus }` 或 `{ kind: "lookup", text }`；`focus` 为 `{ kind: "top" }`、`{ kind: "anchor", number }`、`{ kind: "word_family" }` |
| `settings` | — | `Settings` |
| `set_settings` | `{ settings }` | `Settings`；之后的 `entry` 按新设置渲染 |
| `theme_css` | — | 字符串：主题的字体与颜色变量（浅色、深色两套）和 `.dx-*` 基础样式 |

`Settings`：`{ hide_chinese: bool, hide_pronunciation: bool, hide_examples: bool, font_scale: number }`。前三项在 Rust 里变成叠加主题（`spec/v0.md` §7.2）；`font_scale` 由界面用 CSS 变量实现。设置和包的路径保存在应用配置目录的 `config.json`。

错误：命令失败时 reject，值为 `{ code, message }`，`code` 为 `no_pack`、`not_found`、`bad_input`、`internal` 之一。

## 词条 HTML 的约定

`entry` 返回的 HTML 中：

- 带锚点的节点有 `data-anchor="<编号>"`（与联想、检索结果中的 `anchor` 相同，0 是词条本身）；
- 词族节点的 class 是 `dx-word_family`；
- 发音按钮是 `<button class="dx-icon" data-audio="<url> <url>…">`，按顺序回退；
- 图片是 `<img src="<首选>" data-fallback="<url>…">`；
- 链接是 `<a href="#" class="dx-link" data-link="<link 的 JSON，HTML 转义>">`，点击后取 `dataset.link` 解析成 JSON 交给 `follow`（并阻止默认跳转）。

CSS：`theme_css` 只给主题的变量和 `.dx-*` 规则；页面自己定义 `--page`、`--card`（箭头图标用 `--card` 镂空）和正文字号。
