# 词典源目录示例

`en-zh/` 是一部虚构的英汉双解词典在打包前的源目录，格式见 `../pack-v1.md` §1。打包后的表结构见 `../pack-v1.sql`。

| 文件 | 内容 |
|---|---|
| `manifest.toml` | 包信息、语言、文件位置、检索设置、在线资源源 |
| `types.toml` | 类型表：类型、嵌套、属性、roles |
| `themes/default.toml` | 主题 |
| `entries.jsonl` | 6 个词条（harbour、lantern、go、lead¹、lead²、café），每行一棵节点树 |
| `summaries.jsonl` | 短释义：词表里每行下面那句话，一个词条或锚点一条 |
| `resources/` | 音频、图片；`audio:harbour-us` 故意缺省，演示回退到在线源 |

它演示了：
- 两种语言（`targets = ["zh-Hans"]`），按语言选字体；
- 词性标签页（主题 `tabs`）；
- 同形词 lead¹、lead²（`homograph` role），以及 `boost` 让带核心词徽章的 lead¹ 排在前面；
- 变体、屈折形式、习语、只用于检索的隐藏词形（`hidden`）进索引；
- `fold-1` 检索键：café、cafe、CAFÉ 都能查到 café；
- 读法展开：`alt`（`choice` role）给出 go all out / go flat out，`opt`（`optional` role）给出 lead sb the way / lead the way；
- 词条之间的链接（lead¹ ↔ lead²、lantern → harbour 的锚点）；
- 资源打进包里，以及包里没有时回退到在线源。

单语词典写 `targets = []`，其余写法相同。

## 打包后 lookup 里的行

| norm | form | 词条 | anchor | kind |
|---|---|---|---|---|
| harbour | harbour | harbour | 根 | headword |
| harbor | harbor | harbour | 根 | variant |
| harbours / harboured / harbouring | 同左 | harbour | v | inflection |
| safeharbour | safe harbour | harbour | safe-harbour | phrase |
| lantern | lantern | lantern | 根 | headword |
| go | go | go | 根 | headword |
| goes / went / gone / going | 同左 | go | v | inflection |
| onthego | on the go | go | on-the-go | phrase |
| goallout / goflatout | go all out / go flat out | go | go-all-out | phrase |
| lead | lead | lead¹ | 根 | headword |
| leads | leads | lead¹ | 根 | hidden |
| led / leading | 同左 | lead¹ | v | inflection |
| leadsbtheway / leadtheway | lead sb the way / lead the way | lead¹ | lead-the-way | phrase |
| lead | lead | lead² | 根 | headword |
| cafe | café | café | 根 | headword（变体 cafe 的键相同，只留词头这一行） |

anchor 写的是节点 id，包里存的是它的先序编号。
