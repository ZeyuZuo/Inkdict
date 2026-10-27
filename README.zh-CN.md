# Inkdict

Inkdict 是一套开放的词典格式，以及一个读取这种格式的离线查词软件。

一部词典就是一个 `.inkdict` 文件，里面有词条、检索索引和这部词典自己的**主题**：词条长什么样由词典决定，不由软件决定。格式不限语言：检索键、字体和书写方向都按词典声明的语言处理。

![Inkdict 桌面端打开示例词典](docs/images/inkdict.png)

[English](README.md)

## 内容

| | |
| --- | --- |
| [`spec/`](spec/) | 协议第 1 版（已冻结）：词条内容、类型表、主题（[`v1.md`](spec/v1.md)），词典包格式（[`pack-v1.md`](spec/pack-v1.md)），以及一部完整的[示例词典](spec/samples/en-zh/)。协议文档为英文 |
| [`docs/making-a-dictionary.md`](docs/making-a-dictionary.md) | 一步步制作自己的词典（英文） |
| `inkdict` | 命令行工具：`inkdict pack` 把源文件打包成词典；`words`、`show`、`render`、`themes`、`info` 用来检查词典 |
| Inkdict 桌面端 | Linux、Windows、macOS：输入即出词表，词性标签页，发音，交叉引用，历史记录，多部词典切换，浅色和深色 |

除了一部很小的示例词典，Inkdict 不附带任何词典内容。请使用你有权使用的词典。

## 开始使用

1. 从 [Releases](https://github.com/ZeyuZuo/Inkdict/releases) 下载对应系统的软件和 `inkdict` 工具，以及示例词典 `sample-en-zh.inkdict`。
2. 打开软件，点左栏底部的齿轮，选"添加词典…"，选择 `.inkdict` 文件。
3. 制作自己的词典：见 [docs/making-a-dictionary.md](docs/making-a-dictionary.md)。

## 从源码构建

需要 Rust 1.95（`rust-toolchain.toml` 会自动选择）；桌面端另需 Node.js 20.19 或更新版本，以及所在系统的 [Tauri 2 依赖](https://v2.tauri.app/start/prerequisites/)（Linux 上是 WebKitGTK 4.1）。

```bash
cargo build --release -p inkdict-cli       # target/release/inkdict
cargo test                                 # Rust 部分的测试

cd apps/desktop
npm install
npm run tauri build                        # 安装包在 target/release/bundle/
npm run check && npm test                  # 界面的检查和测试
```

桌面端的开发方式见 [`apps/desktop/README.md`](apps/desktop/README.md)。

## 目录

```text
spec/                  协议（CC BY 4.0）和示例词典
docs/                  教程、架构、路线图
crates/
  inkdict-core/        读包：词表、词条、链接、资源、主题和渲染树
  inkdict-pack/        从源目录打包
  inkdict-html/        把渲染树写成 HTML 和主题 CSS（桌面端用）
  inkdict-app/         桌面端的应用逻辑，与 Tauri 无关：词典库、设置、命令
  inkdict-cli/         命令行工具 inkdict
apps/
  desktop/             桌面端（Tauri 2、Svelte 5）
  android/             计划中
scripts/               维护者工具：与另一个仓库 OxDic 里的 Oxford 数据对照
```

各部分如何配合见 [`docs/architecture.md`](docs/architecture.md)，后续计划见 [`docs/roadmap.md`](docs/roadmap.md)。

## 测试

`cargo test` 先用 `inkdict pack` 打出示例词典，再用它测试读包、词表、渲染、命令行和应用逻辑，不需要其他数据。维护者还会用完整的 Oxford 词典和 Python 参考实现做对照；这些数据是私有的（在 OxDic 仓库），没有时对应的测试会跳过。

## 许可证

- 代码：MIT 或 Apache-2.0，任选其一（[`LICENSE-MIT`](LICENSE-MIT)、[`LICENSE-APACHE`](LICENSE-APACHE)）。
- `spec/` 里的协议文档和示例词典：CC BY 4.0（[`spec/LICENSE`](spec/LICENSE)）。按协议实现读取端、打包程序或词典，不需要额外许可。
