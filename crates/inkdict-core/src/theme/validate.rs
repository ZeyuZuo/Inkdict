//! Checking a theme against a registry: OxDic `protocol.py` `Theme.validate`.

use std::collections::{BTreeMap, BTreeSet};

use super::{Pseudo, Theme, ThemeStack, Value};
use crate::registry::Registry;

/// What a property accepts.
#[derive(Clone, Copy)]
enum Kind {
    Font,
    Color,
    Number,
    Text,
    Flag,
    Selector,
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
    ("tabs", Kind::Flag),
    ("tab_label", Kind::Selector),
    ("tab_accent", Kind::Color),
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
    /// Problems with this theme on its own, as `selector: problem`; empty
    /// when valid. An overlay that uses tokens of the theme below it is
    /// checked with [`ThemeStack::validate`] instead.
    pub fn validate(&self, registry: &Registry) -> Vec<String> {
        let colors = color_sets(std::slice::from_ref(self));
        let mut errors = self.font_errors.clone();
        errors.extend(missing_colors(&colors));
        errors.extend(self.rule_problems(
            registry,
            &font_tokens(std::slice::from_ref(self)),
            &all(&colors),
        ));
        errors
    }

    fn rule_problems(
        &self,
        registry: &Registry,
        fonts: &BTreeSet<&str>,
        colors: &BTreeSet<&str>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
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
                    Some(kind) => check(kind, value, fonts, colors),
                };
                if let Some(problem) = problem {
                    errors.push(format!("{text}: {name} {problem}"));
                }
            }
        }
        errors
    }
}

impl ThemeStack {
    /// Problems with the stack as applied (spec v1 §4.3): every layer may
    /// use the font and color tokens of any layer, and each color scheme
    /// must define every token once merged. Problems name their layer.
    pub fn validate(&self, registry: &Registry) -> Vec<String> {
        let layers = self.layers();
        let colors = color_sets(layers);
        let fonts = font_tokens(layers);
        let mut errors = missing_colors(&colors);
        for theme in layers {
            let problems = theme.font_errors.iter().cloned().chain(theme.rule_problems(
                registry,
                &fonts,
                &all(&colors),
            ));
            errors.extend(
                problems
                    .into_iter()
                    .map(|problem| format!("{}: {problem}", theme.name)),
            );
        }
        errors
    }
}

/// Color tokens per scheme, merged over the layers.
fn color_sets(layers: &[Theme]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut schemes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for theme in layers {
        for (scheme, values) in &theme.colors {
            schemes
                .entry(scheme)
                .or_default()
                .extend(values.iter().map(|(token, _)| token.as_str()));
        }
    }
    schemes
}

fn all<'a>(schemes: &BTreeMap<&'a str, BTreeSet<&'a str>>) -> BTreeSet<&'a str> {
    schemes.values().flatten().copied().collect()
}

/// Light and dark must name the same tokens.
fn missing_colors(schemes: &BTreeMap<&str, BTreeSet<&str>>) -> Vec<String> {
    let tokens = all(schemes);
    schemes
        .iter()
        .filter_map(|(scheme, have)| {
            let missing: Vec<_> = tokens.difference(have).collect();
            (!missing.is_empty()).then(|| format!("colors.{scheme}: missing {missing:?}"))
        })
        .collect()
}

fn font_tokens(layers: &[Theme]) -> BTreeSet<&str> {
    layers
        .iter()
        .flat_map(|theme| theme.fonts.iter().map(|(token, _)| token.as_str()))
        .collect()
}

fn check(
    kind: Kind,
    value: &Value,
    fonts: &BTreeSet<&str>,
    colors: &BTreeSet<&str>,
) -> Option<String> {
    let ok = match kind {
        Kind::Selector => value.as_str().is_some_and(|text| {
            super::Selector::parse(text).is_ok_and(|selector| selector.pseudo.is_none())
        }),
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

    const TYPES: &str = "[meta]\nformat = \"inkdict-types\"\nversion = 1\n[types.badge]\nkind = \"inline\"\nattrs = { scheme = \"string\" }\n";

    #[test]
    fn overlays_may_use_tokens_of_lower_layers() {
        let registry = Registry::parse(TYPES).unwrap();
        let base = Theme::parse(
            "base",
            "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n[fonts.text]\ndefault = [\"Serif\"]\n\
                                [colors.light]\nfg = \"#000\"\n[colors.dark]\nfg = \"#fff\"\n",
        )
        .unwrap();
        let overlay = Theme::parse(
            "overlay",
            "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n\
                                   [style.badge]\nfont = \"text\"\ncolor = \"fg\"\n",
        )
        .unwrap();
        assert_eq!(
            overlay.validate(&registry).len(),
            2,
            "alone, its tokens are unknown"
        );
        let mut stack = ThemeStack::new(base);
        stack.push(overlay);
        assert!(
            stack.validate(&registry).is_empty(),
            "{:?}",
            stack.validate(&registry)
        );
        stack.push(
            Theme::parse(
                "bad",
                "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n[colors.light]\naccent = \"#f00\"\n",
            )
            .unwrap(),
        );
        assert_eq!(
            stack.validate(&registry),
            ["colors.dark: missing [\"accent\"]"]
        );
    }

    #[test]
    fn validates_v1_font_stacks_language_keys_and_tab_properties() {
        let registry = Registry::parse(TYPES).unwrap();
        let parse = |body: &str| {
            Theme::parse(
                "t",
                &format!("[meta]\nformat = \"inkdict-theme\"\nversion = 1\n{body}"),
            )
            .unwrap()
        };
        let theme = parse(
            r##"
[fonts.any_token]
default = ["A"]
"zh-Hans" = ["B"]
[colors.light]
accent = "#123"
[style.badge]
font = "any_token"
tabs = true
tab_label = "badge[scheme=core] > *[lang=zh]"
tab_accent = "accent"
"##,
        );
        assert!(theme.validate(&registry).is_empty());
        for body in [
            "[fonts]\na = [\"old syntax\"]",
            "[fonts.a]\nzh = [\"No default\"]",
            "[fonts.a]\ndefault = []",
            "[fonts.a]\ndefault = [1]",
            "[fonts.a]\ndefault = \"not a list\"",
            "[fonts.a]\ndefault = [\"A\"]\nEN = [\"B\"]",
            "[fonts.a]\ndefault = [\"A\"]\n\"en-x\" = [\"B\"]",
            "[style.badge]\ntabs = 1",
            "[style.badge]\ntab_label = \"badge::before\"",
            "[style.badge]\ntab_label = \"badge >\"",
            "[style.badge]\ntab_label = false",
            "[style.badge]\ntab_accent = \"missing\"",
            "[style.\"badge::before\"]\ntabs = true",
            "[style.\"badge::icon\"]\ntab_label = \"badge\"",
        ] {
            let theme = parse(body);
            assert!(!theme.validate(&registry).is_empty(), "{body}");
            assert!(
                !ThemeStack::new(theme).validate(&registry).is_empty(),
                "{body}"
            );
        }
    }

    #[test]
    fn reports_unknown_types_attrs_props_and_tokens() {
        let registry = Registry::parse(TYPES).unwrap();
        let theme = Theme::parse(
            "t",
            "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n[fonts.ui]\ndefault = [\"Inter\"]\n\
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
