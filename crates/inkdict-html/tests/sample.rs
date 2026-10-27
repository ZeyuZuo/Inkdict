mod common;

use inkdict_core::{Pack, theme::ThemeStack};
use inkdict_html::{Markup, entry_html, entry_html_with_resources, theme_css};

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
    // Without a resource base (previews) only online URLs appear.
    assert!(!html.contains("inkdict-res"));
}

#[test]
fn sample_html_tries_files_the_pack_stores_first() {
    let pack = Pack::open(common::sample_pack()).unwrap();
    let registry = pack.registry().unwrap();
    let stack = ThemeStack::new(pack.theme("default").unwrap());
    let renderer = pack.renderer(&registry, &stack);
    let base = Some("inkdict-res://localhost/sample-en-zh/");
    let harbour = entry_html_with_resources(
        &renderer.render(&pack.entry("harbour").unwrap()),
        Markup::App,
        base,
    );
    assert!(harbour.contains(
        "data-audio=\"inkdict-res://localhost/sample-en-zh/audio/harbour-gb https://media.example.org/sample-en-zh/audio/harbour-gb.mp3\""
    ));
    assert!(
        harbour
            .contains("data-audio=\"https://media.example.org/sample-en-zh/audio/harbour-us.mp3\"")
    );
    let lantern = entry_html_with_resources(
        &renderer.render(&pack.entry("lantern").unwrap()),
        Markup::App,
        base,
    );
    assert!(lantern.contains("<img src=\"inkdict-res://localhost/sample-en-zh/image/lantern\""));
    let preview = entry_html_with_resources(
        &renderer.render(&pack.entry("lantern").unwrap()),
        Markup::Preview,
        base,
    );
    assert!(
        !preview.contains("inkdict-res"),
        "previews keep the reference HTML"
    );
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
