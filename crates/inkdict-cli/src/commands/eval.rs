//! Top-1 and top-5 headword recall for spelling correction.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde_json::json;
use inkdict_core::Pack;

use super::print_json;

pub fn run(pack: &Pack, path: &Path, json: bool, out: &mut impl Write) -> Result<()> {
    let input = std::fs::read_to_string(path)
        .with_context(|| format!("reading correction cases from {}", path.display()))?;
    let cases = cases(&input).with_context(|| format!("in {}", path.display()))?;
    let mut top1 = 0;
    let mut top5 = 0;
    let mut misses = Vec::new();
    for &(typo, expected) in &cases {
        let hits = pack.words(typo, 5)?;
        let got: Vec<_> = hits.iter().map(|hit| hit.headword.as_str()).collect();
        top1 += usize::from(got.first().copied() == Some(expected));
        if got.iter().take(5).any(|headword| *headword == expected) {
            top5 += 1;
        } else if misses.len() < 20 {
            misses.push(json!({ "typo": typo, "expected": expected, "got": got }));
        }
    }
    let count = cases.len();
    let percent = |hits| hits as f64 * 100.0 / count as f64;
    let stats = json!({
        "count": count,
        "top1": top1,
        "top1_percent": percent(top1),
        "top5": top5,
        "top5_percent": percent(top5),
        "misses": count - top5,
        "miss_percent": percent(count - top5),
        "first_misses": misses,
    });
    if json {
        return print_json(out, &stats);
    }
    writeln!(out, "count: {count}")?;
    writeln!(out, "top1: {top1} ({:.2}%)", percent(top1))?;
    writeln!(out, "top5: {top5} ({:.2}%)", percent(top5))?;
    writeln!(
        out,
        "misses: {} ({:.2}%)",
        count - top5,
        percent(count - top5)
    )?;
    for miss in misses {
        writeln!(
            out,
            "{} -> {}: got {}",
            miss["typo"].as_str().unwrap(),
            miss["expected"].as_str().unwrap(),
            miss["got"]
        )?;
    }
    Ok(())
}

fn cases(input: &str) -> Result<Vec<(&str, &str)>> {
    let mut cases = Vec::new();
    for (index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let (typo, expected) = line
            .split_once('\t')
            .with_context(|| format!("line {}: expected typo<TAB>expected_headword", index + 1))?;
        ensure!(
            !typo.trim().is_empty() && !expected.trim().is_empty() && !expected.contains('\t'),
            "line {}: expected two nonempty TSV columns",
            index + 1
        );
        cases.push((typo, expected));
    }
    ensure!(!cases.is_empty(), "contains no correction cases");
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_skip_blank_lines_and_report_invalid_line_numbers() {
        assert_eq!(
            cases("\nrecieve\treceive\r\n\t\n").unwrap(),
            [("recieve", "receive")]
        );
        assert!(cases(" \n").is_err());
        for invalid in ["missing tab", "typo\t", "\tword", "typo\tword\textra"] {
            let error = cases(&format!("a\ta\n{invalid}\n")).unwrap_err();
            assert!(error.to_string().contains("line 2"), "{error}");
        }
    }
}
