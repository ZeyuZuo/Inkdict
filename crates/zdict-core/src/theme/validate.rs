//! Checking a theme against a registry: OxDic `protocol.py` `Theme.validate`.

use std::collections::BTreeSet;

use super::{Pseudo, Theme, Value};
use crate::registry::Registry;

/// What a property accepts.
#[derive(Clone, Copy)]
enum Kind {
    Font,
    Color,
    Number,
    Text,
    Flag,
    OneOf(&'static [&'static str]),
}

const ICONS: &[&str] = &["none", "speaker", "key", "arrow", "diamond"];

/// Text properties: inherited by descendants.
const TEXT: &[(&str, Kind)] = &[
    ("font", Kind::Font),
    ("size", Kind::Number),
    ("weight", Kind::OneOf(&["normal", "bold"])),
    ("style", Kind::OneOf(&["normal", "italic"])),
    (
        "case",
        Kind::OneOf(&["none", "upper", "lower", "small-caps"]),
    ),
    ("decoration", Kind::OneOf(&["none", "strike", "underline"])),
    ("color", Kind::Color),
    ("baseline", Kind::OneOf(&["normal", "super", "sub"])),
    ("line_height", Kind::Number),
];

/// Box properties: the node itself only.
const BOX: &[(&str, Kind)] = &[
    ("display", Kind::OneOf(&["block", "inline", "none"])),
    ("before", Kind::Text),
    ("after", Kind::Text),
    ("join", Kind::Text),
    ("title", Kind::Text),
    ("marker", Kind::Text),
    ("hide_text", Kind::Flag),
    ("space_before", Kind::Number),
    ("space_after", Kind::Number),
    ("space_start", Kind::Number),
    ("space_end", Kind::Number),
    ("indent", Kind::Number),
    ("background", Kind::Color),
    ("padding", Kind::Number),
    ("radius", Kind::Number),
    ("divider", Kind::Color),
    ("collapse", Kind::OneOf(&["none", "open", "closed"])),
    ("icon", Kind::OneOf(ICONS)),
    ("max_width", Kind::Number),
];

fn allowed(pseudo: Option<Pseudo>, name: &str) -> Option<Kind> {
    let find = |list: &[(&str, Kind)]| {
        list.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, kind)| *kind)
    };
    match pseudo {
        None => find(TEXT).or_else(|| find(BOX)),
        Some(Pseudo::Title) if name == "space_after" => Some(Kind::Number),
        Some(Pseudo::Marker) if name == "icon" => Some(Kind::OneOf(ICONS)),
        Some(_) => find(TEXT),
    }
}

impl Theme {
    /// Problems with this theme, as `selector: problem`; empty when valid.
    pub fn validate(&self, registry: &Registry) -> Vec<String> {
        let mut errors = Vec::new();
        let tokens: BTreeSet<&str> = self
            .colors
            .iter()
            .flat_map(|(_, values)| values.iter().map(|(token, _)| token.as_str()))
            .collect();
        for (scheme, values) in &self.colors {
            let have: BTreeSet<&str> = values.iter().map(|(token, _)| token.as_str()).collect();
            let missing: Vec<_> = tokens.difference(&have).collect();
            if !missing.is_empty() {
                errors.push(format!("colors.{scheme}: missing {missing:?}"));
            }
        }
        let fonts: BTreeSet<&str> = self.fonts.iter().map(|(token, _)| token.as_str()).collect();
        for rule in &self.rules {
            let text = &rule.selector.text;
            for part in &rule.selector.parts {
                let Some(kind) = &part.kind else { continue };
                let Some(spec) = registry.get(kind) else {
                    errors.push(format!("{text}: unknown type {kind}"));
                    continue;
                };
                for (attr, _) in &part.attrs {
                    if attr != "lang" && !spec.attrs.contains_key(attr) {
                        errors.push(format!("{text}: {kind} has no attr {attr}"));
                    }
                }
            }
            for (name, value) in rule.props.iter() {
                let problem = match allowed(rule.selector.pseudo, name) {
                    None => Some("not allowed here".to_owned()),
                    Some(kind) => check(kind, value, &fonts, &tokens),
                };
                if let Some(problem) = problem {
                    errors.push(format!("{text}: {name} {problem}"));
                }
            }
        }
        errors
    }
}

fn check(
    kind: Kind,
    value: &Value,
    fonts: &BTreeSet<&str>,
    colors: &BTreeSet<&str>,
) -> Option<String> {
    let ok = match kind {
        Kind::Number => matches!(value, Value::Int(_) | Value::Float(_)),
        Kind::Text => matches!(value, Value::Str(_)),
        Kind::Flag => matches!(value, Value::Bool(_)),
        Kind::OneOf(options) => value.as_str().is_some_and(|v| options.contains(&v)),
        Kind::Font => value.as_str().is_some_and(|v| fonts.contains(v)),
        Kind::Color => value.as_str().is_some_and(|v| colors.contains(v)),
    };
    (!ok).then(|| format!("has invalid value {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: &str = "[types.badge]\nkind = \"inline\"\nattrs = { scheme = \"string\" }\n";

    #[test]
    fn reports_unknown_types_attrs_props_and_tokens() {
        let registry = Registry::parse(TYPES).unwrap();
        let theme = Theme::parse(
            "t",
            "[meta]\nformat = \"zdict-theme\"\n[fonts]\nui = [\"Inter\"]\n\
             [colors.light]\nfg = \"#000\"\nkey = \"#f80\"\n[colors.dark]\nfg = \"#fff\"\n\
             [style.badge]\nfont = \"ui\"\ncolor = \"fg\"\nweight = \"heavy\"\n\
             [style.\"badge[level]\"]\nsize = \"big\"\n\
             [style.ghost]\ncolor = \"nope\"\n\
             [style.\"badge::before\"]\nindent = 1\n\
             [style.\"badge::marker\"]\nicon = \"key\"\n",
        )
        .unwrap();
        let errors = theme.validate(&registry);
        let wanted = [
            "colors.dark: missing [\"key\"]",
            "badge: weight has invalid value heavy",
            "badge[level]: badge has no attr level",
            "badge[level]: size has invalid value big",
            "ghost: unknown type ghost",
            "ghost: color has invalid value nope",
            "badge::before: indent not allowed here",
        ];
        assert_eq!(errors, wanted);
    }
}
