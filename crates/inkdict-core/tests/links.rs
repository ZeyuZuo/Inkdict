//! Resource URLs, anchors and links against the Python reader and the
//! pack build report.

mod common;

use common::{load, oxdata};
use inkdict_core::{Attr, Entry, Focus, LinkTarget, Node, Pack};

#[test]
fn resource_urls_match_reference() {
    let Some((pack, golden)) = load("resources.json") else {
        return;
    };
    let cases = golden.as_array().unwrap();
    for case in cases {
        let reference = case[0].as_str().unwrap();
        let urls = serde_json::to_value(pack.resource_urls(reference)).unwrap();
        assert_eq!(urls, case[1], "{reference}");
        assert_eq!(
            pack.resource(reference).unwrap().is_some(),
            case[2].as_bool().unwrap(),
            "stored {reference}"
        );
    }
    assert!(cases.len() > 2000);
}

#[test]
fn anchors_match_reference() {
    let Some((pack, golden)) = load("anchors.json") else {
        return;
    };
    for case in golden.as_array().unwrap() {
        let entry = pack.entry(case["entry"].as_str().unwrap()).unwrap();
        let ids: Vec<_> = entry.anchors().iter().map(|node| node.id.clone()).collect();
        assert_eq!(
            serde_json::to_value(&ids).unwrap(),
            case["anchors"],
            "{}",
            entry.key
        );
        for (number, id) in ids.iter().enumerate() {
            let id = id.as_deref().unwrap();
            assert_eq!(entry.anchor_number(id), Some(number as u32));
        }
    }
}

fn links<'a>(node: &'a Node, found: &mut Vec<&'a Node>) {
    if node.kind == "target" || node.kind == "xword" {
        found.push(node);
    }
    for child in node.child_nodes() {
        links(child, found);
    }
}

/// Every link resolves; the ones whose target is missing are exactly those
/// the build report lists (pack_report.json), and they fall back.
#[test]
fn every_link_resolves_as_the_build_report_expects() {
    let Some((pack, golden)) = load("pack.json") else {
        return;
    };
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(oxdata().join("pack_report.json")).unwrap())
            .unwrap();
    let expected = |kind: &str| report["problems"][kind]["count"].as_u64().unwrap_or(0);
    let (mut missing_entries, mut missing_anchors, mut total) = (0, 0, 0);
    for id in 1..=golden["entry_rows"].as_i64().unwrap() {
        let current = pack.entry_by_id(id).unwrap();
        let mut found = Vec::new();
        links(&current.root, &mut found);
        for node in found {
            total += 1;
            let resolved = pack.resolve_link(&current, node).unwrap();
            let (entry_missing, anchor_missing) = check(&pack, &current, node);
            missing_entries += entry_missing as u64;
            missing_anchors += anchor_missing as u64;
            let anchored = matches!(
                resolved,
                LinkTarget::Entry {
                    focus: Focus::Anchor(_),
                    ..
                }
            );
            let wants_anchor = node.attr("anchor").is_some();
            assert_eq!(
                anchored,
                wants_anchor && !anchor_missing && !entry_missing,
                "{} {node:?} -> {resolved:?}",
                current.key
            );
        }
    }
    assert_eq!(missing_entries, expected("entry not in pack"));
    assert_eq!(missing_anchors, expected("anchor not found"));
    assert!(total > 40_000, "{total} links");
}

/// (entry attribute names no entry in the pack, anchor not in its target)
fn check(pack: &Pack, current: &Entry, node: &Node) -> (bool, bool) {
    let attr = |name| node.attr(name).and_then(Attr::as_str);
    let target = match attr("entry") {
        Some(key) => match pack.entry(key) {
            Ok(entry) => entry,
            Err(_) => return (true, false),
        },
        None => current.clone(),
    };
    let anchor_missing = attr("anchor").is_some_and(|id| target.anchor_number(id).is_none());
    (false, anchor_missing)
}
