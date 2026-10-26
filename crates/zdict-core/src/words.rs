//! The word list (spec/pack-v1.md §5): forms starting with the input, words
//! containing it, then close spellings when neither found anything. One row
//! per entry.
//!
//! The `lookup` table and the entries' headwords are read into memory on
//! first use (about 0.1 s for Oxford, see [`Pack::prepare_words`]); every
//! query then runs without SQLite except for the summaries it shows. A port
//! of OxDic's reference reader `oxdic/pack/reader.py` (`Pack.words`).

use std::collections::{HashMap, HashSet};

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::fuzzy::{FuzzyIndex, letter_difference, max_distance};
use crate::pack::Pack;
use crate::registry::IndexKind;
use crate::text::norm;

impl IndexKind {
    /// Rows that show as the entry's headword instead of their own form.
    fn shows_headword(self) -> bool {
        matches!(self, IndexKind::Reading | IndexKind::Hidden)
    }

    /// Forms of the entry's own word: without a summary of their own they
    /// show the entry's.
    fn same_word(self) -> bool {
        matches!(
            self,
            IndexKind::Variant | IndexKind::Inflection | IndexKind::Reading | IndexKind::Hidden
        )
    }

    /// Kinds the "contains" group lists.
    fn in_contains(self) -> bool {
        matches!(
            self,
            IndexKind::Headword | IndexKind::Variant | IndexKind::Phrase
        )
    }
}

/// Which part of the word list a row belongs to (spec/pack-v1.md §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    /// The key starts with the input.
    Prefix,
    /// The key contains the input after its first character.
    Contains,
    /// The key is a few edits away; only when the other groups are empty.
    Close,
}

/// One row of the word list: one entry. Field for field the Python reader's `Word`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    pub group: Group,
    /// What the row shows: the form, or the headword for readings and hidden forms.
    pub form: String,
    /// Index kind of the representative row.
    pub kind: IndexKind,
    /// Entry key.
    pub entry: String,
    pub headword: String,
    pub homograph: Option<i64>,
    /// Where opening the row lands; 0 is the top of the entry.
    pub anchor: u32,
    /// Matched readings, when the representative row is a reading.
    pub readings: Vec<String>,
    pub summary: Option<String>,
    /// Language of the summary, to pick its font.
    pub summary_lang: Option<String>,
    /// Edits from the input, in the close group.
    pub distance: Option<usize>,
    pub rank: i64,
}

impl Word {
    /// Headword a variant or inflection points to, when it differs from the form.
    pub fn target(&self) -> Option<&str> {
        let points = matches!(self.kind, IndexKind::Variant | IndexKind::Inflection);
        (points && self.form != self.headword).then_some(self.headword.as_str())
    }
}

/// One `lookup` row.
struct Row {
    norm: Box<str>,
    /// Characters in `norm`, as SQLite's `length()` counts them.
    length: usize,
    rank: i64,
    entry: i64,
    anchor: u32,
    form: Box<str>,
    kind: IndexKind,
}

/// The `entries` columns a word list row shows.
struct EntryInfo {
    key: String,
    headword: String,
    homograph: Option<i64>,
}

/// A merged row before it becomes a [`Word`]: built for every match, so it
/// borrows instead of copying.
struct Merged<'a> {
    form: &'a str,
    /// Lowest rank of the merged rows.
    best: &'a Row,
    distance: Option<usize>,
    readings: Vec<&'a str>,
}

/// The whole `lookup` table in key order, and what the rows show.
pub(crate) struct WordIndex {
    /// Sorted by the table's primary key: norm, rank, entry, anchor.
    rows: Vec<Row>,
    entries: HashMap<i64, EntryInfo>,
    keys: FuzzyIndex,
}

impl WordIndex {
    fn build(pack: &Pack) -> Result<Self> {
        let db = pack.db();
        let mut statement = db.prepare("SELECT id, key, headword, homograph FROM entries")?;
        let entries = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    EntryInfo {
                        key: row.get(1)?,
                        headword: row.get(2)?,
                        homograph: row.get(3)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?;
        let mut statement = db.prepare(
            "SELECT norm, rank, entry, anchor, form, kind FROM lookup \
             ORDER BY norm, rank, entry, anchor",
        )?;
        let mut rows = Vec::new();
        let mut found = statement.query([])?;
        while let Some(row) = found.next()? {
            let kind: String = row.get(5)?;
            let kind = IndexKind::parse(&kind).ok_or_else(|| Error::BadFile {
                name: "lookup".to_owned(),
                reason: format!("unknown index kind {kind:?}"),
            })?;
            let norm: String = row.get(0)?;
            let entry = row.get(2)?;
            // A row of a missing entry could not be opened: leave it out.
            if !entries.contains_key(&entry) {
                continue;
            }
            rows.push(Row {
                length: norm.chars().count(),
                norm: norm.into(),
                rank: row.get(1)?,
                entry,
                anchor: row.get(3)?,
                form: row.get::<_, String>(4)?.into(),
                kind,
            });
        }
        let keys = FuzzyIndex::build(
            rows.chunk_by(|a, b| a.norm == b.norm)
                .map(|same| (&*same[0].norm, same[0].rank)),
        )?;
        Ok(WordIndex {
            rows,
            entries,
            keys,
        })
    }

    /// Rows whose key is exactly `key`, in primary key order.
    fn equal(&self, key: &str) -> &[Row] {
        let start = self.rows.partition_point(|row| *row.norm < *key);
        let end = start + self.rows[start..].partition_point(|row| *row.norm == *key);
        &self.rows[start..end]
    }

    /// Keys starting with `key`: that key itself first, then shorter keys, then rank.
    fn prefix(&self, key: &str) -> Vec<(&Row, Option<usize>)> {
        let start = self.rows.partition_point(|row| *row.norm < *key);
        let mut found: Vec<_> = self.rows[start..]
            .iter()
            .take_while(|row| row.norm.starts_with(key))
            .collect();
        found.sort_by_key(|row| {
            (
                *row.norm != *key,
                row.length,
                row.rank,
                row.entry,
                row.anchor,
            )
        });
        found.into_iter().map(|row| (row, None)).collect()
    }

    /// Headwords, variants and phrases containing `key` after their first character.
    fn contains(&self, key: &str) -> Vec<(&Row, Option<usize>)> {
        let mut found: Vec<_> = self
            .rows
            .iter()
            .filter(|row| row.kind.in_contains() && after_first(&row.norm).contains(key))
            .collect();
        found.sort_by_key(|row| (row.rank, row.length, &row.norm, row.entry, row.anchor));
        found.into_iter().map(|row| (row, None)).collect()
    }

    /// Keys within the allowed edits of `key`, ordered by key (distance,
    /// letter difference, best rank, length difference, the key), each
    /// key's rows by rank and entry.
    fn close(&self, key: &str, length: usize) -> Vec<(&Row, Option<usize>)> {
        let mut candidates = self.keys.search(key, max_distance(length));
        candidates.sort_by_cached_key(|(candidate, distance, rank)| {
            (
                *distance,
                letter_difference(key, candidate),
                *rank,
                candidate.chars().count().abs_diff(length),
                candidate.clone(),
            )
        });
        candidates
            .iter()
            .flat_map(|(candidate, distance, _)| {
                self.equal(candidate)
                    .iter()
                    .map(|row| (row, Some(*distance)))
            })
            .collect()
    }

    /// Rows with the same shown form and entry become one; the lowest rank
    /// represents it, the earliest row places it (spec/pack-v1.md §5.2).
    /// Items in `shown` belong to an earlier group and are skipped.
    fn merge<'a>(
        &'a self,
        rows: Vec<(&'a Row, Option<usize>)>,
        shown: &mut HashSet<(&'a str, i64)>,
    ) -> Vec<Merged<'a>> {
        let mut merged: Vec<Merged> = Vec::new();
        let mut at: HashMap<(&str, i64), usize> = HashMap::new();
        for (row, distance) in rows {
            let form = if row.kind.shows_headword() {
                self.entries[&row.entry].headword.as_str()
            } else {
                &row.form
            };
            let item = (form, row.entry);
            let i = match at.get(&item) {
                Some(&i) => {
                    if row.rank < merged[i].best.rank {
                        merged[i].best = row;
                    }
                    i
                }
                None => {
                    if !shown.insert(item) {
                        continue;
                    }
                    at.insert(item, merged.len());
                    merged.push(Merged {
                        form,
                        best: row,
                        distance,
                        readings: Vec::new(),
                    });
                    merged.len() - 1
                }
            };
            if row.kind == IndexKind::Reading && !merged[i].readings.contains(&&*row.form) {
                merged[i].readings.push(&row.form);
            }
        }
        merged
    }

    /// The word and its entry's row id.
    fn word(&self, group: Group, merged: Merged) -> (i64, Word) {
        let info = &self.entries[&merged.best.entry];
        let kind = merged.best.kind;
        let readings = if kind == IndexKind::Reading {
            merged
                .readings
                .iter()
                .map(|&reading| reading.to_owned())
                .collect()
        } else {
            Vec::new()
        };
        let word = Word {
            group,
            form: merged.form.to_owned(),
            kind,
            entry: info.key.clone(),
            headword: info.headword.clone(),
            homograph: info.homograph,
            anchor: merged.best.anchor,
            readings,
            summary: None,
            summary_lang: None,
            distance: merged.distance,
            rank: merged.best.rank,
        };
        (merged.best.entry, word)
    }
}

/// The string without its first character.
fn after_first(text: &str) -> &str {
    let mut chars = text.chars();
    chars.next();
    chars.as_str()
}

impl Pack {
    /// The word list for an input, at most `limit` rows per group
    /// (spec/pack-v1.md §5). Groups are merged and deduplicated in full
    /// before each is cut, so `limit` never moves a row between groups.
    pub fn words(&self, text: &str, limit: usize) -> Result<Vec<Word>> {
        let key = norm(text, self.meta().norm());
        if key.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let index = self.word_index()?;
        let length = key.chars().count();
        let mut shown = HashSet::new();
        let mut groups = vec![(Group::Prefix, index.merge(index.prefix(&key), &mut shown))];
        if length >= 2 {
            groups.push((
                Group::Contains,
                index.merge(index.contains(&key), &mut shown),
            ));
        }
        let mut words: Vec<(i64, Word)> = groups
            .into_iter()
            .flat_map(|(group, merged)| {
                merged
                    .into_iter()
                    .take(limit)
                    .map(move |merged| index.word(group, merged))
            })
            .collect();
        if words.is_empty() && length >= 3 {
            let merged = index.merge(index.close(&key, length), &mut shown);
            words = merged
                .into_iter()
                .take(limit)
                .map(|merged| index.word(Group::Close, merged))
                .collect();
        }
        for (entry, word) in &mut words {
            (word.summary, word.summary_lang) = self.summary(*entry, word)?.unzip();
        }
        Ok(words.into_iter().map(|(_, word)| word).collect())
    }

    /// Read the word index now instead of on the first [`Pack::words`];
    /// returns the number of lookup rows.
    pub fn prepare_words(&self) -> Result<usize> {
        Ok(self.word_index()?.rows.len())
    }

    /// The row's own summary; without one, forms of the entry's word and
    /// rows at the top of the entry show the entry's (spec/pack-v1.md §5.2).
    fn summary(&self, entry: i64, word: &Word) -> Result<Option<(String, String)>> {
        let db = self.db();
        let mut statement =
            db.prepare_cached("SELECT text, lang FROM summaries WHERE entry = ? AND anchor = ?")?;
        let mut anchors = vec![word.anchor];
        if word.anchor != 0 && word.kind.same_word() {
            anchors.push(0);
        }
        for anchor in anchors {
            let found = statement
                .query_row(rusqlite::params![entry, anchor], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?;
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }

    fn word_index(&self) -> Result<&WordIndex> {
        if let Some(index) = self.words.get() {
            return Ok(index);
        }
        let index = WordIndex::build(self)?;
        Ok(self.words.get_or_init(|| index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_pack::TestPack;

    fn entry(test: &TestPack, id: i64, headword: &str, homograph: Option<i64>) {
        let json = format!(r#"{{"type":"entry","id":"k{id}","lang":"en"}}"#);
        test.entry(id, headword, homograph, &json);
    }

    /// (group, form, kind, entry, anchor) of each row.
    fn rows(
        pack: &Pack,
        text: &str,
        limit: usize,
    ) -> Vec<(Group, String, &'static str, String, u32)> {
        pack.words(text, limit)
            .unwrap()
            .into_iter()
            .map(|w| (w.group, w.form, w.kind.name(), w.entry, w.anchor))
            .collect()
    }

    fn row(
        group: Group,
        form: &str,
        kind: &'static str,
        entry: &str,
        anchor: u32,
    ) -> (Group, String, &'static str, String, u32) {
        (group, form.to_owned(), kind, entry.to_owned(), anchor)
    }

    /// lead¹ and lead² (homographs), leave with its inflection "left", left
    /// (adjective) with a hidden full form, and mislead containing "lead".
    fn sample() -> TestPack {
        let test = TestPack::new();
        entry(&test, 1, "lead", Some(1));
        entry(&test, 2, "lead", Some(2));
        entry(&test, 3, "leave", None);
        entry(&test, 4, "left", None);
        entry(&test, 5, "mislead", None);
        test.lookup("lead", 0, 1, 0, "lead", "headword")
            .lookup("leads", 4, 1, 0, "leads", "inflection")
            .lookup("lead", 1, 2, 0, "lead", "headword")
            .lookup("leave", 0, 3, 0, "leave", "headword")
            .lookup("left", 4, 3, 0, "left", "inflection")
            .lookup("left", 0, 4, 0, "left", "headword")
            .lookup("leftie", 6, 4, 0, "leftie", "hidden")
            .lookup("leadup", 3, 1, 2, "lead-up", "phrase")
            .lookup("mislead", 0, 5, 0, "mislead", "headword")
            .lookup("misleads", 4, 5, 0, "misleads", "inflection");
        test.sql(
            "INSERT INTO summaries VALUES (1, 0, 'zh-Hans', '带路'), (3, 0, 'zh-Hans', '离开'),
                                          (1, 2, 'zh-Hans', '准备阶段');",
        );
        test
    }

    #[test]
    fn homographs_and_inflections_of_other_entries_get_their_own_rows() {
        let pack = sample().open();
        assert_eq!(
            rows(&pack, "lead", 20),
            [
                row(Group::Prefix, "lead", "headword", "k1", 0),
                row(Group::Prefix, "lead", "headword", "k2", 0),
                row(Group::Prefix, "leads", "inflection", "k1", 0),
                row(Group::Prefix, "lead-up", "phrase", "k1", 2),
                row(Group::Contains, "mislead", "headword", "k5", 0),
            ]
        );
        // "left" the adjective and "left" of leave; leftie shows as its headword
        // and merges into the adjective's row.
        assert_eq!(
            rows(&pack, "left", 20),
            [
                row(Group::Prefix, "left", "headword", "k4", 0),
                row(Group::Prefix, "left", "inflection", "k3", 0),
            ]
        );
        let words = pack.words("left", 20).unwrap();
        assert_eq!(words[1].target(), Some("leave"));
        assert_eq!(words[0].target(), None);
        assert_eq!(pack.words("lead", 1).unwrap()[0].homograph, Some(1));
    }

    #[test]
    fn hidden_forms_show_the_headword_without_readings() {
        let pack = sample().open();
        let words = pack.words("leftie", 20).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(
            (
                words[0].form.as_str(),
                words[0].kind,
                words[0].readings.len()
            ),
            ("left", IndexKind::Hidden, 0)
        );
    }

    #[test]
    fn groups_never_repeat_a_row_and_limits_cut_each_group() {
        let pack = sample().open();
        // "misleads" is a prefix match of k5 only through its inflection; the
        // contains group does not list k5 again for "lead".
        assert_eq!(
            rows(&pack, "misl", 20),
            [
                row(Group::Prefix, "mislead", "headword", "k5", 0),
                row(Group::Prefix, "misleads", "inflection", "k5", 0),
            ]
        );
        assert_eq!(
            rows(&pack, "lead", 1),
            [
                row(Group::Prefix, "lead", "headword", "k1", 0),
                row(Group::Contains, "mislead", "headword", "k5", 0),
            ]
        );
        assert!(pack.words(" -", 5).unwrap().is_empty());
        assert!(pack.words("lead", 0).unwrap().is_empty());
    }

    #[test]
    fn close_spellings_only_when_nothing_else_matches() {
        let pack = sample().open();
        assert_eq!(
            rows(&pack, "laeve", 20),
            [row(Group::Close, "leave", "headword", "k3", 0)]
        );
        assert_eq!(pack.words("laeve", 20).unwrap()[0].distance, Some(1));
        // Prefix matches leave no room for close spellings.
        assert!(
            pack.words("lea", 20)
                .unwrap()
                .iter()
                .all(|w| w.group != Group::Close)
        );
        assert!(
            pack.words("lx", 20).unwrap().is_empty(),
            "two letters: no close group"
        );
    }

    #[test]
    fn summaries_fall_back_only_for_forms_of_the_same_word() {
        let pack = sample().open();
        let summary = |text: &str, n: usize| pack.words(text, 20).unwrap()[n].summary.clone();
        assert_eq!(summary("lead", 0).as_deref(), Some("带路"));
        assert_eq!(summary("lead", 1), None); // lead² has none
        assert_eq!(summary("leads", 0).as_deref(), Some("带路"));
        assert_eq!(summary("leadup", 0).as_deref(), Some("准备阶段"));
        assert_eq!(summary("left", 1).as_deref(), Some("离开"));
        assert_eq!(
            pack.words("lead", 20).unwrap()[0].summary_lang.as_deref(),
            Some("zh-Hans")
        );
    }

    #[test]
    fn readings_merge_into_the_headword_row() {
        let test = TestPack::new();
        entry(&test, 1, "和", None);
        test.lookup("和", 0, 1, 0, "和", "headword")
            .lookup("he", 5, 1, 0, "hé", "reading")
            .lookup("huo", 5, 1, 0, "huó", "reading")
            .lookup("he", 5, 1, 3, "hè", "reading");
        let words = test.open().words("h", 20).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(
            (words[0].form.as_str(), words[0].kind, words[0].anchor),
            ("和", IndexKind::Reading, 0)
        );
        assert_eq!(words[0].readings, ["hé", "hè", "huó"]);
    }
}
