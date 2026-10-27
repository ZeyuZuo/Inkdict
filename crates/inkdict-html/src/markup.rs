//! Render tree to HTML. Every display decision is already in the tree;
//! this only prints it. `Markup::Preview` is exactly OxDic's
//! `oxdic/preview.py` (checked byte for byte); `Markup::App` adds the
//! attributes the desktop app needs (apps/desktop/README.md).

use std::cell::RefCell;
use std::collections::BTreeMap;

use inkdict_core::theme::Style;
use inkdict_core::{Generated, Icon, RenderChild, RenderNode, Tab, TextStyle};

/// Which attributes links, audio, images and anchors carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Markup {
    /// The reference renderer's HTML: `href="#entry"` links, one audio URL.
    Preview,
    /// For the app: `data-link` JSON on links, every audio and image URL,
    /// `data-anchor` numbers on anchors.
    App,
}

/// Python's `html.escape` with its default `quote=True`.
pub(crate) fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(crate) fn css_of(style: &Style, block: bool) -> String {
    let mut rules = Vec::new();
    for (name, value) in style.iter() {
        // Theme strings are tokens (`fg`, `bold`, `ui`); anything else could
        // break out of the declaration or the attribute, so it is dropped.
        if value.as_str().is_some_and(|text| !is_token(text)) {
            continue;
        }
        let rule = match name {
            "size" => format!("font-size:{value}em"),
            "weight" => format!(
                "font-weight:{}",
                if value.as_str() == Some("bold") {
                    700
                } else {
                    400
                }
            ),
            "style" => format!("font-style:{value}"),
            "case" => match value.as_str().unwrap_or_default() {
                "upper" => "text-transform:uppercase",
                "lower" => "text-transform:lowercase",
                "small-caps" => "font-variant:small-caps",
                "none" => "text-transform:none;font-variant:normal",
                _ => continue,
            }
            .to_owned(),
            "decoration" => format!(
                "text-decoration:{}",
                match value.as_str().unwrap_or_default() {
                    "strike" => "line-through",
                    "underline" => "underline",
                    "none" => "none",
                    _ => continue,
                }
            ),
            "color" | "background" => format!("{name}:var(--c-{value})"),
            "line_height" => format!("line-height:{value}"),
            "baseline" => format!(
                "vertical-align:{}",
                if value.as_str() == Some("normal") {
                    "baseline".to_owned()
                } else {
                    value.to_string()
                }
            ),
            "padding" if block => format!("padding:{value}em"),
            "padding" => format!("padding:0.12em {value}em"),
            "radius" => format!("border-radius:{value}em"),
            "space_before" => format!("margin-top:{value}em"),
            "space_after" => format!("margin-bottom:{value}em"),
            "indent" => format!("padding-left:{value}em"),
            "space_start" => format!("margin-left:{value}em"),
            "space_end" => format!("margin-right:{value}em"),
            "divider" => format!("border-bottom:1px solid var(--c-{value})"),
            "max_width" => format!("max-width:min({value}em,100%)"),
            _ => continue,
        };
        rules.push(rule);
    }
    rules.join(";")
}

fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Font declarations precede all other properties, as in preview.py.
fn styled_css(style: &Style, text: &TextStyle, block: bool, restate: bool) -> String {
    let css = css_of(style, block);
    let Some(font) = text
        .font
        .as_ref()
        .filter(|font| restate && is_token(&font.token) && is_token(&font.key))
    else {
        return css;
    };
    let key = if font.key == "default" {
        String::new()
    } else {
        format!("--{}", font.key)
    };
    let font = format!("font-family:var(--font-{}{key})", font.token);
    if css.is_empty() {
        font
    } else {
        format!("{font};{css}")
    }
}

/// SVG body of a theme icon, as in preview.py's `ICONS`.
pub(crate) fn svg_body(name: &str) -> &'static str {
    // Literal SVG bodies from preview.py's ICONS, including attribute order.
    match name {
        "speaker" => concat!(
            "<path fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linejoin=\"round\" ",
            "d=\"M2.2 6h2.6l3.7-3v10L4.8 10H2.2z\"/><path fill=\"none\" stroke=\"currentColor\" ",
            "stroke-width=\"1.5\" stroke-linecap=\"round\" d=\"M10.8 5.8a3 3 0 0 1 0 4.4M12.6 4a5.6 5.6 0 0 1 0 8\"/>"
        ),
        "key" => concat!(
            "<circle cx=\"8\" cy=\"4.6\" r=\"2.6\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\"/>",
            "<path fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" ",
            "d=\"M8 7.2v6.6M8 10.6h2.2M8 12.6h1.6\"/>"
        ),
        "arrow" => concat!(
            "<circle cx=\"8\" cy=\"8\" r=\"7\" fill=\"currentColor\"/><path fill=\"none\" stroke=\"var(--card)\" ",
            "stroke-width=\"1.7\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M4.5 8h6.2M8 5.2 10.8 8 8 10.8\"/>"
        ),
        "diamond" => "<path fill=\"currentColor\" d=\"M8 3.5 12.5 8 8 12.5 3.5 8z\"/>",
        _ => "",
    }
}

pub(crate) fn svg(name: &str) -> String {
    let body = svg_body(name);
    if body.is_empty() {
        return String::new();
    }
    format!(
        "<svg viewBox=\"0 0 16 16\" width=\"1em\" height=\"1em\" aria-hidden=\"true\" class=\"dx-svg\">{body}</svg>"
    )
}

/// Prints one tree. In `App` markup, styles become classes named after a
/// hash of their CSS, collected into one `<style>` block (inline styles
/// repeated on every node made the entry `go` 744 KB).
struct Writer {
    markup: Markup,
    classes: RefCell<BTreeMap<String, String>>,
}

impl Writer {
    /// ` class="…"{extra} style="…"`, or ` class="… zs…"{extra}` in App markup.
    fn attrs(&self, class: &str, extra: &str, css: &str) -> String {
        if self.markup == Markup::Preview {
            return format!(" class=\"{class}\"{extra} style=\"{css}\"");
        }
        if css.is_empty() {
            return format!(" class=\"{class}\"{extra}");
        }
        let mut classes = self.classes.borrow_mut();
        let name = classes
            .entry(css.to_owned())
            .or_insert_with(|| style_class(css))
            .clone();
        format!(" class=\"{class} {name}\"{extra}")
    }

    fn stylesheet(&self) -> String {
        let classes = self.classes.borrow();
        if classes.is_empty() {
            return String::new();
        }
        let rules: String = classes
            .iter()
            .map(|(css, name)| format!(".{name}{{{css}}}"))
            .collect();
        format!("<style>{rules}</style>")
    }

    fn icon(&self, icon: &Icon, element: &str, extra: &str) -> String {
        if icon.name.is_empty() || icon.name == "none" {
            return String::new();
        }
        let attrs = self.attrs(
            "dx-icon",
            extra,
            &styled_css(
                &icon.style,
                &icon.text_style,
                false,
                icon.style.get("font").is_some(),
            ),
        );
        format!("<{element}{attrs}>{}</{element}>", self.svg(&icon.name))
    }

    /// Inline SVG for Preview; in App a masked span drawn by `theme_css`
    /// (one copy per page instead of one per button). The arrow stays
    /// inline: it is cut out with the page's `--card` color.
    fn svg(&self, name: &str) -> String {
        if self.markup == Markup::App && name != "arrow" && !svg_body(name).is_empty() {
            return format!("<span class=\"dx-svg dx-i-{name}\" aria-hidden=\"true\"></span>");
        }
        svg(name)
    }

    fn pseudo(&self, generated: Option<&Generated>, which: &str, block: bool) -> String {
        let Some(generated) = generated else {
            return String::new();
        };
        let tag = if block { "div" } else { "span" };
        let attrs = self.attrs(
            &format!("dx-{which}"),
            "",
            &styled_css(
                &generated.style,
                &generated.text_style,
                block,
                generated.style.get("font").is_some(),
            ),
        );
        format!("<{tag}{attrs}>{}</{tag}>", escape(&generated.text))
    }

    fn child(&self, child: &RenderChild) -> String {
        match child {
            RenderChild::Text(text) => escape(text),
            RenderChild::Node(node) => self.node(node),
            RenderChild::Tabs(tabs) => self.tabs(tabs),
        }
    }

    fn tabs(&self, tabs: &[Tab]) -> String {
        let mut buttons = String::new();
        let mut panels = String::new();
        for (index, tab) in tabs.iter().enumerate() {
            let text = tab
                .label
                .as_ref()
                .map(|label| self.node(label))
                .unwrap_or_else(|| (index + 1).to_string());
            let accent = tab
                .accent
                .as_ref()
                .filter(|token| is_token(token))
                .map(|token| format!(" style=\"--tab-accent:var(--c-{})\"", escape(token)))
                .unwrap_or_default();
            let selected = if index == 0 { "true" } else { "false" };
            buttons.push_str(&format!("<button class=\"dx-tab\" role=\"tab\" aria-selected=\"{selected}\"{accent}>{text}</button>"));
            let hidden = if index == 0 { "" } else { " hidden" };
            panels.push_str(&format!(
                "<div class=\"dx-tab-panel\" role=\"tabpanel\"{hidden}>{}</div>",
                self.node(&tab.panel)
            ));
        }
        format!(
            "<div class=\"dx-tabs\"><div class=\"dx-tab-bar\" role=\"tablist\">{buttons}</div>{panels}</div>"
        )
    }

    fn children(&self, node: &RenderNode) -> String {
        node.children.iter().map(|item| self.child(item)).collect()
    }

    fn node(&self, node: &RenderNode) -> String {
        let markup = self.markup;
        let mut extra = String::new();
        if let Some(lang) = &node.lang {
            extra.push_str(&format!(" lang=\"{}\"", escape(lang)));
        }
        if let Some(anchor) = &node.anchor {
            extra.push_str(&format!(" id=\"{}\"", escape(&anchor.id)));
            if markup == Markup::App {
                extra.push_str(&format!(" data-anchor=\"{}\"", anchor.number));
            }
        }
        let mut css = styled_css(
            &node.style,
            &node.text,
            node.block,
            node.style.get("font").is_some() || node.lang.is_some(),
        );
        if !node.block && node.style.get("background").is_some() {
            css.push_str(";display:inline-block");
            if node.style.flag("hide_text") {
                css.push_str(concat!(
                    ";display:inline-flex;align-items:center;justify-content:center;",
                    "width:1.6em;height:1.6em;padding:0;vertical-align:middle"
                ));
            }
        }
        if node.marker.is_some() {
            css.push_str(";position:relative");
        }
        // Content may use types the registry does not know; a name that is
        // not a token must not add classes of its own (`x dx-tab`).
        let kind: String = node
            .kind
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let class = format!("dx-{kind}");
        if let Some(collapse) = &node.collapse {
            let state = if collapse.open { " open" } else { "" };
            let summary = collapse
                .summary
                .as_deref()
                .map(|summary| self.child(summary))
                .unwrap_or_default();
            let attrs = self.attrs(&class, &format!("{state}{extra}"), &css);
            return format!(
                "<details{attrs}><summary>{summary}</summary>{}</details>",
                self.children(node)
            );
        }
        let mut body = self.children(node);
        if let Some(audio) = &node.audio {
            let url = match markup {
                Markup::Preview => audio.urls.first().cloned().unwrap_or_default(),
                Markup::App => audio.urls.join(" "),
            };
            let region = audio.region.as_deref().unwrap_or_default();
            let mut extra = format!(" data-audio=\"{}\"", escape(&url));
            if markup == Markup::App {
                extra.push_str(&format!(" title=\"{}\"", escape(region)));
            }
            body.push_str(&self.icon(&audio.icon, "button", &extra));
        }
        if let Some(image) = &node.image {
            let url = image.urls.first().map(String::as_str).unwrap_or_default();
            let fallback = match markup {
                Markup::App if image.urls.len() > 1 => {
                    format!(" data-fallback=\"{}\"", escape(&image.urls[1..].join(" ")))
                }
                _ => String::new(),
            };
            body.push_str(&format!(
                "<img src=\"{}\"{fallback} alt=\"\" loading=\"lazy\" style=\"width:100%\">",
                escape(url)
            ));
        }
        if let Some(link) = &node.link {
            body = match markup {
                Markup::App => {
                    let json = serde_json::to_string(link).expect("a link serializes");
                    format!(
                        "<a href=\"#\" class=\"dx-link\" data-link=\"{}\">{body}</a>",
                        escape(&json)
                    )
                }
                Markup::Preview => {
                    let target = link.entry.as_deref().unwrap_or_default();
                    let title = if target.is_empty() { "look up" } else { target };
                    format!(
                        "<a href=\"#{}\" class=\"dx-link\" title=\"{}\">{body}</a>",
                        escape(target),
                        escape(title)
                    )
                }
            };
        }
        let lead = node
            .icon
            .as_ref()
            .map(|lead| self.icon(lead, "span", ""))
            .unwrap_or_default();
        let before = self.pseudo(node.before.as_ref(), "before", false);
        let after = self.pseudo(node.after.as_ref(), "after", false);
        let marker = node.marker.as_ref().map_or_else(String::new, |marker| {
            let mark = marker
                .icon
                .as_deref()
                .map(|name| self.svg(name))
                .unwrap_or_else(|| escape(&marker.text));
            let css = format!(
                "width:{}em;{}",
                escape(&marker.width.to_string()),
                styled_css(
                    &marker.style,
                    &marker.text_style,
                    false,
                    marker.style.get("font").is_some()
                )
            );
            format!("<span{}>{mark}</span>", self.attrs("dx-marker", "", &css))
        });
        let title = self.pseudo(node.title.as_ref(), "title", true);
        let tag = if node.block { "div" } else { "span" };
        let attrs = self.attrs(&class, &extra, &css);
        format!("<{tag}{attrs}>{title}{marker}{lead}{before}{body}{after}</{tag}>")
    }
}

/// `zs` + 32-bit FNV-1a of the CSS: stable across entries, so an entry's
/// classes never clash with another one's on the same page.
fn style_class(css: &str) -> String {
    let hash = css.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    format!("zs{hash:08x}")
}

/// HTML of a rendered node and its descendants; in `App` markup the
/// entry's `<style>` block comes first.
pub fn html(node: &RenderNode, markup: Markup) -> String {
    let writer = Writer {
        markup,
        classes: RefCell::new(BTreeMap::new()),
    };
    let body = writer.node(node);
    writer.stylesheet() + &body
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkdict_core::theme::Value;
    use inkdict_core::{Anchor, Image, Link};

    fn node(kind: &str, children: Vec<RenderChild>) -> RenderNode {
        let mut node = RenderNode::new(kind, false);
        node.children = children;
        node
    }

    #[test]
    fn app_markup_carries_links_audio_fallbacks_and_anchor_numbers() {
        let mut word = node("xword", vec![RenderChild::Text("pear".into())]);
        word.link = Some(Link {
            entry: Some("k\"2".into()),
            anchor: None,
            anchor_type: None,
            text: "pear".into(),
        });
        let app = html(&word, Markup::App);
        assert_eq!(
            app,
            "<span class=\"dx-xword\"><a href=\"#\" class=\"dx-link\" data-link=\"{&quot;entry&quot;:&quot;k\\&quot;2&quot;,&quot;text&quot;:&quot;pear&quot;}\">pear</a></span>"
        );
        let mut sense = node("sense", Vec::new());
        sense.anchor = Some(Anchor {
            id: "s1".into(),
            number: 3,
        });
        assert!(html(&sense, Markup::App).contains(" id=\"s1\" data-anchor=\"3\""));
        assert!(!html(&sense, Markup::Preview).contains("data-anchor"));
        let mut pron = node("pron", Vec::new());
        pron.audio = Some(inkdict_core::Audio {
            urls: vec!["a.mp3".into(), "b.mp3".into()],
            icon: inkdict_core::Icon {
                name: "speaker".into(),
                style: Style::default(),
                text_style: Default::default(),
            },
            region: Some("GB".into()),
        });
        let button = html(&pron, Markup::App);
        assert!(button.contains("<button class=\"dx-icon\" data-audio=\"a.mp3 b.mp3\" title=\"GB\"><span class=\"dx-svg dx-i-speaker\""), "{button}");
        assert!(html(&pron, Markup::Preview).contains("<svg viewBox"));
        let mut picture = node("image", Vec::new());
        picture.image = Some(Image {
            urls: vec!["a.jpg".into(), "b.jpg".into()],
        });
        assert!(html(&picture, Markup::App).contains("src=\"a.jpg\" data-fallback=\"b.jpg\""));
    }

    #[test]
    fn tabs_draw_labels_numbers_accents_and_selected_panels_in_both_markups() {
        let mut panel = node("panel", Vec::new());
        panel.anchor = Some(Anchor {
            id: "p1".into(),
            number: 1,
        });
        let tabs = vec![
            Tab {
                label: Some(node("label", vec![RenderChild::Text("Noun".into())])),
                accent: Some("accent".into()),
                panel,
            },
            Tab {
                label: None,
                accent: None,
                panel: node("panel", Vec::new()),
            },
        ];
        let root = node("entry", vec![RenderChild::Tabs(tabs)]);
        let preview = html(&root, Markup::Preview);
        assert_eq!(
            preview,
            concat!(
                "<span class=\"dx-entry\" style=\"\"><div class=\"dx-tabs\"><div class=\"dx-tab-bar\" role=\"tablist\">",
                "<button class=\"dx-tab\" role=\"tab\" aria-selected=\"true\" style=\"--tab-accent:var(--c-accent)\">",
                "<span class=\"dx-label\" style=\"\">Noun</span></button>",
                "<button class=\"dx-tab\" role=\"tab\" aria-selected=\"false\">2</button></div>",
                "<div class=\"dx-tab-panel\" role=\"tabpanel\"><span class=\"dx-panel\" id=\"p1\" style=\"\"></span></div>",
                "<div class=\"dx-tab-panel\" role=\"tabpanel\" hidden><span class=\"dx-panel\" style=\"\"></span></div></div></span>"
            )
        );
        let app = html(&root, Markup::App);
        assert!(app.contains("class=\"dx-tab-bar\" role=\"tablist\""));
        assert!(app.contains("role=\"tab\" aria-selected=\"false\">2</button>"));
        assert!(app.contains("class=\"dx-panel\" id=\"p1\" data-anchor=\"1\""));
        assert!(app.contains("role=\"tabpanel\" hidden>"));
    }

    #[test]
    fn restates_resolved_node_fonts_and_only_explicit_pseudo_fonts() {
        let mut child = node("body", Vec::new());
        child.lang = Some("zh-Hans".into());
        child.text.font = Some(inkdict_core::Font {
            token: "text".into(),
            key: "zh".into(),
        });
        let mut pseudo = Generated {
            text: "before".into(),
            style: Style::default(),
            text_style: child.text.clone(),
        };
        child.before = Some(pseudo.clone());
        let preview = html(&child, Markup::Preview);
        assert!(preview.starts_with(
            "<span class=\"dx-body\" lang=\"zh-Hans\" style=\"font-family:var(--font-text--zh)\">"
        ));
        assert!(preview.contains("<span class=\"dx-before\" style=\"\">before</span>"));
        pseudo.style.set("font", Value::Str("text".into()));
        child.before = Some(pseudo);
        child.lang = None;
        let preview = html(&child, Markup::Preview);
        assert!(preview.starts_with("<span class=\"dx-body\" style=\"\">"));
        assert!(
            preview
                .contains("<span class=\"dx-before\" style=\"font-family:var(--font-text--zh)\">")
        );
    }

    #[test]
    fn hostile_values_cannot_leave_their_attribute() {
        let mut bad = node("x\"><script>", vec![RenderChild::Text("<b>".into())]);
        bad.lang = Some("en\" onclick=\"x".into());
        bad.style.set("color", Value::Str("red;}<x".into()));
        bad.style.set("weight", Value::Str("bold".into()));
        bad.style.set("font", Value::Str("bad".into()));
        bad.text.font = Some(inkdict_core::Font {
            token: "bad;}<x".into(),
            key: "default".into(),
        });
        let html = html(&bad, Markup::App);
        assert!(!html.contains("<script") && !html.contains("<b>") && !html.contains("\" onclick"));
        let class = style_class("font-weight:700");
        let sheet = format!("<style>.{class}{{font-weight:700}}</style>");
        assert!(html.starts_with(&sheet), "{html}");
        // The type name keeps only token characters: no extra class.
        assert!(
            html.contains(&format!("class=\"dx-x___script_ {class}\"")),
            "{html}"
        );
    }
}
