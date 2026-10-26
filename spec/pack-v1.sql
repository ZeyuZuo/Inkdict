-- ZDict 词典包 v1（.zdpack）的表结构。一个词典 = 一个文件。
-- 打包程序把源目录（pack-v1.md §1，示例 samples/en-zh/）转换成这里的表；读取端只读打开。

PRAGMA application_id = 0x5A44504B;   -- 'ZDPK'
PRAGMA user_version = 1;              -- 格式版本

-- 包信息（pack-v1.md §2.2）。键：
--   format = zdict-pack        format_version = 1
--   id, title, edition, license
--   source_lang = en           target_langs = ["zh-Hans"]（单语为 []）
--   themes = ["default"]（第一个是默认主题）
--   norm = fold-1              unicode_version = 14.0.0（构建检索键时的 Unicode 版本）
--   compression = zstd         zstd_level = 19          zstd_dict = zstd.dict
--   entries = 词条数           resources = 包内资源数
--   built_at, source_sha256, builder
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;

-- 类型表、主题、zstd 字典：types.toml、themes/<name>.toml、zstd.dict
CREATE TABLE files(name TEXT PRIMARY KEY, body BLOB NOT NULL) WITHOUT ROWID;

-- 词条：body 是 zstd 压缩的节点树 JSON（源文件里的一个词条）。
-- key = 根节点 id；headword、homograph 供词表直接显示，不用解压。
CREATE TABLE entries(
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    headword TEXT NOT NULL,
    homograph INTEGER,
    body BLOB NOT NULL
);

-- 索引（相当于 MDX 的 key block）：每个带 index 角色的节点生成一行或几行。
-- anchor：节点所在的、最近的带 id 节点的编号（先序编号，根为 0）。
-- kind：类型表里声明的索引种类。
-- 查词：norm 前缀范围扫描，只读这张表的页，不碰 entries。
CREATE TABLE lookup(
    norm TEXT NOT NULL,
    rank INTEGER NOT NULL,
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    form TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('headword', 'variant', 'inflection', 'phrase', 'reading', 'hidden')),
    PRIMARY KEY (norm, rank, entry, anchor)
) WITHOUT ROWID;

-- 短释义（可选）：词表里一行下面的那句话。anchor = 0 表示整个词条；一个位置一条，lang 用来选字体。
CREATE TABLE summaries(
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    lang TEXT NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (entry, anchor)
) WITHOUT ROWID;

-- 资源（相当于 MDD，可选）：节点里的资源引用 "类别:键" 先在这里找。
CREATE TABLE resources(
    kind TEXT NOT NULL,
    key TEXT NOT NULL,
    mime TEXT NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY (kind, key)
) WITHOUT ROWID;

-- 在线资源源（可选）：resources 里没有时，按 priority 顺序用模板拼网址。
CREATE TABLE resource_sources(
    kind TEXT NOT NULL,
    priority INTEGER NOT NULL,
    template TEXT NOT NULL,
    PRIMARY KEY (kind, priority)
) WITHOUT ROWID;
