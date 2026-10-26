# 词典源目录示例

`en-zh/` 是一部虚构的英汉双解词典在打包前的源目录，格式见 `../pack-v1.md` §1。打包后的表结构见 `../pack-v1.sql`。

| 文件 | 内容 |
|---|---|
| `manifest.toml` | 包信息、语言、文件位置、检索设置、在线资源源 |
| `types.toml` | 类型表：类型、嵌套、属性、roles |
| `themes/default.toml` | 主题 |
| `entries.jsonl` | 3 个词条（harbour、lantern、go），每行一棵节点树 |
| `summaries.jsonl` | 短释义：词表里每行下面那句话，一个词条或锚点一条 |
| `resources/` | 音频、图片；`audio:harbour-us` 故意缺省，演示回退到在线源 |

它演示了：两种语言（`targets = ["zh-Hans"]`）、词性标签页（主题 `tabs`）、变体 / 屈折形式 / 习语进索引、资源打进包里。单语词典写 `targets = []`，其余写法相同。

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

anchor 写的是节点 id，包里存的是它的先序编号。
