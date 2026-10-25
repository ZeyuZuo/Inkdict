//! Render tree to HTML. Every display decision is already in the tree;
//! this only prints it. `Markup::Preview` is exactly OxDic's
//! `oxdic/preview.py` (checked byte for byte); `Markup::App` adds the
//! attributes the desktop app needs (apps/desktop/README.md).

use zdict_core::theme::Style;
use zdict_core::{Generated, Icon, RenderChild, RenderNode};

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
        if value.as_str().is_some_and(|text| {
            !text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        }) {
            continue;
        }
        let rule = match name {
            "font" => format!("font-family:var(--font-{value})"),
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

pub(crate) fn svg(name: &str) -> String {
    // Literal SVG bodies from preview.py's ICONS, including attribute order.
    let body = match name {
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
        _ => return String::new(),
    };
    format!(
        "<svg viewBox=\"0 0 16 16\" width=\"1em\" height=\"1em\" aria-hidden=\"true\" class=\"dx-svg\">{body}</svg>"
    )
}

fn icon(icon: &Icon, element: &str, extra: &str) -> String {
    if icon.name.is_empty() || icon.name == "none" {
        return String::new();
    }
    format!(
        "<{element} class=\"dx-icon\"{extra} style=\"{}\">{}</{element}>",
        css_of(&icon.style, false),
        svg(&icon.name)
    )
}

fn pseudo(generated: Option<&Generated>, which: &str, block: bool) -> String {
    let Some(generated) = generated else {
        return String::new();
    };
    let tag = if block { "div" } else { "span" };
    format!(
        "<{tag} class=\"dx-{which}\" style=\"{}\">{}</{tag}>",
        css_of(&generated.style, block),
        escape(&generated.text)
    )
}

fn child(child: &RenderChild, markup: Markup) -> String {
    match child {
        RenderChild::Text(text) => escape(text),
        RenderChild::Node(node) => html(node, markup),
    }
}

fn children(node: &RenderNode, markup: Markup) -> String {
    node.children
        .iter()
        .map(|item| child(item, markup))
        .collect()
}

/// HTML of a rendered node and its descendants.
pub fn html(node: &RenderNode, markup: Markup) -> String {
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
    let mut css = css_of(&node.style, node.block);
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
    if let Some(collapse) = &node.collapse {
        let state = if collapse.open { " open" } else { "" };
        let summary = collapse
            .summary
            .as_deref()
            .map(|summary| child(summary, markup))
            .unwrap_or_default();
        return format!(
            "<details class=\"dx-{}\"{state}{extra} style=\"{css}\"><summary>{summary}</summary>{}</details>",
            escape(&node.kind),
            children(node, markup)
        );
    }
    let mut body = children(node, markup);
    if let Some(audio) = &node.audio {
        let url = match markup {
            Markup::Preview => audio.urls.first().cloned().unwrap_or_default(),
            Markup::App => audio.urls.join(" "),
        };
        let region = audio.region.as_deref().unwrap_or_default();
        body.push_str(&icon(
            &audio.icon,
            "button",
            &format!(
                " data-audio=\"{}\" title=\"{}\"",
                escape(&url),
                escape(region)
            ),
        ));
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
    if let Some(link) = &node.link
        && markup == Markup::App
    {
        let json = serde_json::to_string(link).expect("a link serializes");
        body = format!(
            "<a href=\"#\" class=\"dx-link\" data-link=\"{}\">{body}</a>",
            escape(&json)
        );
    } else if let Some(link) = &node.link {
        let target = link.entry.as_deref().unwrap_or_default();
        let title = if target.is_empty() { "look up" } else { target };
        body = format!(
            "<a href=\"#{}\" class=\"dx-link\" title=\"{}\">{body}</a>",
            escape(target),
            escape(title)
        );
    }
    let lead = node
        .icon
        .as_ref()
        .map(|lead| icon(lead, "span", ""))
        .unwrap_or_default();
    let before = pseudo(node.before.as_ref(), "before", false);
    let after = pseudo(node.after.as_ref(), "after", false);
    let marker = node.marker.as_ref().map_or_else(String::new, |marker| {
        let mark = marker
            .icon
            .as_deref()
            .map(svg)
            .unwrap_or_else(|| escape(&marker.text));
        format!(
            "<span class=\"dx-marker\" style=\"width:{}em;{}\">{mark}</span>",
            escape(&marker.width.to_string()),
            css_of(&marker.style, false)
        )
    });
    let title = pseudo(node.title.as_ref(), "title", true);
    let tag = if node.block { "div" } else { "span" };
    format!(
        "<{tag} class=\"dx-{}\"{extra} style=\"{css}\">{title}{marker}{lead}{before}{body}{after}</{tag}>",
        escape(&node.kind)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zdict_core::theme::Value;
    use zdict_core::{Anchor, Image, Link};

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
            "<span class=\"dx-xword\" style=\"\"><a href=\"#\" class=\"dx-link\" data-link=\"{&quot;entry&quot;:&quot;k\\&quot;2&quot;,&quot;text&quot;:&quot;pear&quot;}\">pear</a></span>"
        );
        let mut sense = node("sense", Vec::new());
        sense.anchor = Some(Anchor {
            id: "s1".into(),
            number: 3,
        });
        assert!(html(&sense, Markup::App).contains(" id=\"s1\" data-anchor=\"3\""));
        assert!(!html(&sense, Markup::Preview).contains("data-anchor"));
        let mut picture = node("image", Vec::new());
        picture.image = Some(Image {
            urls: vec!["a.jpg".into(), "b.jpg".into()],
        });
        assert!(html(&picture, Markup::App).contains("src=\"a.jpg\" data-fallback=\"b.jpg\""));
    }

    #[test]
    fn hostile_values_cannot_leave_their_attribute() {
        let mut bad = node("x\"><script>", vec![RenderChild::Text("<b>".into())]);
        bad.lang = Some("en\" onclick=\"x".into());
        bad.style.set("color", Value::Str("red;}<x".into()));
        bad.style.set("weight", Value::Str("bold".into()));
        let html = html(&bad, Markup::App);
        assert!(!html.contains("<script") && !html.contains("<b>") && !html.contains("\" onclick"));
        assert!(html.contains("style=\"font-weight:700\""), "{html}");
    }
}
