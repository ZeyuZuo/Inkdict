//! Opening a pack (spec/pack-v0.md §1, §2).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use zstd::dict::DecoderDictionary;

use crate::error::{Error, Result};
use crate::meta::Meta;
use crate::{APPLICATION_ID, FORMAT_VERSION};

/// An open `.zdpack`, read-only. `Send + Sync`, so apps can share one
/// instance (UniFFI objects, Tauri state); queries run one at a time.
pub struct Pack {
    db: Mutex<Connection>,
    meta: Meta,
    /// Prepared once: every entry body is compressed with it (spec/pack-v0.md §2.3).
    pub(crate) dictionary: Option<DecoderDictionary<'static>>,
}

impl Pack {
    /// Open a pack after checking its identification and `meta`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = open_immutable(path.as_ref())?;
        // SQLite stores both PRAGMAs as signed 32-bit integers.
        let application_id =
            db.query_row("PRAGMA application_id", [], |row| row.get::<_, i32>(0))? as u32;
        if application_id != APPLICATION_ID {
            return Err(Error::NotAPack(application_id));
        }
        let version = db.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))? as u32;
        if version != FORMAT_VERSION {
            return Err(Error::UnsupportedVersion(version));
        }
        let meta = Meta::new(read_meta(&db)?)?;
        let dictionary = match meta.zstd_dict() {
            Some(name) => Some(DecoderDictionary::copy(&read_file(&db, name)?)),
            None => None,
        };
        Ok(Pack {
            db: Mutex::new(db),
            meta,
            dictionary,
        })
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    /// Body of a file in `files`: `types.toml`, `themes/<name>.toml`, `zstd.dict`.
    pub fn file(&self, name: &str) -> Result<Vec<u8>> {
        read_file(&self.db(), name)
    }

    /// Names in `files`, sorted.
    pub fn file_names(&self) -> Result<Vec<String>> {
        let db = self.db();
        let mut statement = db.prepare("SELECT name FROM files ORDER BY name")?;
        let names = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(names)
    }

    pub(crate) fn db(&self) -> MutexGuard<'_, Connection> {
        // A panic while querying leaves a read-only connection usable.
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Packs never change once built: `immutable=1` skips locking and journal
/// checks, so a pack opens from read-only storage even if built in WAL mode.
fn open_immutable(path: &Path) -> Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Some(path) = path.to_str() else {
        // A URI must be UTF-8; such paths open without the immutable hint.
        return Ok(Connection::open_with_flags(path, flags)?);
    };
    let mut uri = String::from("file:");
    for ch in path.chars() {
        match ch {
            '%' | '?' | '#' => uri.push_str(&format!("%{:02X}", ch as u32)),
            _ => uri.push(ch),
        }
    }
    uri.push_str("?immutable=1");
    Ok(Connection::open_with_flags(
        uri,
        flags | OpenFlags::SQLITE_OPEN_URI,
    )?)
}

fn read_meta(db: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = db.prepare("SELECT key, value FROM meta")?;
    let values = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(values)
}

fn read_file(db: &Connection, name: &str) -> Result<Vec<u8>> {
    db.query_row("SELECT body FROM files WHERE name = ?", [name], |row| {
        row.get(0)
    })
    .optional()?
    .ok_or_else(|| Error::MissingFile(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MetaProblem;
    use crate::test_pack::{META, TestPack};

    fn with_meta(extra: (&str, &str)) -> TestPack {
        let mut meta = META.to_vec();
        meta.push(extra);
        TestPack::with(APPLICATION_ID, FORMAT_VERSION, &meta)
    }

    #[test]
    fn opens_a_pack_without_dictionary() {
        let test = TestPack::new();
        test.file("types.toml", b"[meta]");
        let pack = test.open();
        assert_eq!(pack.meta().id(), "test");
        assert_eq!(pack.meta().fts_langs(), ["en", "zh-Hans"]);
        assert_eq!(pack.meta().zstd_dict(), None);
        assert!(pack.dictionary.is_none());
        assert_eq!(pack.file_names().unwrap(), ["types.toml"]);
        assert_eq!(pack.file("types.toml").unwrap(), b"[meta]");
        assert!(
            matches!(pack.file("themes/x.toml"), Err(Error::MissingFile(name)) if name == "themes/x.toml")
        );
    }

    #[test]
    fn loads_the_named_dictionary() {
        let test = with_meta(("zstd_dict", "zstd.dict"));
        test.file("zstd.dict", b"dictionary");
        assert!(test.open().dictionary.is_some());
    }

    #[test]
    fn empty_dictionary_name_means_none() {
        assert!(with_meta(("zstd_dict", "")).open().dictionary.is_none());
    }

    #[test]
    fn opens_paths_with_uri_characters() {
        let test = TestPack::new();
        let odd = test.dir().join("a b?#%20.zdpack");
        std::fs::rename(test.path(), &odd).unwrap();
        assert_eq!(Pack::open(odd).unwrap().meta().id(), "test");
    }

    #[cfg(unix)]
    #[test]
    fn opens_a_wal_pack_in_a_read_only_directory() {
        use std::os::unix::fs::PermissionsExt;
        let test = TestPack::new();
        test.sql("PRAGMA journal_mode = WAL");
        let mode = |bits| std::fs::Permissions::from_mode(bits);
        std::fs::set_permissions(test.dir(), mode(0o555)).unwrap();
        let opened = Pack::open(test.path());
        std::fs::set_permissions(test.dir(), mode(0o755)).unwrap();
        assert_eq!(opened.unwrap().meta().id(), "test");
    }

    #[test]
    fn pack_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Pack>();
    }

    #[test]
    fn rejects_other_sqlite_files() {
        let test = TestPack::with(0, FORMAT_VERSION, &META);
        assert!(matches!(Pack::open(test.path()), Err(Error::NotAPack(0))));
    }

    #[test]
    fn rejects_other_format_versions() {
        let test = TestPack::with(APPLICATION_ID, 1, &META);
        assert!(matches!(
            Pack::open(test.path()),
            Err(Error::UnsupportedVersion(1))
        ));
    }

    #[test]
    fn rejects_files_that_are_not_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text.zdpack");
        std::fs::write(&path, "not a database, just some text that is long enough").unwrap();
        assert!(matches!(Pack::open(path), Err(Error::Sqlite(_))));
    }

    #[test]
    fn rejects_bad_meta() {
        // (key, replacement value or None to drop the key)
        let cases = [
            ("title", None),
            ("format", Some("zdict-theme")),
            ("format_version", Some("1")),
            ("compression", Some("gzip")),
            ("fts_langs", Some("en")),
            ("fts_langs", Some("[]")),
        ];
        for (key, value) in cases {
            let mut meta: Vec<_> = META.iter().copied().filter(|(k, _)| *k != key).collect();
            meta.extend(value.map(|value| (key, value)));
            let test = TestPack::with(APPLICATION_ID, FORMAT_VERSION, &meta);
            match Pack::open(test.path()) {
                Err(Error::Meta {
                    key: found,
                    problem,
                }) => {
                    assert_eq!(found, key);
                    assert_eq!(
                        problem == MetaProblem::Missing,
                        value.is_none(),
                        "{key}: {problem}"
                    );
                }
                other => panic!("{key}: expected a meta error, got {:?}", other.err()),
            }
        }
    }

    #[test]
    fn rejects_a_missing_dictionary() {
        let test = with_meta(("zstd_dict", "zstd.dict"));
        assert!(
            matches!(Pack::open(test.path()), Err(Error::MissingFile(name)) if name == "zstd.dict")
        );
    }
}
