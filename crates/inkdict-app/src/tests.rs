use std::path::PathBuf;
use std::time::Instant;

use super::*;

// Unit tests do not receive Cargo’s integration-test temporary directory.
pub fn sample_pack() -> PathBuf {
    static PACK: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PACK.get_or_init(|| {
        let dir = PathBuf::from(
            option_env!("CARGO_TARGET_TMPDIR")
                .unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tmp")),
        )
        .join("inkdict-app-unit-sample");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.inkdict");
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../spec/samples/en-zh/manifest.toml");
        let mut options = inkdict_pack::Options::new(manifest);
        options.output = Some(path.clone());
        inkdict_pack::build(options).unwrap();
        path
    })
    .clone()
}

fn oxford() -> Option<Dictionary> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata/oalecd10.inkdict");
    if !path.is_file() {
        eprintln!("skipped: {} missing", path.display());
        return None;
    }
    Some(Dictionary::open(&path, Settings::default(), None).unwrap())
}

const TYPES: &str = "[meta]\nformat = \"inkdict-types\"\nversion = 1\n\
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
            &Theme::parse("t", "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n").unwrap(),
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
    let path = dir.path().join("inkdict/config.json");
    assert_eq!(Config::load(&path), Config::default());
    let config = Config {
        themes: [("oalecd10".into(), "classic".into())].into(),
        packs: vec!["/data/oalecd10.inkdict".into()],
        active: Some("/data/oalecd10.inkdict".into()),
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
    assert_eq!(go.word.kind, inkdict_core::IndexKind::Inflection);
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

#[test]
fn sample_commands_info_words_and_entry() {
    let dictionary = Dictionary::open(&sample_pack(), Settings::default(), None).unwrap();
    dictionary.warm_up().unwrap();
    let info = dictionary.info();
    assert_eq!(info.id, "sample-en-zh");
    assert_eq!(info.title, "示例英汉双解词典");
    assert_eq!(info.entries, Some(6));
    assert_eq!(info.source_lang, "en");
    assert_eq!(info.target_langs, ["zh-Hans"]);
    assert_eq!(info.theme, "default");
    assert_eq!(
        info.themes,
        vec![
            ThemeInfo {
                name: "default".into(),
                title: "Default".into()
            },
            ThemeInfo {
                name: "plain".into(),
                title: "Plain".into()
            },
        ]
    );
    assert_eq!(
        info.switches,
        Switches {
            pronunciation: true,
            example: true
        }
    );
    let words = dictionary.words("lead", 20).unwrap();
    assert_eq!(words.len(), 5);
    assert_eq!(words[0].word.entry, "lead-1");
    assert_eq!(words[0].word.homograph, Some(1));
    assert_eq!(words[1].word.entry, "lead-2");
    assert_eq!(words[1].word.homograph, Some(2));
    let went = dictionary.words("went", 1).unwrap();
    assert_eq!(went.len(), 1);
    assert_eq!(went[0].word.kind, inkdict_core::IndexKind::Inflection);
    assert_eq!(went[0].word.summary.as_deref(), Some("v. 去；走"));
    assert_eq!(serde_json::to_value(&went[0]).unwrap()["target"], "go");
    assert_eq!(went[0].target.as_deref(), Some("go"));
    assert_eq!(dictionary.words("CAFÉ", 1).unwrap()[0].word.entry, "cafe");
    let close = dictionary.words("harbur", 1).unwrap();
    assert_eq!(close[0].word.form, "harbour");
    assert_eq!(close[0].word.distance, Some(1));
    let view = dictionary.entry("harbour").unwrap();
    assert_eq!(view.key, "harbour");
    assert_eq!(view.headword, "harbour");
    assert_eq!(view.homograph, None);
    assert!(view.html.contains("data-anchor=\"2\""));
    assert!(view.html.contains("class=\"dx-tabs\""));
    assert!(view.html.contains("港口；海港"));
    let lead = dictionary.entry("lead-2").unwrap();
    assert_eq!(lead.homograph, Some(2));
    assert!(lead.html.contains("data-link=\""));
    assert_eq!(dictionary.entry("missing").unwrap_err().code, "not_found");
}

#[test]
fn sample_follow_links_and_fallbacks() {
    let dictionary = Dictionary::open(&sample_pack(), Settings::default(), None).unwrap();
    for (from, entry, anchor, text, target) in [
        (
            "lead-1",
            Some("lead-2"),
            None,
            "lead",
            Target::Entry {
                key: "lead-2".into(),
                focus: FocusView::Top,
            },
        ),
        (
            "lead-2",
            Some("lead-1"),
            Some("v-1"),
            "lead",
            Target::Entry {
                key: "lead-1".into(),
                focus: FocusView::Anchor { number: 2 },
            },
        ),
        (
            "lantern",
            Some("harbour"),
            Some("n-1"),
            "harbour",
            Target::Entry {
                key: "harbour".into(),
                focus: FocusView::Anchor { number: 2 },
            },
        ),
        (
            "lead-1",
            Some("lead-2"),
            Some("missing"),
            "lead",
            Target::Entry {
                key: "lead-2".into(),
                focus: FocusView::Top,
            },
        ),
        (
            "lead-1",
            Some("missing"),
            Some("v-1"),
            "lead",
            Target::Entry {
                key: "lead-1".into(),
                focus: FocusView::Anchor { number: 2 },
            },
        ),
        (
            "lead-1",
            Some("missing"),
            Some("missing"),
            "café",
            Target::Lookup {
                text: "café".into(),
            },
        ),
    ] {
        let link = Link {
            entry: entry.map(str::to_owned),
            anchor: anchor.map(str::to_owned),
            text: text.into(),
        };
        let actual = dictionary.follow(from, &link).unwrap();
        assert_eq!(actual, target);
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(target).unwrap()
        );
    }
}

#[test]
fn sample_settings_hide_pronunciation_and_examples_independently() {
    let mut dictionary = Dictionary::open(&sample_pack(), Settings::default(), None).unwrap();
    let original = dictionary.entry("harbour").unwrap();
    let words = dictionary.words("harbour", 20).unwrap();
    for (hide_pronunciation, hide_examples) in
        [(true, false), (false, true), (true, true), (false, false)]
    {
        dictionary
            .set_settings(Settings {
                hide_pronunciation,
                hide_examples,
                ..Settings::default()
            })
            .unwrap();
        assert!(dictionary.stack.validate(&dictionary.registry).is_empty());
        let view = dictionary.entry("harbour").unwrap();
        assert_eq!(view.html.contains("ˈhɑːbə(r)"), !hide_pronunciation);
        assert_eq!(view.html.contains("data-audio=\""), !hide_pronunciation);
        assert_eq!(view.html.contains("class=\"dx-example "), !hide_examples);
        assert_eq!(
            view.html.contains("The fishing boats were back"),
            !hide_examples
        );
        assert_eq!(view.html.contains("暴风雨来临前"), !hide_examples);
        assert!(view.html.contains("港口；海港"));
        assert!(view.html.contains("a stretch of water"));
        assert_eq!(dictionary.words("harbour", 20).unwrap(), words);
    }
    assert_eq!(dictionary.entry("harbour").unwrap(), original);
    assert!(dictionary.theme_css().contains("--c-fg:#1b1c1f"));
}

#[test]
fn sample_resources_are_served_by_path_and_tried_first() {
    let dir = tempfile::tempdir().unwrap();
    let (mut session, error) =
        Session::start(Some(dir.path().join("config.json")), Some(sample_pack()));
    assert!(error.is_none());
    session.set_resource_base("inkdict-res://localhost/");
    let dictionary = session.dictionary().unwrap();
    let html = dictionary.entry("lantern").unwrap().html;
    assert!(html.contains("src=\"inkdict-res://localhost/sample-en-zh/image/lantern\""));
    let (mime, body) = dictionary.resource("/sample-en-zh/image/lantern").unwrap();
    assert_eq!(mime, "image/svg+xml");
    assert!(body.starts_with(b"<svg") || body.starts_with(b"<?xml"));
    assert_eq!(
        dictionary
            .resource("sample-en-zh/audio/harbour-gb")
            .unwrap()
            .0,
        "audio/wav"
    );
    assert!(dictionary.resource("other-pack/image/lantern").is_none());
    assert!(
        dictionary
            .resource("sample-en-zh/audio/harbour-us")
            .is_none()
    );
    assert!(
        dictionary
            .resource("sample-en-zh/image/lan%74ern")
            .is_some()
    );
    assert!(dictionary.resource("sample-en-zh/image/%zz").is_none());
    assert!(dictionary.resource("sample-en-zh/image").is_none());
}

#[test]
fn sample_theme_switch_rebuilds_rendering_and_keeps_settings() {
    let settings = Settings {
        hide_pronunciation: true,
        font_scale: 1.2,
        ..Settings::default()
    };
    let mut dictionary = Dictionary::open(&sample_pack(), settings.clone(), None).unwrap();
    let original = dictionary.entry("harbour").unwrap();
    let css = dictionary.theme_css();
    assert!(original.html.contains("class=\"dx-tabs\""));
    dictionary.set_theme("plain").unwrap();
    let plain = dictionary.entry("harbour").unwrap();
    assert_eq!(dictionary.info().theme, "plain");
    assert_ne!(dictionary.theme_css(), css);
    assert_ne!(plain, original);
    assert!(!plain.html.contains("class=\"dx-tabs\""));
    assert!(!plain.html.contains("◆"));
    assert!(!plain.html.contains("ˈhɑːbə(r)"));
    assert!(plain.html.contains("The fishing boats were back"));
    assert_eq!(dictionary.settings(), &settings);
    assert!(dictionary.stack.validate(&dictionary.registry).is_empty());

    let plain_css = dictionary.theme_css();
    assert_eq!(
        dictionary.set_theme("missing").unwrap_err().code,
        "bad_input"
    );
    assert_eq!(dictionary.info().theme, "plain");
    assert_eq!(dictionary.theme_css(), plain_css);
    assert_eq!(dictionary.entry("harbour").unwrap(), plain);
    assert_eq!(dictionary.settings(), &settings);

    dictionary
        .set_settings(Settings {
            hide_examples: true,
            ..settings
        })
        .unwrap();
    assert!(
        !dictionary
            .entry("harbour")
            .unwrap()
            .html
            .contains("The fishing boats were back")
    );
    dictionary.set_theme("default").unwrap();
    assert_eq!(dictionary.theme_css(), css);
    assert!(
        !dictionary
            .entry("harbour")
            .unwrap()
            .html
            .contains("The fishing boats were back")
    );
}

#[test]
fn opening_parses_every_theme_and_rejects_a_broken_alternative() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.inkdict");
    std::fs::copy(sample_pack(), &path).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE files SET body = ? WHERE name = 'themes/plain.toml'",
        [b"not valid TOML".as_slice()],
    )
    .unwrap();
    drop(db);
    let error = Dictionary::open(&path, Settings::default(), None)
        .err()
        .unwrap();
    assert_eq!(error.code, "bad_input");
    assert!(error.message.contains("plain"));
}
