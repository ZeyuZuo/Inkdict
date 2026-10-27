//! `Markup::Preview` is byte for byte OxDic's reference renderer
//! (`oxdic/preview.py`) on oxdata/golden/html.json.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use serde_json::Value;
use inkdict_core::theme::ThemeStack;
use inkdict_core::{Entry, Node, Pack};
use inkdict_html::{Markup, entry_html};

fn oxdata() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata")
}

/// The Oxford pack and html.json; None (with a note) without oxdata/.
fn load(golden: &str) -> Option<(Pack, Value)> {
    let pack = oxdata().join("oalecd10.inkdict");
    let golden = oxdata().join("golden").join(golden);
    if !pack.is_file() {
        eprintln!("skipped: {} missing", pack.display());
        return None;
    }
    assert!(
        golden.is_file(),
        "{} missing: run scripts/make-golden.py",
        golden.display()
    );
    let golden = serde_json::from_str(&std::fs::read_to_string(golden).unwrap()).unwrap();
    Some((Pack::open(pack).unwrap(), golden))
}

/// The checked-in examples as make-golden.py rendered them: from their JSON
/// files (book.excerpt.json is shorter than the pack entry with its key).
fn examples() -> HashMap<String, Node> {
    let mut found = HashMap::new();
    for file in std::fs::read_dir(oxdata().join("examples")).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let root: Node = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            found.insert(root.id.clone().unwrap(), root);
        }
    }
    found
}

/// About 120 Unicode characters around a byte offset, including end-of-file
/// differences. Never slice a UTF-8 string at the differing byte itself.
fn context(text: &str, byte: usize) -> String {
    let character = text
        .char_indices()
        .take_while(|(offset, _)| *offset < byte)
        .count();
    text.chars()
        .skip(character.saturating_sub(60))
        .take(120)
        .collect()
}

fn difference(entry: &str, expected: &str, actual: &str) -> String {
    let byte = expected
        .bytes()
        .zip(actual.bytes())
        .position(|(left, right)| left != right)
        .unwrap_or(expected.len().min(actual.len()));
    format!(
        "{entry}: first difference at byte {byte} (reference {} bytes, Rust {} bytes)\n  reference: {:?}\n  Rust:      {:?}",
        expected.len(),
        actual.len(),
        context(expected, byte),
        context(actual, byte)
    )
}

#[test]
fn html_matches_reference() {
    let started = Instant::now();
    let Some((pack, golden)) = load("html.json") else {
        return;
    };
    assert_eq!(golden["theme_errors"], serde_json::json!([]));
    let registry = pack.registry().unwrap();
    let theme = pack.theme("default").unwrap();
    let errors = theme.validate(&registry);
    assert!(errors.is_empty(), "default theme errors: {errors:?}");
    let stack = ThemeStack::new(theme);
    let renderer = pack.renderer(&registry, &stack);
    let cases = golden["cases"].as_array().unwrap();
    assert!(!cases.is_empty(), "html.json has no reference cases");

    let examples = examples();
    let mut mismatches = Vec::new();
    for case in cases {
        let key = case["entry"].as_str().unwrap();
        let entry = match examples.get(key) {
            Some(root) => Entry {
                id: 0,
                key: key.to_owned(),
                headword: String::new(),
                homograph: None,
                root: root.clone(),
            },
            None => pack.entry(key).unwrap(),
        };
        let actual = entry_html(&renderer.render(&entry), Markup::Preview);
        let expected = case["html"].as_str().unwrap();
        if actual != expected {
            mismatches.push(difference(key, expected, &actual));
        }
    }
    let elapsed = started.elapsed();
    eprintln!(
        "HTML reference: {}/{} cases identical, elapsed {elapsed:.3?}",
        cases.len() - mismatches.len(),
        cases.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} of {} HTML cases differ; first {}:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.len().min(5),
        mismatches
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
}
