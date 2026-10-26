//! Small packs built in a temporary directory, for unit tests.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::pack::Pack;
use crate::{APPLICATION_ID, FORMAT_VERSION};

pub(crate) const META: [(&str, &str); 10] = [
    ("format", "zdict-pack"),
    ("format_version", "1"),
    ("compression", "zstd"),
    ("id", "test"),
    ("title", "Test"),
    ("source_lang", "en"),
    ("target_langs", r#"["zh-Hans"]"#),
    ("themes", r#"["default"]"#),
    ("norm", "fold-1"),
    ("unicode_version", "14.0.0"),
];

/// The tables of spec/pack-v1.sql.
const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE files(name TEXT PRIMARY KEY, body BLOB NOT NULL) WITHOUT ROWID;
CREATE TABLE entries(id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE,
    headword TEXT NOT NULL, homograph INTEGER, body BLOB NOT NULL);
CREATE TABLE lookup(norm TEXT NOT NULL, rank INTEGER NOT NULL, entry INTEGER NOT NULL,
    anchor INTEGER NOT NULL, form TEXT NOT NULL, kind TEXT NOT NULL,
    PRIMARY KEY(norm, rank, entry, anchor)) WITHOUT ROWID;
CREATE TABLE summaries(entry INTEGER NOT NULL, anchor INTEGER NOT NULL, lang TEXT NOT NULL,
    text TEXT NOT NULL, PRIMARY KEY(entry, anchor)) WITHOUT ROWID;
CREATE TABLE resources(kind TEXT NOT NULL, key TEXT NOT NULL, mime TEXT NOT NULL,
    body BLOB NOT NULL, PRIMARY KEY(kind, key)) WITHOUT ROWID;
CREATE TABLE resource_sources(kind TEXT NOT NULL, priority INTEGER NOT NULL,
    template TEXT NOT NULL, PRIMARY KEY(kind, priority)) WITHOUT ROWID;
";

pub(crate) struct TestPack {
    dir: tempfile::TempDir,
    path: PathBuf,
}

impl TestPack {
    /// A valid, empty pack.
    pub fn new() -> Self {
        Self::with(APPLICATION_ID, FORMAT_VERSION, &META)
    }

    pub fn with(application_id: u32, version: u32, meta: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.zdpack");
        let pack = TestPack { dir, path };
        pack.sql(&format!(
            "PRAGMA application_id = {}; PRAGMA user_version = {version}; {SCHEMA}",
            application_id as i32
        ));
        for (key, value) in meta {
            pack.db()
                .execute("INSERT INTO meta VALUES (?, ?)", [key, value])
                .unwrap();
        }
        pack
    }

    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn open(&self) -> Pack {
        Pack::open(&self.path).unwrap()
    }

    pub fn sql(&self, sql: &str) -> &Self {
        self.db().execute_batch(sql).unwrap();
        self
    }

    pub fn file(&self, name: &str, body: &[u8]) -> &Self {
        self.db()
            .execute("INSERT INTO files VALUES (?, ?)", params![name, body])
            .unwrap();
        self
    }

    /// An entry compressed without a dictionary; `json` is stored as given.
    pub fn entry(&self, id: i64, headword: &str, homograph: Option<i64>, json: &str) -> &Self {
        let key = serde_json::from_str::<serde_json::Value>(json).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let body = zstd::bulk::compress(json.as_bytes(), 3).unwrap();
        self.db()
            .execute(
                "INSERT INTO entries VALUES (?, ?, ?, ?, ?)",
                params![id, key, headword, homograph, body],
            )
            .unwrap();
        self
    }

    /// A lookup row; `norm` is stored as given.
    pub fn lookup(
        &self,
        norm: &str,
        rank: i64,
        entry: i64,
        anchor: u32,
        form: &str,
        kind: &str,
    ) -> &Self {
        self.db()
            .execute(
                "INSERT INTO lookup VALUES (?, ?, ?, ?, ?, ?)",
                params![norm, rank, entry, anchor, form, kind],
            )
            .unwrap();
        self
    }

    fn db(&self) -> Connection {
        Connection::open(&self.path).unwrap()
    }
}
