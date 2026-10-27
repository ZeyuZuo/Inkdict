//! Themes and their cascade (spec/v1.md §4).
//!
//! A port of OxDic's `oxdic/protocol.py` `Theme`: rules apply in order of
//! (layer, specificity, position in the file), and each property keeps the
//! position where it was first set, so output follows the reference order.

mod selector;
mod validate;

use std::collections::HashMap;
use std::fmt;

use serde::ser::{Serialize, SerializeMap, Serializer};

use crate::error::{Error, Result};
use crate::pack::Pack;

pub(crate) use selector::python_str;
pub use selector::{Combinator, Compound, Frame, Pseudo, Selector};

/// A property value as written in the theme TOML.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(number) => Some(*number as f64),
            Value::Float(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    fn from_toml(value: &toml::Value) -> Option<Self> {
        Some(match value {
            toml::Value::Boolean(flag) => Value::Bool(*flag),
            toml::Value::Integer(number) => Value::Int(*number),
            toml::Value::Float(number) => Value::Float(*number),
            toml::Value::String(text) => Value::Str(text.clone()),
            _ => return None,
        })
    }
}

/// Python's `str()`: `1`, `1.0`, `0.8`, `True`, so generated CSS matches the reference.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(flag) => f.write_str(if *flag { "True" } else { "False" }),
            Value::Int(number) => write!(f, "{number}"),
            Value::Float(number) if number.fract() == 0.0 && number.abs() < 1e16 => {
                write!(f, "{number:.1}")
            }
            Value::Float(number) => write!(f, "{number}"),
            Value::Str(text) => f.write_str(text),
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Value::Bool(flag) => serializer.serialize_bool(*flag),
            Value::Int(number) => serializer.serialize_i64(*number),
            Value::Float(number) => serializer.serialize_f64(*number),
            Value::Str(text) => serializer.serialize_str(text),
        }
    }
}

/// Properties in the order they were first set; later rules overwrite values
/// in place (Python `dict.update`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style(Vec<(String, Value)>);

impl Style {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn str(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(Value::as_str)
    }

    pub fn number(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(Value::as_f64)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.get(name).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn set(&mut self, name: &str, value: Value) {
        match self.0.iter_mut().find(|(key, _)| key == name) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((name.to_owned(), value)),
        }
    }

    fn update(&mut self, other: &Style) {
        for (name, value) in &other.0 {
            self.set(name, value.clone());
        }
    }
}

impl Serialize for Style {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in &self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

#[derive(Debug, Clone)]
struct Rule {
    selector: Selector,
    props: Style,
    /// Position in the file.
    order: usize,
}

/// Language key to candidate font families, in declaration order.
pub type FontStacks = Vec<(String, Vec<String>)>;

/// One theme file (`inkdict-theme`).
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    /// Font token -> language key -> candidate families, in file order.
    pub fonts: Vec<(String, FontStacks)>,
    font_errors: Vec<String>,
    /// Scheme (`light`, `dark`) -> color token -> CSS color, in file order.
    pub colors: Vec<(String, Vec<(String, String)>)>,
    rules: Vec<Rule>,
    /// Rule positions by the type they style; `*` rules under "".
    by_subject: HashMap<String, Vec<usize>>,
}

impl Theme {
    /// Parse a theme file; `name` is only used in error messages.
    pub fn parse(name: &str, text: &str) -> Result<Self> {
        let bad = |reason: String| Error::BadFile {
            name: name.to_owned(),
            reason,
        };
        let data: toml::Table = text
            .parse()
            .map_err(|err: toml::de::Error| bad(err.to_string()))?;
        let meta = data.get("meta").and_then(toml::Value::as_table);
        if meta
            .and_then(|meta| meta.get("format"))
            .and_then(toml::Value::as_str)
            != Some("inkdict-theme")
        {
            return Err(bad("meta.format is not \"inkdict-theme\"".to_owned()));
        }
        if meta
            .and_then(|m| m.get("version"))
            .and_then(toml::Value::as_integer)
            != Some(1)
        {
            return Err(bad("not an inkdict-theme v1 theme".to_owned()));
        }
        let title = meta
            .and_then(|m| m.get("name"))
            .and_then(toml::Value::as_str);
        let mut fonts = Vec::new();
        let mut font_errors = Vec::new();
        for (token, stacks) in section(&data, "fonts") {
            let mut parsed = Vec::new();
            if let Some(stacks) = stacks
                .as_table()
                .filter(|stacks| stacks.contains_key("default"))
            {
                for (lang, stack) in stacks {
                    if lang != "default" && !valid_lang(lang) {
                        font_errors.push(format!("fonts.{token}: invalid lang {lang:?}"));
                    }
                    let families: Option<Vec<String>> = stack.as_array().and_then(|items| {
                        items.iter().map(|v| v.as_str().map(String::from)).collect()
                    });
                    if families.as_ref().is_none_or(Vec::is_empty) {
                        font_errors.push(format!(
                            "fonts.{token}.{lang}: must be a list of font names"
                        ));
                    }
                    parsed.push((lang.clone(), families.unwrap_or_default()));
                }
            } else {
                font_errors.push(format!("fonts.{token}: needs a table with a default list"));
            }
            fonts.push((token.clone(), parsed));
        }
        let mut colors = Vec::new();
        for (scheme, tokens) in section(&data, "colors") {
            let tokens = tokens
                .as_table()
                .and_then(|table| {
                    table
                        .iter()
                        .map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
                        .collect()
                })
                .ok_or_else(|| bad(format!("colors.{scheme} is not a table of strings")))?;
            colors.push((scheme.clone(), tokens));
        }
        let mut rules = Vec::new();
        for (order, (text, props)) in section(&data, "style").enumerate() {
            let selector = Selector::parse(text).map_err(bad)?;
            let mut style = Style::default();
            for (prop, value) in props
                .as_table()
                .ok_or_else(|| bad(format!("style.\"{text}\" is not a table")))?
            {
                let value = Value::from_toml(value)
                    .ok_or_else(|| bad(format!("{text}: {prop} is not a scalar")))?;
                style.set(prop, value);
            }
            rules.push(Rule {
                selector,
                props: style,
                order,
            });
        }
        let mut by_subject: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, rule) in rules.iter().enumerate() {
            let subject = rule.selector.subject().unwrap_or_default().to_owned();
            by_subject.entry(subject).or_default().push(index);
        }
        Ok(Theme {
            name: title.unwrap_or(name).to_owned(),
            fonts,
            font_errors,
            colors,
            rules,
            by_subject,
        })
    }

    /// Exact language tag, then primary subtag, then `default`.
    pub fn font_key<'a>(&'a self, token: &str, lang: Option<&str>) -> &'a str {
        font_key(
            self.fonts
                .iter()
                .find(|(name, _)| name == token)
                .map(|(_, stacks)| stacks.as_slice()),
            lang,
        )
    }

    fn candidates<'a>(&'a self, kind: &str) -> impl Iterator<Item = &'a Rule> {
        let typed = self.by_subject.get(kind).into_iter().flatten();
        let any = self.by_subject.get("").into_iter().flatten();
        typed.chain(any).map(|&index| &self.rules[index])
    }
}

/// Themes applied together: the pack's default at the bottom, then app and
/// user layers. A higher layer outranks any rule below it (spec v1 §4.3).
#[derive(Debug, Clone)]
pub struct ThemeStack {
    layers: Vec<Theme>,
}

impl ThemeStack {
    pub fn new(base: Theme) -> Self {
        ThemeStack { layers: vec![base] }
    }

    pub fn push(&mut self, overlay: Theme) {
        self.layers.push(overlay);
    }

    pub fn layers(&self) -> &[Theme] {
        &self.layers
    }

    /// Font tokens merged by name; an upper layer replaces a lower one's stack.
    pub fn fonts(&self) -> Vec<(String, FontStacks)> {
        merge(self.layers.iter().map(|theme| theme.fonts.as_slice()))
    }

    /// Resolve a token using the highest layer that defines it.
    pub fn font_key<'a>(&'a self, token: &str, lang: Option<&str>) -> &'a str {
        self.layers
            .iter()
            .rev()
            .find(|theme| theme.fonts.iter().any(|(name, _)| name == token))
            .map_or("default", |theme| theme.font_key(token, lang))
    }

    /// Color tokens of one scheme (`light`, `dark`), merged by name.
    pub fn colors(&self, scheme: &str) -> Vec<(String, String)> {
        merge(self.layers.iter().filter_map(|theme| {
            theme
                .colors
                .iter()
                .find(|(name, _)| name == scheme)
                .map(|(_, tokens)| tokens.as_slice())
        }))
    }

    /// Properties the matching rules set on the last node of `path` (or on
    /// its pseudo-element), cascade applied.
    pub fn own_style(&self, path: &[Frame], pseudo: Option<Pseudo>) -> Style {
        let Some(node) = path.last() else {
            return Style::default();
        };
        let mut matched: Vec<_> = self
            .layers
            .iter()
            .enumerate()
            .flat_map(|(layer, theme)| theme.candidates(node.kind).map(move |rule| (layer, rule)))
            .filter(|(_, rule)| rule.selector.pseudo == pseudo && rule.selector.matches(path))
            .collect();
        matched.sort_by_key(|(layer, rule)| (*layer, rule.selector.specificity(), rule.order));
        let mut style = Style::default();
        for (_, rule) in matched {
            style.update(&rule.props);
        }
        style
    }
}

fn font_key<'a>(stacks: Option<&'a [(String, Vec<String>)]>, lang: Option<&str>) -> &'a str {
    let Some(stacks) = stacks else {
        return "default";
    };
    let lang = lang.unwrap_or_default();
    let primary = lang.split('-').next().unwrap_or_default();
    [lang, primary]
        .into_iter()
        .find_map(|wanted| {
            stacks
                .iter()
                .find(|(key, _)| key == wanted)
                .map(|(key, _)| key.as_str())
        })
        .unwrap_or("default")
}

/// The language grammar accepted by protocol.py.
fn valid_lang(lang: &str) -> bool {
    let mut parts = lang.split('-');
    let primary = parts.next().unwrap_or_default();
    (2..=3).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_lowercase())
        && parts.all(|part| {
            (2..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

fn merge<'a, T: Clone + 'a>(layers: impl Iterator<Item = &'a [(String, T)]>) -> Vec<(String, T)> {
    let mut merged: Vec<(String, T)> = Vec::new();
    for layer in layers {
        for (name, value) in layer {
            match merged.iter_mut().find(|(key, _)| key == name) {
                Some((_, slot)) => *slot = value.clone(),
                None => merged.push((name.clone(), value.clone())),
            }
        }
    }
    merged
}

fn section<'a>(
    data: &'a toml::Table,
    key: &str,
) -> impl Iterator<Item = (&'a String, &'a toml::Value)> {
    data.get(key)
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
}

impl Pack {
    /// Names of the themes the pack ships (`themes/<name>.toml`).
    pub fn theme_names(&self) -> Result<Vec<String>> {
        Ok(self
            .file_names()?
            .into_iter()
            .filter_map(|file| {
                file.strip_prefix("themes/")
                    .and_then(|rest| rest.strip_suffix(".toml"))
                    .map(String::from)
            })
            .collect())
    }

    /// A theme shipped in the pack, by name (`default` for `themes/default.toml`).
    pub fn theme(&self, name: &str) -> Result<Theme> {
        let file = format!("themes/{name}.toml");
        Theme::parse(&file, &self.text_file(&file)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn theme(style: &str) -> Theme {
        Theme::parse(
            "t",
            &format!("[meta]\nformat = \"inkdict-theme\"\nversion = 1\n{style}"),
        )
        .unwrap()
    }

    #[test]
    fn formats_numbers_like_python() {
        let cases = [
            (Value::Int(1), "1"),
            (Value::Float(1.0), "1.0"),
            (Value::Float(0.8), "0.8"),
            (Value::Float(1.25), "1.25"),
            (Value::Bool(true), "True"),
        ];
        for (value, text) in cases {
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn cascades_by_specificity_then_order_keeping_first_positions() {
        let stack = ThemeStack::new(theme(
            "[style.\"sense > definition\"]\nsize = 0.9\ncolor = \"a\"\n\
             [style.definition]\ncolor = \"b\"\nweight = \"bold\"\n\
             [style.\"definition[lang=zh]\"]\ncolor = \"c\"\n",
        ));
        let attrs = BTreeMap::new();
        let path = [
            Frame {
                kind: "sense",
                attrs: &attrs,
                lang: Some("en"),
            },
            Frame {
                kind: "definition",
                attrs: &attrs,
                lang: Some("zh-Hans"),
            },
        ];
        let style = stack.own_style(&path, None);
        // definition (0,1) < sense > definition (0,2) < definition[lang=zh] (1,1)
        let order: Vec<_> = style.iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(order, ["color=c", "weight=bold", "size=0.9"]);
        assert!(stack.own_style(&path, Some(Pseudo::Before)).is_empty());
    }

    #[test]
    fn upper_layers_outrank_more_specific_rules() {
        let mut stack = ThemeStack::new(theme(
            "[fonts.text]\ndefault = [\"A\"]\n[fonts.ui]\ndefault = [\"B\"]\n[colors.light]\nfg = \"#000\"\n\
             [style.\"entry > definition[lang=zh]\"]\ndisplay = \"block\"\n",
        ));
        stack.push(theme(
            "[fonts.text]\ndefault = [\"C\"]\n[style.\"*[lang=zh]\"]\ndisplay = \"none\"\n",
        ));
        let attrs = BTreeMap::new();
        let path = [
            Frame {
                kind: "entry",
                attrs: &attrs,
                lang: Some("en"),
            },
            Frame {
                kind: "definition",
                attrs: &attrs,
                lang: Some("zh"),
            },
        ];
        assert_eq!(stack.own_style(&path, None).str("display"), Some("none"));
        let fonts = stack.fonts();
        assert_eq!(
            fonts,
            [
                (
                    "text".to_owned(),
                    vec![("default".to_owned(), vec!["C".to_owned()])]
                ),
                (
                    "ui".to_owned(),
                    vec![("default".to_owned(), vec!["B".to_owned()])]
                )
            ]
        );
        assert_eq!(
            stack.colors("light"),
            [("fg".to_owned(), "#000".to_owned())]
        );
    }

    #[test]
    fn font_keys_try_exact_primary_then_default_in_the_highest_layer() {
        let base = theme(
            r#"
[fonts.text]
default = ["Default"]
zh = ["Primary"]
"zh-Hans" = ["Exact"]
[fonts.ui]
default = ["UI"]
"#,
        );
        for (lang, key) in [
            (Some("zh-Hans"), "zh-Hans"),
            (Some("zh-Hant"), "zh"),
            (Some("zh-Hans-CN"), "zh"),
            (Some("en"), "default"),
            (None, "default"),
        ] {
            assert_eq!(base.font_key("text", lang), key);
        }
        let mut stack = ThemeStack::new(base);
        stack.push(theme("[fonts.text]\ndefault = [\"Override\"]"));
        assert_eq!(stack.font_key("text", Some("zh-Hans")), "default");
        assert_eq!(stack.font_key("ui", Some("zh")), "default");
        assert_eq!(stack.fonts()[1].0, "ui");
    }

    #[test]
    fn rejects_v0_and_missing_theme_versions() {
        for version in ["", "version = 0", "version = \"1\""] {
            assert!(
                Theme::parse(
                    "t",
                    &format!("[meta]\nformat = \"inkdict-theme\"\n{version}")
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_files_that_are_not_themes() {
        assert!(Theme::parse("t", "[style.a]\nsize = 1").is_err());
        assert!(
            Theme::parse(
                "t",
                "[meta]\nformat = \"inkdict-theme\"\n[style.\"a >\"]\nsize = 1"
            )
            .is_err()
        );
    }
}
