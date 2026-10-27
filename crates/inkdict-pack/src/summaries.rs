use crate::{
    Problems, Result,
    source::{jsonl_lines, parse_json, read},
};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub(crate) struct Target {
    pub number: usize,
    pub anchors: HashMap<String, usize>,
    pub lang: String,
}

pub(crate) fn write_summaries(
    db: &Connection,
    path: Option<&Path>,
    targets: &HashMap<String, Target>,
    problems: &mut Problems,
) -> Result<usize> {
    let Some(path) = path else {
        return Ok(0);
    };
    let raw = read(path)?;
    let mut rows = BTreeMap::new();
    for (i, line) in jsonl_lines(&raw).enumerate() {
        let number = i + 1;
        let item = parse_json(path, number, line)?;
        if !item.is_object()
            || !item.get("entry").is_some_and(Value::is_string)
            || item.get("anchor").is_some_and(|a| !a.is_string())
            || item.get("lang").is_some_and(|l| !l.is_string())
        {
            problems.add(
                "summary not an object with entry, anchor, lang",
                None,
                number,
            );
            continue;
        }
        let key = item["entry"].as_str().unwrap();
        let anchor_id = item.get("anchor").and_then(Value::as_str);
        let Some(target) = targets.get(key) else {
            problems.add("summary for missing entry", Some(key), number);
            continue;
        };
        let anchor = match anchor_id {
            None => Some(0),
            Some(id) => target.anchors.get(id).copied(),
        };
        let Some(anchor) = anchor else {
            problems.add("summary for missing anchor", Some(key), anchor_id.unwrap());
            continue;
        };
        let Some(text) = item
            .get("text")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty() && !s.contains('\n'))
        else {
            problems.add("summary not one line of text", Some(key), number);
            continue;
        };
        if rows.contains_key(&(target.number, anchor)) {
            problems.add(
                "duplicate summary",
                Some(key),
                anchor_id.unwrap_or_default(),
            );
            continue;
        }
        let lang = item
            .get("lang")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(&target.lang);
        rows.insert((target.number, anchor), (lang.to_owned(), text.to_owned()));
    }
    let mut insert = db.prepare("INSERT INTO summaries VALUES (?, ?, ?, ?)")?;
    for ((entry, anchor), (lang, text)) in &rows {
        insert.execute(params![*entry as i64, *anchor as i64, lang, text])?;
    }
    Ok(rows.len())
}

pub(crate) fn check_links(
    targets: &HashMap<String, Target>,
    links: &[(String, Vec<BTreeMap<String, Value>>)],
    problems: &mut Problems,
) -> usize {
    let mut count = 0;
    for (key, links) in links {
        for link in links {
            count += 1;
            let target_value = link
                .get("entry")
                .cloned()
                .unwrap_or_else(|| Value::String(key.clone()));
            let target_key = target_value.as_str();
            let target = target_key.and_then(|s| targets.get(s));
            if let Some(target) = target {
                if let Some(anchor) = link.get("anchor")
                    && !anchor
                        .as_str()
                        .is_some_and(|a| target.anchors.contains_key(a))
                {
                    let shown = anchor
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| anchor.to_string());
                    problems.add(
                        "anchor not found",
                        Some(key),
                        format!("{}#{shown}", target_key.unwrap()),
                    );
                }
            } else {
                problems.add("entry not in pack", Some(key), target_value);
            }
        }
    }
    count
}
