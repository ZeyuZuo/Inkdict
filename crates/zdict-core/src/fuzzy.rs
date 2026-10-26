//! Spelling correction over lookup keys (docs/roadmap.md M2).
//!
//! The distinct `lookup.norm` keys go into an FST (value: their best rank),
//! built in memory on first use: about 1 MB for Oxford instead of a
//! SymSpell delete table of tens of MB. A depth-first walk carries one row
//! of the optimal-string-alignment (Damerau) distance table per character
//! and prunes branches whose row minimum exceeds the limit.

use std::collections::{HashMap, HashSet};

use fst::raw::{Fst, Node, Output};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::lookup::Suggestion;
use crate::pack::Pack;
use crate::text::norm;

/// Lookup keys as an FST map: key -> lowest rank of its rows.
pub(crate) struct FuzzyIndex {
    keys: Fst<Vec<u8>>,
}

/// A form close to what was typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Correction {
    #[serde(flatten)]
    pub suggestion: Suggestion,
    /// Edits (insert, delete, substitute, swap neighbours) between the
    /// typed key and this form's key.
    pub distance: usize,
}

/// Edits allowed for a typed key of `chars` characters: none for one or
/// two (everything is close), one for three or four, two from five (a
/// five-letter typo is often a longer word with two letters dropped).
/// Distance sorts first, so a looser limit only adds candidates after.
pub fn max_distance(chars: usize) -> usize {
    match chars {
        0..=2 => 0,
        3..=4 => 1,
        _ => 2,
    }
}

impl FuzzyIndex {
    pub(crate) fn build(pack: &Pack) -> Result<Self> {
        let db = pack.db();
        let mut statement =
            db.prepare("SELECT norm, min(rank) FROM lookup GROUP BY norm ORDER BY norm")?;
        let mut builder = fst::MapBuilder::memory();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let rank: i64 = row.get(1)?;
            builder
                .insert(key.as_bytes(), u64::try_from(rank).unwrap_or(0))
                .map_err(index_error)?;
        }
        let bytes = builder.into_inner().map_err(index_error)?;
        Ok(FuzzyIndex {
            keys: Fst::new(bytes).map_err(index_error)?,
        })
    }

    /// Keys within `limit` edits of `query`: (key, distance, rank), unsorted.
    pub(crate) fn search(&self, query: &str, limit: usize) -> Vec<(String, usize, u64)> {
        let mut walk = Walk {
            fst: &self.keys,
            query: query.chars().collect(),
            limit,
            key: Vec::new(),
            chars: Vec::new(),
            rows: Vec::new(),
            found: Vec::new(),
        };
        walk.rows.push((0..=walk.query.len()).collect());
        walk.visit(self.keys.root(), Output::zero(), 0);
        walk.found
    }

    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }
}

struct Walk<'a> {
    fst: &'a Fst<Vec<u8>>,
    query: Vec<char>,
    limit: usize,
    /// Bytes of the key so far.
    key: Vec<u8>,
    /// Characters of the key so far, one distance row each (plus the first).
    chars: Vec<char>,
    rows: Vec<Vec<usize>>,
    found: Vec<(String, usize, u64)>,
}

impl Walk<'_> {
    /// `pending`: bytes of a character not yet complete.
    fn visit(&mut self, node: Node, output: Output, pending: usize) {
        if pending == 0 && node.is_final() {
            let distance = self
                .rows
                .last()
                .and_then(|row| row.last())
                .copied()
                .unwrap_or(usize::MAX);
            if distance <= self.limit {
                let key = String::from_utf8_lossy(&self.key).into_owned();
                self.found
                    .push((key, distance, output.cat(node.final_output()).value()));
            }
        }
        for transition in node.transitions() {
            let next = self.fst.node(transition.addr);
            let output = output.cat(transition.out);
            self.key.push(transition.inp);
            let needed = if pending == 0 {
                utf8_len(transition.inp)
            } else {
                pending
            };
            if needed > 1 {
                self.visit(next, output, needed - 1);
            } else if let Some(ch) = last_char(&self.key) {
                if self.push_row(ch) {
                    self.visit(next, output, 0);
                }
                self.rows.pop();
                self.chars.pop();
            }
            self.key.pop();
        }
    }

    /// Distance row for the key extended by `ch`; false when every cell is
    /// over the limit (no longer key can come back within it).
    fn push_row(&mut self, ch: char) -> bool {
        let previous = self.rows.last().expect("first row");
        let i = self.chars.len() + 1;
        let mut row = Vec::with_capacity(previous.len());
        row.push(i);
        for j in 1..previous.len() {
            let cost = usize::from(self.query[j - 1] != ch);
            let mut best = (previous[j] + 1)
                .min(row[j - 1] + 1)
                .min(previous[j - 1] + cost);
            // Swapped neighbours: the key's last two characters are the query's, reversed.
            if j > 1
                && let Some(&last) = self.chars.last()
                && ch == self.query[j - 2]
                && last == self.query[j - 1]
            {
                best = best.min(self.rows[self.rows.len() - 2][j - 2] + 1);
            }
            row.push(best);
        }
        let alive = row.iter().min().is_some_and(|&least| least <= self.limit);
        self.rows.push(row);
        self.chars.push(ch);
        alive
    }
}

fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn last_char(bytes: &[u8]) -> Option<char> {
    let start = (0..bytes.len()).rev().find(|&i| bytes[i] & 0xc0 != 0x80)?;
    std::str::from_utf8(&bytes[start..]).ok()?.chars().next()
}

/// Characters in one string without a partner in the other, both ways.
fn letter_difference(a: &str, b: &str) -> usize {
    let mut counts: HashMap<char, isize> = HashMap::new();
    for ch in a.chars() {
        *counts.entry(ch).or_default() += 1;
    }
    for ch in b.chars() {
        *counts.entry(ch).or_default() -= 1;
    }
    counts.values().map(|count| count.unsigned_abs()).sum()
}

fn index_error(err: fst::Error) -> Error {
    Error::BadFile {
        name: "lookup (fuzzy index)".to_owned(),
        reason: err.to_string(),
    }
}

impl Pack {
    /// Forms within a few typing errors of the input, one per lookup key,
    /// closest first, then by rank. The allowed distance grows with the input length
    /// ([`max_distance`]); exact matches come first with distance 0.
    pub fn correct(&self, text: &str, limit: usize) -> Result<Vec<Correction>> {
        let key = norm(text, self.meta().norm());
        let allowed = max_distance(key.chars().count());
        if key.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let index = self.fuzzy_index()?;
        let length = key.chars().count();
        let mut candidates = index.search(&key, allowed);
        // Equal distance: same letters first (a swap, `teh` -> `the`, keeps
        // them; a substitution does not), then rank, length, key.
        candidates.sort_by_key(|(candidate, distance, rank)| {
            (
                *distance,
                letter_difference(&key, candidate),
                *rank,
                candidate.chars().count().abs_diff(length),
                candidate.clone(),
            )
        });
        let db = self.db();
        let mut statement = db.prepare_cached(
            "SELECT l.form, l.kind, e.key, e.headword, e.homograph, l.anchor FROM lookup l \
             JOIN entries e ON e.id = l.entry WHERE l.norm = ? ORDER BY l.rank, l.entry LIMIT 1",
        )?;
        let mut seen = HashSet::new();
        let mut found = Vec::new();
        // One result per candidate key, its best row: homographs and the
        // forms of one entry would otherwise fill the list with one word.
        for (candidate, distance, _) in candidates {
            let rows = statement.query_map([&candidate], |row| {
                Ok(Suggestion {
                    form: row.get(0)?,
                    kind: row.get(1)?,
                    entry: row.get(2)?,
                    headword: row.get(3)?,
                    homograph: row.get(4)?,
                    anchor: row.get(5)?,
                })
            })?;
            for row in rows {
                let suggestion = row?;
                if seen.insert((suggestion.entry.clone(), suggestion.anchor)) {
                    found.push(Correction {
                        suggestion,
                        distance,
                    });
                    if found.len() == limit {
                        return Ok(found);
                    }
                }
            }
        }
        Ok(found)
    }

    /// Build the correction index now (about 0.1 s for Oxford) instead of on
    /// the first [`Pack::correct`]; returns the number of keys.
    pub fn prepare_correction(&self) -> Result<usize> {
        Ok(self.fuzzy_index()?.len())
    }

    fn fuzzy_index(&self) -> Result<&FuzzyIndex> {
        if let Some(index) = self.fuzzy.get() {
            return Ok(index);
        }
        let index = FuzzyIndex::build(self)?;
        Ok(self.fuzzy.get_or_init(|| index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_pack::TestPack;

    fn pack(words: &[(&str, i64)]) -> Pack {
        let test = TestPack::new();
        for (id, (word, rank)) in words.iter().enumerate() {
            let id = id as i64 + 1;
            let json = format!(r#"{{"type":"entry","id":"k{id}","lang":"en"}}"#);
            test.entry(id, word, None, &json).lookup(
                &norm(word, crate::text::Scheme::Fold),
                *rank,
                id,
                0,
                word,
                "headword",
            );
        }
        test.open()
    }

    fn corrections(pack: &Pack, text: &str) -> Vec<(String, usize)> {
        pack.correct(text, 10)
            .unwrap()
            .into_iter()
            .map(|c| (c.suggestion.form, c.distance))
            .collect()
    }

    #[test]
    fn finds_insertions_deletions_substitutions_and_swaps() {
        let pack = pack(&[
            ("receive", 0),
            ("recipe", 3),
            ("relieve", 1),
            ("the", 0),
            ("café", 2),
        ]);
        let found = |text: &str| corrections(&pack, text);
        let recieve = [
            ("receive".into(), 1),
            ("relieve".into(), 1),
            ("recipe".into(), 2),
        ];
        assert_eq!(found("recieve"), recieve); // equal distance: lower rank first
        assert_eq!(
            found("receeve"),
            [("receive".into(), 1), ("relieve".into(), 2)]
        );
        assert_eq!(found("teh"), [("the".into(), 1)]);
        assert_eq!(found("cafe"), [("café".into(), 0)]);
        assert_eq!(found("recipee")[0], ("recipe".into(), 1));
        assert!(found("th").is_empty(), "two letters: exact only");
        assert!(found("xyzzyq").is_empty());
    }

    #[test]
    fn walks_multi_byte_keys() {
        let pack = pack(&[("naïve", 0), ("苹果", 0), ("苹果树", 1)]);
        let index = pack.fuzzy_index().unwrap();
        let mut found = index.search("苹果", 1);
        found.sort();
        assert_eq!(found, [("苹果".into(), 0, 0), ("苹果树".into(), 1, 1)]);
        assert_eq!(index.search("naive", 1)[0].1, 0); // norm folds ï
    }

    #[test]
    fn prefers_swaps_among_equal_distances() {
        let pack = pack(&[("tea", 0), ("ten", 0), ("the", 0)]);
        assert_eq!(corrections(&pack, "teh")[0], ("the".into(), 1));
        assert_eq!(letter_difference("teh", "the"), 0);
        assert_eq!(letter_difference("teh", "tea"), 2);
    }

    #[test]
    fn returns_one_result_per_key() {
        let test = TestPack::new();
        for id in 1..=3 {
            let json = format!(r#"{{"type":"entry","id":"k{id}","lang":"en"}}"#);
            test.entry(id, "right", Some(id), &json)
                .lookup("right", id, id, 0, "right", "headword");
        }
        test.lookup("right", 9, 1, 2, "right", "phrase");
        let found = test.open().correct("rihgt", 10).unwrap();
        let entries: Vec<_> = found.iter().map(|c| c.suggestion.entry.as_str()).collect();
        assert_eq!(entries, ["k1"]);
    }

    #[test]
    fn allowed_distance_grows_with_length() {
        assert_eq!([2, 3, 4, 5, 12].map(max_distance), [0, 1, 1, 2, 2]);
    }
}
