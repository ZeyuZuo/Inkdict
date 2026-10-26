# ZDict 词典包 v1

状态：草案，2026-10-26。取代 `pack-v0.md`；不兼容 v0。

打包程序把一部词典的**源目录**（§1）转换成一个 SQLite 文件 `.zdpack`（§2–§4，表结构见 `pack-v1.sql`），读取端只读打开。§5 是读取端查词的规则，两个客户端都按它实现，结果一致。

参考实现：打包程序在 OxDic 仓库 `oxdic/pack/`，读取端是本仓库的 `crates/zdict-core`。

## 1. 源目录

```text
<词典>/
  manifest.toml        从这里开始读
  types.toml           类型表（v1.md §3）
  themes/*.toml        主题（v1.md §4）
  entries.jsonl        词条，每行一个
  summaries.jsonl      短释义（可选）
  resources/<类别>/    资源文件（可选）
```

完整示例：`samples/en-zh/`。文件名和位置以 manifest 为准，上面是惯例。

### 1.1 manifest.toml

| 键 | 必填 | 说明 |
|---|---|---|
| `meta.format`、`meta.version` | 是 | `zdict-source`、`1` |
| `pack.id` | 是 | 词典 ID：小写字母、数字、`-` |
| `pack.title` | 是 | 显示名 |
| `pack.edition`、`pack.license` | 否 | 版本、版权与使用限制 |
| `languages.source` | 是 | 词头和原文的语言（BCP 47） |
| `languages.targets` | 是 | 译文语言的数组；单语词典写 `[]` |
| `files.types` | 是 | 类型表路径 |
| `files.themes` | 是 | 主题路径的数组，第一个是默认主题 |
| `files.entries` | 是 | 词条文件路径 |
| `files.summaries` | 否 | 短释义文件路径 |
| `files.resources` | 否 | 资源目录路径 |
| `lookup.norm` | 是 | 检索键方案：`fold-1` 或 `exact-1`（§3.2） |
| `lookup.boost` | 否 | 选择器数组，用于排序（§3.3） |
| `[[source]]` | 否 | 在线资源源：`kind`、`template`，同一类别按出现顺序回退（§4） |

路径都相对 manifest 所在目录。

### 1.2 entries.jsonl

UTF-8，每行一个词条，即 `v1.md` §2 的一棵节点树。行的顺序就是包内词条编号的顺序。

### 1.3 summaries.jsonl

每行一条短释义：

```json
{"entry": "harbour", "anchor": "safe-harbour", "lang": "zh-Hans", "text": "安全港；免责保护"}
```

| 键 | 说明 |
|---|---|
| `entry` | 词条 key |
| `anchor` | 可选；词条内的节点 id，不写表示整个词条 |
| `lang` | 可选；文字的语言，用来选字体；不写时取该词条根节点的 `lang` |
| `text` | 一行纯文本 |

同一个 `(entry, anchor)` 只能有一条。短释义由词典的数据仓库生成，怎么写、用哪种语言都由词典决定。

### 1.4 resources/

`resources/<类别>/<键>.<扩展名>`，例如 `resources/audio/harbour-gb.wav` 对应资源引用 `audio:harbour-gb`。键是去掉最后一个扩展名的文件名，同一类别内不能重复。媒体类型按扩展名确定：

| 扩展名 | 媒体类型 |
|---|---|
| `mp3` `wav` `ogg` `m4a` | `audio/mpeg` `audio/wav` `audio/ogg` `audio/mp4` |
| `jpg` `jpeg` `png` `webp` `svg` | `image/jpeg` `image/jpeg` `image/png` `image/webp` `image/svg+xml` |

其他扩展名是构建错误。资源很大的词典可以不放这个目录，只用在线资源源。

## 2. 包格式

### 2.1 识别

| 项 | 值 |
|---|---|
| `PRAGMA application_id` | `0x5A44504B`（ASCII `ZDPK`） |
| `PRAGMA user_version` | `1` |
| `meta.format`、`meta.format_version` | `zdict-pack`、`1` |

读取端先检查这几项，不符就拒绝打开。

### 2.2 meta

| 键 | 值 |
|---|---|
| `id`、`title`、`edition`、`license` | 取自 manifest；没有的键不写 |
| `source_lang`、`target_langs` | `languages.source`；`languages.targets` 的 JSON 数组 |
| `themes` | 主题名的 JSON 数组，顺序同 manifest，第一个是默认主题 |
| `norm`、`unicode_version` | 检索键方案；打包时所用 Unicode 表的版本（如 `14.0.0`） |
| `compression`、`zstd_level`、`zstd_dict` | `zstd`、压缩级别、压缩字典在 `files` 中的名字（没有字典时不写） |
| `entries`、`resources` | 词条数、包内资源数 |
| `built_at`、`source_sha256`、`builder` | 构建时间（RFC 3339，UTC）、`entries.jsonl` 的 SHA-256、打包程序名称和版本 |

### 2.3 files 与 entries

`files` 存 `types.toml`、`themes/<名字>.toml` 和压缩字典 `zstd.dict`。读取端只用包内的这些文件，不依赖源目录或 `spec/`。

`entries` 的列：

| 列 | 值 |
|---|---|
| `id` | 按 `entries.jsonl` 的行序从 1 编号，只在本包内有效 |
| `key` | 根节点的 `id`。交叉引用、深链、历史记录都用它 |
| `headword` | 锚点为 0 的第一个 `headword` 种类索引节点的 `form`（§3.1）；没有时取任意位置的第一个；都没有时取 `key` |
| `homograph` | 锚点为 0 的第一个 `homograph` 角色节点的文字，按十进制整数解析；没有时为 NULL |
| `body` | 节点树的紧凑 JSON（UTF-8），经 zstd 压缩；有 `zstd_dict` 时用该字典 |

## 3. 词形索引 lookup

### 3.1 哪些节点进索引

类型表里带 `index` 角色的每个节点生成一行或几行：

| 列 | 值 |
|---|---|
| `norm` | 检索键（§3.2） |
| `form` | 显示形式 |
| `entry` | `entries.id` |
| `anchor` | 节点自身或最近的、带 `id` 的祖先节点的锚点编号（§3.4） |
| `kind` | 该类型在类型表里的 `index` 种类（`v1.md` §3.1） |
| `rank` | 排序值，越小越靠前（§3.3） |

一个节点的**读法**按它的子节点展开，最多 32 种，超出的按顺序截掉并写入报告：

- 字符串：原样拼接；其中每对 `(…)` 可有可无（`by your (own) bootstraps` → 有无 `own` 两种）；
- 带 `optional` 角色的子节点：可有可无（`the lower middle classes` → 有无 `the` 两种）；
- 连续的、带 `choice` 角色的子节点（中间只隔着空白字符串）：每种读法取其一（`pull` `drag` → 两种）；
- 其他子节点：取它自己的读法拼接。

每种读法去掉排版标记 `ˈ` `ˌ` `·` `‧` `↔`（U+02C8、U+02CC、U+00B7、U+2027、U+2194），合并连续空白并去掉首尾空白，就是 `form`。`norm` 为空的读法不进索引。同一词条内 `(norm, anchor)` 只留一行，取 `rank` 最小的。

### 3.2 检索键

查询和打包使用同一个方案，记录在 `meta.norm`。两个方案的步骤：

| 步 | `fold-1` | `exact-1` |
|---|---|---|
| 1 | 去掉 §3.1 的排版标记 | 同左 |
| 2 | NFKD | NFKC |
| 3 | 去掉基字符属于拉丁、希腊、西里尔、希伯来、阿拉伯字母的组合附加符（下文） | — |
| 4 | NFC | — |
| 5 | Unicode 完全大小写折叠（casefold） | 同左 |
| 6 | 只保留一般类别为 L、M、N 的字符；结果为空时，改为保留第 5 步结果中除空白以外的全部字符 | 同左 |

第 3 步：一般类别为 Mn 的字符，它的基字符是前面最近的非 Mn 字符；基字符落在以下范围内（或没有基字符）时去掉：

| 文字 | 范围 |
|---|---|
| 拉丁 | U+0000–024F、1E00–1EFF、2C60–2C7F、A720–A7FF、AB30–AB6F |
| 希腊 | U+0370–03FF、1F00–1FFF |
| 西里尔 | U+0400–052F、1C80–1C8F、2DE0–2DFF、A640–A69F |
| 希伯来 | U+0590–05FF、FB1D–FB4F |
| 阿拉伯 | U+0600–06FF、0750–077F、0870–08FF、FB50–FDFF、FE70–FEFF |

于是 `fold-1` 下 `Café` = `cafe`、`mother-in-law` = `mother in law`、`كِتَاب` = `كتاب`，而印地语 `किताब`、日语 `が` 保持原样。词头本身就带附加符、不能去掉的语言用 `exact-1`。

Unicode 表的版本会影响结果。读取端可以用更新的表：在 `meta.unicode_version` 中已分配的字符，结果必须与打包时相同；之后才分配的字符，结果不作规定。

### 3.3 排序

```text
rank = class × (B + 1) + importance
```

| class | 行 |
|---|---|
| 0 | `headword`，锚点为 0 |
| 1 | `headword`，其他锚点（派生词、词性分组里的词头等） |
| 2 | `variant` |
| 3 | `phrase` |
| 4 | `inflection` |
| 5 | `reading` |
| 6 | `hidden` |

`B` 是 `lookup.boost` 的长度。`importance` 是词条内任意节点能匹配的第一个 boost 选择器的下标（从 0 起）；都不匹配时为 `B`。选择器写法同 `v1.md` §4.1，不含 `::` 伪元素。例：Oxford 写 `boost = ["badge[scheme=ox3000]", "badge[scheme=ox5000]"]`。

### 3.4 锚点编号

词条内所有带 `id` 的节点按先序遍历编号，根节点为 0。编号只在本包内有效，对外的深链用 `<词条 key>#<节点 id>`。

## 4. 资源

资源引用 `类别:键`（`v1.md` §7）按顺序解析：

1. `resources` 表中 `(kind, key)` 存在：用它的 `body` 和 `mime`。
2. 否则按 `resource_sources` 中该类别的模板，`priority` 从 1 起依次尝试。模板中 `{key}` 原样代入，`{key:uri}` 按 `encodeURIComponent` 编码后代入。

`resource_sources` 来自 manifest 的 `[[source]]`，`priority` 为同一类别内的出现顺序。读取端可以缓存在线取到的资源。

## 5. 词表（读取端）

输入 `q`，检索键 `k = norm(q)`。`k` 为空时不显示词表。

### 5.1 三组结果

| 组 | 何时出现 | 匹配 | 顺序 |
|---|---|---|---|
| 前缀 | 总是 | `norm` 以 `k` 开头 | `norm = k` 的在前，再按 `length(norm)`、`rank`、`entry` |
| 含有 | `k` 至少 2 个字符 | `norm` 在第 2 个字符或之后含有 `k`；只取 `headword`、`variant`、`phrase` | `rank`、`length(norm)`、`norm`、`entry` |
| 拼写接近 | 前两组都为空，且 `k` 至少 3 个字符 | 与 `k` 的编辑距离（OSA，相邻两个字符对调算 1 处）不超过上限：`k` 为 3–4 个字符时 1，5 个起 2 | 距离；与 `k` 字符相同只是顺序不同的在前；`rank` |

前缀组走 `lookup` 主键的范围扫描（`norm >= k AND norm < k || char(0x10FFFF)`）。另外两组可以在内存中对 `lookup.norm` 计算。每组显示多少行由软件决定。

### 5.2 一行一个词条

`lookup` 的行先合并成词表的行，再显示。

- **合并**：显示形式与词条都相同的行合并为一行。显示形式是 `form`；`reading` 和 `hidden` 的显示形式是该词条的 `headword`，所以它们并入词头那一行。
- **代表行**：合并后的行用其中 `rank` 最小的那一行；它在列表中的位置取其中最靠前的一行。
- **不跨组重复**：出现在前面一组的行，后面的组不再列出。
- **不同词条一定分行**：同形词 `lead¹`、`lead²`，以及 `left`（词头）和 `left`（`leave` 的屈折形式）都各占一行。选中一行，只打开一个词条。

每行显示：

| 代表行的种类 | 显示 |
|---|---|
| `headword`、`phrase` | 显示形式；有 `homograph` 时在后面标出编号 |
| `variant`、`inflection` | 显示形式 → 所属词条的 `headword`（两者相同时不写箭头） |
| `reading`、`hidden` | 词条的 `headword`；`reading` 后面接匹配到的读法 |

下面一行是短释义：先找 `summaries` 中代表行的 `(entry, anchor)`，没有时找 `(entry, 0)`，再没有就不显示。

### 5.3 打开

选中一行：打开代表行的词条，定位到代表行的锚点；锚点在某个标签页内时，显示那个标签页（`v1.md` §4.4）。

## 6. 构建检查

下列问题使构建失败：manifest 或类型表不合法；某行不是合法的 JSON；根节点缺 `id` 或 `lang`；词条 key 重复；资源文件扩展名不认识或键重复。

下列问题写入构建报告，不中断构建：

- 类型表里没有的类型、类型表不允许的子节点、未声明或类型不符的属性；
- 同一词条内节点 `id` 重复；带 `anchor` 角色的节点没有 `id`；
- `link` 节点的 `entry` 不在包内，或 `anchor` 不在目标词条内（读取端按 `v1.md` §6 回退）；
- 资源引用既不在 `resources` 中，该类别也没有在线资源源；
- 短释义指向不存在的词条或锚点，或同一位置有多条；
- `homograph` 节点的文字不是正整数；
- 读法超过 32 种；`index` 节点的读法全部为空。

## 7. 与 v0 的差别

| v0 | v1 |
|---|---|
| 源目录不作规定 | §1 规定源目录 |
| `lookup.kind` 是类型名 | 是索引种类 |
| 排序用 Oxford 的类别表和 `priority_badges` | 按索引种类和 `boost` 选择器 |
| 检索键去掉所有组合附加符 | `fold-1` / `exact-1`，记录 `unicode_version` |
| 有 `fts` 全文检索表 | 去掉 |
| 资源只用在线模板 | 可打进 `resources` 表 |
| 没有短释义 | `summaries` 表 |
