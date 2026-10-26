//! Lookup keys and the word list against the Python reader.

mod common;

use std::time::Instant;

use common::load;
use zdict_core::text::{Scheme, norm};

/// fold-1 and exact-1 keys of every Oxford form and the edge cases.
#[test]
fn norm_matches_reference() {
    let Some((_, golden)) = load("norm.json") else {
        return;
    };
    let cases = golden.as_array().unwrap();
    let mut wrong = Vec::new();
    for case in cases {
        let text = case[0].as_str().unwrap();
        for (scheme, wanted) in [(Scheme::Fold, &case[1]), (Scheme::Exact, &case[2])] {
            let key = norm(text, scheme);
            if key != wanted.as_str().unwrap() {
                wrong.push(format!("{} {text:?}: {key:?} != {wanted}", scheme.name()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {}: {:?}",
        wrong.len(),
        cases.len() * 2,
        &wrong[..wrong.len().min(20)]
    );
}

/// Every query of words.json: prefixes, whole forms, uppercase, typing
/// errors, homographs, several limits.
#[test]
fn words_match_reference() {
    let Some((pack, golden)) = load("words.json") else {
        return;
    };
    let started = Instant::now();
    pack.prepare_words().unwrap();
    eprintln!("word index: {:?}", started.elapsed());
    let cases = golden["cases"].as_array().unwrap();
    let mut wrong = Vec::new();
    let started = Instant::now();
    for case in cases {
        let query = case["query"].as_str().unwrap();
        let limit = case["limit"].as_u64().unwrap() as usize;
        let found = serde_json::to_value(pack.words(query, limit).unwrap()).unwrap();
        if found != case["results"] {
            wrong.push((query.to_owned(), limit, found, case["results"].clone()));
        }
    }
    eprintln!("{} queries: {:?}", cases.len(), started.elapsed());
    if let Some((query, limit, found, wanted)) = wrong.first() {
        let first = found
            .as_array()
            .unwrap()
            .iter()
            .zip(wanted.as_array().unwrap())
            .position(|(a, b)| a != b);
        panic!(
            "{} of {} queries differ: {:?}\nfirst: {query:?} limit {limit}, row {first:?} of {} (reference {})\n  Rust:      {}\n  reference: {}",
            wrong.len(),
            cases.len(),
            wrong.iter().map(|w| &w.0).take(20).collect::<Vec<_>>(),
            found.as_array().unwrap().len(),
            wanted.as_array().unwrap().len(),
            first.map_or(String::new(), |i| found[i].to_string()),
            first.map_or(String::new(), |i| wanted[i].to_string()),
        );
    }
    for group in ["prefix", "contains", "close"] {
        assert!(
            cases.iter().any(|case| case["results"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["group"] == group)),
            "no {group} rows in words.json"
        );
    }
}
