# 桌面

Tauri 2。查词由 Tauri 的 Rust 部分（`src-tauri/`）直接调用 `zdict-core`，完全离线。界面用 Svelte 5 + TypeScript（Vite）写在 `src/`，只运行在 Tauri 的 WebView 里，不单独部署。

词条的 HTML 由 Rust 生成（`crates/zdict-html`，输入是核心的渲染树），界面只负责插入页面和处理点击，不在 TypeScript 里实现主题或排版规则。

阶段：`docs/roadmap.md` M3。

## 界面与 Rust 之间的命令

所有命令用 Tauri `invoke` 调用，参数和返回值是 JSON。字段名用 snake_case，与 `zdict-core` 的类型一致。

| 命令 | 参数 | 返回 |
| --- | --- | --- |
| `pack_info` | — | `{ id, title, entries, source_lang, target_langs, switches }`，`switches` 为 `{ pronunciation, example }`：包里有带该 role 的类型时为 true，设置只显示这些开关；没有打开的包时报错 `no_pack` |
| `library` | — | `Library`：`{ active: string 或 null, packs: LibraryItem[] }`；每项为 `{ path, title, entries, bytes, error }`，`path` 为字符串，`title` 为字符串或 null，`entries` 和 `bytes` 为数字或 null，`error` 为字符串或 null；无法打开的包仍保留在列表中，并返回错误原因 |
| `add_packs` | `{ paths: string[] }`（支持对话框多选） | `{ library: Library, rejected: [{ path, message }] }`；验证后按顺序追加，比较并保存规范路径；重复、无效路径进入 `rejected`；未选择活动包时打开首个新增包 |
| `remove_pack` | `{ path }` | `Library`；仅移出列表，文件保留；移除活动包后打开列表中首个能打开的包，否则 `active` 为 null |
| `move_pack` | `{ path, index }`，`index` 从 0 开始，超出范围时移到末尾 | `Library`；只改变显示顺序 |
| `use_pack` | `{ path }` | 同 `pack_info`；打开库中该包并设为活动包，不在库中时报错 `not_found` |
| `words` | `{ text, limit }`，`limit` 是每组的行数上限 | 词表（`spec/pack-v1.md` §5）：`{ group, form, kind, entry, headword, homograph, anchor, readings, summary, summary_lang, distance, rank, target }[]`；`group` 为 `prefix`、`contains`、`close`，`kind` 为索引种类，`target` 是 variant、inflection 指向的词头（与 `form` 相同时为 null） |
| `entry` | `{ key }` | `{ key, headword, homograph, html }` |
| `follow` | `{ current, link }`，`link` 为 `{ entry?, anchor?, anchor_type?, text }` | `{ kind: "entry", key, focus }` 或 `{ kind: "lookup", text }`；`focus` 为 `{ kind: "top" }`、`{ kind: "anchor", number }`、`{ kind: "word_family" }` |
| `settings` | — | `Settings` |
| `set_settings` | `{ settings }` | `Settings`；之后的 `entry` 按新设置渲染 |
| `theme_css` | — | 字符串：主题的字体与颜色变量（浅色、深色两套）和 `.dx-*` 基础样式 |

`Settings`：`{ hide_pronunciation: bool, hide_examples: bool, font_scale: number }`。前两项隐藏带 `pronunciation`、`example` role 的类型（`spec/v1.md` §3.2），在 Rust 里变成包主题之上的叠加主题；`font_scale` 由界面用 CSS 变量实现。设置和库保存在应用配置目录的 `config.json`：`{ "packs": ["/a.zdpack", "/b.zdpack"], "active": "/a.zdpack", "settings": { "hide_pronunciation": false, "hide_examples": false, "font_scale": 1.0 } }`。`active` 必须是 `packs` 中的路径或 null；旧格式 `"pack": "/x.zdpack"` 读取为单项库和活动路径，之后仅写新格式。`--pack` / `ZDICT_PACK` 将路径加入库并激活。`zdict serve` 用相同命令和 JSON（`POST /<command>`），库只存在于内存，不保存配置。

错误：命令失败时 reject，值为 `{ code, message }`，`code` 为 `no_pack`、`not_found`、`bad_input`、`internal` 之一。

## 词条 HTML 的约定

`entry` 返回的 HTML 中：

- 带锚点的节点有 `data-anchor="<编号>"`（与词表中的 `anchor` 相同，0 是词条本身）；
- 标签页（`spec/v1.md` §4.4）是 `<div class="dx-tabs">`：先是 `<div class="dx-tab-bar" role="tablist">` 里每页一个 `<button class="dx-tab" role="tab" aria-selected>`，再是每页一个 `<div class="dx-tab-panel" role="tabpanel">`，未选中的带 `hidden`；要打开的锚点在某一页内时，界面切到那一页；
- 词族节点的 class 是 `dx-word_family`；
- 发音按钮是 `<button class="dx-icon" data-audio="<url> <url>…">`，按顺序回退；
- 图片是 `<img src="<首选>" data-fallback="<url>…">`；
- 链接是 `<a href="#" class="dx-link" data-link="<link 的 JSON，HTML 转义>">`，点击后取 `dataset.link` 解析成 JSON 交给 `follow`（并阻止默认跳转）。

CSS：`theme_css` 只给主题的变量和 `.dx-*` 规则；页面自己定义 `--page`、`--card`（箭头图标用 `--card` 镂空）和正文字号。
