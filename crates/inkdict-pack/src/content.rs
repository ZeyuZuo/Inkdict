//! Validate raw JSON, before any typed-node decoding can discard bad fields.
use crate::source::{valid_lang, valid_res};
use inkdict_core::Registry;
use serde_json::Value;
use std::collections::HashSet;

pub(crate) fn check_attr(kind: &str, value: &Value) -> bool {
    let kind = kind.strip_suffix('?').unwrap_or(kind);
    if let Some(options) = kind.strip_prefix("enum(").and_then(|s| s.strip_suffix(')')) {
        return value
            .as_str()
            .is_some_and(|v| options.split('|').any(|o| o == v));
    }
    match kind {
        "string" | "ref" => value.as_str().is_some_and(|v| !v.is_empty()),
        "int" => value.is_i64() || value.is_u64(),
        "bool" => value.is_boolean(),
        "res" => value.as_str().is_some_and(valid_res),
        _ => false,
    }
}

pub(crate) fn validate_entry(registry: &Registry, entry: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if !entry.get("id").is_some_and(Value::is_string)
        || entry
            .get("lang")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        errors.push("/: the root needs an id and a lang".into());
    }
    walk(registry, entry, "", &mut HashSet::new(), &mut errors);
    errors
}

fn walk(
    registry: &Registry,
    node: &Value,
    pointer: &str,
    ids: &mut HashSet<String>,
    errors: &mut Vec<String>,
) {
    let Some(object) = node.as_object() else {
        errors.push(format!("{pointer}: node must be an object"));
        return;
    };
    let node_type = node.get("type").unwrap_or(&Value::Null);
    let Some(spec) = node_type.as_str().and_then(|t| registry.get(t)) else {
        errors.push(format!("{pointer}: unknown type {node_type}"));
        return;
    };
    let kind = node_type.as_str().unwrap();
    let pointer = format!("{pointer}<{kind}>");
    for key in object.keys() {
        if !["type", "id", "lang", "attrs", "text", "children"].contains(&key.as_str()) {
            errors.push(format!(
                "{pointer}: unknown key {}",
                serde_json::to_string(key).expect("strings serialize")
            ));
        }
    }
    if let Some(lang) = node.get("lang")
        && !lang.as_str().is_some_and(valid_lang)
    {
        errors.push(format!("{pointer}: invalid lang {lang}"));
    }
    if let Some(id) = node.get("id") {
        match id.as_str().filter(|s| !s.is_empty()) {
            None => errors.push(format!("{pointer}: invalid id")),
            Some(id) if !ids.insert(id.to_owned()) => {
                errors.push(format!("{pointer}: duplicate id {id}"))
            }
            Some(_) => (),
        }
    }
    let attrs = node.get("attrs").and_then(Value::as_object);
    if let Some(attrs) = attrs {
        for (name, value) in attrs {
            match spec.attrs.get(name) {
                None => errors.push(format!(
                    "{pointer}: undeclared attr {}",
                    serde_json::to_string(name).expect("strings serialize")
                )),
                Some(kind) if !check_attr(kind, value) => {
                    errors.push(format!("{pointer}: attr {name}={value} is not {kind}"))
                }
                _ => (),
            }
        }
    } else if node.get("attrs").is_some() {
        errors.push(format!("{pointer}: attrs must be an object"));
    }
    for name in &spec.attr_order {
        if !spec.attrs[name].ends_with('?') && !attrs.is_some_and(|a| a.contains_key(name)) {
            errors.push(format!(
                "{pointer}: missing required attr {}",
                serde_json::to_string(name).expect("strings serialize")
            ));
        }
    }
    if node.get("text").is_some() && node.get("children").is_some() {
        errors.push(format!("{pointer}: use either text or children"));
    }
    if let Some(text) = node.get("text")
        && (!spec.children.contains("#text") || text.as_str().is_none_or(str::is_empty))
    {
        errors.push(format!("{pointer}: text not allowed or empty"));
    }
    if let Some(children) = node.get("children") {
        let Some(children) = children.as_array() else {
            errors.push(format!("{pointer}: children must be an array"));
            return;
        };
        for (i, child) in children.iter().enumerate() {
            let next = format!("{pointer}/{i}");
            if let Some(text) = child.as_str() {
                if !spec.children.contains("#text") || text.is_empty() {
                    errors.push(format!("{next}: text not allowed or empty"));
                }
                continue;
            }
            let child_type = child.get("type").unwrap_or(&Value::Null);
            if child.is_object()
                && !child_type
                    .as_str()
                    .is_some_and(|t| spec.children.contains(t))
            {
                errors.push(format!("{next}: {child_type} not allowed in {kind}"));
            }
            walk(registry, child, &next, ids, errors);
        }
    }
}
