//! Exercise the built binary against the optional local Oxford pack.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

fn pack() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata/oalecd10.zdpack");
    if !path.is_file() {
        eprintln!("skipped: {} missing", path.display());
        return None;
    }
    Some(path)
}

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zdict"));
    command.env_remove("ZDICT_PACK");
    command
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "zdict failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

struct Queries(PathBuf);

impl Queries {
    fn new(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("zdict-cli-{name}-{}.txt", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        Self(path)
    }
}

impl Drop for Queries {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn correct_recieve_prefers_receive_and_supports_json_and_limits() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["correct", "recieve"])
            .output()
            .unwrap(),
    );
    let fields: Vec<_> = output.lines().next().unwrap().split('\t').collect();
    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0], "receive");
    assert_eq!(fields[1], "1");
    assert_eq!(fields[2].split_whitespace().next(), Some("receive"));
    assert!(fields[3].contains('#'));
    assert!(output.lines().count() <= 10);
    for limit in [0, 1] {
        let output = success(
            command()
                .arg("--pack")
                .arg(&pack)
                .args(["correct", "recieve", "-n", &limit.to_string(), "--json"])
                .output()
                .unwrap(),
        );
        let hits: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(hits.as_array().unwrap().len(), limit);
        if limit == 1 {
            assert_eq!(hits[0]["form"], "receive");
            assert_eq!(hits[0]["distance"], 1);
        }
    }
}

#[test]
fn render_resolves_apple_and_keys_and_applies_overlays_in_order() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["render", "apple"])
            .output()
            .unwrap(),
    );
    let tree: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(tree["type"], "entry");
    assert!(tree["children"].is_array());
    assert!(tree["style"].is_object());
    let key = tree["anchor"]["id"].as_str().unwrap();
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["render", key, "--theme", "default"])
            .output()
            .unwrap(),
    );
    assert_eq!(tree, serde_json::from_str::<Value>(&output).unwrap());
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (first, second, expected) in [
        ("overlay-first.toml", "overlay-second.toml", 1.75),
        ("overlay-second.toml", "overlay-first.toml", 1.25),
    ] {
        let output = success(
            command()
                .arg("--pack")
                .arg(&pack)
                .args(["render", "apple", "--overlay"])
                .arg(fixtures.join(first))
                .arg("--overlay")
                .arg(fixtures.join(second))
                .output()
                .unwrap(),
        );
        let tree: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(tree["style"]["size"], expected);
        assert_eq!(tree["text"]["size"], expected);
    }
}

#[test]
fn themes_validate_the_shipped_themes() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .arg("themes")
            .output()
            .unwrap(),
    );
    assert!(output.lines().any(|line| line == "default"));
    let output = success(
        command()
            .arg("--pack")
            .arg(pack)
            .args(["themes", "--json"])
            .output()
            .unwrap(),
    );
    let themes: Value = serde_json::from_str(&output).unwrap();
    let themes = themes.as_array().unwrap();
    assert!(themes.iter().any(|theme| theme["name"] == "default"));
    assert!(
        themes
            .iter()
            .all(|theme| theme["problems"] == serde_json::json!([]))
    );
}

#[test]
fn eval_correct_counts_hits_and_misses_and_rejects_bad_tsv() {
    let Some(pack) = pack() else { return };
    let queries = Queries::new(
        "eval",
        "recieve\treceive\n\napple\tapple\nrecieve\trelieve\nzzzzzzzzzzzz\treceive\n",
    );
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["eval", "correct"])
            .arg(&queries.0)
            .arg("--json")
            .output()
            .unwrap(),
    );
    let stats: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(stats["count"], 4);
    assert_eq!(stats["top1"], 2);
    assert_eq!(stats["top5"], 3);
    assert_eq!(stats["misses"], 1);
    assert_eq!(stats["top1_percent"], 50.0);
    assert_eq!(stats["top5_percent"], 75.0);
    assert_eq!(stats["first_misses"][0]["typo"], "zzzzzzzzzzzz");
    assert_eq!(stats["first_misses"][0]["expected"], "receive");
    assert_eq!(stats["first_misses"][0]["got"], serde_json::json!([]));
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["eval", "correct"])
            .arg(&queries.0)
            .output()
            .unwrap(),
    );
    assert!(output.contains("top1: 2 (50.00%)"));
    assert!(output.contains("zzzzzzzzzzzz -> receive: got []"));
    std::fs::write(&queries.0, "zzzzzzzzzzzz\treceive\n".repeat(25)).unwrap();
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["eval", "correct"])
            .arg(&queries.0)
            .arg("--json")
            .output()
            .unwrap(),
    );
    let stats: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(stats["misses"], 25);
    assert_eq!(stats["first_misses"].as_array().unwrap().len(), 20);
    std::fs::write(&queries.0, "recieve\treceive\nno-tab\n").unwrap();
    let output = command()
        .arg("--pack")
        .arg(&pack)
        .args(["eval", "correct"])
        .arg(&queries.0)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("line 2"));
}

#[test]
fn correct_and_render_benchmarks_report_timings_and_preparation() {
    let Some(pack) = pack() else { return };
    let queries = Queries::new("bench-correct", "recieve\treceive\n\naple\tapple\n");
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["bench", "correct"])
            .arg(&queries.0)
            .args(["--repeat", "2", "--json"])
            .output()
            .unwrap(),
    );
    let stats: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(stats["count"], 4);
    assert!(stats["prepare_ms"].as_f64().unwrap() > 0.0);
    assert!(stats["p50_ms"].as_f64().unwrap() > 0.0);

    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["show", "apple", "--json"])
            .output()
            .unwrap(),
    );
    let entry: Value = serde_json::from_str(&output).unwrap();
    std::fs::write(&queries.0, format!("{}\n", entry["id"].as_str().unwrap())).unwrap();
    let output = success(
        command()
            .arg("--pack")
            .arg(pack)
            .args(["bench", "render"])
            .arg(&queries.0)
            .arg("--json")
            .output()
            .unwrap(),
    );
    let stats: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(stats["count"], 1);
    assert!(stats["p50_ms"].as_f64().unwrap() > 0.0);
    assert!(stats.get("prepare_ms").is_none());
}

#[test]
fn missing_pack_is_an_error() {
    let output = command().arg("info").output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("--pack"), "{error}");
    assert!(error.contains("ZDICT_PACK"), "{error}");
}

#[test]
fn went_suggests_go_as_an_inflected_form() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(pack)
            .args(["suggest", "went"])
            .output()
            .unwrap(),
    );
    assert!(output.lines().any(|line| {
        let fields: Vec<_> = line.split('\t').collect();
        fields.len() == 4
            && fields[0] == "went"
            && fields[1] == "infl_form"
            && fields[2].split_whitespace().next() == Some("go")
            && fields[3].contains('#')
    }));
}

#[test]
fn chinese_search_returns_hits_and_supports_json_and_columns() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["search", "苹果"])
            .output()
            .unwrap(),
    );
    assert!(!output.is_empty());
    for line in output.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].split('.').nth(1).unwrap().len(), 4);
        assert!(fields[3].starts_with("zh"));
    }
    for column in ["text", "example"] {
        let output = success(
            command()
                .arg("--pack")
                .arg(&pack)
                .args(["search", "苹果", "--column", column, "-n", "3", "--json"])
                .output()
                .unwrap(),
        );
        let hits: Value = serde_json::from_str(&output).unwrap();
        assert!(hits.as_array().unwrap().len() <= 3);
    }
}

#[test]
fn show_resolves_words_and_keys_to_entry_json() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["show", "apple", "--json"])
            .output()
            .unwrap(),
    );
    let entry: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(entry["type"], "entry");
    let key = entry["id"].as_str().unwrap();
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["show", key, "--json"])
            .output()
            .unwrap(),
    );
    assert_eq!(entry, serde_json::from_str::<Value>(&output).unwrap());
    let output = success(
        command()
            .arg("--pack")
            .arg(pack)
            .args(["show", key])
            .output()
            .unwrap(),
    );
    assert!(output.lines().any(|line| line.trim() == "headword: apple"));
}

#[test]
fn pack_environment_fallback_and_explicit_override() {
    let Some(pack) = pack() else { return };
    let output = success(
        command()
            .env("ZDICT_PACK", &pack)
            .args(["info", "--json"])
            .output()
            .unwrap(),
    );
    let info: Value = serde_json::from_str(&output).unwrap();
    assert!(info["meta"]["title"].is_string());
    assert!(!info["files"].as_array().unwrap().is_empty());
    let output = success(
        command()
            .env("ZDICT_PACK", "missing-pack.zdpack")
            .args(["suggest", "went", "--pack"])
            .arg(pack)
            .args(["-n", "1", "--json"])
            .output()
            .unwrap(),
    );
    let hits: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["form"], "went");
}

#[test]
fn repeat_zero_is_rejected_before_opening_a_pack() {
    let output = command()
        .args(["bench", "suggest", "queries.txt", "--repeat", "0"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--repeat"));
}

#[test]
fn benchmarks_count_repeats_and_reject_empty_inputs() {
    let Some(pack) = pack() else { return };
    let queries = Queries::new("bench", "went\n\napple\r\n");
    let output = success(
        command()
            .arg("--pack")
            .arg(&pack)
            .args(["bench", "suggest"])
            .arg(&queries.0)
            .args(["--repeat", "2", "--json"])
            .output()
            .unwrap(),
    );
    let stats: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(stats["count"], 4);
    let timings: Vec<_> = ["p50_ms", "p95_ms", "p99_ms", "max_ms"]
        .map(|field| stats[field].as_f64().unwrap())
        .into();
    assert!(timings.iter().all(|time| *time >= 0.0));
    assert!(timings.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(stats["total_seconds"].as_f64().unwrap() > 0.0);

    std::fs::write(&queries.0, "\n \t\n").unwrap();
    let output = command()
        .arg("--pack")
        .arg(pack)
        .args(["bench", "entry"])
        .arg(&queries.0)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("contains no queries"));
}
