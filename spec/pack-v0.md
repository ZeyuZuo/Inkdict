# ZDict 词典包格式 v0

词典包是一个 SQLite 数据库文件，扩展名 `.zdpack`。它装着一本词典的全部正文（按 `v0.md` 的内容格式）和检索所需的索引，可以直接随应用分发，也可以导入 D1。媒体文件不在包内，只通过资源源模板解析（§6）。

参考实现：构建器在 OxDic 仓库的 `oxdic/pack/build.py`，读取器为本仓库的 `crates/zdict-core`（实现前以 OxDic 的 `oxdic/pack/reader.py` 为准）。

## 1. 识别

| 项 | 值 |
| --- | --- |
| `PRAGMA application_id` | `0x5A44504B`（ASCII `ZDPK`） |
| `PRAGMA user_version` | 包格式版本，v0 为 `0` |
| `meta.format` | `zdict-pack` |

读取器先检查这两个 PRAGMA，不符就拒绝打开。

## 2. 表

```sql
meta(key TEXT PRIMARY KEY, value TEXT)                 -- 见 §2.1
files(name TEXT PRIMARY KEY, body BLOB)                -- 类型表、主题、zstd 字典
entries(id INTEGER PRIMARY KEY, key TEXT UNIQUE, headword TEXT, homograph INTEGER, body BLOB)
lookup(norm, rank, entry, anchor, form, kind)          -- 词形检索，主键 (norm, rank, entry, anchor)
fts USING fts5(text, example, content='')              -- 全文检索，无内容表
resource_sources(kind, priority, template)             -- 资源源，主键 (kind, priority)
```

### 2.1 meta

| 键 | 含义 |
| --- | --- |
| `format`、`format_version` | `zdict-pack`、`0` |
| `spec_version` | 内容格式版本（`types.toml` 的 `meta.version`） |
| `id`、`title` | 词典 ID、显示名 |
| `source_lang`、`target_lang` | BCP-47 |
| `license` | 版权与使用限制 |
| `built_at`、`source_sha256` | 构建时间、输入 `entries.jsonl` 的 SHA256 |
| `entries` | 词条数 |
| `compression`、`zstd_level` | `zstd`、压缩级别 |
| `zstd_dict` | 字典在 `files` 中的名字；词条少于 1,000 条时不训练字典，此键不存在 |
| `fts_langs` | JSON 数组，全文检索 rowid 中语言编号到 BCP-47 的映射 |

### 2.2 files

| 名字 | 内容 |
| --- | --- |
| `types.toml` | 构建时使用的类型表 |
| `themes/<name>.toml` | 词典自带的主题，应用可以换用自己的 |
| `zstd.dict` | 词条压缩字典（可选） |

包内带类型表和主题，读取端不依赖仓库里的 `spec/`。

### 2.3 entries

- `id`：包内整数编号，按输入顺序从 1 开始，只在本包内有效。
- `key`：词条稳定 ID（内容的 `entry.id`）。交叉引用、深链和历史记录都用它。
- `headword`、`homograph`：列表显示用，取词条头部第一个词头（去掉音节点和重音符）和同形词编号。
- `body`：词条内容 JSON（UTF-8，紧凑格式）经 zstd 压缩；有 `zstd_dict` 时用该字典。

## 3. 词形检索 lookup

带 `index` 角色的节点（v0 中为 `headword`、`variant_form`、`infl_form`、`form_index`、`phrase`）各生成若干行：

| 列 | 含义 |
| --- | --- |
| `norm` | 检索键（§3.1） |
| `form` | 显示形式：去掉 `ˈ ˌ · ‧ ↔`，合并空白 |
| `entry` | `entries.id` |
| `anchor` | 锚点编号（§3.3），0 表示词条本身 |
| `kind` | 节点类型名 |
| `rank` | 排序值，越小越靠前（§3.2） |

同一词条内 `(norm, anchor)` 只保留一行，取 `rank` 最小的。

### 3.1 检索键与展开

`norm(text)`：去掉上述标记 → NFKD → 去掉组合附加符 → casefold → 只保留字母和数字。全是符号的形式（`&`）保留符号、去掉空格。于是 `Café`/`cafe`、`mother-in-law`/`mother in law` 键相同。

一个节点的读法按顺序展开，最多 32 种：

- 相邻的 `alt` 子节点是一组选项，每种读法取其一（`pull/drag yourself up` → 两种）；
- `weak` 子节点可有可无（`the lower middle classes` → 有 `the` 和无 `the` 两种）；
- 文本中的 `(…)` 可有可无（`by your (own) bootstraps` → 两种）；
- 其他子节点（`infl_ending`、`sub`、`placeholder` 等）取文本直接拼接（`amig` + `os` → `amigos`）。

### 3.2 排序

`rank = class × (P + 1) + importance`，其中：

| class | 形式 |
| --- | --- |
| 0 | 词条级词头 |
| 1 | 嵌套词头（派生词、义项、词性分组里的词头） |
| 2 | 变体 `variant_form` |
| 3 | 短语 `phrase`（习语、短语动词） |
| 4 | 屈折形式 `infl_form` |
| 5 | 索引形式 `form_index` |
| 6 | 其他带 `index` 角色的类型 |

`importance` 是词条（含词性分组）带的徽章在清单 `lookup.priority_badges` 中最靠前的位置，P 是该列表长度，都没有时为 P。Oxford 包为 `["ox3000", "ox5000"]`。

联想查询（前缀匹配走主键 B 树）：

```sql
SELECT ... FROM lookup WHERE norm >= :key AND norm < :key || char(0x10FFFF)
ORDER BY norm = :key DESC, length(norm), rank, entry LIMIT :n
```

结果按 `(entry, anchor)` 去重。模糊搜索（拼写错误）不在包内建索引，由各端从 `lookup.norm` 在内存中构建（Rust 核心计划用 FST + SymSpell）。

### 3.3 锚点编号

词条内所有带 `id` 的节点按文档顺序（先序遍历）编号，词条根节点为 0。v0 中带 `id` 的正好是带 `anchor` 角色的类型。编号只在同一个包内有效；对外的深链用 `entry.key#node.id`。

## 4. 全文检索 fts

FTS5 无内容表，分词器 `porter unicode61 remove_diacritics 2`，两列：

- `example`：`example` 节点里的文字；
- `text`：其余文字（词头、释义、标签、说明框等）。

每个文档是一个锚点下、一种语言的全部文字，不含子锚点的文字。清单 `fts.skip` 列出的类型整棵子树不进索引（Oxford 包：`pron_list`、`badge`、`homograph`）。带 `index` 角色的节点用它的全部读法代替原始片段。元素边界视为词边界。

写入前的处理 `fts_text`：去掉 §3 的标记；每个 CJK 字前后加空格，使每个汉字成为一个词元。查询时 `fts_query` 把连续的 CJK 字符变成短语（`苹果` → `"苹 果"`），其他词各自成为一个词项，词项之间是 AND。这样 1 个字的查询也能命中，并且只依赖各平台都有的 `unicode61`，不依赖 `trigram`。

行号编码位置，不需要额外的表：

```text
rowid = entry << 24 | anchor << 8 | lang
```

`lang` 是 `meta.fts_langs` 中的下标。锚点编号不能超过 65,535，语言数不能超过 256，构建时检查。排序用 `bm25(fts, 2.0, 1.0)`，即 `text` 列权重为 2；只搜例句用列过滤 `{example}: (...)`。

## 5. 构建清单

每本词典一个清单（Oxford 为 OxDic 仓库的 `packs/oalecd10.toml`）：`[pack]` 给出 meta 的字段和类型表、主题的路径；`[lookup]`、`[fts]` 是 §3.2、§4 的参数；`[[source]]` 是资源源。

构建会检查以下几项，问题写入报告，不中断构建：

- 词条 key 是否重复；
- `entry` 引用是否指向包内词条；
- `anchor` 引用是否在目标词条内（目标为 `entry`，没有时为本词条）；
- `box` 引用是否在本词条内；
- 每个资源引用的类别都有资源源；给了下载目录时，还检查资源是否都在目录里。

## 6. 资源源

`resource_sources` 按 `(kind, priority)` 存放 `v0.md` §5 的模板，`priority` 从 1 开始，按顺序回退。`{key:uri}` 按 `encodeURIComponent` 编码，`{key}` 原样代入。

v0 不建资源清单表。只要 R2 上的对象按 `类别/键` 命名，模板就能直接算出 URL。如果改为按内容哈希命名，就需要另加一张 `resources(kind, key, sha256)` 表，按 Oxford 的 20 万个资源估算约 20 MB。

## 7. Oxford 包实测（2026-10-25）

| 项 | 数值 |
| --- | --- |
| 词条 | 39,960 |
| 包大小 | 57.2 MB（再整体 zstd -19 后 27.5 MB，用于下载） |
| 其中 | 正文 19.1 MB（压缩后正文 14.6 MB），FTS 25.9 + 6.0 MB，lookup 3.9 MB，key 索引 2.1 MB |
| lookup 行数 | 94,099 |
| FTS 文档 | 424,100 |
| 构建耗时 | 约 90 秒 |
| 联想 | p50 0.07 ms，p95 2.3 ms |
| 打开词条（解压 + 解析） | p50 0.03 ms |
| 全文检索 | 英文 p95 0.8 ms；中文 p50 1 ms，p95 41 ms（“的”这类常用单字） |
| 未解析的引用 | 76 个锚点、1 个词条（THAMES），指向的内容 API 中都没有；读取端按 `v0.md` §6 回退 |

延迟在本机 SSD 上用 Python 参考读取器测得，样本为 2,000 个随机词头前缀和 2,000 个随机词条。
