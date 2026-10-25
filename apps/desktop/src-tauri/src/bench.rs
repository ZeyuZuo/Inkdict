//! Benchmark mode: with `ZDICT_BENCH=<out.json>` the UI types the queries
//! of `ZDICT_BENCH_QUERIES` (one per line) key by key, opens the entries of
//! `ZDICT_BENCH_ENTRIES`, and reports input-to-paint times measured in the
//! WebView; the report is written to the output file and the app exits.

use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;
use zdict_app::{AppError, Result};

/// Lines kept from each input file unless `ZDICT_BENCH_LIMIT` says otherwise.
const DEFAULT_LIMIT: usize = 300;

#[derive(Serialize)]
pub struct BenchPlan {
    queries: Vec<String>,
    entries: Vec<String>,
}

fn lines(variable: &str, keep: usize) -> Result<Vec<String>> {
    let Some(path) = std::env::var_os(variable) else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|err| AppError::new("bad_input", format!("{variable}: {err}")))?;
    Ok(text
        .lines()
        .map(|line| {
            line.split('\t')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .take(keep)
        .collect())
}

/// The work to do, or None outside benchmark mode.
#[tauri::command]
pub fn bench_plan() -> Result<Option<BenchPlan>> {
    if std::env::var_os("ZDICT_BENCH").is_none() {
        return Ok(None);
    }
    let limit = std::env::var("ZDICT_BENCH_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_LIMIT);
    Ok(Some(BenchPlan {
        queries: lines("ZDICT_BENCH_QUERIES", limit)?,
        entries: lines("ZDICT_BENCH_ENTRIES", limit)?,
    }))
}

/// Progress from the UI, on stderr.
#[tauri::command]
pub fn bench_log(message: String) {
    eprintln!("bench: {message}");
}

/// Write the UI's measurements and quit.
#[tauri::command]
pub fn bench_report(app: AppHandle, report: serde_json::Value) -> Result<()> {
    let path = PathBuf::from(
        std::env::var_os("ZDICT_BENCH")
            .ok_or_else(|| AppError::new("bad_input", "not benchmarking"))?,
    );
    let text = serde_json::to_string_pretty(&report).expect("JSON value serializes");
    std::fs::write(&path, text)
        .map_err(|err| AppError::new("internal", format!("{}: {err}", path.display())))?;
    app.exit(0);
    Ok(())
}
