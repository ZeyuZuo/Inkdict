mod common;

use inkdict_core::{Pack, theme::ThemeStack};
use inkdict_html::{Markup, entry_html, theme_css};

#[test]
fn sample_html_tabs_anchors_and_audio_urls() {
    let pack = Pack::open(common::sample_pack()).unwrap();
    let registry = pack.registry().unwrap();
    let stack = ThemeStack::new(pack.theme("default").unwrap());
    let tree = pack
        .renderer(&registry, &stack)
        .render(&pack.entry("harbour").unwrap());
    let html = entry_html(&tree, Markup::App);
    assert_eq!(html.matches("class=\"dx-tabs\"").count(), 1);
    assert_eq!(html.matches("role=\"tab\"").count(), 2);
    assert_eq!(html.matches("role=\"tabpanel\"").count(), 2);
    assert_eq!(html.matches("role=\"tabpanel\" hidden").count(), 1);
    assert!(html.contains(">noun</span></button>"));
    assert!(html.contains(">verb</span></button>"));
    // Document-order anchors from the source: root, noun, noun sense, ... verb.
    for number in [0, 1, 2, 7, 8] {
        assert!(html.contains(&format!("data-anchor=\"{number}\"")));
    }
    for region in ["gb", "us"] {
        assert!(html.contains(&format!(
            "data-audio=\"https://media.example.org/sample-en-zh/audio/harbour-{region}.mp3\""
        )));
    }
    assert_eq!(
        html.matches("<button class=\"dx-icon\" data-audio=")
            .count(),
        2
    );
    // The GB bytes are embedded, but the current renderer exposes only source URLs.
    // Embedded-resource URLs need a product change; byte retrieval is tested in core.
    assert!(pack.resource("audio:harbour-gb").unwrap().is_some());
    assert!(pack.resource("audio:harbour-us").unwrap().is_none());
}

#[test]
fn sample_html_link_and_hidden_form() {
    let pack = Pack::open(common::sample_pack()).unwrap();
    let registry = pack.registry().unwrap();
    let stack = ThemeStack::new(pack.theme("default").unwrap());
    let tree = pack
        .renderer(&registry, &stack)
        .render(&pack.entry("lead-1").unwrap());
    let html = entry_html(&tree, Markup::App);
    assert!(html.contains(
        "data-link=\"{&quot;entry&quot;:&quot;lead-2&quot;,&quot;text&quot;:&quot;lead&quot;}\""
    ));
    assert!(!html.contains("dx-form_index"));
    assert!(!html.contains("leads"));
    assert_eq!(pack.words("leads", 1).unwrap()[0].entry, "lead-1");
}

#[test]
fn sample_theme_css_light_dark_and_language_fonts() {
    let pack = Pack::open(common::sample_pack()).unwrap();
    let stack = ThemeStack::new(pack.theme("default").unwrap());
    let css = theme_css(&stack);
    assert!(css.contains("--c-fg:#1b1c1f"));
    assert!(css.contains("--c-fg:#e5e5e7"));
    assert!(css.contains("[data-theme=\"dark\"]"));
    assert!(css.contains("Source Serif 4"));
    assert!(css.contains("Noto Serif CJK SC"));
    assert!(css.contains("Charis SIL"));
}
