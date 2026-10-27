//! Render tree: entry content with the theme cascade applied (spec/v1.md §4).
//!
//! Renderers (Compose, HTML) only draw it: the rules of §4.2–4.4 are decided
//! here. Behaviour follows OxDic's reference renderer `oxdic/preview.py`;
//! the oxdata tests check that its HTML is reproduced byte for byte.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::link::Link;
use crate::node::{Attr, Child, Entry, Node};
use crate::pack::Pack;
use crate::registry::{Kind, Registry, Role};
use crate::theme::{Frame, Pseudo, Selector, Style, ThemeStack, Value, python_str};

/// A node as it is drawn.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderNode {
    #[serde(rename = "type")]
    pub kind: String,
    pub block: bool,
    /// `lang` written on this node in the content (not the inherited one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Set on anchor-role nodes with an id: the scroll target of links and
    /// search hits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// Properties the theme rules set on this node, in cascade order.
    pub style: Style,
    /// Text properties after inheritance (for renderers without CSS).
    pub text: TextStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Generated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<Marker>,
    /// Icon drawn before the content (not for audio nodes, see `audio`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Generated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Generated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
    /// Collapsible block: `summary` stays visible, `children` fold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapse: Option<Collapse>,
    pub children: Vec<RenderChild>,
}

impl RenderNode {
    /// An empty node of this type: no style, generated parts or children.
    pub fn new(kind: &str, block: bool) -> Self {
        RenderNode {
            kind: kind.to_owned(),
            block,
            lang: None,
            anchor: None,
            style: Style::default(),
            text: TextStyle::default(),
            title: None,
            marker: None,
            icon: None,
            before: None,
            after: None,
            audio: None,
            image: None,
            link: None,
            collapse: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RenderChild {
    Text(String),
    Node(Box<RenderNode>),
    Tabs(Vec<Tab>),
}

/// One panel in a group of adjacent tab siblings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tab {
    /// Descendant drawn in the bar and omitted from the panel; otherwise a number.
    pub label: Option<RenderNode>,
    /// The theme's `tab_accent` color token.
    pub accent: Option<String>,
    pub panel: RenderNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Anchor {
    pub id: String,
    pub number: u32,
}

/// Text a theme generates (`before`, `after`, `title`), with its own style.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Generated {
    pub text: String,
    pub style: Style,
    pub text_style: TextStyle,
}

/// A hanging marker in the block's left indent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Marker {
    pub text: String,
    /// Icon drawn instead of the text, when the `::marker` style sets one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Width of the indent it hangs in, em (the node's `indent`, default 1.2).
    pub width: Value,
    pub style: Style,
    pub text_style: TextStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Icon {
    /// `speaker`, `key`, `arrow`, `diamond`, or `none` for a hidden play button.
    pub name: String,
    pub style: Style,
    pub text_style: TextStyle,
}

/// A play control.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Audio {
    /// Online candidate URLs, best first.
    pub urls: Vec<String>,
    /// The reference (`kind:key`) when the pack stores the file itself;
    /// the app serves it and tries it before `urls`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded: Option<String>,
    pub icon: Icon,
    /// The node's `region` attribute, e.g. `GB`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Image {
    pub urls: Vec<String>,
    /// As for [`Audio::embedded`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Collapse {
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Box<RenderChild>>,
}

/// A font token and the language key selected from its family stacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Font {
    pub token: String,
    pub key: String,
}

/// Inherited text properties (spec v1 §4.2); `size` is relative to the root.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextStyle {
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            size: 1.0,
            font: None,
            weight: None,
            style: None,
            case: None,
            decoration: None,
            color: None,
            baseline: None,
            line_height: None,
        }
    }
}

impl TextStyle {
    /// This style with a node's own text properties applied.
    fn inherit(&self, own: &Style, themes: &ThemeStack, lang: Option<&str>) -> TextStyle {
        let mut next = self.clone();
        for (name, value) in own.iter() {
            let text = || value.as_str().map(String::from);
            match name {
                "size" => next.size *= value.as_f64().unwrap_or(1.0),
                "line_height" => next.line_height = value.as_f64(),
                "font" => {
                    next.font = text().map(|token| Font {
                        key: String::new(),
                        token,
                    })
                }
                "weight" => next.weight = text(),
                "style" => next.style = text(),
                "case" => next.case = text(),
                "decoration" => next.decoration = text(),
                "color" => next.color = text(),
                "baseline" => next.baseline = text(),
                _ => {}
            }
        }
        if let Some(font) = &mut next.font {
            font.key = themes.font_key(&font.token, lang).to_owned();
        }
        next
    }
}

/// Renders entries of one pack with one theme stack.
pub struct Renderer<'a> {
    pack: &'a Pack,
    registry: &'a Registry,
    themes: &'a ThemeStack,
}

/// Per-entry anchor numbers and labels already moved into a tab bar.
struct Context {
    anchors: HashMap<*const Node, u32>,
    hidden_labels: RefCell<HashSet<*const Node>>,
}

type Path<'e> = Vec<(Frame<'e>, TextStyle)>;

impl<'a> Renderer<'a> {
    pub fn new(pack: &'a Pack, registry: &'a Registry, themes: &'a ThemeStack) -> Self {
        Renderer {
            pack,
            registry,
            themes,
        }
    }

    pub fn render(&self, entry: &Entry) -> RenderNode {
        let root = &entry.root;
        // By node, not by id: with duplicate ids each node keeps its own
        // number, the one lookup rows and links use (spec/pack-v1.md §3.4).
        let anchors = entry
            .anchors()
            .into_iter()
            .enumerate()
            .map(|(number, node)| (std::ptr::from_ref(node), number as u32))
            .collect();
        let context = Context {
            anchors,
            hidden_labels: RefCell::new(HashSet::new()),
        };
        self.node(&context, root, &Vec::new())
    }

    fn frame<'e>(&self, node: &'e Node, path: &Path<'e>) -> Frame<'e> {
        let inherited = path.last().and_then(|(frame, _)| frame.lang);
        Frame {
            kind: &node.kind,
            attrs: &node.attrs,
            lang: node.lang.as_deref().or(inherited),
        }
    }

    fn style(&self, path: &Path, pseudo: Option<Pseudo>) -> Style {
        let frames: Vec<Frame> = path.iter().map(|(frame, _)| *frame).collect();
        self.themes.own_style(&frames, pseudo)
    }

    fn node<'e>(&self, cx: &Context, node: &'e Node, path: &Path<'e>) -> RenderNode {
        let parent_text = path
            .last()
            .map(|(_, text)| text.clone())
            .unwrap_or_default();
        let mut path = path.clone();
        path.push((self.frame(node, &path), TextStyle::default()));
        let style = self.style(&path, None);
        let lang = path.last().and_then(|(frame, _)| frame.lang);
        let text = parent_text.inherit(&style, self.themes, lang);
        path.last_mut().expect("just pushed").1 = text.clone();
        let block = match style.str("display") {
            Some(display) => display == "block",
            None => self.registry.kind(&node.kind) == Kind::Block,
        };
        let has = |role| self.registry.has_role(&node.kind, role);
        let mut rendered = RenderNode::new(&node.kind, block);
        rendered.lang = node.lang.clone();
        rendered.text = text.clone();
        if has(Role::Anchor)
            && let Some(id) = node.id.as_deref()
        {
            let number = cx
                .anchors
                .get(&std::ptr::from_ref(node))
                .copied()
                .unwrap_or_default();
            rendered.anchor = Some(Anchor {
                id: id.to_owned(),
                number,
            });
        }
        if let Some(state) = style.str("collapse").filter(|state| *state != "none") {
            // The first child, string or node, stays visible (spec v1 §4.2);
            // a hidden first node leaves no summary. The reference renderer
            // takes the first node and drops a leading string instead.
            let (summary, rest) = match &node.text {
                Some(own) => (Some(RenderChild::Text(own.clone())), Vec::new()),
                None => {
                    let split = node.children.len().min(1);
                    let first = self.children(cx, &node.children[..split], &path);
                    (
                        first.into_iter().next(),
                        self.children(cx, &node.children[split..], &path),
                    )
                }
            };
            rendered.children = rest;
            rendered.collapse = Some(Collapse {
                open: state == "open",
                summary: summary.map(Box::new),
            });
        } else if !style.flag("hide_text") {
            rendered.children = match &node.text {
                Some(own) => vec![RenderChild::Text(own.clone())],
                None => self.children(cx, &node.children, &path),
            };
        }
        let attrs = &node.attrs;
        let resource = self
            .registry
            .res_attrs(&node.kind)
            .iter()
            .find_map(|name| attrs.get(name))
            .and_then(Attr::as_str)
            .filter(|value| !value.is_empty());
        let audio_ref = if has(Role::Audio) { resource } else { None };
        if let Some(reference) = audio_ref {
            let name = style.str("icon").unwrap_or("speaker").to_owned();
            rendered.audio = Some(Audio {
                urls: self.pack.resource_urls(reference),
                embedded: self.embedded(reference),
                icon: self.icon(&path, &text, name),
                region: attrs.get("region").map(python_str),
            });
        }
        if has(Role::Image)
            && let Some(reference) = resource
        {
            rendered.image = Some(Image {
                urls: self.pack.resource_urls(reference),
                embedded: self.embedded(reference),
            });
        }
        if has(Role::Link) {
            rendered.link = Some(Link::of(node));
        }
        if !has(Role::Audio) {
            rendered.icon = style
                .str("icon")
                .filter(|name| *name != "none")
                .map(|name| self.icon(&path, &text, name.to_owned()));
        }
        rendered.before = self.generated(&path, &text, Pseudo::Before, style.str("before"), attrs);
        rendered.after = self.generated(&path, &text, Pseudo::After, style.str("after"), attrs);
        let marker = interpolate(style.str("marker").unwrap_or_default(), attrs);
        if block && !marker.is_empty() {
            let marker_style = self.style(&path, Some(Pseudo::Marker));
            rendered.marker = Some(Marker {
                text: marker,
                icon: marker_style
                    .str("icon")
                    .filter(|n| *n != "none")
                    .map(String::from),
                // Only a number: the width ends up inside CSS.
                width: style
                    .get("indent")
                    .filter(|indent| indent.as_f64().is_some())
                    .cloned()
                    .unwrap_or(Value::Float(1.2)),
                text_style: text.inherit(&marker_style, self.themes, lang),
                style: marker_style,
            });
        }
        rendered.title = self.generated(&path, &text, Pseudo::Title, style.str("title"), attrs);
        rendered.style = style;
        rendered
    }

    fn children<'e>(
        &self,
        cx: &Context,
        children: &'e [Child],
        path: &Path<'e>,
    ) -> Vec<RenderChild> {
        let mut rendered = Vec::new();
        let mut previous: Option<&str> = None;
        let mut tabs = Vec::new();
        for child in children {
            let node = match child {
                Child::Text(text) => {
                    if !tabs.is_empty() && text.trim().is_empty() {
                        continue;
                    }
                    self.tab_group(cx, &mut tabs, path, &mut rendered);
                    rendered.push(RenderChild::Text(text.clone()));
                    previous = None;
                    continue;
                }
                Child::Node(node) => node,
            };
            if cx
                .hidden_labels
                .borrow()
                .contains(&std::ptr::from_ref(node))
            {
                continue;
            }
            let mut child_path = path.clone();
            child_path.push((self.frame(node, path), TextStyle::default()));
            let style = self.style(&child_path, None);
            if style.str("display") == Some("none") {
                continue;
            }
            if style.flag("tabs") {
                tabs.push((node, style));
                previous = Some(&node.kind);
                continue;
            }
            self.tab_group(cx, &mut tabs, path, &mut rendered);
            if previous == Some(node.kind.as_str())
                && let Some(join) = style.str("join").filter(|join| !join.is_empty())
            {
                rendered.push(RenderChild::Text(join.to_owned()));
            }
            rendered.push(RenderChild::Node(Box::new(self.node(cx, node, path))));
            previous = Some(&node.kind);
        }
        self.tab_group(cx, &mut tabs, path, &mut rendered);
        rendered
    }

    fn tab_group<'e>(
        &self,
        cx: &Context,
        tabs: &mut Vec<(&'e Node, Style)>,
        path: &Path<'e>,
        rendered: &mut Vec<RenderChild>,
    ) {
        if tabs.len() < 2 {
            for (node, _) in tabs.drain(..) {
                rendered.push(RenderChild::Node(Box::new(self.node(cx, node, path))));
            }
            return;
        }
        let mut group = Vec::new();
        for (node, style) in tabs.drain(..) {
            // A label lives in the bar: it inherits the group's text style,
            // while selector matching retains its original ancestor frames.
            let inherited = path
                .last()
                .map(|(_, text)| text.clone())
                .unwrap_or_default();
            let mut child_path = path.clone();
            child_path.push((self.frame(node, path), inherited));
            let label = style
                .str("tab_label")
                .and_then(|text| Selector::parse(text).ok())
                .and_then(|selector| self.tab_label(node, &child_path, &selector));
            let label = label.map(|(label, parent)| {
                let shown = self.node(cx, label, &parent);
                cx.hidden_labels
                    .borrow_mut()
                    .insert(std::ptr::from_ref(label));
                shown
            });
            group.push(Tab {
                label,
                accent: style.str("tab_accent").map(String::from),
                panel: self.node(cx, node, path),
            });
        }
        rendered.push(RenderChild::Tabs(group));
    }

    fn tab_label<'e>(
        &self,
        node: &'e Node,
        path: &Path<'e>,
        selector: &Selector,
    ) -> Option<(&'e Node, Path<'e>)> {
        let inherited = path
            .last()
            .map(|(_, text)| text.clone())
            .unwrap_or_default();
        let mut stack: Vec<_> = node
            .child_nodes()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|child| (child, path.clone()))
            .collect();
        while let Some((item, parent)) = stack.pop() {
            let mut item_path = parent.clone();
            item_path.push((self.frame(item, &parent), inherited.clone()));
            let frames: Vec<_> = item_path.iter().map(|(frame, _)| *frame).collect();
            if selector.matches(&frames) {
                return Some((item, parent));
            }
            stack.extend(
                item.child_nodes()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|child| (child, item_path.clone())),
            );
        }
        None
    }

    fn embedded(&self, reference: &str) -> Option<String> {
        self.pack
            .has_resource(reference)
            .then(|| reference.to_owned())
    }

    fn icon(&self, path: &Path, text: &TextStyle, name: String) -> Icon {
        let style = self.style(path, Some(Pseudo::Icon));
        Icon {
            name,
            text_style: text.inherit(
                &style,
                self.themes,
                path.last().and_then(|(frame, _)| frame.lang),
            ),
            style,
        }
    }

    fn generated(
        &self,
        path: &Path,
        text: &TextStyle,
        pseudo: Pseudo,
        template: Option<&str>,
        attrs: &std::collections::BTreeMap<String, Attr>,
    ) -> Option<Generated> {
        let filled = interpolate(template?, attrs);
        if filled.is_empty() {
            return None;
        }
        let style = self.style(path, Some(pseudo));
        Some(Generated {
            text: filled,
            text_style: text.inherit(
                &style,
                self.themes,
                path.last().and_then(|(frame, _)| frame.lang),
            ),
            style,
        })
    }
}

impl Pack {
    /// Render an entry with the pack's registry and a theme stack.
    pub fn renderer<'a>(&'a self, registry: &'a Registry, themes: &'a ThemeStack) -> Renderer<'a> {
        Renderer::new(self, registry, themes)
    }
}

/// Fill `{attr}` fields; a template naming a missing attribute gives "".
pub fn interpolate(template: &str, attrs: &std::collections::BTreeMap<String, Attr>) -> String {
    let mut filled = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let field = after.find('}').map(|close| &after[..close]).filter(|name| {
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        });
        match field {
            Some(name) => {
                let Some(value) = attrs.get(name) else {
                    return String::new();
                };
                filled.push_str(&rest[..open]);
                filled.push_str(&python_str(value));
                rest = &after[name.len() + 1..];
            }
            None => {
                filled.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    filled.push_str(rest);
    filled
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_pack::TestPack;
    use crate::theme::Theme;

    const TYPES: &str = r##"
[meta]
format = "inkdict-types"
version = 1
[types.entry]
kind = "block"
roles = ["anchor"]
[types.panel]
kind = "block"
roles = ["anchor"]
attrs = { plain = "bool?" }
[types.head]
kind = "block"
[types.label]
kind = "inline"
[types.body]
kind = "block"
[types.sound]
kind = "inline"
roles = ["audio"]
attrs = { z_sound = "res?", a_fallback = "res?", audio = "string?" }
[types.picture]
kind = "block"
roles = ["image"]
attrs = { source = "res?" }
"##;

    fn render(theme: &str, children: &str) -> RenderNode {
        let test = TestPack::new();
        test.sql("INSERT INTO resource_sources VALUES ('a', 1, 'https://audio/{key}'), ('i', 1, 'https://image/{key}')");
        let root = serde_json::from_str(&format!(
            r#"{{"type":"entry","id":"e","lang":"en","children":[{children}]}}"#
        ))
        .unwrap();
        let entry = Entry {
            id: 1,
            key: "e".into(),
            headword: "e".into(),
            homograph: None,
            root,
        };
        let pack = test.open();
        let registry = Registry::parse(TYPES).unwrap();
        let themes = ThemeStack::new(
            Theme::parse(
                "t",
                &format!("[meta]\nformat = \"inkdict-theme\"\nversion = 1\n{theme}"),
            )
            .unwrap(),
        );
        pack.renderer(&registry, &themes).render(&entry)
    }

    fn node(child: &RenderChild) -> &RenderNode {
        match child {
            RenderChild::Node(node) => node,
            other => panic!("expected node: {other:?}"),
        }
    }

    const TABS: &str = r#"
[style.panel]
tabs = true
tab_label = "head > label"
tab_accent = "accent"
[style."panel > head > label"]
weight = "bold"
"#;
    const TWO: &str = r#"{"type":"panel","id":"p1","children":[{"type":"head","children":[{"type":"label","text":"Noun"}]},{"type":"body","text":"one"}]}," ",{"type":"panel","id":"p2","children":[{"type":"head","children":[{"type":"label","text":"Verb"}]},{"type":"body","text":"two"}]}"#;

    #[test]
    fn groups_tabs_and_moves_the_first_matching_labels_out_of_panels() {
        let root = render(TABS, TWO);
        let [RenderChild::Tabs(tabs)] = root.children.as_slice() else {
            panic!("{root:?}")
        };
        assert_eq!(tabs.len(), 2);
        for (tab, label) in tabs.iter().zip(["Noun", "Verb"]) {
            let shown = tab.label.as_ref().unwrap();
            assert_eq!(shown.children, [RenderChild::Text(label.into())]);
            assert_eq!(shown.text.weight.as_deref(), Some("bold"));
            assert_eq!(tab.accent.as_deref(), Some("accent"));
            assert!(node(&tab.panel.children[0]).children.is_empty());
        }
        assert_eq!(tabs[1].panel.anchor.as_ref().unwrap().number, 2);
        let json = serde_json::to_value(&root).unwrap();
        assert_eq!(json["children"][0][1]["label"]["type"], "label");
    }

    #[test]
    fn a_single_tab_retains_its_label_and_text_breaks_a_group() {
        let one = r#"{"type":"panel","children":[{"type":"head","children":[{"type":"label","text":"Noun"}]}]}"#;
        let root = render(TABS, one);
        assert_eq!(
            node(&node(&node(&root.children[0]).children[0]).children[0]).kind,
            "label"
        );
        let root = render(TABS, &format!("{one},\"separate\",{one}"));
        assert_eq!(root.children.len(), 3);
        assert!(matches!(root.children[0], RenderChild::Node(_)));
        assert!(matches!(root.children[2], RenderChild::Node(_)));
    }

    #[test]
    fn missing_labels_use_numbers_and_hidden_siblings_do_not_break_tabs() {
        let root = render(
            &format!("{TABS}\n[style.body]\ndisplay = \"none\""),
            r#"{"type":"panel"},"\n\t",{"type":"body"},{"type":"panel"}"#,
        );
        let [RenderChild::Tabs(tabs)] = root.children.as_slice() else {
            panic!("{root:?}")
        };
        assert!(tabs.iter().all(|tab| tab.label.is_none()));
    }

    #[test]
    fn joins_after_tabs_follow_the_last_sibling_type() {
        let theme = format!("{TABS}\n[style.\"panel[plain]\"]\ntabs = false\njoin = \" / \"");
        let root = render(
            &theme,
            r#"{"type":"panel"},{"type":"panel"},{"type":"panel","attrs":{"plain":true}}"#,
        );
        assert!(matches!(root.children[0], RenderChild::Tabs(_)));
        assert_eq!(root.children[1], RenderChild::Text(" / ".into()));
        assert!(matches!(root.children[2], RenderChild::Node(_)));
        let root = render(
            &format!("{TABS}\n[style.\"panel[plain]\"]\ntabs = false"),
            r#"{"type":"panel","attrs":{"plain":true}},{"type":"panel","attrs":{"plain":true}}"#,
        );
        assert!(
            root.children
                .iter()
                .all(|child| matches!(child, RenderChild::Node(_)))
        );
    }

    #[test]
    fn inherited_fonts_and_generated_parts_resolve_language_keys() {
        let theme = r#"
[fonts.text]
default = ["Default"]
zh = ["Primary"]
"zh-Hans" = ["Exact"]
[fonts.ui]
default = ["UI"]
zh = ["Chinese UI"]
[style.entry]
font = "text"
size = 1.25
[style.body]
before = "("
after = ")"
title = "Title"
marker = "1."
icon = "key"
[style."body::before"]
font = "ui"
[style."body::after"]
font = "ui"
[style."body::title"]
font = "ui"
[style."body::marker"]
font = "ui"
[style."body::icon"]
font = "ui"
"#;
        let root = render(
            theme,
            r#"{"type":"body","lang":"zh-Hans","children":[{"type":"mystery","text":"文本"}]},{"type":"body","lang":"zh-Hant","text":"字"}"#,
        );
        let body = node(&root.children[0]);
        assert_eq!(
            body.text.font,
            Some(Font {
                token: "text".into(),
                key: "zh-Hans".into()
            })
        );
        assert_eq!(node(&body.children[0]).text.font, body.text.font);
        assert!(!node(&body.children[0]).block);
        assert_eq!(body.text.size, 1.25);
        let generated = [
            &body.before.as_ref().unwrap().text_style,
            &body.after.as_ref().unwrap().text_style,
            &body.title.as_ref().unwrap().text_style,
            &body.marker.as_ref().unwrap().text_style,
            &body.icon.as_ref().unwrap().text_style,
        ];
        for text in generated {
            assert_eq!(
                text.font,
                Some(Font {
                    token: "ui".into(),
                    key: "zh".into()
                })
            );
        }
        assert_eq!(
            node(&root.children[1]).text.font.as_ref().unwrap().key,
            "zh"
        );
        assert_eq!(root.text.font.as_ref().unwrap().key, "default");
    }

    #[test]
    fn media_use_the_first_present_declared_resource_attribute() {
        let root = render(
            "",
            r#"{"type":"sound","attrs":{"audio":"wrong","z_sound":"a:first","a_fallback":"a:second"}},{"type":"sound","attrs":{"a_fallback":"a:second"}},{"type":"sound","attrs":{"audio":"wrong"}},{"type":"picture","attrs":{"source":"i:picture"}},{"type":"picture"},{"type":"audio","attrs":{"res":"a:unknown"},"text":"unknown"}"#,
        );
        assert_eq!(
            node(&root.children[0]).audio.as_ref().unwrap().urls,
            ["https://audio/first"]
        );
        assert_eq!(
            node(&root.children[1]).audio.as_ref().unwrap().urls,
            ["https://audio/second"]
        );
        assert!(node(&root.children[2]).audio.is_none());
        assert_eq!(
            node(&root.children[3]).image.as_ref().unwrap().urls,
            ["https://image/picture"]
        );
        assert!(node(&root.children[4]).image.is_none());
        assert!(node(&root.children[5]).audio.is_none());
        assert_eq!(
            node(&root.children[5]).children,
            [RenderChild::Text("unknown".into())]
        );
    }

    #[test]
    fn nested_collapses_render_each_node_once() {
        let mut children = String::from(r#"{"type":"body","text":"leaf"}"#);
        for _ in 0..24 {
            children = format!(r#"{{"type":"panel","children":[{children},{{"type":"body"}}]}}"#);
        }
        let started = std::time::Instant::now();
        render("[style.panel]\ncollapse = \"closed\"", &children);
        // Rendering each collapse twice would take 2^24 renders: far beyond this.
        assert!(started.elapsed().as_millis() < 3000, "24 nested collapses");
    }

    #[test]
    fn a_collapse_keeps_the_labels_of_tabs_it_splits() {
        let theme = format!("{TABS}\n[style.entry]\ncollapse = \"open\"");
        let root = render(&theme, TWO);
        let summary = root.collapse.as_ref().unwrap().summary.as_deref().unwrap();
        let first_head = node(&node(summary).children[0]);
        assert_eq!(
            node(&first_head.children[0]).children,
            [RenderChild::Text("Noun".into())]
        );
        // The blank string between the panels stays with the rest.
        assert_eq!(root.children[0], RenderChild::Text(" ".into()));
        let second_head = node(&node(&root.children[1]).children[0]);
        assert_eq!(
            node(&second_head.children[0]).children,
            [RenderChild::Text("Verb".into())]
        );
    }

    #[test]
    fn duplicate_ids_keep_their_document_order_numbers() {
        let root = render(
            "",
            r#"{"type":"panel","id":"x"},{"type":"panel","id":"y"},{"type":"panel","id":"x"}"#,
        );
        let numbers: Vec<_> = root
            .children
            .iter()
            .map(|child| node(child).anchor.as_ref().unwrap().number)
            .collect();
        assert_eq!(numbers, [1, 2, 3]);
    }

    #[test]
    fn collapses_a_leading_string_or_own_text_and_skips_hidden_summaries() {
        let theme = "[style.panel]\ncollapse = \"closed\"\n[style.label]\ndisplay = \"none\"";
        let root = render(
            theme,
            r#"{"type":"panel","children":["lead",{"type":"body","text":"rest"}]},{"type":"panel","text":"own"},{"type":"panel","children":[{"type":"label","text":"x"},"tail"]}"#,
        );
        let summary = |n: usize| {
            node(&root.children[n])
                .collapse
                .as_ref()
                .unwrap()
                .summary
                .clone()
        };
        assert_eq!(
            summary(0).as_deref(),
            Some(&RenderChild::Text("lead".into()))
        );
        assert_eq!(node(&node(&root.children[0]).children[0]).kind, "body");
        assert_eq!(
            summary(1).as_deref(),
            Some(&RenderChild::Text("own".into()))
        );
        assert!(node(&root.children[1]).children.is_empty());
        assert_eq!(summary(2), None);
        assert_eq!(
            node(&root.children[2]).children,
            [RenderChild::Text("tail".into())]
        );
    }

    #[test]
    fn collapses_the_first_node_and_preserves_the_remaining_children() {
        let root = render(
            "[style.panel]\ncollapse = \"open\"",
            r#"{"type":"panel","children":[{"type":"label","text":"summary"},{"type":"body","text":"rest"}]}"#,
        );
        let panel = node(&root.children[0]);
        let collapse = panel.collapse.as_ref().unwrap();
        assert!(collapse.open);
        assert_eq!(
            node(collapse.summary.as_deref().unwrap()).children,
            [RenderChild::Text("summary".into())]
        );
        assert_eq!(node(&panel.children[0]).kind, "body");
    }

    #[test]
    fn interpolates_attributes_or_nothing() {
        let attrs = std::collections::BTreeMap::from([("n".to_owned(), Attr::Int(3))]);
        assert_eq!(interpolate("{n}.", &attrs), "3.");
        assert_eq!(interpolate("({x})", &attrs), "");
        assert_eq!(interpolate("{N} {{n}}", &attrs), "{N} {3}");
    }
}
