//! Print the render tree using the exact markup of `oxdic/preview.py`.
//! Theme selection, interpolation, joins and visibility belong to the tree.

use zdict_core::theme::Style;
use zdict_core::{Generated, Icon, RenderChild, RenderNode};

/// Python's `html.escape` with its default `quote=True`.
fn escape(text: &str) -> String {
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

fn css_of(style: &Style, block: bool) -> String {
    let mut rules = Vec::new();
    for (name, value) in style.iter() {
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
            "case" => match value.as_str().unwrap() {
                "upper" => "text-transform:uppercase",
                "lower" => "text-transform:lowercase",
                "small-caps" => "font-variant:small-caps",
                "none" => "text-transform:none;font-variant:normal",
                other => panic!("unknown case: {other}"),
            }
            .to_owned(),
            "decoration" => format!(
                "text-decoration:{}",
                match value.as_str().unwrap() {
                    "strike" => "line-through",
                    "underline" => "underline",
                    "none" => "none",
                    other => panic!("unknown decoration: {other}"),
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

fn svg(name: &str) -> String {
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
        other => panic!("unknown icon: {other}"),
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

fn child(child: &RenderChild) -> String {
    match child {
        RenderChild::Text(text) => escape(text),
        RenderChild::Node(node) => html(node),
    }
}

fn children(node: &RenderNode) -> String {
    node.children.iter().map(child).collect()
}

pub fn html(node: &RenderNode) -> String {
    let mut extra = String::new();
    if let Some(lang) = &node.lang {
        // The reference writes lang verbatim (unlike id).
        extra.push_str(&format!(" lang=\"{lang}\""));
    }
    if let Some(anchor) = &node.anchor {
        extra.push_str(&format!(" id=\"{}\"", escape(&anchor.id)));
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
        let summary = collapse.summary.as_deref().map(child).unwrap_or_default();
        return format!(
            "<details class=\"dx-{}\"{state}{extra} style=\"{css}\"><summary>{summary}</summary>{}</details>",
            node.kind,
            children(node)
        );
    }
    let mut body = children(node);
    if let Some(audio) = &node.audio {
        let url = audio.urls.first().map(String::as_str).unwrap_or_default();
        let region = audio.region.as_deref().unwrap_or_default();
        body.push_str(&icon(
            &audio.icon,
            "button",
            &format!(
                " data-audio=\"{}\" title=\"{}\"",
                escape(url),
                escape(region)
            ),
        ));
    }
    if let Some(image) = &node.image {
        let url = image.urls.first().map(String::as_str).unwrap_or_default();
        body.push_str(&format!(
            "<img src=\"{}\" alt=\"\" loading=\"lazy\" style=\"width:100%\">",
            escape(url)
        ));
    }
    if let Some(link) = &node.link {
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
            marker.width,
            css_of(&marker.style, false)
        )
    });
    let title = pseudo(node.title.as_ref(), "title", true);
    let tag = if node.block { "div" } else { "span" };
    format!(
        "<{tag} class=\"dx-{}\"{extra} style=\"{css}\">{title}{marker}{lead}{before}{body}{after}</{tag}>",
        node.kind
    )
}
