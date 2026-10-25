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
    struct Queries(PathBuf);
    impl Drop for Queries {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let queries =
        Queries(std::env::temp_dir().join(format!("zdict-cli-bench-{}.txt", std::process::id())));
    std::fs::write(&queries.0, "went\n\napple\r\n").unwrap();
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
