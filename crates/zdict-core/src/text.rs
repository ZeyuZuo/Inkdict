//! Text keys for lookup and full-text search (spec/pack-v0.md §3.1, §4).
//!
//! A port of OxDic's `oxdic/pack/text.py`, which built the pack: a key
//! computed here must equal the one stored there. Python's `str.isspace`,
//! `str.isalnum` and `re` `\w` are reproduced by the `is_*` functions;
//! oxdata/golden/chars.json checks them over every code point.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::canonical_combining_class;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

/// Stress marks, syllable dots and the movable-particle arrow of phrasal verbs.
const MARKS: [char; 5] = ['\u{2c8}', '\u{2cc}', '\u{b7}', '\u{2027}', '\u{2194}'];

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

/// Lookup key: no diacritics, case-folded, letters and digits only.
///
/// `Café`, `cafe` and `CAFÉ` share a key, as do `mother-in-law` and
/// `mother in law`. Forms made only of symbols (`&`, `%`) keep them.
pub fn norm(text: &str) -> String {
    let decomposed: String = display(text)
        .nfkd()
        .filter(|&ch| canonical_combining_class(ch) == 0)
        .collect();
    let folded = caseless::default_case_fold_str(&decomposed);
    let key: String = folded.chars().filter(|&ch| is_alnum(ch)).collect();
    if key.is_empty() {
        folded.replace(' ', "")
    } else {
        key
    }
}

/// FTS5 MATCH expression for user input: each run of CJK characters becomes
/// a phrase of single characters, every other word a quoted term; terms are
/// ANDed. Empty when the input has no words.
pub fn fts_query(text: &str) -> String {
    let mut terms: Vec<String> = Vec::new();
    let mut run = String::new();
    let mut run_is_cjk = false;
    for ch in text.chars().filter(|ch| !MARKS.contains(ch)) {
        let cjk = is_cjk(ch);
        if is_space(ch) || (!run.is_empty() && cjk != run_is_cjk) {
            push_run(&mut terms, &run, run_is_cjk);
            run.clear();
        }
        if !is_space(ch) {
            run.push(ch);
            run_is_cjk = cjk;
        }
    }
    push_run(&mut terms, &run, run_is_cjk);
    terms.join(" ")
}

fn push_run(terms: &mut Vec<String>, run: &str, cjk: bool) {
    if run.is_empty() {
        return;
    }
    if cjk {
        let chars: Vec<String> = run.chars().map(String::from).collect();
        terms.push(format!("\"{}\"", chars.join(" ")));
    } else {
        terms.extend(
            run.split(|ch: char| !is_word(ch))
                .filter(|word| !word.is_empty())
                .map(|word| format!("\"{word}\"")),
        );
    }
}

/// Python `str.isspace`: Unicode White_Space plus the separators U+001C–U+001F.
pub(crate) fn is_space(ch: char) -> bool {
    ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch)
}

/// Python `str.isalnum`: letters (L*) and numbers (N*).
pub(crate) fn is_alnum(ch: char) -> bool {
    matches!(
        ch.general_category_group(),
        GeneralCategoryGroup::Letter | GeneralCategoryGroup::Number
    )
}

/// Python `re` `\w` on str: `isalnum` or underscore.
pub(crate) fn is_word(ch: char) -> bool {
    ch == '_' || is_alnum(ch)
}

/// CJK ideographs indexed one character per token (text.py `CJK_CHARS`).
pub(crate) fn is_cjk(ch: char) -> bool {
    matches!(ch, '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{2ffff}')
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
    fn norm_matches_text_py() {
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
            ("苹果", "苹果"),
        ];
        for (text, key) in cases {
            assert_eq!(norm(text), key, "{text:?}");
        }
    }

    #[test]
    fn fts_query_quotes_words_and_spaces_cjk() {
        assert_eq!(fts_query("apple pie"), r#""apple" "pie""#);
        assert_eq!(fts_query("don't"), r#""don" "t""#);
        assert_eq!(fts_query("苹果apple 的"), r#""苹 果" "apple" "的""#);
        assert_eq!(fts_query("run* AND \"x\""), r#""run" "AND" "x""#);
        assert_eq!(fts_query("ˈæpl"), r#""æpl""#);
        assert_eq!(fts_query(" - "), "");
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
        let checks: [Check; 4] = [
            ("isspace", is_space),
            ("isalnum", is_alnum),
            ("word", is_word),
            ("combining", |ch| canonical_combining_class(ch) != 0),
        ];
        let classes: Vec<_> = checks
            .iter()
            .map(|(name, check)| (*name, *check, ranges(&golden, name)))
            .collect();
        let norms: std::collections::HashMap<u32, String> = golden["norm_single"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p[0].as_u64().unwrap() as u32,
                    p[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let mut wrong = Vec::new();
        for &(lo, hi) in &assigned {
            for cp in lo..=hi {
                let Some(ch) = char::from_u32(cp) else {
                    continue;
                };
                for (name, check, python) in &classes {
                    if check(ch) != contains(python, cp) {
                        wrong.push(format!("{name} U+{cp:04X}"));
                    }
                }
                let wanted = norms.get(&cp).cloned().unwrap_or_else(|| ch.to_string());
                if norm(&ch.to_string()) != wanted {
                    wrong.push(format!(
                        "norm U+{cp:04X} {:?} != {wanted:?}",
                        norm(&ch.to_string())
                    ));
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
