# ZDict

ZDict 是一套开放的词典格式和配套应用。这个仓库只放协议和软件：

- **协议**：词条内容、类型表、主题，以及词典包 `.zdpack` 的格式；
- **软件**：一个共享的 Rust 核心，以及 Web、桌面、Android 三个应用。

词典数据（正文、音频、图片）不放在这里。每本词典由各自的数据仓库转换和打包；Oxford 数据在 OxDic 仓库，那里有抓取、转换、打包工具。

## 目录

```text
spec/               协议（已冻结的 v0）
  v0.md             内容格式、类型表格式、主题格式、资源引用、链接目标
  pack-v0.md        词典包 .zdpack（SQLite）的格式；类型表和主题随包分发，不在这里
crates/
  zdict-core/       Rust 核心：读包、联想、全文检索、纠错、解码、主题计算
apps/
  web/              Web 前端（TypeScript）
  desktop/          桌面端（Tauri 2，复用 Web 前端）
  android/          Android（Kotlin + Jetpack Compose）
server/             Web 版的后端（Cloudflare Worker）
scripts/
  sync-oxdata.sh    从 OxDic 复制已打包的 Oxford 数据到 oxdata/（不入库）
  make-golden.py    用 OxDic 的 Python 参考读取器生成对照输出 oxdata/golden/
docs/
  architecture.md   各部分怎么配合，核心如何被各端使用
  roadmap.md        开发阶段与退出条件
```

## 开发

```bash
scripts/sync-oxdata.sh   # 复制 Oxford 测试数据到 oxdata/ 并生成对照输出（需要本机有 OxDic 仓库及其 .venv）
cargo test               # Rust 核心
```

需要 Rust 1.95（`rust-toolchain.toml` 会自动选择）。各应用的构建方式见各自目录的 README，应用开始开发时补上。

## 状态

| 部分 | 状态 |
| --- | --- |
| 协议 v0 | 已冻结 |
| 词典包格式 v0 | 已定，Oxford 包已按此格式构建（OxDic 仓库） |
| Rust 核心 | M1 进行中：打开包、读 meta 和 files |
| Web / 桌面 / Android | 未开始 |
