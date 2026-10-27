
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE files(name TEXT PRIMARY KEY, body BLOB NOT NULL) WITHOUT ROWID;
CREATE TABLE entries(
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    headword TEXT NOT NULL,
    homograph INTEGER,
    body BLOB NOT NULL
);
CREATE TABLE lookup(
    norm TEXT NOT NULL,
    rank INTEGER NOT NULL,
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    form TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('headword', 'variant', 'inflection', 'phrase', 'reading', 'hidden')),
    PRIMARY KEY (norm, rank, entry, anchor)
) WITHOUT ROWID;
CREATE TABLE summaries(
    entry INTEGER NOT NULL REFERENCES entries(id),
    anchor INTEGER NOT NULL,
    lang TEXT NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (entry, anchor)
) WITHOUT ROWID;
CREATE TABLE resources(
    kind TEXT NOT NULL,
    key TEXT NOT NULL,
    mime TEXT NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY (kind, key)
) WITHOUT ROWID;
CREATE TABLE resource_sources(
    kind TEXT NOT NULL,
    priority INTEGER NOT NULL,
    template TEXT NOT NULL,
    PRIMARY KEY (kind, priority)
) WITHOUT ROWID;
