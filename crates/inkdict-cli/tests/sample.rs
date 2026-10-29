mod common;

use serde_json::{Value, json};
use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_inkdict"))
        .env_remove("INKDICT_PACK")
        .arg("--pack")
        .arg(common::sample_pack())
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn json(args: &[&str]) -> Value {
    let mut args = args.to_vec();
    args.push("--json");
    serde_json::from_str(&run(&args)).unwrap()
}

#[test]
fn sample_words_text_json_and_group_limits() {
    let text = run(&["words", "lead"]);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(
        lines[0],
        "prefix\tlead\theadword\tlead 1\tlead-1#0\tv. 带路；引领"
    );
    assert_eq!(
        lines[1],
        "prefix\tlead\theadword\tlead 2\tlead-2#0\tn. 铅；铅笔芯"
    );
    assert_eq!(
        lines[2],
        "prefix\tleading\tinflection\tlead 1\tlead-1#1\tv. 带路；引领"
    );
    let words = json(&["words", "lead"]);
    assert_eq!(words.as_array().unwrap().len(), 5);
    for (i, (form, key, homograph, anchor)) in [
        ("lead", "lead-1", 1, 0),
        ("lead", "lead-2", 2, 0),
        ("leading", "lead-1", 1, 1),
        ("lead the way", "lead-1", 1, 4),
        ("lead sb the way", "lead-1", 1, 4),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(words[i]["form"], form);
        assert_eq!(words[i]["entry"], key);
        assert_eq!(words[i]["homograph"], homograph);
        assert_eq!(words[i]["anchor"], anchor);
        assert_eq!(words[i]["group"], "prefix");
        assert_eq!(words[i]["summary_lang"], "zh-Hans");
    }
    assert_eq!(json(&["words", "lead", "-n", "0"]), json!([]));
    let limited = json(&["words", "har", "-n", "1"]);
    assert_eq!(limited.as_array().unwrap().len(), 2);
    assert_eq!(limited[0]["group"], "prefix");
    assert_eq!(limited[0]["form"], "harbor");
    assert_eq!(limited[1]["group"], "contains");
    assert_eq!(limited[1]["form"], "safe harbour");
    assert_eq!(
        run(&["words", "harbur", "-n", "1"]),
        "close\tharbour\theadword\tharbour\tharbour#0\tn. 港口；避风港　v. 心怀\t1\n"
    );
    for (query, form, key, kind) in [
        ("CAFÉ", "café", "cafe", "headword"),
        ("cafe", "café", "cafe", "headword"),
        ("go flat", "go flat out", "go", "phrase"),
        ("lead sb", "lead sb the way", "lead-1", "phrase"),
        ("leads", "lead", "lead-1", "hidden"),
    ] {
        let words = json(&["words", query]);
        assert_eq!(words[0]["form"], form);
        assert_eq!(words[0]["entry"], key);
        assert_eq!(words[0]["kind"], kind);
    }
}

#[test]
fn sample_show_text_json_resolves_keys_forms_and_spelling() {
    for (query, key, headword) in [
        ("harbour", "harbour", "harbour"),
        ("harbor", "harbour", "harbour"),
        ("harbur", "harbour", "harbour"),
        ("went", "go", "go"),
        ("CAFÉ", "cafe", "café"),
        ("lead-2", "lead-2", "lead"),
    ] {
        let entry = json(&["show", query]);
        assert_eq!(entry["type"], "entry");
        assert_eq!(entry["id"], key);
        assert_eq!(entry["lang"], "en");
        assert_eq!(entry["children"][0]["children"][0]["text"], headword);
        assert!(
            run(&["show", query])
                .lines()
                .any(|line| line.trim() == format!("headword: {headword}"))
        );
    }
    assert!(run(&["show", "harbour"]).contains("definition: 港口；海港"));
}

#[test]
fn sample_render_json_and_overlay_order() {
    // Render always emits JSON, including when the global --json flag is absent.
    let tree: Value = serde_json::from_str(&run(&["render", "harbour"])).unwrap();
    assert_eq!(tree, json(&["render", "harbour"]));
    assert_eq!(tree["type"], "entry");
    assert_eq!(tree["anchor"], json!({"id":"harbour", "number":0}));
    let tabs = tree["children"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(Value::as_array)
        .unwrap();
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[0]["label"]["children"], json!(["noun"]));
    assert_eq!(tabs[1]["label"]["children"], json!(["verb"]));
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (first, second, size) in [
        ("overlay-first.toml", "overlay-second.toml", 1.75),
        ("overlay-second.toml", "overlay-first.toml", 1.25),
    ] {
        let first = fixtures.join(first);
        let second = fixtures.join(second);
        let tree = json(&[
            "render",
            "harbour",
            "--overlay",
            first.to_str().unwrap(),
            "--overlay",
            second.to_str().unwrap(),
        ]);
        assert_eq!(tree["style"]["size"], size);
        assert_eq!(tree["text"]["size"], size);
    }
}

#[test]
fn sample_themes_and_info_text_json() {
    assert_eq!(run(&["themes"]), "default\nplain\n");
    assert_eq!(
        json(&["themes"]),
        json!([{"name":"default", "problems":[]}, {"name":"plain", "problems":[]}])
    );
    let info = json(&["info"]);
    for (key, value) in [
        ("id", "sample-en-zh"),
        ("title", "示例英汉双解词典"),
        ("entries", "6"),
        ("norm", "fold-1"),
        ("resources", "4"),
        ("target_langs", "[\"zh-Hans\"]"),
        ("themes", "[\"default\", \"plain\"]"),
    ] {
        assert_eq!(info["meta"][key], value);
        assert!(
            run(&["info"])
                .lines()
                .any(|line| line == format!("{key}\t{value}"))
        );
    }
    assert_eq!(
        info["files"],
        json!(["themes/default.toml", "themes/plain.toml", "types.toml"])
    );
    assert!(
        run(&["info"])
            .contains("files:\n  themes/default.toml\n  themes/plain.toml\n  types.toml\n")
    );
}
