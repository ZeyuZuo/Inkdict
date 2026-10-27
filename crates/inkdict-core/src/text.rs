//! Lookup keys (spec/pack-v1.md §3.1, §3.2).
//!
//! A port of OxDic's `oxdic/pack/text.py`, which builds packs: a key
//! computed here must equal the one stored there. Python's `str.isspace`
//! and general categories are reproduced by the `is_*` functions;
//! oxdata/golden/chars.json checks them over every code point.
//!
//! Unicode versions: a pack records the version it was built with in
//! `meta.unicode_version` (the Oxford pack: 14.0.0); this crate uses newer
//! tables (normalization 18, case folding 16, categories 17). Keys of
//! characters assigned by the pack's version must not change; the golden
//! test checks that for Unicode 14. Keys of characters assigned later are
//! not specified: packs never contain them.
//!
//! One Unicode 14 character changed category since: U+1171E AHOM CONSONANT
//! SIGN MEDIAL RA, Mn in 14 and Mc now. Its key differs only where it has no
//! base letter; after an Ahom consonant both versions keep it.

use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

/// Stress marks, syllable dots and the movable-particle arrow of phrasal verbs.
const MARKS: [char; 5] = ['\u{2c8}', '\u{2cc}', '\u{b7}', '\u{2027}', '\u{2194}'];

/// Lookup key scheme of a pack, `meta.norm` (spec/pack-v1.md §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// `fold-1`: diacritics of Latin, Greek, Cyrillic, Hebrew and Arabic letters go.
    Fold,
    /// `exact-1`: every combining mark stays.
    Exact,
}

impl Scheme {
    /// The scheme named in `meta.norm`; None for one this reader does not know.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "fold-1" => Some(Scheme::Fold),
            "exact-1" => Some(Scheme::Exact),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Scheme::Fold => "fold-1",
            Scheme::Exact => "exact-1",
        }
    }
}

/// Form as shown in a suggestion list: marks removed, spaces collapsed and trimmed.
pub fn display(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars().filter(|ch| !MARKS.contains(ch)) {
        if is_space(ch) {
            space = true;
            continue;
        }
        if space && !shown.is_empty() {
            shown.push(' ');
        }
        space = false;
        shown.push(ch);
    }
    shown
}

/// Base letters whose diacritics `fold-1` removes: Latin, Greek, Cyrillic,
/// Hebrew and Arabic (spec/pack-v1.md §3.2).
const FOLD_RANGES: [(u32, u32); 18] = [
    (0x0000, 0x024F),
    (0x1E00, 0x1EFF),
    (0x2C60, 0x2C7F),
    (0xA720, 0xA7FF),
    (0xAB30, 0xAB6F),
    (0x0370, 0x03FF),
    (0x1F00, 0x1FFF),
    (0x0400, 0x052F),
    (0x1C80, 0x1C8F),
    (0x2DE0, 0x2DFF),
    (0xA640, 0xA69F),
    (0x0590, 0x05FF),
    (0xFB1D, 0xFB4F),
    (0x0600, 0x06FF),
    (0x0750, 0x077F),
    (0x0870, 0x08FF),
    (0xFB50, 0xFDFF),
    (0xFE70, 0xFEFF),
];

/// Lookup key of `text` under a pack's scheme (spec/pack-v1.md §3.2).
///
/// `fold-1`: `Café`, `cafe` and `CAFÉ` share a key, as do `mother-in-law`
/// and `mother in law`; Hindi and Japanese keep their marks. `exact-1`
/// keeps every mark. Forms made only of symbols (`&`, `%`) keep them.
pub fn norm(text: &str, scheme: Scheme) -> String {
    let unmarked = text.chars().filter(|ch| !MARKS.contains(ch));
    let normalized: String = match scheme {
        Scheme::Fold => strip_folded_marks(unmarked.nfkd()).nfc().collect(),
        Scheme::Exact => unmarked.nfkc().collect(),
    };
    let folded = caseless::default_case_fold_str(&normalized);
    let key: String = folded.chars().filter(|&ch| is_kept(ch)).collect();
    if key.is_empty() {
        folded.chars().filter(|&ch| !is_space(ch)).collect()
    } else {
        key
    }
}

/// Drop nonspacing marks whose base (the last character that is not one)
/// is a folded letter, or that have no base.
fn strip_folded_marks(chars: impl Iterator<Item = char>) -> impl Iterator<Item = char> {
    let mut base: Option<char> = None;
    chars.filter(move |&ch| {
        if is_nonspacing(ch) {
            !base.is_none_or(folds)
        } else {
            base = Some(ch);
            true
        }
    })
}

fn folds(ch: char) -> bool {
    let point = ch as u32;
    FOLD_RANGES
        .iter()
        .any(|&(low, high)| (low..=high).contains(&point))
}

/// Python `str.isspace`: Unicode White_Space plus the separators U+001C–U+001F.
pub(crate) fn is_space(ch: char) -> bool {
    ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch)
}

/// General category Mn.
pub(crate) fn is_nonspacing(ch: char) -> bool {
    ch.general_category() == GeneralCategory::NonspacingMark
}

/// Letters, marks and numbers (general categories L*, M*, N*): what a key keeps.
pub(crate) fn is_kept(ch: char) -> bool {
    matches!(
        ch.general_category_group(),
        GeneralCategoryGroup::Letter | GeneralCategoryGroup::Mark | GeneralCategoryGroup::Number
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_drops_marks_and_collapses_spaces() {
        assert_eq!(display("  ˈæp·l \t pie\n"), "æpl pie");
        assert_eq!(display("pull↔up"), "pullup");
        assert_eq!(display("\u{1c}a\u{85}b"), "a b");
    }

    #[test]
    fn fold_matches_text_py() {
        let cases = [
            ("Café", "cafe"),
            ("CAFÉ", "cafe"),
            ("mother-in-law", "motherinlaw"),
            ("mother in law", "motherinlaw"),
            ("straße", "strasse"),
            ("ﬁ", "fi"),
            ("Ⅻ", "xii"),
            ("&", "&"),
            ("A & E", "ae"),
            ("& %", "&%"),
            ("  ", ""),
            ("ˈæp·l", "æpl"),
            ("苹果", "苹果"),
            ("كِتَاب", "كتاب"),
            ("Ἑλλάς", "ελλασ"),
            ("Ёлка", "елка"),
            // Marks on other scripts stay: Devanagari vowel signs, kana voicing.
            ("किताब", "किताब"),
            ("が", "が"),
        ];
        for (text, key) in cases {
            assert_eq!(norm(text, Scheme::Fold), key, "{text:?}");
        }
    }

    #[test]
    fn exact_keeps_marks() {
        assert_eq!(norm("Café", Scheme::Exact), "café");
        assert_eq!(norm("ｶﾞ", Scheme::Exact), "ガ");
        assert_eq!(norm("mother-in-law", Scheme::Exact), "motherinlaw");
        assert_eq!(norm("&", Scheme::Exact), "&");
    }

    #[test]
    fn schemes_round_trip_their_names() {
        for scheme in [Scheme::Fold, Scheme::Exact] {
            assert_eq!(Scheme::parse(scheme.name()), Some(scheme));
        }
        assert_eq!(Scheme::parse("fold"), None);
    }
}

/// Against Python's tables in oxdata/golden/chars.json (skipped without it).
#[cfg(test)]
mod golden {
    use super::*;
    use std::path::Path;

    fn ranges(golden: &serde_json::Value, name: &str) -> Vec<(u32, u32)> {
        golden[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r[0].as_u64().unwrap() as u32, r[1].as_u64().unwrap() as u32))
            .collect()
    }

    fn contains(ranges: &[(u32, u32)], cp: u32) -> bool {
        let i = ranges.partition_point(|&(_, hi)| hi < cp);
        i < ranges.len() && ranges[i].0 <= cp
    }

    /// Changed general category after Unicode 14 (see the module docs).
    const RECATEGORIZED: [u32; 1] = [0x1171E];

    #[test]
    fn char_classes_and_single_char_keys_match_python() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../oxdata/golden/chars.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("skipped: {} missing", path.display());
            return;
        };
        let golden: serde_json::Value = serde_json::from_str(&text).unwrap();
        let assigned = ranges(&golden, "assigned");
        type Check = (&'static str, fn(char) -> bool);
        let checks: [Check; 3] = [
            ("isspace", is_space),
            ("nonspacing", is_nonspacing),
            ("kept", is_kept),
        ];
        let classes: Vec<_> = checks
            .iter()
            .map(|(name, check)| (*name, *check, ranges(&golden, name)))
            .collect();
        let singles = |name: &str| -> std::collections::HashMap<u32, String> {
            golden[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    (
                        p[0].as_u64().unwrap() as u32,
                        p[1].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        };
        let keys = [
            (Scheme::Fold, singles("norm_single")),
            (Scheme::Exact, singles("exact_single")),
        ];
        let mut wrong = Vec::new();
        for &(lo, hi) in &assigned {
            for cp in lo..=hi {
                let Some(ch) = char::from_u32(cp) else {
                    continue;
                };
                if RECATEGORIZED.contains(&cp) {
                    continue;
                }
                for (name, check, python) in &classes {
                    if check(ch) != contains(python, cp) {
                        wrong.push(format!("{name} U+{cp:04X}"));
                    }
                }
                for (scheme, singles) in &keys {
                    let wanted = singles.get(&cp).cloned().unwrap_or_else(|| ch.to_string());
                    let key = norm(&ch.to_string(), *scheme);
                    if key != wanted {
                        wrong.push(format!(
                            "{} U+{cp:04X} {key:?} != {wanted:?}",
                            scheme.name()
                        ));
                    }
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} differences: {:?}",
            wrong.len(),
            &wrong[..wrong.len().min(40)]
        );
    }
}
