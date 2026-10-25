//! Full-text search in the `fts` table (spec/pack-v0.md §4).

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::pack::Pack;
use crate::text::fts_query;

/// FTS rowid layout: `entry << 24 | anchor << 8 | lang`.
pub const ANCHOR_BITS: u32 = 16;
pub const LANG_BITS: u32 = 8;

/// One matching document: the text of one anchor in one language.
/// Field for field the Python reader's `Hit`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hit {
    pub entry: String,
    pub headword: String,
    pub homograph: Option<i64>,
    /// Anchor number inside the entry; 0 is the entry itself.
    pub anchor: u32,
    /// BCP-47 tag from `meta.fts_langs`.
    pub lang: String,
    /// bm25 with the `text` column weighted 2; lower is better.
    pub score: f64,
}

/// The FTS column to search; `None` in [`Pack::search`] searches both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    /// Headwords, definitions, labels, notes: everything but examples.
    Text,
    Example,
}

impl Column {
    fn name(self) -> &'static str {
        match self {
            Column::Text => "text",
            Column::Example => "example",
        }
    }
}

/// Entry row id, anchor number and language index of an FTS rowid.
pub fn split_rowid(rowid: i64) -> (i64, u32, usize) {
    let anchor = (rowid >> LANG_BITS) & ((1 << ANCHOR_BITS) - 1);
    let lang = rowid & ((1 << LANG_BITS) - 1);
    (
        rowid >> (ANCHOR_BITS + LANG_BITS),
        anchor as u32,
        lang as usize,
    )
}

impl Pack {
    /// Best matches for the words of `text`, all of which must occur.
    /// CJK input matches as character phrases (spec/pack-v0.md §4).
    pub fn search(&self, text: &str, limit: usize, column: Option<Column>) -> Result<Vec<Hit>> {
        let mut query = fts_query(text);
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        if let Some(column) = column {
            query = format!("{{{}}}: ({query})", column.name());
        }
        let db = self.db();
        let mut matches = db.prepare_cached(
            "SELECT rowid, bm25(fts, 2.0, 1.0) AS score FROM fts WHERE fts MATCH ? \
             ORDER BY score LIMIT ?",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows: Vec<(i64, f64)> = matches
            .query_map(rusqlite::params![query, limit], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut entries =
            db.prepare_cached("SELECT key, headword, homograph FROM entries WHERE id = ?")?;
        let mut hits = Vec::with_capacity(rows.len());
        for (rowid, score) in rows {
            let (entry, anchor, lang) = split_rowid(rowid);
            let (key, headword, homograph): (String, String, Option<i64>) = entries
                .query_row([entry], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .optional()?
                .ok_or_else(|| Error::MissingEntry(entry.to_string()))?;
            let lang =
                self.meta()
                    .fts_langs()
                    .get(lang)
                    .cloned()
                    .ok_or_else(|| Error::BadEntry {
                        key: key.clone(),
                        reason: format!("FTS language {lang} not in meta.fts_langs"),
                    })?;
            hits.push(Hit {
                entry: key,
                headword,
                homograph,
                anchor,
                lang,
                score,
            });
        }
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_pack::TestPack;

    fn rowid(entry: i64, anchor: i64, lang: i64) -> i64 {
        entry << 24 | anchor << 8 | lang
    }

    fn pack() -> Pack {
        let test = TestPack::new();
        test.entry(
            1,
            "apple",
            None,
            r#"{"type":"entry","id":"k1","lang":"en"}"#,
        )
        .entry(
            2,
            "pie",
            Some(1),
            r#"{"type":"entry","id":"k2","lang":"en"}"#,
        );
        test.sql(&format!(
            "INSERT INTO fts(rowid, text, example) VALUES
               ({}, 'apple a round fruit', NULL),
               ({}, NULL, 'an apple pie'),
               ({}, '苹 果', NULL),
               ({}, 'pie baked dish', NULL);",
            rowid(1, 0, 0),
            rowid(1, 3, 0),
            rowid(1, 0, 1),
            rowid(2, 0, 0),
        ));
        test.open()
    }

    #[test]
    fn splits_rowids() {
        assert_eq!(split_rowid(rowid(39960, 65535, 255)), (39960, 65535, 255));
        assert_eq!(split_rowid(rowid(1, 0, 1)), (1, 0, 1));
    }

    #[test]
    fn finds_words_phrases_and_columns() {
        let pack = pack();
        let found = |q, column| -> Vec<_> {
            pack.search(q, 10, column)
                .unwrap()
                .into_iter()
                .map(|h| (h.entry, h.anchor, h.lang))
                .collect()
        };
        let hit = |e: &str, a, l: &str| (e.to_owned(), a, l.to_owned());
        assert_eq!(found("apples", None).len(), 2); // porter stemming
        assert_eq!(found("apple", Some(Column::Example)), [hit("k1", 3, "en")]);
        assert_eq!(found("apple pie", Some(Column::Text)), []);
        assert_eq!(found("苹果", None), [hit("k1", 0, "zh-Hans")]);
        assert_eq!(found("果苹", None), []);
        assert_eq!(found("PIE", Some(Column::Text)), [hit("k2", 0, "en")]);
        assert!(found(" - ", None).is_empty());
        assert!(pack.search("apple", 0, None).unwrap().is_empty());
    }

    #[test]
    fn reports_rowids_outside_the_pack() {
        let test = TestPack::new();
        test.sql(&format!(
            "INSERT INTO fts(rowid, text) VALUES ({}, 'orphan')",
            rowid(9, 0, 0)
        ));
        let found = test.open().search("orphan", 5, None);
        assert!(matches!(found, Err(Error::MissingEntry(id)) if id == "9"));
    }
}
