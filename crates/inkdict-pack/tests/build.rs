mod common;
use common::{Fixture, sample};
use inkdict_core::Pack;
use inkdict_pack::{Options, Problems};
use serde_json::{Value, json};
use std::fs;

#[test]
fn sample_words_and_raw_entry_bodies_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = Options::new(sample().join("manifest.toml"));
    options.output = Some(dir.path().join("sample.inkdict"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!(
        (report.entries, report.lookup_rows, report.resources),
        (6, 23, 4)
    );
    assert_eq!(report.problems.0.len(), 0);
    let pack = Pack::open(&report.output).unwrap();
    for (form, key) in [
        ("harbor", "harbour"),
        ("harbouring", "harbour"),
        ("safe harbour", "harbour"),
        ("lantern", "lantern"),
        ("went", "go"),
        ("on the go", "go"),
    ] {
        assert!(
            pack.words(form, 20).unwrap().iter().any(|w| w.entry == key),
            "{form}"
        );
    }
    let text = fs::read_to_string(sample().join("entries.jsonl")).unwrap();
    for line in text.lines() {
        let entry: Value = serde_json::from_str(line).unwrap();
        let key = entry["id"].as_str().unwrap();
        assert_eq!(pack.entry_json(key).unwrap(), line);
        assert_eq!(
            serde_json::to_value(pack.entry(key).unwrap().root).unwrap(),
            entry
        );
    }
    assert_eq!(report.meta["builder"], "inkdict-pack 0.1.0");
    assert!(report.meta["built_at"].ends_with("+00:00"));
    let db = rusqlite::Connection::open(&report.output).unwrap();
    for (pragma, expected) in [
        ("application_id", inkdict_core::APPLICATION_ID),
        ("user_version", inkdict_core::FORMAT_VERSION),
    ] {
        assert_eq!(
            db.query_row(&format!("PRAGMA {pragma}"), [], |r| r.get::<_, u32>(0))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn index_classes_boosts_ties_and_fold_keys() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"variant","text":"Café"}, {"type":"word","text":"CAFÉ"}, {"type":"word","text":"cafe"},
        {"type":"badge","attrs":{"flag":true}}, {"type":"badge","attrs":{"level":1}},
        {"type":"phrase","text":"phrase"}, {"type":"infl","text":"infl"},
        {"type":"reading","text":"reading"}, {"type":"hidden","text":"hidden"},
        {"type":"group","id":"g","children":[{"type":"word","text":"nested"}]}]})]);
    let report = f.build();
    assert!(report.problems.0.is_empty());
    let db = f.db();
    let rows = db
        .prepare("SELECT norm,rank,form,kind FROM lookup ORDER BY rank")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        rows.iter().map(|r| r.1).collect::<Vec<_>>(),
        [0, 3, 9, 12, 15, 18]
    );
    assert_eq!(
        rows[0],
        ("cafe".into(), 0, "CAFÉ".into(), "headword".into())
    );
    f.replace("manifest.toml", "fold-1", "exact-1");
    drop(db); // Windows cannot replace a file that is open
    f.build();
    let db = f.db();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM lookup WHERE norm IN ('café','cafe')",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
}

#[test]
fn boost_default_and_first_matching_selector_determine_importance() {
    let f = Fixture::new(&[
        json!({"id":"a","lang":"en","type":"entry","children":[{"type":"word","text":"same"}]}),
        json!({"id":"b","lang":"en","type":"entry","children":[{"type":"word","text":"same"},{"type":"badge","attrs":{"flag":true}}]}),
        json!({"id":"c","lang":"en","type":"entry","children":[{"type":"word","text":"same"},{"type":"badge","attrs":{"level":1}}]}),
    ]);
    f.build();
    let ranks: Vec<i64> = f
        .db()
        .prepare("SELECT rank FROM lookup ORDER BY entry")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(ranks, [2, 1, 0]);
    f.replace(
        "manifest.toml",
        "badge[level=1]",
        "entry[lang=en] > badge[level=1]",
    );
    f.build();
    assert_eq!(
        f.db()
            .query_row("SELECT rank FROM lookup WHERE entry=3", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn first_homograph_at_root_decides_even_when_invalid() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"group","id":"g","children":[{"type":"hom","text":"3"},{"type":"word","text":"fallback"}]},
        {"type":"hom","text":"0"},{"type":"hom","text":"2"}]})]);
    let report = f.build();
    let pack = Pack::open(report.output).unwrap();
    let entry = pack.entry("e").unwrap();
    assert_eq!(entry.headword, "fallback");
    assert_eq!(entry.homograph, None);
    assert_eq!(
        report.problems.0["homograph not a positive integer"].samples[0].detail,
        "0"
    );
    drop(pack); // Windows cannot replace a file that is open
    f.replace("entries.jsonl", "\"text\":\"0\"", "\"text\":\"٢\"");
    assert_eq!(
        Pack::open(f.build().output)
            .unwrap()
            .entry("e")
            .unwrap()
            .homograph,
        Some(2)
    );
}

#[test]
fn readings_overflow_and_empty_index_problems() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"word","text":"(a)(b)(c)(d)(e)(f)"},{"type":"variant","text":"  "}]})]);
    let report = f.build();
    assert_eq!(report.lookup_rows, 32);
    assert_eq!(
        report.problems.0["readings cut at 32"].samples[0].detail,
        "(a)(b)(c)(d)(e)(f)"
    );
    assert_eq!(report.problems.0["index node without readings"].count, 1);
}

#[test]
fn links_and_summaries_use_first_duplicate_anchor() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"group","id":"g"},{"type":"group","id":"g"},
        {"type":"link","attrs":{"anchor":"g"}}, {"type":"link","attrs":{"entry":"missing"}},
        {"type":"link","attrs":{"anchor":"missing"}}, {"type":"link","attrs":{"entry":"e","anchor":"g"}}]})]);
    f.write(
        "summaries.jsonl",
        "{\"entry\":\"e\",\"anchor\":\"g\",\"text\":\"first\"}\n",
    );
    let mut options = f.options();
    options.summaries = Some(f.path("summaries.jsonl"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!(report.links, 4);
    assert_eq!(report.problems.0["entry not in pack"].count, 1);
    assert_eq!(
        report.problems.0["anchor not found"].samples[0].detail,
        "e#missing"
    );
    assert_eq!(
        f.db()
            .query_row("SELECT anchor FROM summaries", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn summary_validation_order_and_sorting() {
    let f = Fixture::new(&[
        json!({"id":"e","lang":"en","type":"entry","children":[{"type":"group","id":"g"}]}),
    ]);
    f.write(
        "summaries.jsonl",
        &[
            json!([]),
            json!({"entry":"e","anchor":null,"text":"x"}),
            json!({"entry":"missing","text":"x"}),
            json!({"entry":"e","anchor":"absent","text":""}),
            json!({"entry":"e","text":" \n"}),
            json!({"entry":"e","text":"one\ntwo"}),
            json!({"entry":"e","anchor":"g","text":"group","lang":"zh"}),
            json!({"entry":"e","text":"root","lang":""}),
            json!({"entry":"e","text":"duplicate"}),
            json!({"entry":"e","anchor":"g","text":"duplicate"}),
        ]
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n"),
    );
    let mut options = f.options();
    options.summaries = Some(f.path("summaries.jsonl"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!(report.summaries, 2);
    for (kind, count) in [
        ("summary not an object with entry, anchor, lang", 2),
        ("summary for missing entry", 1),
        ("summary for missing anchor", 1),
        ("summary not one line of text", 2),
        ("duplicate summary", 2),
    ] {
        assert_eq!(report.problems.0[kind].count, count, "{kind}");
    }
    let rows: Vec<(i64, String)> = f
        .db()
        .prepare("SELECT anchor,lang FROM summaries ORDER BY entry,anchor")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows, [(0, "en".into()), (1, "zh".into())]);
}

#[test]
fn resource_refs_sources_priorities_and_mimes() {
    let f = Fixture::new(&[json!({"id":"e","lang":"en","type":"entry","children":[
        {"type":"media","attrs":{"z":"audio:local","a":"remote:key"}},
        {"type":"media","attrs":{"z":"absent:key","a":"absent:key"}}]})]);
    f.replace(
        "manifest.toml",
        "entries='entries.jsonl'",
        "entries='entries.jsonl'\nresources='resources'",
    );
    f.write("resources/audio/local.MP3", "bytes");
    f.write("resources/ignored.txt", "ignored");
    let manifest = fs::read_to_string(f.path("manifest.toml")).unwrap();
    f.write("manifest.toml", &format!("{manifest}\n[[source]]\nkind='remote'\ntemplate='https://a/{{key}}'\n[[source]]\nkind='remote'\ntemplate='https://b/{{key}}'\n[[source]]\nkind='audio'\ntemplate='https://c/{{key}}'\n"));
    let report = f.build();
    assert_eq!((report.resources, report.resource_refs), (1, 4));
    assert_eq!(report.problems.0["resource without source"].count, 2);
    assert_eq!(
        f.db()
            .query_row("SELECT mime FROM resources", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "audio/mpeg"
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT max(priority) FROM resource_sources WHERE kind='remote'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn raw_validation_reports_types_keys_attributes_and_bad_children() {
    let f = Fixture::new(&[
        json!({"id":"e","lang":"bad!","type":"entry","extra":true,"children":[
        {"type":"required","id":"e","attrs":{"z":"","a":true,"b":1,"c":"z","d":"bad","e":"","other":0},"text":"x","children":[false]},
        {"type":"required"}, {"type":"unknown"}, {"type":"word","text":12}, 42]}),
    ]);
    let report = f.build();
    assert!(report.problems.0["invalid content"].count > 15);
    assert_eq!(report.problems.0["invalid content"].samples.len(), 5);
    assert_eq!(
        report.problems.0["invalid content"].samples[0].detail,
        "<entry>: unknown key \"extra\""
    );
    assert_eq!(
        report.problems.0["invalid content"].samples[1].detail,
        "<entry>: invalid lang \"bad!\""
    );
    assert_eq!(
        report.problems.0["invalid content"].samples[2].detail,
        "<entry>/0<required>: duplicate id e"
    );
}

#[test]
fn required_attributes_follow_declaration_order() {
    let f = Fixture::new(&[
        json!({"id":"e","lang":"en","type":"entry","children":[{"type":"required"}]}),
    ]);
    let report = f.build();
    let samples = &report.problems.0["invalid content"].samples;
    assert_eq!(
        samples[0].detail,
        "<entry>/0<required>: missing required attr \"z\""
    );
    assert_eq!(
        samples[1].detail,
        "<entry>/0<required>: missing required attr \"a\""
    );
}

#[test]
fn reports_sort_kinds_and_cap_samples() {
    let mut problems = Problems::default();
    for i in 0..9 {
        problems.add("z", None, i);
    }
    problems.add("a", Some("e"), "detail");
    let value = serde_json::to_string(&problems).unwrap();
    assert!(value.starts_with("{\"a\":"));
    assert_eq!(problems.0["z"].count, 9);
    assert_eq!(problems.0["z"].samples.len(), 5);
    assert!(problems.0["z"].samples[0].entry.is_none());
}

#[test]
fn dictionary_training_and_large_pack_round_trip() {
    let entries: Vec<_> = (0..1000).map(|i|json!({"type":"entry","id":format!("entry-{i}"),"lang":"en","children":[{"type":"word","text":format!("word-{i} A distinct reading {}",i*i)}]})).collect();
    let f = Fixture::new(&entries);
    let report = f.build();
    assert_eq!(report.entries, 1000);
    assert_eq!(report.meta["zstd_dict"], "zstd.dict");
    let pack = Pack::open(report.output).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&pack.entry_json("entry-999").unwrap()).unwrap(),
        entries[999]
    );
}

#[test]
fn entries_preserve_crlf_source_lines_and_empty_source_is_valid() {
    let f = Fixture::new(&[]);
    f.write("entries.jsonl", "{\"type\":\"entry\",\"id\":\"a\",\"lang\":\"en\"}\r\n{\"type\":\"entry\",\"id\":\"b\",\"lang\":\"en\"}\r");
    let report = f.build();
    assert_eq!(report.entries, 2);
    assert_eq!(report.meta["target_langs"], "[\"zh-Hans\", \"fr\"]");
    let pack = Pack::open(report.output).unwrap();
    assert_eq!(
        pack.entry_json("a").unwrap(),
        "{\"type\":\"entry\",\"id\":\"a\",\"lang\":\"en\"}"
    );
    drop(pack); // Windows cannot replace a file that is open
    f.write("entries.jsonl", "");
    assert_eq!(f.build().entries, 0);
}
