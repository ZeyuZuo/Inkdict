//! Theme selectors (spec/v0.md §4.1), a port of OxDic's `oxdic/protocol.py`.

use std::collections::BTreeMap;

use crate::node::Attr;

/// Generated content a rule can style instead of the node itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pseudo {
    Before,
    After,
    Title,
    Marker,
    Icon,
}

impl Pseudo {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "before" => Pseudo::Before,
            "after" => Pseudo::After,
            "title" => Pseudo::Title,
            "marker" => Pseudo::Marker,
            "icon" => Pseudo::Icon,
            _ => return None,
        })
    }
}

/// One step of a selector: a type (`None` = `*`) and attribute conditions
/// (`None` value = the attribute is present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    pub kind: Option<String>,
    pub attrs: Vec<(String, Option<String>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// Left to right; the last part matches the styled node.
    pub parts: Vec<Compound>,
    /// Between consecutive parts.
    pub combinators: Vec<Combinator>,
    pub pseudo: Option<Pseudo>,
    pub text: String,
}

/// What a selector sees of a node: its type, attributes and inherited language.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub kind: &'a str,
    pub attrs: &'a BTreeMap<String, Attr>,
    pub lang: Option<&'a str>,
}

impl Selector {
    pub fn parse(text: &str) -> Result<Self, String> {
        let (body, pseudo) = match text.trim().split_once("::") {
            Some((body, name)) => {
                let pseudo = Pseudo::parse(name)
                    .ok_or_else(|| format!("{text}: unknown pseudo-element ::{name}"))?;
                (body, Some(pseudo))
            }
            None => (text.trim(), None),
        };
        let spaced = body.replace('>', " > ");
        let mut parts = Vec::new();
        let mut combinators = Vec::new();
        let mut pending = Combinator::Descendant;
        let mut dangling = false;
        for token in spaced.split_whitespace() {
            if token == ">" {
                pending = Combinator::Child;
                dangling = true;
                continue;
            }
            let compound = parse_compound(token)
                .ok_or_else(|| format!("{text}: invalid selector part {token:?}"))?;
            if !parts.is_empty() {
                combinators.push(pending);
            }
            parts.push(compound);
            pending = Combinator::Descendant;
            dangling = false;
        }
        if parts.is_empty() || dangling {
            return Err(format!("{text}: incomplete selector"));
        }
        Ok(Selector {
            parts,
            combinators,
            pseudo,
            text: text.to_owned(),
        })
    }

    /// (attribute conditions, typed parts): more of either wins.
    pub fn specificity(&self) -> (usize, usize) {
        let attrs = self.parts.iter().map(|part| part.attrs.len()).sum();
        let types = self.parts.iter().filter(|part| part.kind.is_some()).count();
        (attrs, types)
    }

    /// Type of the styled node, or None for `*`.
    pub fn subject(&self) -> Option<&str> {
        self.parts.last().and_then(|part| part.kind.as_deref())
    }

    /// Whether the selector matches the last frame of `path` (root first).
    pub fn matches(&self, path: &[Frame]) -> bool {
        !path.is_empty() && self.match_from(self.parts.len() - 1, path.len() - 1, path)
    }

    fn match_from(&self, part: usize, frame: usize, path: &[Frame]) -> bool {
        if !self.parts[part].matches(&path[frame]) {
            return false;
        }
        if part == 0 {
            return true;
        }
        match self.combinators[part - 1] {
            Combinator::Child => frame > 0 && self.match_from(part - 1, frame - 1, path),
            Combinator::Descendant => (0..frame).rev().any(|i| self.match_from(part - 1, i, path)),
        }
    }
}

impl Compound {
    fn matches(&self, frame: &Frame) -> bool {
        if self.kind.as_deref().is_some_and(|kind| kind != frame.kind) {
            return false;
        }
        self.attrs.iter().all(|(name, value)| {
            if name == "lang" {
                return value
                    .as_deref()
                    .is_some_and(|wanted| lang_matches(frame.lang, wanted));
            }
            match (frame.attrs.get(name), value) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(actual), Some(wanted)) => {
                    python_str(actual).to_lowercase() == wanted.to_lowercase()
                }
            }
        })
    }
}

/// `zh` matches `zh` and `zh-Hans`.
fn lang_matches(actual: Option<&str>, wanted: &str) -> bool {
    actual.is_some_and(|lang| {
        lang == wanted
            || lang
                .strip_prefix(wanted)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// An attribute as Python's `str()` writes it (the reference compares strings).
pub(crate) fn python_str(attr: &Attr) -> String {
    match attr {
        Attr::Bool(true) => "True".to_owned(),
        Attr::Bool(false) => "False".to_owned(),
        Attr::Int(value) => value.to_string(),
        Attr::Str(value) => value.clone(),
    }
}

/// `type[attr][attr=value]…` with lower-case names; values run to `]`.
fn parse_compound(token: &str) -> Option<Compound> {
    let end = token.find('[').unwrap_or(token.len());
    let (name, mut rest) = token.split_at(end);
    let kind = match name {
        "*" => None,
        _ if is_name(name) => Some(name.to_owned()),
        _ => return None,
    };
    let mut attrs = Vec::new();
    while !rest.is_empty() {
        let inner = rest.strip_prefix('[')?;
        let close = inner.find(']')?;
        let (condition, after) = (&inner[..close], &inner[close + 1..]);
        let (attr, value) = match condition.split_once('=') {
            Some((attr, value)) if !value.is_empty() => (attr, Some(value.to_owned())),
            Some(_) => return None,
            None => (condition, None),
        };
        if !is_name(attr) {
            return None;
        }
        attrs.push((attr.to_owned(), value));
        rest = after;
    }
    Some(Compound { kind, attrs })
}

fn is_name(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<'a>(
        kind: &'a str,
        attrs: &'a BTreeMap<String, Attr>,
        lang: Option<&'a str>,
    ) -> Frame<'a> {
        Frame { kind, attrs, lang }
    }

    #[test]
    fn parses_parts_combinators_and_pseudo() {
        let selector = Selector::parse("entry > head badge[scheme=cefr][n]::before").unwrap();
        assert_eq!(selector.parts.len(), 3);
        assert_eq!(
            selector.combinators,
            [Combinator::Child, Combinator::Descendant]
        );
        assert_eq!(selector.pseudo, Some(Pseudo::Before));
        assert_eq!(selector.specificity(), (2, 3));
        assert_eq!(selector.subject(), Some("badge"));
        assert_eq!(Selector::parse("*[lang=zh]").unwrap().specificity(), (1, 0));
        for bad in ["", "a >", "A", "a::nope", "a[b=]", "a[B]", "a[b"] {
            assert!(Selector::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn matches_types_attrs_languages_and_combinators() {
        let none = BTreeMap::new();
        let cefr = BTreeMap::from([("scheme".to_owned(), Attr::Str("CEFR".into()))]);
        let flag = BTreeMap::from([("wrong".to_owned(), Attr::Bool(true))]);
        let path = [
            frame("entry", &none, Some("en")),
            frame("head", &none, Some("en")),
            frame("badge", &cefr, Some("zh-Hans")),
        ];
        let yes = [
            "badge",
            "entry badge",
            "head > badge",
            "entry > head > badge",
            "badge[scheme=cefr]",
            "*[lang=zh]",
            "badge[scheme]",
        ];
        let no = [
            "entry > badge",
            "badge[scheme=exam]",
            "*[lang=z]",
            "*[lang]",
            "head",
            "badge[n]",
        ];
        for text in yes {
            assert!(Selector::parse(text).unwrap().matches(&path), "{text}");
        }
        for text in no {
            assert!(!Selector::parse(text).unwrap().matches(&path), "{text}");
        }
        let example = [frame("example", &flag, None)];
        assert!(
            Selector::parse("example[wrong=true]")
                .unwrap()
                .matches(&example)
        );
    }
}
