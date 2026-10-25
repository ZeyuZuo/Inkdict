//! Suggestions from the `lookup` table (spec/pack-v0.md §3).

use std::collections::HashSet;

use serde::Serialize;

use crate::error::Result;
use crate::pack::Pack;
use crate::text::norm;

/// Exclusive upper bound of a prefix range: no stored key continues past it.
const PREFIX_END: char = '\u{10FFFF}';

/// One suggested form, field for field the Python reader's `Suggestion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    /// The matched form, e.g. `went`.
    pub form: String,
    /// Type of the form node: `headword`, `infl_form`, `phrase`…
    pub kind: String,
    /// Entry key.
    pub entry: String,
    /// Headword of that entry, e.g. `go`.
    pub headword: String,
    pub homograph: Option<i64>,
    /// Anchor number inside the entry; 0 is the entry itself.
    pub anchor: u32,
}

impl Pack {
    /// Forms starting with the input: exact matches first, then shorter
    /// forms, then by rank. One result per (entry, anchor).
    pub fn suggest(&self, text: &str, limit: usize) -> Result<Vec<Suggestion>> {
        let key = norm(text);
        if key.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let db = self.db();
        let mut statement = db.prepare_cached(
            "SELECT l.form, l.kind, e.key, e.headword, e.homograph, l.anchor FROM lookup l \
             JOIN entries e ON e.id = l.entry WHERE l.norm >= ?1 AND l.norm < ?2 \
             ORDER BY l.norm = ?1 DESC, length(l.norm), l.rank, l.entry LIMIT ?3",
        )?;
        // Rows repeat per (entry, anchor) under different keys; fetch extra.
        let fetch = i64::try_from(limit.saturating_mul(3)).unwrap_or(i64::MAX);
        let rows = statement.query_map(
            rusqlite::params![key, format!("{key}{PREFIX_END}"), fetch],
            |row| {
                Ok(Suggestion {
                    form: row.get(0)?,
                    kind: row.get(1)?,
                    entry: row.get(2)?,
                    headword: row.get(3)?,
                    homograph: row.get(4)?,
                    anchor: row.get(5)?,
                })
            },
        )?;
        let mut seen = HashSet::new();
        let mut found = Vec::new();
        for row in rows {
            let suggestion = row?;
            if seen.insert((suggestion.entry.clone(), suggestion.anchor)) {
                found.push(suggestion);
                if found.len() == limit {
                    break;
                }
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_pack::TestPack;

    fn entry(id: i64) -> String {
        format!(r#"{{"type":"entry","id":"k{id}","lang":"en"}}"#)
    }

    #[test]
    fn orders_exact_then_shorter_then_rank_and_dedups() {
        let test = TestPack::new();
        for id in 1..=3 {
            test.entry(id, &format!("w{id}"), None, &entry(id));
        }
        test.lookup("go", 2, 1, 0, "go", "headword")
            .lookup("goes", 8, 1, 0, "goes", "infl_form") // same (entry, anchor) as "go"
            .lookup("gone", 0, 2, 0, "gone", "headword")
            .lookup("goa", 9, 3, 0, "Goa", "headword")
            .lookup("go", 0, 3, 4, "go", "phrase")
            .lookup("ga", 0, 3, 0, "ga", "headword");
        let pack = test.open();
        let found: Vec<_> = pack
            .suggest("GO", 10)
            .unwrap()
            .into_iter()
            .map(|s| (s.form, s.entry, s.anchor))
            .collect();
        let wanted = [
            ("go", "k3", 4),
            ("go", "k1", 0),
            ("Goa", "k3", 0),
            ("gone", "k2", 0),
        ];
        let wanted: Vec<_> = wanted
            .iter()
            .map(|(f, e, a)| (f.to_string(), e.to_string(), *a))
            .collect();
        assert_eq!(found, wanted);
        assert_eq!(pack.suggest("go", 1).unwrap().len(), 1);
        assert!(pack.suggest("go", 0).unwrap().is_empty());
        assert!(pack.suggest(" -", 5).unwrap().is_empty());
        assert_eq!(pack.suggest("go", 2).unwrap()[1].headword, "w1");
    }
}
