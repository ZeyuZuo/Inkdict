# 桌面

Tauri 2。查词由 Tauri 的 Rust 部分（`src-tauri/`）直接调用 `inkdict-core`，完全离线。界面用 Svelte 5 + TypeScript（Vite）写在 `src/`，只运行在 Tauri 的 WebView 里，不单独部署。

词条的 HTML 由 Rust 生成（`crates/inkdict-html`，输入是核心的渲染树），界面只负责插入页面和处理点击，不在 TypeScript 里实现主题或排版规则。

阶段：`docs/roadmap.md` M3。

## 界面与 Rust 之间的命令

所有命令用 Tauri `invoke` 调用，参数和返回值是 JSON。字段名用 snake_case，与 `inkdict-core` 的类型一致。

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
| `follow` | `{ current, link }`，`link` 为 `{ entry?, anchor?, text }` | `{ kind: "entry", key, focus }` 或 `{ kind: "lookup", text }`；`focus` 为 `{ kind: "top" }` 或 `{ kind: "anchor", number }` |
| `settings` | — | `Settings` |
| `set_settings` | `{ settings }` | `Settings`；之后的 `entry` 按新设置渲染 |
| `theme_css` | — | 字符串：主题的字体与颜色变量（浅色、深色两套）和 `.dx-*` 基础样式 |

`Settings`：`{ hide_pronunciation: bool, hide_examples: bool, font_scale: number }`。前两项隐藏带 `pronunciation`、`example` role 的类型（`spec/v1.md` §3.2），在 Rust 里变成包主题之上的叠加主题；`font_scale` 由界面用 CSS 变量实现。设置和库保存在应用配置目录的 `config.json`：`{ "packs": ["/a.inkdict", "/b.inkdict"], "active": "/a.inkdict", "settings": { "hide_pronunciation": false, "hide_examples": false, "font_scale": 1.0 } }`。`active` 必须是 `packs` 中的路径或 null；旧格式 `"pack": "/x.inkdict"` 读取为单项库和活动路径，之后仅写新格式。`--pack` / `INKDICT_PACK` 将路径加入库并激活。`inkdict serve` 用相同命令和 JSON（`POST /<command>`），库只存在于内存，不保存配置。

错误：命令失败时 reject，值为 `{ code, message }`，`code` 为 `no_pack`、`not_found`、`bad_input`、`internal` 之一。

## 词条 HTML 的约定

`entry` 返回的 HTML 中：

- 带锚点的节点有 `data-anchor="<编号>"`（与词表中的 `anchor` 相同，0 是词条本身）；
- 标签页（`spec/v1.md` §4.4）是 `<div class="dx-tabs">`：先是 `<div class="dx-tab-bar" role="tablist">` 里每页一个 `<button class="dx-tab" role="tab" aria-selected>`，再是每页一个 `<div class="dx-tab-panel" role="tabpanel">`，未选中的带 `hidden`；要打开的锚点在某一页内时，界面切到那一页；
- 发音按钮是 `<button class="dx-icon" data-audio="<url> <url>…">`，按顺序回退；
- 包里自带的音频和图片排在最前，网址是 `inkdict-res://localhost/<包 id>/<类别>/<键>`（Windows 上是 `http://inkdict-res.localhost/…`），由 Tauri 的自定义协议从当前词典读出；`inkdict serve` 用 `http://127.0.0.1:<端口>/resource/…` 提供同样的文件；
- 图片是 `<img src="<首选>" data-fallback="<url>…">`；
- 链接是 `<a href="#" class="dx-link" data-link="<link 的 JSON，HTML 转义>">`，点击后取 `dataset.link` 解析成 JSON 交给 `follow`（并阻止默认跳转）。

CSS：`theme_css` 只给主题的变量和 `.dx-*` 规则；页面自己定义 `--page`、`--card`（箭头图标用 `--card` 镂空）和正文字号。

## 开发

```bash
npm install
INKDICT_PACK=$PWD/../../oxdata/oalecd10.inkdict npm run tauri dev   # 真实窗口（WebKitGTK）；INKDICT_PACK 可省，库记在配置里
npm run check && npm test                                    # svelte-check、vitest
```

不开窗口时在浏览器里调界面：

```bash
cargo build --release -p inkdict-cli
../../target/release/inkdict --pack ../../oxdata/oalecd10.inkdict serve --port 1430
VITE_INKDICT_BRIDGE=http://127.0.0.1:1430 npx vite --port 1421   # 打开 http://localhost:1421
```

不设 `VITE_INKDICT_BRIDGE` 时用 `src/lib/mock.ts` 的几个虚构词条。开发构建把界面控制器挂在 `window.__inkdict` 上，供脚本检查。发布构建：`npm run tauri build`。

## 界面

- 左栏：搜索框、词表（前缀匹配；"含有 …"；无结果时"拼写接近"），底部是词典切换和设置。宽窗口里首行的词条随输入显示，↑↓ 即时显示所选行，Enter 或点击把它记为一次历史。
- 窗口窄于 720 px：单栏，词表从搜索框下拉，选中后收起；顶栏有"固定在最前"。
- 词条：词性标签页吸顶；要打开的锚点在未选中的标签页里时切过去；从链接到达的词条顶部显示"‹ 来源词"。
- 历史记录词条、滚动位置、各标签组的选中页和搜索框内容。

| 按键 | 作用 |
| --- | --- |
| 直接打字 | 输入到搜索框 |
| Ctrl+L / Ctrl+F、Esc | 选中搜索框；搜索框里按 Esc 清空（窄窗口先收起下拉） |
| ↑ ↓、Enter | 在词表里移动、打开 |
| Alt+← / Alt+→、鼠标侧键 | 后退 / 前进 |
| Ctrl+, | 设置 |
| Ctrl+Shift+D | 延迟统计（最近 200 次） |

## 基准测试

```bash
INKDICT_BENCH=$PWD/out.json INKDICT_BENCH_QUERIES=$PWD/../../oxdata/bench/words.txt \
INKDICT_BENCH_ENTRIES=$PWD/../../oxdata/bench/entry.txt npm run tauri dev
```

界面逐键输入每个查询、打开每个词条（各取前 300 行，`INKDICT_BENCH_LIMIT` 可改），把"输入到词表画出""打开到词条画出"的耗时写进 `out.json`（`words`、`entry` 两项）后退出。输入文件由 `scripts/bench.py queries` 生成，每行第一列是查询或词条 key。
