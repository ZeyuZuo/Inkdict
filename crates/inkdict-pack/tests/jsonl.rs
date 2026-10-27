mod common;
use common::Fixture;
use inkdict_core::Pack;
use inkdict_pack::Error;
use serde_json::{Value, json};

#[test]
fn crlf_and_unterminated_records_work_for_both_inputs() {
    let f = Fixture::new(&[]);
    let entries: Vec<_> = (1..=3)
        .map(|n| json!({"id":format!("e{n}"),"lang":"en","type":"entry"}).to_string())
        .collect();
    f.write(
        "entries.jsonl",
        &format!("{}\r\n{}\n{}\r", entries[0], entries[1], entries[2]),
    );
    let texts = ["one\u{85}two\u{2028}three", "second", "third"];
    let summaries: Vec<_> = texts
        .iter()
        .enumerate()
        .map(|(i, text)| json!({"entry":format!("e{}",i+1),"text":text}).to_string())
        .collect();
    f.write(
        "summaries.jsonl",
        &format!("{}\r\n{}\n{}", summaries[0], summaries[1], summaries[2]),
    );
    let mut options = f.options();
    options.summaries = Some(f.path("summaries.jsonl"));
    let report = inkdict_pack::build(options).unwrap();
    assert_eq!((report.entries, report.summaries), (3, 3));
    assert!(report.problems.0.is_empty());
    let pack = Pack::open(&report.output).unwrap();
    assert_eq!(pack.entry_json("e1").unwrap(), entries[0]);
    assert_eq!(pack.entry_json("e2").unwrap(), entries[1]);
    // A final CR without LF is source content, not a record terminator.
    assert_eq!(pack.entry_json("e3").unwrap(), format!("{}\r", entries[2]));
    let saved: Vec<String> = f
        .db()
        .prepare("SELECT text FROM summaries ORDER BY entry")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(saved, texts);
}

#[test]
fn bare_cr_does_not_separate_records() {
    let entry = json!({"id":"e","lang":"en","type":"entry"});
    let summary = json!({"entry":"e","text":"summary"});
    for summaries in [false, true] {
        let f = Fixture::new(std::slice::from_ref(&entry));
        let mut options = f.options();
        let name = if summaries {
            "summaries.jsonl"
        } else {
            "entries.jsonl"
        };
        let record = if summaries { &summary } else { &entry };
        f.write(name, &format!("{record}\r{record}"));
        if summaries {
            options.summaries = Some(f.path(name));
        }
        let error = inkdict_pack::build(options).unwrap_err();
        assert!(matches!(error, Error::Source(_)));
        assert!(
            error
                .to_string()
                .contains(":1: not JSON (trailing characters"),
            "{error}"
        );
    }
}

#[test]
fn blank_lines_keep_their_line_numbers_and_fail_both_inputs() {
    let entry = json!({"id":"e","lang":"en","type":"entry"});
    let summary = json!({"entry":"e","text":"summary"});
    for summaries in [false, true] {
        let f = Fixture::new(std::slice::from_ref(&entry));
        let mut options = f.options();
        let name = if summaries {
            "summaries.jsonl"
        } else {
            "entries.jsonl"
        };
        let record = if summaries { &summary } else { &entry };
        f.write(name, &format!("{record}\r\n\r\n"));
        if summaries {
            options.summaries = Some(f.path(name));
        }
        let error = inkdict_pack::build(options).unwrap_err();
        assert!(matches!(error, Error::Source(_)));
        let detail = serde_json::from_slice::<Value>(b"").unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("{}:2: not JSON ({detail})", f.path(name).display())
        );
    }
}

#[test]
fn bad_json_uses_native_serde_errors_in_both_inputs() {
    let entry = json!({"id":"e","lang":"en","type":"entry"});
    for bad in ["bad", "{", "[1,]", "\"\\z\"", "\u{feff}{}"] {
        for summaries in [false, true] {
            let f = Fixture::new(std::slice::from_ref(&entry));
            let mut options = f.options();
            let name = if summaries {
                "summaries.jsonl"
            } else {
                "entries.jsonl"
            };
            f.write(name, bad);
            if summaries {
                options.summaries = Some(f.path(name));
            }
            let error = inkdict_pack::build(options).unwrap_err();
            assert!(matches!(error, Error::Source(_)));
            let detail = serde_json::from_slice::<Value>(bad.as_bytes()).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("{}:1: not JSON ({detail})", f.path(name).display())
            );
        }
    }
}
