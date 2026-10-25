//! Full-text search against the Python reader.

mod common;

use common::load;
use serde_json::Value;
use zdict_core::text::fts_query;
use zdict_core::{Column, Hit};

fn column(value: &Value) -> Option<Column> {
    match value.as_str() {
        None => None,
        Some("text") => Some(Column::Text),
        Some("example") => Some(Column::Example),
        Some(other) => panic!("unknown column {other}"),
    }
}

fn same(hit: &Hit, wanted: &Value) -> bool {
    let score = wanted["score"].as_f64().unwrap();
    hit.entry == wanted["entry"]
        && hit.headword == wanted["headword"]
        && hit.homograph == wanted["homograph"].as_i64()
        && hit.anchor == wanted["anchor"]
        && hit.lang == wanted["lang"]
        && (hit.score - score).abs() <= 1e-9 * score.abs().max(1.0)
}

#[test]
fn search_matches_reference() {
    let Some((pack, golden)) = load("search.json") else {
        return;
    };
    let cases = golden["cases"].as_array().unwrap();
    let mut wrong = Vec::new();
    for case in cases {
        let query = case["query"].as_str().unwrap();
        assert_eq!(fts_query(query), case["fts_query"], "{query:?}");
        let limit = case["limit"].as_u64().unwrap() as usize;
        let hits = pack.search(query, limit, column(&case["column"])).unwrap();
        let wanted = case["results"].as_array().unwrap();
        if hits.len() != wanted.len() || !hits.iter().zip(wanted).all(|(h, w)| same(h, w)) {
            wrong.push(format!("{query:?} {}", case["column"]));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} queries differ: {:?}",
        wrong.len(),
        cases.len(),
        &wrong[..wrong.len().min(20)]
    );
}
