//! Reading entries (spec/pack-v0.md §2.3).

use std::io::Read;

use rusqlite::{OptionalExtension, ToSql};

use crate::error::{Error, Result};
use crate::node::{Entry, Node};
use crate::pack::Pack;

/// Decoded entries larger than this are rejected (Oxford's largest: 225 KB),
/// so a hostile pack cannot exhaust memory.
const MAX_ENTRY_BYTES: u64 = 8 << 20;
/// Largest zstd window the decoder allocates: 8 MiB, the same bound.
const WINDOW_LOG_MAX: u32 = 23;

const SELECT: &str = "SELECT id, key, headword, homograph, body FROM entries WHERE";

struct Row {
    id: i64,
    key: String,
    headword: String,
    homograph: Option<i64>,
    body: Vec<u8>,
}

impl Pack {
    /// The entry with this key (`entry.id` of the content).
    pub fn entry(&self, key: &str) -> Result<Entry> {
        self.decode(self.row("key", &key, key)?)
    }

    /// The entry with this row id, as found in lookup and search results.
    pub fn entry_by_id(&self, id: i64) -> Result<Entry> {
        self.decode(self.row("id", &id, &id.to_string())?)
    }

    /// The entry's content JSON exactly as stored, for apps that pass it on
    /// without building a node tree.
    pub fn entry_json(&self, key: &str) -> Result<String> {
        let row = self.row("key", &key, key)?;
        let json = self.decompress(&row.key, &row.body)?;
        String::from_utf8(json).map_err(|err| bad_entry(&row.key, err))
    }

    fn row(&self, column: &str, value: &dyn ToSql, shown: &str) -> Result<Row> {
        let db = self.db();
        let mut statement = db.prepare_cached(&format!("{SELECT} {column} = ?"))?;
        statement
            .query_row([value], |row| {
                Ok(Row {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    headword: row.get(2)?,
                    homograph: row.get(3)?,
                    body: row.get(4)?,
                })
            })
            .optional()?
            .ok_or_else(|| Error::MissingEntry(shown.to_owned()))
    }

    fn decode(&self, row: Row) -> Result<Entry> {
        let json = self.decompress(&row.key, &row.body)?;
        let root: Node = serde_json::from_slice(&json).map_err(|err| bad_entry(&row.key, err))?;
        Ok(Entry {
            id: row.id,
            key: row.key,
            headword: row.headword,
            homograph: row.homograph,
            root,
        })
    }

    fn decompress(&self, key: &str, body: &[u8]) -> Result<Vec<u8>> {
        use zstd::stream::read::Decoder;
        let mut decoder = match &self.dictionary {
            Some(dictionary) => Decoder::with_prepared_dictionary(body, dictionary),
            None => Decoder::with_buffer(body),
        }
        .map_err(|err| bad_entry(key, err))?;
        decoder
            .window_log_max(WINDOW_LOG_MAX)
            .map_err(|err| bad_entry(key, err))?;
        let mut json = Vec::new();
        decoder
            .take(MAX_ENTRY_BYTES + 1)
            .read_to_end(&mut json)
            .map_err(|err| bad_entry(key, err))?;
        if json.len() as u64 > MAX_ENTRY_BYTES {
            return Err(bad_entry(
                key,
                format!("larger than {MAX_ENTRY_BYTES} bytes"),
            ));
        }
        Ok(json)
    }
}

fn bad_entry(key: &str, reason: impl ToString) -> Error {
    Error::BadEntry {
        key: key.to_owned(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Attr, Child};
    use crate::test_pack::TestPack;

    const APPLE: &str = r#"{"type":"entry","id":"k1","lang":"en","children":[{"type":"head","children":[{"type":"headword","text":"apple"}]},{"type":"sense","id":"s1","attrs":{"n":1},"children":["a ",{"type":"definition","text":"fruit"}]}]}"#;

    fn pack() -> Pack {
        let test = TestPack::new();
        test.entry(7, "apple", Some(2), APPLE);
        test.open()
    }

    #[test]
    fn reads_an_entry_by_key_and_by_id() {
        let pack = pack();
        let entry = pack.entry("k1").unwrap();
        assert_eq!(
            (entry.id, entry.headword.as_str(), entry.homograph),
            (7, "apple", Some(2))
        );
        assert_eq!(entry.root.lang.as_deref(), Some("en"));
        let sense = entry.root.child_nodes().nth(1).unwrap();
        assert_eq!(sense.attr("n"), Some(&Attr::Int(1)));
        assert_eq!(sense.children[0], Child::Text("a ".into()));
        assert_eq!(pack.entry_by_id(7).unwrap(), entry);
        assert_eq!(pack.entry_json("k1").unwrap(), APPLE);
    }

    #[test]
    fn reports_missing_entries() {
        let pack = pack();
        assert!(matches!(pack.entry("nope"), Err(Error::MissingEntry(key)) if key == "nope"));
        assert!(matches!(pack.entry_by_id(8), Err(Error::MissingEntry(key)) if key == "8"));
    }

    #[test]
    fn reports_corrupt_bodies() {
        let test = TestPack::new();
        test.sql("INSERT INTO entries VALUES (1, 'k', 'w', NULL, x'00010203')");
        test.entry(2, "w", None, r#"{"id":"k2","text":"no type"}"#);
        let pack = test.open();
        assert!(matches!(pack.entry("k"), Err(Error::BadEntry { key, .. }) if key == "k"));
        assert!(matches!(pack.entry("k2"), Err(Error::BadEntry { key, .. }) if key == "k2"));
    }

    #[test]
    fn rejects_entries_over_the_size_limit() {
        let test = TestPack::new();
        let text = "a".repeat(MAX_ENTRY_BYTES as usize);
        test.entry(
            1,
            "big",
            None,
            &format!(r#"{{"type":"entry","id":"big","text":"{text}"}}"#),
        );
        let found = test.open().entry_json("big");
        assert!(matches!(found, Err(Error::BadEntry { reason, .. }) if reason.contains("larger")));
    }
}
