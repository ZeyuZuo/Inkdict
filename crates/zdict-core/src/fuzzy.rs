//! Close spellings over lookup keys (spec/pack-v1.md §5.1).
//!
//! The distinct `lookup.norm` keys go into an FST (value: their best rank),
//! built in memory with the word index: about 1 MB for Oxford instead of a
//! SymSpell delete table of tens of MB. A depth-first walk carries one row
//! of the optimal-string-alignment distance table per character and prunes
//! branches whose row minimum exceeds the limit.

use std::collections::HashMap;

use fst::raw::{Fst, Node, Output};

use crate::error::{Error, Result};

/// Lookup keys as an FST map: key -> lowest rank of its rows.
pub(crate) struct FuzzyIndex {
    keys: Fst<Vec<u8>>,
}

/// Edits allowed for an input key of `chars` characters: none for one or
/// two, one for three or four, two from five (spec/pack-v1.md §5.1).
pub fn max_distance(chars: usize) -> usize {
    match chars {
        0..=2 => 0,
        3..=4 => 1,
        _ => 2,
    }
}

impl FuzzyIndex {
    /// From (key, lowest rank) pairs in key order, each key once.
    pub(crate) fn build<'a>(keys: impl Iterator<Item = (&'a str, i64)>) -> Result<Self> {
        let mut builder = fst::MapBuilder::memory();
        for (key, rank) in keys {
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

/// Characters in one string without a partner in the other, both ways:
/// 0 when one only reorders the other.
pub(crate) fn letter_difference(a: &str, b: &str) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn index(keys: &[(&str, i64)]) -> FuzzyIndex {
        let mut keys = keys.to_vec();
        keys.sort();
        FuzzyIndex::build(keys.into_iter()).unwrap()
    }

    fn found(index: &FuzzyIndex, query: &str, limit: usize) -> Vec<(String, usize, u64)> {
        let mut found = index.search(query, limit);
        found.sort();
        found
    }

    #[test]
    fn finds_insertions_deletions_substitutions_and_swaps() {
        let index = index(&[("receive", 0), ("recipe", 3), ("relieve", 1), ("the", 0)]);
        assert_eq!(
            found(&index, "recieve", 2),
            [
                ("receive".into(), 1, 0),
                ("recipe".into(), 2, 3),
                ("relieve".into(), 1, 1)
            ]
        );
        assert_eq!(found(&index, "teh", 1), [("the".into(), 1, 0)]);
        assert_eq!(found(&index, "recipee", 1), [("recipe".into(), 1, 3)]);
        assert!(found(&index, "th", 0).is_empty());
        assert!(found(&index, "xyzzyq", 2).is_empty());
    }

    #[test]
    fn walks_multi_byte_keys() {
        let index = index(&[("naive", 0), ("苹果", 0), ("苹果树", 1)]);
        assert_eq!(
            found(&index, "苹果", 1),
            [("苹果".into(), 0, 0), ("苹果树".into(), 1, 1)]
        );
        assert_eq!(found(&index, "naïve", 1), [("naive".into(), 1, 0)]);
    }

    #[test]
    fn counts_letters_both_ways() {
        assert_eq!(letter_difference("teh", "the"), 0);
        assert_eq!(letter_difference("teh", "tea"), 2);
        assert_eq!(letter_difference("ab", "abc"), 1);
    }

    #[test]
    fn allowed_distance_grows_with_length() {
        assert_eq!([2, 3, 4, 5, 12].map(max_distance), [0, 1, 1, 2, 2]);
    }
}
