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
