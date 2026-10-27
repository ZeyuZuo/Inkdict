-- Tables of an Inkdict pack, version 1 (.inkdict). One dictionary = one file.
-- A packer turns a source directory (pack-v1.md §1, example samples/en-zh/)
-- into these tables; readers open the file read-only.

PRAGMA application_id = 0x494E4B44;   -- 'INKD'
PRAGMA user_version = 1;              -- format version

-- Pack information (pack-v1.md §2.2). Keys:
--   format = inkdict-pack      format_version = 1
--   id, title, edition, license
--   source_lang = en           target_langs = ["zh-Hans"] ([] when monolingual)
--   themes = ["default"] (the first is the default theme)
--   norm = fold-1              unicode_version = 17.0.0 (Unicode tables used for lookup keys)
--   compression = zstd         zstd_level = 19          zstd_dict = zstd.dict
--   entries = number of entries, resources = number of stored files
--   built_at, source_sha256, builder
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;

-- Type registry, themes, zstd dictionary: types.toml, themes/<name>.toml, zstd.dict
CREATE TABLE files(name TEXT PRIMARY KEY, body BLOB NOT NULL) WITHOUT ROWID;

-- Entries: body is the node tree JSON (one line of the source), zstd-compressed.
-- key = the root's id; headword and homograph let the word list show rows
-- without decompressing.
CREATE TABLE entries(
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    headword TEXT NOT NULL,
    homograph INTEGER,
    body BLOB NOT NULL
);

-- Form index: every node with the index role gives one or more rows.
-- anchor: number of the nearest node with an id (pre-order, the root is 0).
-- kind: the index kind declared in the type registry.
-- Lookup is a prefix range scan of norm that reads only this table.
CREATE TABLE lookup(
    norm TEXT NOT NULL,
    rank INTEGER NOT NULL,
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    form TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('headword', 'variant', 'inflection', 'phrase', 'reading', 'hidden')),
    PRIMARY KEY (norm, rank, entry, anchor)
) WITHOUT ROWID;

-- Summaries (optional): the line under a word-list row. anchor = 0 is the whole
-- entry; one per place; lang chooses the fonts.
CREATE TABLE summaries(
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    lang TEXT NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (entry, anchor)
) WITHOUT ROWID;

-- Stored files (optional): a resource reference "kind:key" is looked up here first.
CREATE TABLE resources(
    kind TEXT NOT NULL,
    key TEXT NOT NULL,
    mime TEXT NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY (kind, key)
) WITHOUT ROWID;

-- Online sources (optional): when a file is not stored, its URL is built from
-- these templates in priority order.
CREATE TABLE resource_sources(
    kind TEXT NOT NULL,
    priority INTEGER NOT NULL,
    template TEXT NOT NULL,
    PRIMARY KEY (kind, priority)
) WITHOUT ROWID;
