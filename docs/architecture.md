# 架构

## 一句话

`zdict-core` 是一个**库**，不是服务器。每个应用都把它编进自己的程序里，各端只有界面不同。

```text
                         spec/（协议）
                             │
                 crates/zdict-core（Rust 库）
          读 .zdpack · 联想 · 全文检索 · 纠错 · 解压解析 · 主题计算
        ┌──────────────┬───────────────┼──────────────────┐
     UniFFI          直接调用        WebAssembly         WebAssembly
        │              │               │                  │
 apps/android     apps/desktop      apps/web           server/
 Kotlin+Compose   Tauri 2（Rust）   浏览器内查词        Cloudflare Worker
   完全离线        完全离线          （离线模式）        （在线模式）
                       └── 界面复用 apps/web ──┘
```

## 前端和后端

| 端 | 有没有后端 | 查词在哪里发生 |
| --- | --- | --- |
| Android | 没有 | 手机上：核心直接读本地 `.zdpack` |
| 桌面 | 没有 | 电脑上：Tauri 的 Rust 部分直接调用核心 |
| Web 在线模式 | `server/` | Worker 里查，返回词条 JSON；浏览器只负责显示 |
| Web 离线模式 | 没有 | 浏览器下载整个包（压缩后约 27.5 MB），用 WASM 版核心在本地查 |

所以“后端”只在 Web 在线模式里存在，而且它用的也是同一个核心。Android 和桌面不需要任何服务器，媒体除外：音频和图片按资源源模板从 R2 读取，可以缓存或另下媒体包。

## 核心的职责

1. **读包**：检查 `application_id`/`user_version`，读 `meta`、`files`（类型表、主题、zstd 字典）。
2. **词条**：按 key 取出 body，zstd 解压，解析为节点树。
3. **联想**：`norm` 规范化输入，前缀查 `lookup`，排序去重（与 `spec/pack-v0.md` §3 一致）。
4. **纠错**：从 `lookup.norm` 在内存中建 FST + SymSpell，处理拼写错误。
5. **全文检索**：`fts_query` 生成 MATCH 表达式，rowid 解码出词条、锚点、语言。
6. **主题计算**：把主题的选择器和层叠规则算好，给每个节点输出最终样式，生成“渲染树”。这样 Compose 和 Web 两个渲染器只负责画，不各自实现一遍层叠规则。
7. **资源和链接**：资源引用 → URL 列表（按优先级回退）；链接按 `spec/v0.md` §6 解析。

## 存储层

不同平台访问 SQLite 的方式不同，核心通过一个存储接口隔离：

| 平台 | SQLite 来源 |
| --- | --- |
| Android、桌面 | `rusqlite`，自带固定版本的 SQLite（保证 FTS5 可用） |
| 浏览器 | SQLite 的 WASM 版本，读下载到本地的包 |
| Worker | D1（Cloudflare 的 SQLite），把包导入 D1 |

规范化、查询拼写、rowid 解码、排序、解压、主题计算这些逻辑与存储无关，三种存储共用。

## 渲染器

只维护两套：Compose（Android）和 Web（Web 与桌面共用）。两者都只消费核心输出的渲染树。颜色、字号、间距等设计参数来自主题文件，不在各端代码里写死。

## 一致性

Python 参考实现（OxDic 仓库的 `oxdic/pack/reader.py`）对一组固定查询的输出，作为核心的对照测试：联想结果、全文检索结果、词条内容必须一致。

## 主题与用户自定义

词条怎么显示完全由主题文件（TOML，格式见 `spec/v0.md` §4）决定，渲染器里不写死任何样式。主题按栈叠加（`spec/v0.md` §7.2），上层优先：

| 层 | 来源 | 例子 |
| --- | --- | --- |
| 词典默认主题 | 包内 `themes/default.toml` | Oxford 的版式：钥匙图标、发音胶囊、义项编号 |
| 应用主题 | 应用内置或用户导入的主题文件 | 紧凑版、大字版、护眼配色 |
| 用户设置 | 设置开关自动生成的叠加层 | 隐藏中文、折叠例句、隐藏发音、调字号 |

用户能调的：字体、字号、颜色（浅色/深色各一套）、任何类型的显示或隐藏、块与行内、编号和括号等前后缀文字、间距和缩进、背景和圆角、说明框是否折叠、图标。普通用户用设置开关；高级用户可以直接编辑或导入 TOML 主题。

不能调的：内容本身、任意 CSS 或脚本、类型表之外的版面（如多栏）。限制是有意的：同一份主题要能在 Compose 和 Web 上画出一样的结果，而 Compose 不执行 CSS；不允许脚本也避免了 MDX 那样的安全问题。

## 测试数据

词典数据不进仓库。本地开发和测试用 `scripts/sync-oxdata.sh` 从 OxDic 数据仓库复制到 `oxdata/`（已被 `.gitignore` 忽略）：

| 文件 | 用途 |
| --- | --- |
| `oxdata/oalecd10.zdpack` | Oxford 词典包，核心的读包、联想、检索测试 |
| `oxdata/entries.jsonl` | 全部词条内容（未压缩），渲染树和解码测试 |
| `oxdata/examples/*.json` | 5 个金样例词条 |
| `oxdata/*_report.json`、`SHA256SUMS` | 构建报告与校验和 |
| `oxdata/golden/*.json` | Python 参考读取器的输出（`scripts/make-golden.py`，同步时自动生成），核心的对照测试 |

依赖 `oxdata/` 的测试在目录不存在时跳过，不让没有数据的环境失败。
