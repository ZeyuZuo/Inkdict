# 架构

## 一句话

`zdict-core` 是一个**库**，不是服务器。两个客户端都把它编进自己的程序里，只有界面不同。没有 Web 版，也没有后端（2026-10-25 决定，见 `docs/roadmap.md` D7）。

```text
                  spec/（协议）
                      │
          crates/zdict-core（Rust 库）
   读 .zdpack · 词表 · 解压解析 · 主题计算 · 渲染树
             ┌────────┴─────────┐
          UniFFI      crates/zdict-html + zdict-app
             │                  │
       apps/android        apps/desktop
      Kotlin + Compose   Tauri 2：Rust + WebView 界面（Svelte）
          完全离线            完全离线
```

## 查词在哪里发生

| 端 | 查词在哪里发生 |
| --- | --- |
| Android | 手机上：核心直接读本地 `.zdpack` |
| 桌面 | 电脑上：Tauri 的 Rust 部分调用核心，WebView 里的界面只负责显示 |

两端都不需要服务器，媒体除外：音频和图片按资源源模板从 R2 读取，可以缓存或另下媒体包。桌面界面用 HTML/TS 写，但只运行在 Tauri 的 WebView 里，不对外部署。

## 核心的职责

1. **读包**：检查 `application_id`/`user_version`，读 `meta`（含 `norm`、`unicode_version`）、`files`（类型表、主题、zstd 字典）。
2. **词条**：按 key 取出 body，zstd 解压，解析为节点树。
3. **词表**：按包声明的方案（`fold-1` / `exact-1`）规范化输入，给出前缀、含有、拼写接近三组，一行一个词条，附短释义（`spec/pack-v1.md` §5）。拼写接近用内存中的检索键做编辑距离搜索。
4. **主题计算**：把主题的选择器和层叠规则算好，给每个节点输出最终样式，生成“渲染树”。这样 Compose 和 HTML 两个渲染器只负责画，不各自实现一遍层叠规则。
5. **资源和链接**：资源引用先查包内 `resources` 表，再按资源源模板给出 URL 列表；链接按 `spec/v1.md` §6 解析。

## 存储层

两个客户端都用 `rusqlite`，自带固定版本的 SQLite，以只读、`immutable=1` 方式打开包。核心直接基于 `rusqlite`，不设存储接口。规范化、词表排序这些逻辑写成与数据库无关的函数。

## 渲染器

只维护两套：Compose（Android）和 HTML（桌面端 WebView，`crates/zdict-html`）。两者都只消费核心输出的渲染树。颜色、字号、间距等设计参数来自主题文件，不在各端代码里写死。界面只通过类型表里的 role 接触词条结构（如音标、例句的显示开关），不认词典自己的类型名。

桌面端的应用逻辑（词典库、设置、命令）在 `crates/zdict-app`，Tauri 层（`apps/desktop/src-tauri`）和 `zdict serve`（浏览器开发用）共用它。

## 一致性

Python 参考实现（OxDic 仓库的 `oxdic/pack/reader.py`、`oxdic/preview.py`）对一组固定输入的输出，作为核心的对照测试：词表、检索键、资源、锚点、词条内容和 HTML 必须一致。

## 主题与用户自定义

词条怎么显示完全由主题文件（TOML，格式见 `spec/v1.md` §4）决定，渲染器里不写死任何样式。主题按栈叠加（`spec/v1.md` §4.3），上层优先：

| 层 | 来源 | 例子 |
| --- | --- | --- |
| 词典默认主题 | 包内 `themes/default.toml` | Oxford 的版式：钥匙图标、发音胶囊、义项编号 |
| 应用主题 | 应用内置或用户导入的主题文件 | 紧凑版、大字版、护眼配色 |
| 用户设置 | 设置开关自动生成的叠加层 | 按 role 隐藏音标、例句；字号由界面调整 |

用户能调的：字体、字号、颜色（浅色/深色各一套）、任何类型的显示或隐藏、块与行内、编号和括号等前后缀文字、间距和缩进、背景和圆角、说明框是否折叠、图标。普通用户用设置开关；高级用户可以直接编辑或导入 TOML 主题。

不能调的：内容本身、任意 CSS 或脚本、类型表之外的版面（如多栏）。限制是有意的：同一份主题要能在 Compose 和 HTML 上画出一样的结果，而 Compose 不执行 CSS；不允许脚本也避免了 MDX 那样的安全问题。

## 测试数据

词典数据不进仓库。本地开发和测试用 `scripts/sync-oxdata.sh` 从 OxDic 数据仓库复制到 `oxdata/`（已被 `.gitignore` 忽略）：

| 文件 | 用途 |
| --- | --- |
| `oxdata/oalecd10.zdpack` | Oxford 词典包，核心的读包、词表、渲染测试 |
| `oxdata/entries.jsonl` | 全部词条内容（未压缩），渲染树和解码测试 |
| `oxdata/examples/*.json` | 5 个金样例词条 |
| `oxdata/*_report.json`、`SHA256SUMS` | 构建报告与校验和 |
| `oxdata/golden/*.json` | Python 参考读取器的输出（`scripts/make-golden.py`，同步时自动生成），核心的对照测试 |

依赖 `oxdata/` 的测试在目录不存在时跳过，不让没有数据的环境失败。
