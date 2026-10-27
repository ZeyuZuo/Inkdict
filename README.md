# Inkdict

Inkdict 是一套开放的词典格式和配套应用。这个仓库只放协议和软件：

- **协议**：词条内容、类型表、主题，以及词典包 `.inkdict` 的格式；
- **软件**：一个共享的 Rust 核心，以及桌面、Android 两个客户端（不做 Web 版，没有后端）。

词典数据（正文、音频、图片）不放在这里。每本词典由各自的数据仓库转换和打包；Oxford 数据在 OxDic 仓库，那里有抓取、转换、打包工具。

## 目录

```text
spec/               协议 v1（草案）
  v1.md             内容格式、类型表、主题、语言、链接、资源引用
  pack-v1.md        源目录、打包、词典包 .inkdict（SQLite）的格式、词表规则
  pack-v1.sql       .inkdict 的表结构
  samples/en-zh/    一部虚构英汉词典的完整源目录
crates/
  inkdict-core/     Rust 核心：读包、词表（前缀 / 含有 / 拼写接近）、词条解析、资源、链接、主题层叠与渲染树
  inkdict-html/     把渲染树写成 HTML，给出主题 CSS（桌面端用）
  inkdict-app/      桌面端的应用逻辑：词典库、设置、命令，与 Tauri 无关
  inkdict-pack/     打包程序：把源目录（spec/pack-v1.md §1）打成 .inkdict
  inkdict-cli/      命令行工具 inkdict：pack/info/words/show/render/themes/serve，基准测试与纠错评测
apps/
  desktop/          桌面端（Tauri 2 + Svelte 5，界面跑在 WebView 里）
  android/          Android（Kotlin + Jetpack Compose，未开始）
scripts/
  sync-oxdata.sh    从 OxDic 复制已打包的 Oxford 数据到 oxdata/（不入库）
  make-golden.py    用 OxDic 的 Python 参考读取器生成对照输出 oxdata/golden/
  bench.py          生成基准测试输入，测 Python 参考读取器的耗时
  compare-packs.py  逐表比较两个 .inkdict（核对 inkdict pack 与 OxDic 的 Python 打包程序）
docs/
  architecture.md   各部分怎么配合，核心如何被各端使用
  roadmap.md        开发阶段与退出条件
```

## 开发

```bash
scripts/sync-oxdata.sh   # 复制 Oxford 测试数据到 oxdata/ 并生成对照输出（需要本机有 OxDic 仓库及其 .venv）
cargo test               # Rust 核心、应用逻辑、命令行
```

需要 Rust 1.95（`rust-toolchain.toml` 会自动选择）。桌面端的构建和开发方式见 `apps/desktop/README.md`。

## 状态

| 部分 | 状态 |
| --- | --- |
| 协议 v1、词典包格式 v1 | 草案；Oxford 包已按此格式构建（OxDic 仓库） |
| Rust 核心 | v1：读包、词表、词条、资源、链接、主题层叠与渲染树 |
| 桌面 | M3 主体完成：词表、词条、标签页、历史、词典库、设置、窄窗口、深色；划词和小窗口未做 |
| Android | 未开始 |

## 许可证

- 代码：MIT 或 Apache-2.0，任选其一（`LICENSE-MIT`、`LICENSE-APACHE`）。
- 协议文档与示例词典（`spec/`）：CC BY 4.0（`spec/LICENSE`）。按协议实现读取端、打包程序或词典，不需要额外许可。
