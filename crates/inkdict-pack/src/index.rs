use crate::{
    readings::{forms, plain},
    source::{SourceError, bad},
};
use inkdict_core::{
    Attr, IndexKind, Registry, Role,
    text::{Scheme, display, norm},
    theme::{Frame, Selector},
};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct EntryIndex {
    pub headword: String,
    first_headword: String,
    pub homograph: Option<i64>,
    homograph_seen: bool,
    pub lookup: BTreeMap<(String, usize), (usize, String, IndexKind)>,
    pub anchors: Vec<String>,
    pub links: Vec<BTreeMap<String, Value>>,
    pub resources: Vec<String>,
    pub importance: usize,
    pub problems: Vec<(&'static str, String)>,
}

pub(crate) struct Indexer<'a> {
    registry: &'a Registry,
    scheme: Scheme,
    boost: Vec<Selector>,
}

impl<'a> Indexer<'a> {
    pub fn new(
        registry: &'a Registry,
        scheme: Scheme,
        boost: &[String],
    ) -> std::result::Result<Self, SourceError> {
        let boost = boost
            .iter()
            .map(|text| {
                let selector = Selector::parse(text).map_err(bad)?;
                if selector.pseudo.is_some() {
                    return Err(bad(format!(
                        "boost selector {} must not name a pseudo-element",
                        serde_json::to_string(text).expect("strings serialize")
                    )));
                }
                Ok(selector)
            })
            .collect::<std::result::Result<_, _>>()?;
        Ok(Self {
            registry,
            scheme,
            boost,
        })
    }

    pub fn rank_width(&self) -> usize {
        self.boost.len() + 1
    }

    pub fn index(&self, entry: &Value) -> EntryIndex {
        let mut result = EntryIndex {
            importance: self.boost.len(),
            ..EntryIndex::default()
        };
        self.walk(entry, &mut result, &[], 0);
        if result.headword.is_empty() {
            result.headword = result.first_headword.clone();
        }
        if result.headword.is_empty() {
            result.headword = entry["id"].as_str().unwrap_or_default().into();
        }
        result
    }

    fn walk(&self, node: &Value, result: &mut EntryIndex, path: &[Frame<'_>], mut anchor: usize) {
        let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
        let roles = self.registry.roles(kind);
        let attrs = node.get("attrs").and_then(Value::as_object);
        let frame_attrs = attrs
            .into_iter()
            .flatten()
            .map(|(k, v)| {
                let attr = match v {
                    Value::Bool(b) => Attr::Bool(*b),
                    Value::Number(n) if n.as_i64().is_some() => Attr::Int(n.as_i64().unwrap()),
                    Value::String(s) => Attr::Str(s.clone()),
                    _ => Attr::Str(v.to_string()),
                };
                (k.clone(), attr)
            })
            .collect();
        let lang = node
            .get("lang")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| path.last().and_then(|f| f.lang));
        let mut path = path.to_vec();
        path.push(Frame {
            kind,
            attrs: &frame_attrs,
            lang,
        });
        if let Some(id) = node.get("id") {
            anchor = result.anchors.len();
            result.anchors.push(
                id.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| id.to_string()),
            );
        }
        for (i, selector) in self.boost[..result.importance].iter().enumerate() {
            if selector.matches(&path) {
                result.importance = i;
                break;
            }
        }
        if roles.contains(&Role::Link) {
            let link: BTreeMap<_, _> = ["entry", "anchor"]
                .into_iter()
                .filter_map(|name| {
                    attrs
                        .and_then(|a| a.get(name))
                        .map(|v| (name.to_owned(), v.clone()))
                })
                .collect();
            if !link.is_empty() {
                result.links.push(link);
            }
        }
        for name in self.registry.res_attrs(kind) {
            if let Some(value) = attrs.and_then(|a| a.get(name)).and_then(Value::as_str) {
                result.resources.push(value.to_owned());
            }
        }
        if roles.contains(&Role::Homograph) {
            let text = plain(node);
            let text = text.trim();
            let number = decimal(text).filter(|n| *n > 0);
            if number.is_none() {
                result
                    .problems
                    .push(("homograph not a positive integer", text.to_owned()));
            }
            if anchor == 0 && !result.homograph_seen {
                result.homograph = number;
                result.homograph_seen = true;
            }
        }
        if roles.contains(&Role::Index) {
            self.add_forms(node, kind, result, anchor);
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            for child in children.iter().filter(|c| c.is_object()) {
                self.walk(child, result, &path, anchor);
            }
        }
    }

    fn add_forms(&self, node: &Value, node_type: &str, result: &mut EntryIndex, anchor: usize) {
        let Some(kind) = self.registry.index_kind(node_type) else {
            return;
        };
        let class = match kind {
            IndexKind::Headword => {
                let written = display(&plain(node));
                if anchor == 0 && result.headword.is_empty() {
                    result.headword = written.clone();
                }
                if result.first_headword.is_empty() {
                    result.first_headword = written;
                }
                usize::from(anchor != 0)
            }
            IndexKind::Variant => 2,
            IndexKind::Phrase => 3,
            IndexKind::Inflection => 4,
            IndexKind::Reading => 5,
            IndexKind::Hidden => 6,
        };
        let (found, cut) = forms(node, self.registry);
        if cut {
            result
                .problems
                .push(("readings cut at 32", display(&plain(node))));
        }
        let mut any = false;
        for form in found {
            let key = norm(&form, self.scheme);
            if key.is_empty() {
                continue;
            }
            any = true;
            let lookup_key = (key, anchor);
            if result
                .lookup
                .get(&lookup_key)
                .is_none_or(|row| class < row.0)
            {
                result.lookup.insert(lookup_key, (class, form, kind));
            }
        }
        if !any {
            result
                .problems
                .push(("index node without readings", display(&plain(node))));
        }
    }
}

/// Unicode decimal digits intentionally support homograph numbers in non-Latin dictionaries.
fn decimal(text: &str) -> Option<i64> {
    const ZEROES: &[u32] = &[
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0,
        0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0, 0x16a60, 0x16ac0,
        0x16b50, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e950, 0x1fbf0,
    ];
    if text.is_empty() {
        return None;
    }
    text.chars().try_fold(0i64, |n, c| {
        let cp = c as u32;
        let zero = ZEROES.iter().find(|&&z| cp >= z && cp < z + 10)?;
        n.checked_mul(10)?.checked_add((cp - zero) as i64)
    })
}
