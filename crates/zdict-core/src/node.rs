//! Entry content: a tree of typed nodes (spec/v0.md §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One node. Field names and omission rules follow the JSON form exactly,
/// so a node serializes back to the JSON it was read from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Type name from the pack's type registry.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// BCP 47 tag; absent means inherited from the parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Attr>,
    /// Plain text content; never present together with `children`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Child>,
}

/// An element of `children`: a string or a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Text(String),
    Node(Node),
}

/// An attribute value. The registry's `string`, `res`, `ref` and `enum`
/// types are all strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Attr {
    Bool(bool),
    Int(i64),
    Str(String),
}

impl Attr {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Attr::Str(value) => Some(value),
            _ => None,
        }
    }
}

impl Node {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.get(name)
    }

    /// Child nodes, skipping strings.
    pub fn child_nodes(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|child| match child {
            Child::Node(node) => Some(node),
            Child::Text(_) => None,
        })
    }
}

/// A decoded entry with its row in `entries`.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Row id, valid only inside this pack.
    pub id: i64,
    /// Stable entry ID (`root.id`), used by links, deep links and history.
    pub key: String,
    pub headword: String,
    pub homograph: Option<i64>,
    pub root: Node,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_json_form() {
        let json = r#"{"type":"example","id":"e1","children":[{"type":"example_text","children":["an ",{"type":"collocation","text":"apple pie"}]},{"type":"example_text","lang":"zh-Hans","text":"苹果馅饼"},{"type":"x","attrs":{"n":2,"open":true,"res":"wa:a#b"}}]}"#;
        let node: Node = serde_json::from_str(json).unwrap();
        assert_eq!(node.kind, "example");
        assert_eq!(node.child_nodes().count(), 3);
        let x = node.child_nodes().nth(2).unwrap();
        assert_eq!(x.attr("n"), Some(&Attr::Int(2)));
        assert_eq!(x.attr("open"), Some(&Attr::Bool(true)));
        assert_eq!(x.attr("res").and_then(Attr::as_str), Some("wa:a#b"));
        let back: serde_json::Value = serde_json::to_value(&node).unwrap();
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn rejects_nodes_without_type() {
        assert!(serde_json::from_str::<Node>(r#"{"text":"a"}"#).is_err());
    }
}
