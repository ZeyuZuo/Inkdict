//! Anchors and link targets (spec/pack-v1.md §3.4, spec/v1.md §6).

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::node::{Attr, Child, Entry, Node};
use crate::pack::Pack;
use crate::text::display;

impl Entry {
    /// Nodes with an `id` in document order: position = anchor number, so
    /// the root is anchor 0.
    pub fn anchors(&self) -> Vec<&Node> {
        let mut found = Vec::new();
        let mut stack = vec![&self.root];
        while let Some(node) = stack.pop() {
            if node.id.is_some() {
                found.push(node);
            }
            stack.extend(node.child_nodes().collect::<Vec<_>>().into_iter().rev());
        }
        found
    }

    /// The node with this anchor number, as given by lookup and search results.
    pub fn anchor(&self, number: u32) -> Option<&Node> {
        self.anchors().get(number as usize).copied()
    }

    /// Anchor number of the node with this `id`.
    pub fn anchor_number(&self, id: &str) -> Option<u32> {
        let position = self
            .anchors()
            .iter()
            .position(|node| node.id.as_deref() == Some(id))?;
        u32::try_from(position).ok()
    }
}

/// Where following a link leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// Open the entry with this key and bring `focus` into view.
    Entry { key: String, focus: Focus },
    /// No target in the pack: look this text up instead.
    Lookup(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Top,
    /// The node with this anchor number.
    Anchor(u32),
    /// The entry's first `word_family` node (`anchor_type = wfg`).
    WordFamily,
}

/// What a `link`-role node points at, as written in the content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_type: Option<String>,
    /// The node's text, for the lookup fallback.
    pub text: String,
}

impl Link {
    pub fn of(node: &Node) -> Self {
        let attr = |name| node.attr(name).and_then(Attr::as_str).map(String::from);
        Link {
            entry: attr("entry"),
            anchor: attr("anchor"),
            anchor_type: attr("anchor_type"),
            text: display(&plain(node)),
        }
    }
}

impl Pack {
    /// Target of a `link`-role node (`target`, `xword`) inside `current`.
    pub fn resolve_link(&self, current: &Entry, node: &Node) -> Result<LinkTarget> {
        self.resolve(current, &Link::of(node))
    }

    /// Where `link` leads from `current` (spec/v1.md §6), falling back in order
    /// when a target is not in the pack: entry + anchor → that entry (its
    /// word family for `wfg`) → the anchor in the current entry → a lookup
    /// of the link's text.
    pub fn resolve(&self, current: &Entry, link: &Link) -> Result<LinkTarget> {
        let anchor = link.anchor.as_deref();
        if let Some(key) = link.entry.as_deref() {
            let target = if key == current.key {
                Some(current.clone())
            } else {
                match self.entry(key) {
                    Ok(entry) => Some(entry),
                    Err(Error::MissingEntry(_)) => None,
                    Err(err) => return Err(err),
                }
            };
            if let Some(target) = target {
                let focus = match anchor.and_then(|id| target.anchor_number(id)) {
                    Some(number) => Focus::Anchor(number),
                    None if link.anchor_type.as_deref() == Some("wfg") => Focus::WordFamily,
                    None => Focus::Top,
                };
                return Ok(LinkTarget::Entry {
                    key: target.key,
                    focus,
                });
            }
        }
        if let Some(number) = anchor.and_then(|id| current.anchor_number(id)) {
            return Ok(LinkTarget::Entry {
                key: current.key.clone(),
                focus: Focus::Anchor(number),
            });
        }
        Ok(LinkTarget::Lookup(link.text.clone()))
    }
}

/// All text of a node and its descendants, in order.
pub fn plain(node: &Node) -> String {
    let mut text = node.text.clone().unwrap_or_default();
    for child in &node.children {
        match child {
            Child::Text(part) => text.push_str(part),
            Child::Node(inner) => text.push_str(&plain(inner)),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_pack::TestPack;

    const RUN: &str = r#"{"type":"entry","id":"run","lang":"en","children":[{"type":"head","children":[{"type":"headword","text":"run"}]},{"type":"pos_group","children":[{"type":"sense","id":"run-1"},{"type":"idiom","id":"run-2","children":[{"type":"sense","id":"run-3"}]}]},{"type":"word_family"}]}"#;
    const WALK: &str =
        r#"{"type":"entry","id":"walk","lang":"en","children":[{"type":"sense","id":"walk-1"}]}"#;

    fn setup() -> (Pack, Entry) {
        let test = TestPack::new();
        test.entry(1, "run", None, RUN).entry(2, "walk", None, WALK);
        let pack = test.open();
        let walk = pack.entry("walk").unwrap();
        (pack, walk)
    }

    fn link(attrs: &str, text: &str) -> Node {
        serde_json::from_str(&format!(r#"{{"type":"target","attrs":{{{attrs}}},"children":["{text}",{{"type":"target_phrase","text":"river"}}]}}"#)).unwrap()
    }

    fn entry(key: &str, focus: Focus) -> LinkTarget {
        LinkTarget::Entry {
            key: key.into(),
            focus,
        }
    }

    #[test]
    fn numbers_anchors_in_document_order() {
        let (pack, _) = setup();
        let run = pack.entry("run").unwrap();
        let ids: Vec<_> = run
            .anchors()
            .iter()
            .map(|n| n.id.clone().unwrap())
            .collect();
        assert_eq!(ids, ["run", "run-1", "run-2", "run-3"]);
        assert_eq!(run.anchor(2).unwrap().kind, "idiom");
        assert!(run.anchor(4).is_none());
        assert_eq!(run.anchor_number("run-3"), Some(3));
        assert_eq!(run.anchor_number("walk-1"), None);
    }

    #[test]
    fn resolves_links_in_fallback_order() {
        let (pack, walk) = setup();
        let cases = [
            (
                r#""entry":"run","anchor":"run-3""#,
                entry("run", Focus::Anchor(3)),
            ),
            (r#""entry":"run""#, entry("run", Focus::Top)),
            (
                r#""entry":"run","anchor_type":"wfg""#,
                entry("run", Focus::WordFamily),
            ),
            (r#""entry":"run","anchor":"gone""#, entry("run", Focus::Top)),
            (r#""anchor":"walk-1""#, entry("walk", Focus::Anchor(1))),
            (
                r#""entry":"thames","anchor":"walk-1""#,
                entry("walk", Focus::Anchor(1)),
            ),
            (
                r#""entry":"thames""#,
                LinkTarget::Lookup("Thames river".into()),
            ),
            ("", LinkTarget::Lookup("Thames river".into())),
        ];
        for (attrs, wanted) in cases {
            let node = link(attrs, "Thames ");
            assert_eq!(pack.resolve_link(&walk, &node).unwrap(), wanted, "{attrs}");
        }
    }
}
