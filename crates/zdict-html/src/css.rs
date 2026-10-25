//! The page-level CSS an entry's HTML expects (after OxDic's
//! `oxdic/preview.py` `page()`): theme tokens as custom properties and the
//! `.dx-*` rules that are not per-node styles.

use std::fmt::Write;

use zdict_core::theme::ThemeStack;

use crate::markup::svg_body;

/// Rules for generated parts: icons, markers, folding, links, titles, tables.
const BASE: &str = r#"button.dx-icon{border:0;background:none;color:inherit;cursor:pointer;padding:0 .12em;font:inherit;line-height:1}
button.dx-icon:hover{opacity:.7}
.dx-svg{display:inline-block;vertical-align:-.13em}
.dx-marker{position:absolute;left:0;text-align:left}
span.dx-icon{margin-right:.28em}
details>summary{list-style:none;cursor:pointer;position:relative;padding-right:1.6em}
details>summary::-webkit-details-marker{display:none}
details>summary::after{content:"";position:absolute;right:.2em;top:.35em;width:.45em;height:.45em;border-right:2px solid var(--c-faint);border-bottom:2px solid var(--c-faint);transform:rotate(45deg);transition:transform .15s}
details:not([open])>summary::after{transform:rotate(-45deg)}
.dx-link{color:inherit;text-decoration:none}
.dx-link:hover{text-decoration:underline}
.dx-title{font-family:var(--font-ui);font-weight:700;font-size:.8em;letter-spacing:.06em;text-transform:uppercase;color:var(--c-faint);margin-bottom:.2em}
.dx-table{display:table;border-collapse:collapse}
.dx-row{display:table-row}
.dx-cell{display:table-cell;border-bottom:1px solid var(--c-rule)}
"#;

/// CSS generic families: written bare, everything else quoted.
const GENERIC: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "emoji",
    "math",
];

/// Fonts as `--font-<token>`, colors as `--c-<token>`: light on `:root`,
/// dark under `[data-theme="dark"]` and, unless `[data-theme="light"]`,
/// when the system prefers dark. The page must define `--card` (the
/// background the arrow icon is cut out of).
pub fn theme_css(stack: &ThemeStack) -> String {
    let fonts: Vec<String> = stack
        .fonts()
        .iter()
        .filter(|(token, _)| is_token(token))
        .map(|(token, families)| format!("--font-{token}:{}", family_list(families)))
        .collect();
    let colors = |scheme: &str| -> Vec<String> {
        stack
            .colors(scheme)
            .iter()
            .filter(|(token, value)| is_token(token) && is_color(value))
            .map(|(token, value)| format!("--c-{token}:{value}"))
            .collect()
    };
    let light = [fonts, colors("light")].concat().join(";");
    let dark = colors("dark").join(";");
    let mut css = String::new();
    writeln!(css, ":root{{{light}}}").unwrap();
    writeln!(css, ":root[data-theme=\"dark\"]{{{dark}}}").unwrap();
    writeln!(
        css,
        "@media (prefers-color-scheme: dark){{:root:not([data-theme=\"light\"]){{{dark}}}}}"
    )
    .unwrap();
    css.push_str(BASE);
    css.push_str(&icon_masks());
    css
}

/// App-markup icons (`.dx-i-<name>`): the SVG as a mask over the text color.
fn icon_masks() -> String {
    let mut css = String::from(
        ".dx-svg[class*=\"dx-i-\"]{display:inline-block;width:1em;height:1em;background:currentColor;\
         -webkit-mask:var(--dx-icon) center/contain no-repeat;mask:var(--dx-icon) center/contain no-repeat}\n",
    );
    for name in ["speaker", "key", "diamond"] {
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'>{}</svg>",
            svg_body(name).replace('"', "'")
        );
        writeln!(
            css,
            ".dx-i-{name}{{--dx-icon:url(\"data:image/svg+xml,{}\")}}",
            data_uri(&svg)
        )
        .unwrap();
    }
    css
}

/// Percent-encode what a `data:` URL in CSS cannot carry literally.
fn data_uri(svg: &str) -> String {
    let mut encoded = String::with_capacity(svg.len());
    for ch in svg.chars() {
        match ch {
            '#' => encoded.push_str("%23"),
            '<' => encoded.push_str("%3C"),
            '>' => encoded.push_str("%3E"),
            '"' => encoded.push_str("%22"),
            '%' => encoded.push_str("%25"),
            _ => encoded.push(ch),
        }
    }
    encoded
}

fn family_list(families: &[String]) -> String {
    families
        .iter()
        .filter(|family| !family.is_empty() && !family.contains(['"', '\\', ';', '{', '}', '<']))
        .map(|family| {
            if GENERIC.contains(&family.as_str()) {
                family.clone()
            } else {
                format!("\"{family}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `#rgb`, `#rrggbb(aa)`, or a functional color without characters that
/// could end the declaration.
fn is_color(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"#(),.% -".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zdict_core::theme::Theme;

    #[test]
    fn writes_tokens_for_both_schemes_and_quotes_only_named_families() {
        let theme = Theme::parse(
            "t",
            "[meta]\nformat = \"zdict-theme\"\n[fonts]\ntext = [\"Source Serif 4\", \"serif\"]\n\
             [colors.light]\nfg = \"#111\"\nbad = \"red;}body{x\"\n[colors.dark]\nfg = \"rgb(1, 2, 3)\"\n",
        )
        .unwrap();
        let css = theme_css(&ThemeStack::new(theme));
        let first = css.lines().next().unwrap();
        assert_eq!(
            first,
            ":root{--font-text:\"Source Serif 4\", serif;--c-fg:#111}"
        );
        assert!(css.contains(":root[data-theme=\"dark\"]{--c-fg:rgb(1, 2, 3)}"));
        assert!(!css.contains("body{x"));
        assert!(css.contains("button.dx-icon"));
        assert!(css.contains(".dx-i-speaker{--dx-icon:url(\"data:image/svg+xml,%3Csvg xmlns="));
        assert!(!css.contains(".dx-i-arrow"), "the arrow stays inline");
    }
}
