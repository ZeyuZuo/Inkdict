//! Render tree: entry content with the theme cascade applied (spec/v0.md §4).
//!
//! Renderers (Compose, HTML) only draw it: every rule of §4.3 is decided
//! here. Behaviour follows OxDic's reference renderer `oxdic/preview.py`;
//! the oxdata tests check that its HTML is reproduced byte for byte.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::link::Link;
use crate::node::{Attr, Child, Entry, Node};
use crate::pack::Pack;
use crate::registry::{Kind, Registry, Role};
use crate::theme::{Frame, Pseudo, Style, ThemeStack, Value, python_str};

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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RenderChild {
    Text(String),
    Node(Box<RenderNode>),
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
    /// Candidate URLs, best first.
    pub urls: Vec<String>,
    pub icon: Icon,
    /// The node's `region` attribute, e.g. `GB`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Image {
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Collapse {
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Box<RenderNode>>,
}

/// Inherited text properties (spec v0 §4.2); `size` is relative to the root.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextStyle {
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
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
    fn inherit(&self, own: &Style) -> TextStyle {
        let mut next = self.clone();
        for (name, value) in own.iter() {
            let text = || value.as_str().map(String::from);
            match name {
                "size" => next.size *= value.as_f64().unwrap_or(1.0),
                "line_height" => next.line_height = value.as_f64(),
                "font" => next.font = text(),
                "weight" => next.weight = text(),
                "style" => next.style = text(),
                "case" => next.case = text(),
                "decoration" => next.decoration = text(),
                "color" => next.color = text(),
                "baseline" => next.baseline = text(),
                _ => {}
            }
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

/// Per-entry state: boxes shown at their `box_ref`, anchor numbers.
struct Context<'e> {
    boxes: HashMap<&'e str, &'e Node>,
    referenced: HashSet<&'e str>,
    anchors: HashMap<&'e str, u32>,
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
        let boxes = root
            .child_nodes()
            .filter(|node| node.kind == "box")
            .filter_map(|node| Some((node.id.as_deref()?, node)))
            .collect();
        let mut referenced = HashSet::new();
        collect_box_refs(root, &mut referenced);
        let anchors = entry
            .anchors()
            .into_iter()
            .enumerate()
            .filter_map(|(number, node)| Some((node.id.as_deref()?, number as u32)))
            .collect();
        let context = Context {
            boxes,
            referenced,
            anchors,
        };
        self.node(&context, root, &Vec::new(), None)
            .expect("an entry root is never a box_ref")
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

    /// None for a `box_ref` whose box is not in the entry.
    fn node<'e>(
        &self,
        cx: &Context<'e>,
        node: &'e Node,
        path: &Path<'e>,
        collapse: Option<String>,
    ) -> Option<RenderNode> {
        if node.kind == "box_ref" {
            let target = node.attr("box").and_then(Attr::as_str)?;
            let mut ref_path = path.clone();
            ref_path.push((self.frame(node, path), TextStyle::default()));
            let collapse = self
                .style(&ref_path, None)
                .str("collapse")
                .map(String::from);
            return self.node(cx, cx.boxes.get(target)?, path, collapse);
        }
        let parent_text = path
            .last()
            .map(|(_, text)| text.clone())
            .unwrap_or_default();
        let mut path = path.clone();
        path.push((self.frame(node, &path), TextStyle::default()));
        let style = self.style(&path, None);
        let text = parent_text.inherit(&style);
        path.last_mut().expect("just pushed").1 = text.clone();
        let block = match style.str("display") {
            Some(display) => display == "block",
            None => self.registry.kind(&node.kind) == Kind::Block,
        };
        let has = |role| self.registry.has_role(&node.kind, role);
        let mut rendered = RenderNode {
            kind: node.kind.clone(),
            block,
            lang: node.lang.clone(),
            anchor: None,
            style: Style::default(),
            text: text.clone(),
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
        };
        if has(Role::Anchor)
            && let Some(id) = node.id.as_deref()
        {
            let number = cx.anchors.get(id).copied().unwrap_or_default();
            rendered.anchor = Some(Anchor {
                id: id.to_owned(),
                number,
            });
        }
        let collapse = collapse.or_else(|| style.str("collapse").map(String::from));
        if let Some(state) = collapse.filter(|state| state != "none") {
            let summary = node
                .child_nodes()
                .next()
                .and_then(|first| self.node(cx, first, &path, None));
            let rest = node.children.get(1..).unwrap_or_default();
            rendered.children = self.children(cx, rest, &path);
            rendered.collapse = Some(Collapse {
                open: state == "open",
                summary: summary.map(Box::new),
            });
            rendered.style = style;
            return Some(rendered);
        }
        if !style.flag("hide_text") {
            rendered.children = match &node.text {
                Some(own) => vec![RenderChild::Text(own.clone())],
                None => self.children(cx, &node.children, &path),
            };
        }
        let attrs = &node.attrs;
        let audio_ref = match node.kind.as_str() {
            "audio" => attrs.get("res").or_else(|| attrs.get("audio")),
            _ if has(Role::Audio) => attrs
                .get("audio")
                .and_then(|audio| attrs.get("res").or(Some(audio))),
            _ => None,
        };
        if let Some(reference) = audio_ref.and_then(Attr::as_str) {
            let name = style.str("icon").unwrap_or("speaker").to_owned();
            rendered.audio = Some(Audio {
                urls: self.pack.resource_urls(reference),
                icon: self.icon(&path, &text, name),
                region: attrs.get("region").map(python_str),
            });
        }
        if has(Role::Image) {
            let urls = attrs
                .get("res")
                .and_then(Attr::as_str)
                .map(|r| self.pack.resource_urls(r));
            rendered.image = Some(Image {
                urls: urls.unwrap_or_default(),
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
                width: style.get("indent").cloned().unwrap_or(Value::Float(1.2)),
                text_style: text.inherit(&marker_style),
                style: marker_style,
            });
        }
        rendered.title = self.generated(&path, &text, Pseudo::Title, style.str("title"), attrs);
        rendered.style = style;
        Some(rendered)
    }

    fn children<'e>(
        &self,
        cx: &Context<'e>,
        children: &'e [Child],
        path: &Path<'e>,
    ) -> Vec<RenderChild> {
        let mut rendered = Vec::new();
        let mut previous: Option<&str> = None;
        for child in children {
            let node = match child {
                Child::Text(text) => {
                    rendered.push(RenderChild::Text(text.clone()));
                    previous = None;
                    continue;
                }
                Child::Node(node) => node,
            };
            if node.kind == "box"
                && node
                    .id
                    .as_deref()
                    .is_some_and(|id| cx.referenced.contains(id))
            {
                continue;
            }
            let mut child_path = path.clone();
            child_path.push((self.frame(node, path), TextStyle::default()));
            let style = self.style(&child_path, None);
            if style.str("display") == Some("none") {
                continue;
            }
            if previous == Some(node.kind.as_str())
                && let Some(join) = style.str("join").filter(|join| !join.is_empty())
            {
                rendered.push(RenderChild::Text(join.to_owned()));
            }
            if let Some(node) = self.node(cx, node, path, None) {
                rendered.push(RenderChild::Node(Box::new(node)));
            }
            previous = Some(&node.kind);
        }
        rendered
    }

    fn icon(&self, path: &Path, text: &TextStyle, name: String) -> Icon {
        let style = self.style(path, Some(Pseudo::Icon));
        Icon {
            name,
            text_style: text.inherit(&style),
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
            text_style: text.inherit(&style),
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

fn collect_box_refs<'e>(node: &'e Node, found: &mut HashSet<&'e str>) {
    if node.kind == "box_ref"
        && let Some(target) = node.attr("box").and_then(Attr::as_str)
    {
        found.insert(target);
    }
    for child in node.child_nodes() {
        collect_box_refs(child, found);
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
[types.entry]
kind = "block"
children = ["sense", "box", "box_ref"]
roles = ["anchor"]
[types.sense]
kind = "block"
attrs = { n = "int?" }
children = ["#text", "definition", "xword"]
roles = ["anchor"]
[types.definition]
kind = "block"
children = ["#text"]
[types.xword]
kind = "inline"
children = ["#text"]
roles = ["link"]
[types.box]
kind = "block"
children = ["definition"]
roles = ["anchor"]
[types.box_ref]
kind = "block"
attrs = { box = "ref" }
"##;
    const THEME: &str = r#"
[meta]
format = "zdict-theme"
[style.entry]
size = 1.25
[style.sense]
marker = "{n}."
indent = 2
[style."sense > definition"]
size = 0.8
weight = "bold"
[style.definition]
join = " | "
[style.box]
collapse = "closed"
[style.box_ref]
collapse = "open"
"#;
    const ENTRY: &str = r#"{"type":"entry","id":"e","lang":"en","children":[{"type":"sense","id":"s1","attrs":{"n":1},"children":[{"type":"definition","text":"fruit"},{"type":"definition","lang":"zh-Hans","text":"水果"},{"type":"xword","text":"pear"}]},{"type":"box_ref","attrs":{"box":"b"}},{"type":"box","id":"b","children":[{"type":"definition","text":"title"},{"type":"definition","text":"body"}]}]}"#;

    fn render(overlay: Option<&str>) -> RenderNode {
        let test = TestPack::new();
        test.file("types.toml", TYPES.as_bytes())
            .file("themes/default.toml", THEME.as_bytes())
            .entry(1, "apple", None, ENTRY);
        let pack = test.open();
        let registry = pack.registry().unwrap();
        let mut stack = ThemeStack::new(pack.theme("default").unwrap());
        if let Some(style) = overlay {
            stack.push(
                Theme::parse(
                    "overlay",
                    &format!("[meta]\nformat = \"zdict-theme\"\n{style}"),
                )
                .unwrap(),
            );
        }
        let entry = pack.entry("e").unwrap();
        pack.renderer(&registry, &stack).render(&entry)
    }

    fn node(child: &RenderChild) -> &RenderNode {
        match child {
            RenderChild::Node(node) => node,
            RenderChild::Text(text) => panic!("text {text:?}"),
        }
    }

    #[test]
    fn inherits_text_and_generates_markers_joins_links_and_boxes() {
        let root = render(None);
        assert_eq!(
            root.anchor,
            Some(Anchor {
                id: "e".into(),
                number: 0
            })
        );
        let sense = node(&root.children[0]);
        let marker = sense.marker.as_ref().unwrap();
        assert_eq!(
            (marker.text.as_str(), marker.width.to_string()),
            ("1.", "2".into())
        );
        let definition = node(&sense.children[0]);
        assert_eq!(definition.text.size, 1.25 * 0.8);
        assert_eq!(definition.text.weight.as_deref(), Some("bold"));
        assert_eq!(sense.children[1], RenderChild::Text(" | ".into()));
        let xword = node(&sense.children[3]);
        assert_eq!(xword.link.as_ref().unwrap().text, "pear");
        // The box shows at its box_ref, open there, and not again at the end.
        assert_eq!(root.children.len(), 2);
        let shown = node(&root.children[1]);
        assert_eq!(shown.kind, "box");
        let collapse = shown.collapse.as_ref().unwrap();
        assert!(collapse.open);
        assert_eq!(
            collapse.summary.as_ref().unwrap().children,
            [RenderChild::Text("title".into())]
        );
        assert_eq!(shown.children.len(), 1);
    }

    #[test]
    fn overlays_hide_and_restyle_over_the_base_theme() {
        let root = render(Some(
            "[style.\"*[lang=zh]\"]\ndisplay = \"none\"\n[style.definition]\nsize = 2\n",
        ));
        let sense = node(&root.children[0]);
        let kinds: Vec<_> = sense
            .children
            .iter()
            .map(|c| node(c).kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            ["definition", "xword"],
            "Chinese definition and its join are gone"
        );
        assert_eq!(node(&sense.children[0]).text.size, 1.25 * 2.0);
    }

    #[test]
    fn interpolates_attributes_or_nothing() {
        let attrs = std::collections::BTreeMap::from([("n".to_owned(), Attr::Int(3))]);
        assert_eq!(interpolate("{n}.", &attrs), "3.");
        assert_eq!(interpolate("({x})", &attrs), "");
        assert_eq!(interpolate("{N} {{n}}", &attrs), "{N} {3}");
    }
}
