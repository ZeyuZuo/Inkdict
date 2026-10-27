mod common;
use common::Fixture;
use inkdict_pack::Error;
use serde_json::json;
use std::fs;

#[test]
fn manifest_checks_report_source_errors() {
    for (from, to, message) in [
        (
            "inkdict-source",
            "wrong",
            "not an inkdict-source v1 manifest",
        ),
        (
            "id='test'",
            "id='Test'",
            "pack.id (lowercase letters, digits, -) and pack.title are required",
        ),
        (
            "source='en'",
            "source='english'",
            "languages.source and languages.targets must be BCP 47 tags",
        ),
        (
            "targets=['zh-Hans', 'fr']",
            "targets=[12]",
            "languages.source and languages.targets must be BCP 47 tags",
        ),
        (
            "types='types.toml'",
            "types=''",
            "files.types, files.themes and files.entries are required",
        ),
        ("fold-1", "unknown", "lookup.norm must be fold-1 or exact-1"),
    ] {
        let f = Fixture::new(&[]);
        f.replace("manifest.toml", from, to);
        let error = inkdict_pack::build(f.options()).unwrap_err();
        assert!(matches!(error, Error::Source(_)));
        assert!(error.to_string().ends_with(message), "{error}");
    }
}

#[test]
fn bad_source_templates_and_boosts_stop_builds() {
    let f = Fixture::new(&[]);
    let manifest = fs::read_to_string(f.path("manifest.toml")).unwrap();
    for (kind, template) in [("Bad", "{key}"), ("audio", "https://example/x")] {
        f.write(
            "manifest.toml",
            &format!("{manifest}\n[[source]]\nkind='{kind}'\ntemplate='{template}'"),
        );
        assert!(
            f.error()
                .ends_with("each [[source]] needs a kind and a template with {key}")
        );
    }
    f.write("manifest.toml", &manifest);
    f.replace("manifest.toml", "badge[level=1]", "badge::before");
    assert_eq!(
        f.error(),
        "boost selector \"badge::before\" must not name a pseudo-element"
    );
    f.replace("manifest.toml", "badge::before", "badge >");
    assert!(f.error().contains("incomplete selector"));
}

#[test]
fn missing_inputs_bad_registry_and_invalid_themes_are_source_errors() {
    let f = Fixture::new(&[]);
    fs::remove_file(f.path("entries.jsonl")).unwrap();
    assert!(f.error().ends_with("entries file not found"));
    f.write("entries.jsonl", "");
    f.replace("types.toml", "inkdict-types", "invalid");
    assert!(f.error().contains("not an inkdict-types"));
    f.write("types.toml", common::TYPES);
    f.write(
        "theme.toml",
        "[meta]\nformat='inkdict-theme'\nversion=1\n[style.missing]\nsize=1",
    );
    assert!(
        f.error()
            .ends_with("missing: unknown type missing (1 problems)")
    );
    f.write("theme.toml", "not toml");
    assert!(matches!(
        inkdict_pack::build(f.options()),
        Err(Error::Source(_))
    ));
    f.write("theme.toml", "[meta]\nformat='inkdict-theme'\nversion=1");
    let mut options = f.options();
    options.summaries = Some(f.path("absent"));
    assert!(
        inkdict_pack::build(options)
            .unwrap_err()
            .to_string()
            .ends_with("summaries file not found")
    );
}

#[test]
fn bad_entry_lines_roots_and_duplicate_entry_keys_stop_builds() {
    let f = Fixture::new(&[]);
    for line in [
        "[]",
        "{}",
        "{\"id\":1,\"lang\":\"en\"}",
        "{\"id\":\"e\"}",
        "{\"id\":\"\",\"lang\":\"en\"}",
    ] {
        f.write("entries.jsonl", line);
        assert!(f.error().ends_with(":1: the root needs an id and a lang"));
    }
    f.write("entries.jsonl", "bad");
    assert!(
        f.error()
            .ends_with(":1: not JSON (expected value at line 1 column 1)")
    );
    let line = json!({"id":"e","lang":"en","type":"entry"}).to_string();
    f.write("entries.jsonl", &format!("{line}\n{line}"));
    assert!(f.error().ends_with(":2: duplicate entry key e"));
    f.write("entries.jsonl", &format!("{line}\n\n"));
    assert!(f.error().contains(":2: not JSON"));
}

#[test]
fn resource_extensions_kinds_and_duplicate_keys_stop_builds() {
    for (path, message) in [
        ("resources/audio/key.xyz", "unknown resource extension"),
        ("resources/audio/.mp3", "unknown resource extension"),
        ("resources/Bad/key.mp3", "invalid resource kind \"Bad\""),
    ] {
        let f = Fixture::new(&[]);
        f.replace(
            "manifest.toml",
            "entries='entries.jsonl'",
            "entries='entries.jsonl'\nresources='resources'",
        );
        f.write(path, "x");
        assert!(f.error().ends_with(message));
    }
    let f = Fixture::new(&[]);
    f.replace(
        "manifest.toml",
        "entries='entries.jsonl'",
        "entries='entries.jsonl'\nresources='resources'",
    );
    f.write("resources/audio/key.mp3", "x");
    f.write("resources/audio/key.wav", "x");
    assert!(f.error().ends_with("duplicate resource audio:key"));
}

#[test]
fn atomic_failure_preserves_existing_output_and_cleans_temporary() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry"})]);
    f.write("test.inkdict", "original");
    f.write(".test.inkdict.tmp", "stale");
    f.write("summaries.jsonl", "invalid json");
    let mut options = f.options();
    options.summaries = Some(f.path("summaries.jsonl"));
    assert!(inkdict_pack::build(options).is_err());
    assert_eq!(
        fs::read_to_string(f.path("test.inkdict")).unwrap(),
        "original"
    );
    assert!(!f.path(".test.inkdict.tmp").exists());
    f.build();
    assert!(inkdict_core::Pack::open(f.path("test.inkdict")).is_ok());
}

#[test]
fn entry_and_summary_overrides_are_used() {
    let f = Fixture::new(&[]);
    f.write(
        "override.jsonl",
        &json!({"id":"override","lang":"en","type":"entry"}).to_string(),
    );
    f.write(
        "override-summary.jsonl",
        &json!({"entry":"override","text":"summary"}).to_string(),
    );
    let mut options = f.options();
    options.entries = Some(f.path("override.jsonl"));
    options.summaries = Some(f.path("override-summary.jsonl"));
    options.output = Some(f.path("nested/override.inkdict"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!((report.entries, report.summaries), (1, 1));
    assert!(f.path("nested/override.inkdict").is_file());
}
