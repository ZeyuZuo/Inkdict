mod common;
use common::Fixture;
use inkdict_pack::Error;
use serde_json::json;

#[test]
fn manifest_ids_and_languages_reject_trailing_newlines() {
    for (from, to, message) in [
        (
            "id='test'",
            "id=\"test\\n\"",
            "pack.id (lowercase letters, digits, -) and pack.title are required",
        ),
        (
            "source='en'",
            "source=\"en\\n\"",
            "languages.source and languages.targets must be BCP 47 tags",
        ),
        (
            "targets=['zh-Hans', 'fr']",
            "targets=[\"zh-Hans\\n\"]",
            "languages.source and languages.targets must be BCP 47 tags",
        ),
    ] {
        let f = Fixture::new(&[]);
        f.replace("manifest.toml", from, to);
        let error = inkdict_pack::build(f.options()).unwrap_err();
        assert!(matches!(error, Error::Source(_)));
        assert!(error.to_string().ends_with(message), "{error}");
    }
}

#[test]
fn content_languages_and_resources_reject_trailing_line_breaks() {
    for ending in ["\n", "\r", "\r\n"] {
        let lang = format!("en{ending}");
        let resource = format!("audio:key{ending}");
        let f = Fixture::new(&[json!({"id":"e","lang":lang,"type":"entry","children":[
            {"type":"media","attrs":{"z":resource}}
        ]})]);
        let report = f.build();
        let problems = &report.problems.0["invalid content"];
        assert_eq!(problems.count, 2);
        assert_eq!(
            problems.samples[0].detail,
            format!("<entry>: invalid lang {}", json!(lang))
        );
        assert_eq!(
            problems.samples[1].detail,
            format!("<entry>/0<media>: attr z={} is not res?", json!(resource))
        );
    }
}

#[test]
fn validation_values_use_json_strings_booleans_null_and_containers() {
    for (value, kind) in [
        (json!(""), "string"),
        (json!(false), "string"),
        (json!(null), "string"),
        (json!([1, true]), "string"),
        (json!({"x":"y"}), "string"),
    ] {
        let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
            {"type":"required","attrs":{"z":value,"a":1,"b":true,"c":"x","d":"audio:key","e":"ref"}}
        ]})]);
        let report = f.build();
        let problems = &report.problems.0["invalid content"];
        assert_eq!(problems.count, 1);
        assert_eq!(
            problems.samples[0].detail,
            format!("<entry>/0<required>: attr z={value} is not {kind}")
        );
    }
}

#[test]
fn homographs_and_summary_checks_use_rust_whitespace() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"hom","text":"\u{1c}2\u{1c}"}
    ]})]);
    f.write(
        "summaries.jsonl",
        &json!({"entry":"e","text":"\u{1c}"}).to_string(),
    );
    let mut options = f.options();
    options.summaries = Some(f.path("summaries.jsonl"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!(report.summaries, 1);
    assert_eq!(
        report.problems.0["homograph not a positive integer"].samples[0].detail,
        "\u{1c}2\u{1c}"
    );
    assert!(
        !report
            .problems
            .0
            .contains_key("summary not one line of text")
    );
}
