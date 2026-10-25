//! Per-call timings, with the same warm-up and statistics as scripts/bench.py.

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use zdict_core::Pack;

use super::print_json;
use crate::args::BenchKind;

pub fn run(
    pack: &Pack,
    kind: BenchKind,
    path: &Path,
    repeat: usize,
    json: bool,
    out: &mut impl Write,
) -> Result<()> {
    let input = std::fs::read_to_string(path)
        .with_context(|| format!("reading queries from {}", path.display()))?;
    let queries: Vec<_> = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    ensure!(
        !queries.is_empty(),
        "{} contains no queries",
        path.display()
    );
    let count = queries
        .len()
        .checked_mul(repeat)
        .context("query count times --repeat is too large")?;
    for query in queries.iter().take(100) {
        call(pack, kind, query)?;
    }
    let mut samples = Vec::with_capacity(count);
    let total = Instant::now();
    for _ in 0..repeat {
        for query in &queries {
            let started = Instant::now();
            call(pack, kind, query)?;
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let total_seconds = total.elapsed().as_secs_f64();
    let stats = statistics(&mut samples, total_seconds);
    if json {
        return print_json(out, &stats);
    }
    writeln!(out, "count: {count}")?;
    for field in ["p50_ms", "p95_ms", "p99_ms", "max_ms"] {
        writeln!(out, "{field}: {:.3}", stats[field].as_f64().unwrap())?;
    }
    writeln!(out, "total_seconds: {total_seconds:.3}")?;
    Ok(())
}

/// Include result construction and disposal in each timed call, in both readers.
fn call(pack: &Pack, kind: BenchKind, query: &str) -> Result<()> {
    match kind {
        BenchKind::Suggest => {
            pack.suggest(query, 20)?;
        }
        BenchKind::Search => {
            pack.search(query, 20, None)?;
        }
        BenchKind::Entry => {
            pack.entry(query)?;
        }
    }
    Ok(())
}

/// Nearest-rank percentiles: sorted sample at ceil(percent * count / 100).
fn statistics(samples: &mut [f64], total_seconds: f64) -> Value {
    samples.sort_unstable_by(f64::total_cmp);
    let percentile = |percent: usize| samples[(percent * samples.len()).div_ceil(100) - 1];
    json!({
        "count": samples.len(),
        "p50_ms": percentile(50),
        "p95_ms": percentile(95),
        "p99_ms": percentile(99),
        "max_ms": samples.last().unwrap(),
        "total_seconds": total_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_handles_single_samples_and_tail_outliers() {
        let one = statistics(&mut [2.0], 0.002);
        assert_eq!(one["p99_ms"], 2.0);
        let mut samples: Vec<_> = (1..=100).rev().map(f64::from).collect();
        let stats = statistics(&mut samples, 5.05);
        assert_eq!(stats["count"], 100);
        assert_eq!(stats["p50_ms"], 50.0);
        assert_eq!(stats["p95_ms"], 95.0);
        assert_eq!(stats["p99_ms"], 99.0);
        assert_eq!(stats["max_ms"], 100.0);
        assert_eq!(stats["total_seconds"], 5.05);
    }
}
