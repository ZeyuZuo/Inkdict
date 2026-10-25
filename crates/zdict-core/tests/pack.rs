//! Checks against the Oxford pack and the Python reference outputs in
//! oxdata/ (scripts/sync-oxdata.sh, scripts/make-golden.py). Skipped when
//! oxdata/ is absent: dictionary data is not part of this repository.

use std::path::PathBuf;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use zdict_core::{APPLICATION_ID, FORMAT_VERSION, Pack};

fn oxdata() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata")
}

/// The pack and one golden file; None (with a note) without oxdata/.
/// A synced pack without its golden file fails: the check would be empty.
fn load(golden: &str) -> Option<(Pack, Value)> {
    let pack = oxdata().join("oalecd10.zdpack");
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

#[test]
fn meta_matches_reference() {
    let Some((pack, golden)) = load("pack.json") else {
        return;
    };
    assert_eq!(golden["application_id"], APPLICATION_ID);
    assert_eq!(golden["user_version"], FORMAT_VERSION);
    let meta: Map<_, _> = pack
        .meta()
        .iter()
        .map(|(k, v)| (k.to_owned(), Value::from(v)))
        .collect();
    assert_eq!(Value::Object(meta), golden["meta"]);
    assert_eq!(
        serde_json::to_value(pack.meta().fts_langs()).unwrap(),
        golden["fts_langs"]
    );
    assert_eq!(pack.meta().entries(), golden["entry_rows"].as_u64());
}

#[test]
fn files_match_reference() {
    let Some((pack, golden)) = load("pack.json") else {
        return;
    };
    let wanted = golden["files"].as_object().unwrap();
    assert_eq!(
        pack.file_names().unwrap(),
        wanted.keys().cloned().collect::<Vec<_>>()
    );
    for (name, expected) in wanted {
        let body = pack.file(name).unwrap();
        assert_eq!(
            body.len() as u64,
            expected["bytes"].as_u64().unwrap(),
            "{name}"
        );
        let digest: String = Sha256::digest(&body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, expected["sha256"].as_str().unwrap(), "{name}");
    }
}

/// Every entry decodes to its line in entries.jsonl, byte for byte, and its
/// node tree serializes back to the same JSON value.
#[test]
fn every_entry_matches_its_source_line() {
    let jsonl = oxdata().join("entries.jsonl");
    let Some((pack, golden)) = load("pack.json") else {
        return;
    };
    let source = std::fs::read_to_string(jsonl).unwrap();
    let mut count = 0;
    for (number, line) in source.lines().enumerate() {
        let value: Value = serde_json::from_str(line).unwrap();
        let key = value["id"].as_str().unwrap();
        assert_eq!(pack.entry_json(key).unwrap(), line, "{key}");
        let entry = pack.entry(key).unwrap();
        assert_eq!(entry.id, number as i64 + 1, "{key}");
        assert_eq!(serde_json::to_value(&entry.root).unwrap(), value, "{key}");
        count += 1;
    }
    assert_eq!(Some(count), golden["entry_rows"].as_u64());
    let last = pack.entry_by_id(count as i64).unwrap();
    assert_eq!(pack.entry(&last.key).unwrap(), last);
}
