//! Bounded reading expansion in product and choice order.
use inkdict_core::{Registry, Role, text::display};
use serde_json::Value;

pub(crate) const MAX_FORMS: usize = 32;

pub(crate) fn plain(node: &Value) -> String {
    if let Some(s) = node.as_str() {
        return s.to_owned();
    }
    let mut text = node
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            text.push_str(&plain(child));
        }
    }
    text
}

pub(crate) fn optional_parts(text: &str) -> Vec<String> {
    let mut readings = vec![String::new()];
    let mut start = 0;
    let mut open = None;
    for (i, c) in text.char_indices() {
        if c == '(' {
            open = Some(i);
        }
        if c == ')'
            && let Some(left) = open.take()
        {
            let literal = &text[start..left];
            let part = &text[left + 1..i];
            readings = readings
                .iter()
                .flat_map(|r| [format!("{r}{literal}{part}"), format!("{r}{literal}")])
                .take(MAX_FORMS + 1)
                .collect();
            start = i + 1;
        }
    }
    for reading in &mut readings {
        reading.push_str(&text[start..]);
    }
    readings
}

fn role(node: &Value, registry: &Registry, role: Role) -> bool {
    node.get("type")
        .and_then(Value::as_str)
        .is_some_and(|t| registry.has_role(t, role))
}

fn readings(node: &Value, registry: &Registry, overflow: &mut bool) -> Vec<String> {
    let text;
    let children = if let Some(t) = node.get("text") {
        text = vec![t.clone()];
        &text[..]
    } else {
        node.get("children")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
    };
    let mut found = vec![String::new()];
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        let choices;
        if let Some(s) = child.as_str() {
            choices = optional_parts(s);
            i += 1;
        } else if role(child, registry, Role::Choice) {
            let mut alternatives = Vec::new();
            while i < children.len() {
                let item = &children[i];
                if item.is_object() && role(item, registry, Role::Choice) {
                    alternatives.extend(readings(item, registry, overflow));
                    i += 1;
                } else if item.as_str().is_some_and(|s| s.trim().is_empty())
                    && children
                        .get(i + 1)
                        .is_some_and(|n| n.is_object() && role(n, registry, Role::Choice))
                {
                    i += 1;
                } else {
                    break;
                }
            }
            choices = alternatives;
        } else {
            let mut alternatives = readings(child, registry, overflow);
            if role(child, registry, Role::Optional) {
                alternatives.insert(0, String::new());
            }
            choices = alternatives;
            i += 1;
        }
        if found.len().saturating_mul(choices.len()) > MAX_FORMS {
            *overflow = true;
        }
        found = found
            .iter()
            .flat_map(|r| choices.iter().map(move |c| format!("{r}{c}")))
            .take(MAX_FORMS)
            .collect();
    }
    found
}

pub(crate) fn forms(node: &Value, registry: &Registry) -> (Vec<String>, bool) {
    let mut overflow = false;
    let mut found = Vec::new();
    for reading in readings(node, registry, &mut overflow) {
        let form = display(&reading);
        if !form.is_empty() && !found.contains(&form) {
            found.push(form);
        }
    }
    (found, overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn registry() -> Registry {
        Registry::parse("[meta]\nformat='inkdict-types'\nversion=1\n[types.choice]\nkind='inline'\nroles=['choice']\n[types.optional]\nkind='inline'\nroles=['optional']").unwrap()
    }
    #[test]
    fn brackets_expand_in_product_order() {
        assert_eq!(
            optional_parts("your (own) boot(strap)s"),
            [
                "your own bootstraps",
                "your own boots",
                "your  bootstraps",
                "your  boots"
            ]
        );
        assert_eq!(optional_parts("(a(b)c)"), ["(abc)", "(ac)"]);
        assert_eq!(optional_parts("a()b(c"), ["ab(c", "ab(c"]);
        assert_eq!(optional_parts(&"(x)".repeat(20)).len(), 33);
    }
    #[test]
    fn choices_ignore_blank_separators_and_optional_is_omitted_first() {
        let node = json!({"children":[{"type":"choice","text":"a"}," \t",{"type":"choice","text":"b"},"-",{"type":"optional","text":"x"}]});
        assert_eq!(
            forms(&node, &registry()),
            (
                vec!["a-".into(), "a-x".into(), "b-".into(), "b-x".into()],
                false
            )
        );
    }
    #[test]
    fn cap_precedes_deduplication_and_propagates_from_descendants() {
        let node = json!({"text":"(a)(b)(c)(d)(e)"});
        let (found, cut) = forms(&node, &registry());
        assert_eq!(found.len(), 31); // 32 raw readings, including the empty one
        assert!(!cut);
        let node = json!({"children":[{"text":"(a)(b)(c)(d)(e)(f)"}]});
        let (found, cut) = forms(&node, &registry());
        assert_eq!(found.len(), 32);
        assert!(cut);
        assert_eq!(found[0], "abcdef");
        let node = json!({"text":"()()()()()()"});
        assert_eq!(forms(&node, &registry()), (vec![], true));
    }
}
