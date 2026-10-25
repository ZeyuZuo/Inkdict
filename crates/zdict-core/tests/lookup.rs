//! Lookup keys and suggestions against the Python reader.

mod common;

use common::load;
use zdict_core::text::norm;

#[test]
fn norm_matches_reference() {
    let Some((_, golden)) = load("norm.json") else {
        return;
    };
    let cases = golden.as_array().unwrap();
    let wrong: Vec<_> = cases
        .iter()
        .filter_map(|case| {
            let (text, key) = (case[0].as_str().unwrap(), case[1].as_str().unwrap());
            (norm(text) != key).then(|| format!("{text:?}: {:?} != {key:?}", norm(text)))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {}: {:?}",
        wrong.len(),
        cases.len(),
        &wrong[..wrong.len().min(20)]
    );
}

#[test]
fn suggest_matches_reference() {
    let Some((pack, golden)) = load("suggest.json") else {
        return;
    };
    let cases = golden["cases"].as_array().unwrap();
    let mut wrong = Vec::new();
    for case in cases {
        let query = case["query"].as_str().unwrap();
        let limit = case["limit"].as_u64().unwrap() as usize;
        let found = serde_json::to_value(pack.suggest(query, limit).unwrap()).unwrap();
        if found != case["results"] {
            wrong.push(query.to_owned());
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} queries differ: {:?}",
        wrong.len(),
        cases.len(),
        &wrong[..wrong.len().min(20)]
    );
    assert!(
        cases
            .iter()
            .any(|case| case["results"].as_array().is_some_and(|r| r.len() > 1))
    );
}
