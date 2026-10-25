use std::path::PathBuf;
use std::time::Instant;

use super::*;

fn oxford() -> Option<Dictionary> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata/oalecd10.zdpack");
    if !path.is_file() {
        eprintln!("skipped: {} missing", path.display());
        return None;
    }
    Some(Dictionary::open(&path, Settings::default()).unwrap())
}

#[test]
fn overlay_hides_what_the_settings_ask() {
    let settings = Settings {
        hide_chinese: true,
        hide_examples: true,
        ..Settings::default()
    };
    let overlay = settings.overlay();
    assert!(overlay.contains("[style.\"*[lang=zh]\"]\ndisplay = \"none\""));
    assert!(overlay.contains("[style.\"example\"]"));
    assert!(!overlay.contains("pron_list"));
    Theme::parse("settings", &overlay).unwrap();
}

#[test]
fn config_round_trips_and_tolerates_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zdict/config.json");
    assert_eq!(Config::load(&path), Config::default());
    let config = Config {
        pack: Some("/data/oalecd10.zdpack".into()),
        settings: Settings {
            font_scale: 1.3,
            ..Settings::default()
        },
    };
    config.save(&path).unwrap();
    assert_eq!(Config::load(&path), config);
    std::fs::write(&path, "{not json").unwrap();
    assert_eq!(Config::load(&path), Config::default());
    std::fs::write(&path, r#"{"settings":{"hide_chinese":true}}"#).unwrap();
    assert!(
        Config::load(&path).settings.hide_chinese,
        "missing fields take defaults"
    );
}

#[test]
fn commands_work_on_the_oxford_pack() {
    let Some(mut dictionary) = oxford() else {
        return;
    };
    assert_eq!(dictionary.info().entries, Some(39960));
    let went = dictionary.suggest("went", 5).unwrap();
    let go = went
        .iter()
        .find(|s| s.headword == "go")
        .unwrap()
        .entry
        .clone();
    assert_eq!(
        dictionary.correct("recieve", 1).unwrap()[0].suggestion.form,
        "receive"
    );
    assert!(!dictionary.search("苹果", 5, None).unwrap().is_empty());
    assert_eq!(
        dictionary.search("x", 5, Some("bogus")).unwrap_err().code,
        "bad_input"
    );
    assert_eq!(dictionary.entry("nope").unwrap_err().code, "not_found");

    let started = Instant::now();
    let view = dictionary.entry(&go).unwrap();
    let elapsed = started.elapsed();
    eprintln!("entry go: {} bytes of HTML in {elapsed:?}", view.html.len());
    assert!(view.html.contains("data-anchor=\"0\"") && view.html.contains("data-link=\""));
    assert!(view.html.contains("lang=\"zh-Hans\""));

    dictionary.set_settings(Settings {
        hide_chinese: true,
        ..Settings::default()
    });
    let hidden = dictionary.entry(&go).unwrap();
    assert!(!hidden.html.contains("lang=\"zh-Hans\""));
    assert!(hidden.html.len() < view.html.len());
    assert!(dictionary.theme_css().starts_with(":root{--font-"));
}

#[test]
fn follows_links_and_falls_back_to_lookup() {
    let Some(dictionary) = oxford() else {
        return;
    };
    let apple = dictionary.suggest("apple", 1).unwrap().remove(0).entry;
    let lookup = Link {
        entry: None,
        anchor: None,
        anchor_type: None,
        text: "pear".into(),
    };
    assert_eq!(
        dictionary.follow(&apple, &lookup).unwrap(),
        Target::Lookup {
            text: "pear".into()
        }
    );
    let to_apple = Link {
        entry: Some(apple.clone()),
        anchor: None,
        anchor_type: None,
        text: String::new(),
    };
    let json = serde_json::to_value(dictionary.follow(&apple, &to_apple).unwrap()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"kind": "entry", "key": apple, "focus": {"kind": "top"}})
    );
}

#[test]
fn desktop_theme_layer_validates_against_the_oxford_registry() {
    let Some(dictionary) = oxford() else {
        return;
    };
    let problems = dictionary.stack.validate(&dictionary.registry);
    assert!(problems.is_empty(), "{problems:#?}");
}
