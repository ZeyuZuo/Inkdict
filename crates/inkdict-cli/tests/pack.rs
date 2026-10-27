//! Pack commands work without an existing dictionary or oxdata fixtures.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Directory(PathBuf);
impl Directory {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("inkdict-pack-cli-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_inkdict"));
    cmd.env_remove("INKDICT_PACK");
    cmd
}
fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/samples/en-zh/manifest.toml")
}

#[test]
fn builds_and_saves_full_report_without_pack_argument() {
    let dir = Directory::new("report");
    let pack = dir.0.join("sample.inkdict");
    let report = dir.0.join("sample.json");
    let output = command()
        .arg("pack")
        .arg(sample())
        .arg("-o")
        .arg(&pack)
        .arg("--report")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("6 entries, 23 lookup rows, 10 summaries")
    );
    let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(report["resources"], 4);
    let output = command()
        .arg("--pack")
        .arg(&pack)
        .args(["words", "went", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let words: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(words.as_array().unwrap().iter().any(|w| w["entry"] == "go"));
}

#[test]
fn json_and_overrides_and_default_output() {
    let dir = Directory::new("json");
    let source = sample().parent().unwrap().to_path_buf();
    let manifest = fs::read_to_string(sample()).unwrap();
    // Point registry/theme/resources at the checked-in sample; default output
    // stays beside this copied manifest.
    let manifest = manifest
        .replace("types.toml", &source.join("types.toml").to_string_lossy())
        .replace(
            "themes/default.toml",
            &source.join("themes/default.toml").to_string_lossy(),
        )
        .replace(
            "resources = \"resources\"",
            &format!("resources = {:?}", source.join("resources")),
        );
    fs::write(dir.0.join("manifest.toml"), manifest).unwrap();
    fs::write(
        dir.0.join("entries.jsonl"),
        "{\"type\":\"entry\",\"id\":\"e\",\"lang\":\"en\"}",
    )
    .unwrap();
    fs::write(
        dir.0.join("summaries.jsonl"),
        "{\"entry\":\"e\",\"text\":\"summary\"}",
    )
    .unwrap();
    let output = command()
        .arg("pack")
        .arg(dir.0.join("manifest.toml"))
        .arg("--entries")
        .arg(dir.0.join("entries.jsonl"))
        .arg("--summaries")
        .arg(dir.0.join("summaries.jsonl"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["entries"], 1);
    assert_eq!(report["summaries"], 1);
    assert_eq!(
        report["output"],
        dir.0
            .join("sample-en-zh.inkdict")
            .to_string_lossy()
            .as_ref()
    );
    assert!(dir.0.join("sample-en-zh.inkdict").is_file());
}

#[test]
fn source_error_exits_nonzero_without_creating_output() {
    let dir = Directory::new("error");
    fs::write(
        dir.0.join("manifest.toml"),
        "[meta]\nformat='bad'\nversion=1",
    )
    .unwrap();
    let output = command()
        .arg("pack")
        .arg(dir.0.join("manifest.toml"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not an inkdict-source v1 manifest"));
}
