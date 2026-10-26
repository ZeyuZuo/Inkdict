//! Lookup keys and suggestions against the Python reader.

mod common;

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
