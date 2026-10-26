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

const TYPES: &str = "[meta]\nformat = \"zdict-types\"\nversion = 1\n\
                     [types.pron]\nkind = \"inline\"\nroles = [\"pronunciation\"]\n\
                     [types.ipa]\nkind = \"inline\"\nroles = [\"pronunciation\"]\n\
                     [types.sense]\nkind = \"block\"\n";

#[test]
fn overlay_hides_the_types_of_each_switched_off_role() {
    let registry = Registry::parse(TYPES).unwrap();
    let settings = Settings {
        hide_pronunciation: true,
        hide_examples: true,
        ..Settings::default()
    };
    let overlay = settings.overlay(&registry);
    assert!(overlay.contains("[style.\"ipa\"]\ndisplay = \"none\""));
    assert!(overlay.contains("[style.\"pron\"]\ndisplay = \"none\""));
    assert!(!overlay.contains("sense"));
    Theme::parse("settings", &overlay).unwrap();
    assert!(
        settings::stack(
            &Theme::parse("t", "[meta]\nformat = \"zdict-theme\"\nversion = 1\n").unwrap(),
            &registry,
            &settings
        )
        .is_ok()
    );
    assert_eq!(
        Switches::of(&registry),
        Switches {
            pronunciation: true,
            example: false
        }
    );
}

#[test]
fn config_round_trips_and_tolerates_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zdict/config.json");
    assert_eq!(Config::load(&path), Config::default());
    let config = Config {
        packs: vec!["/data/oalecd10.zdpack".into()],
        active: Some("/data/oalecd10.zdpack".into()),
        settings: Settings {
            font_scale: 1.3,
            ..Settings::default()
        },
    };
    config.save(&path).unwrap();
    assert_eq!(Config::load(&path), config);
    std::fs::write(&path, "{not json").unwrap();
    assert_eq!(Config::load(&path), Config::default());
    // Missing fields take defaults; fields of older versions are ignored.
    std::fs::write(
        &path,
        r#"{"settings":{"hide_chinese":true,"hide_examples":true}}"#,
    )
    .unwrap();
    assert!(Config::load(&path).settings.hide_examples);
}

#[test]
fn commands_work_on_the_oxford_pack() {
    let Some(mut dictionary) = oxford() else {
        return;
    };
    let info = dictionary.info();
    assert_eq!(info.entries, Some(39960));
    assert_eq!(info.target_langs, ["zh-Hans"]);
    assert_eq!(
        info.switches,
        Switches {
            pronunciation: true,
            example: true
        }
    );
    let went = dictionary.words("went", 5).unwrap();
    let go = went
        .iter()
        .find(|w| w.target.as_deref() == Some("go"))
        .unwrap();
    assert_eq!(go.word.kind, zdict_core::IndexKind::Inflection);
    let json = serde_json::to_value(go).unwrap();
    assert_eq!(
        (json["form"].as_str(), json["target"].as_str()),
        (Some("went"), Some("go"))
    );
    let go = go.word.entry.clone();
    let close = dictionary.words("recieve", 1).unwrap();
    assert_eq!(close[0].word.form, "receive");
    assert_eq!(dictionary.entry("nope").unwrap_err().code, "not_found");

    let started = Instant::now();
    let view = dictionary.entry(&go).unwrap();
    let elapsed = started.elapsed();
    eprintln!("entry go: {} bytes of HTML in {elapsed:?}", view.html.len());
    assert!(view.html.contains("data-anchor=\"0\"") && view.html.contains("data-link=\""));
    assert!(
        view.html.contains("class=\"dx-tabs\""),
        "go has several parts of speech"
    );
    assert!(view.html.contains("class=\"dx-example "));

    dictionary
        .set_settings(Settings {
            hide_examples: true,
            ..Settings::default()
        })
        .unwrap();
    let hidden = dictionary.entry(&go).unwrap();
    assert!(!hidden.html.contains("class=\"dx-example "));
    assert!(hidden.html.len() < view.html.len());
    assert!(dictionary.theme_css().starts_with(":root{--font-"));
}

#[test]
fn follows_links_and_falls_back_to_lookup() {
    let Some(dictionary) = oxford() else {
        return;
    };
    let apple = dictionary.words("apple", 1).unwrap().remove(0).word.entry;
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
fn settings_layer_validates_against_the_oxford_registry() {
    let Some(dictionary) = oxford() else {
        return;
    };
    let problems = dictionary.stack.validate(&dictionary.registry);
    assert!(problems.is_empty(), "{problems:#?}");
}
