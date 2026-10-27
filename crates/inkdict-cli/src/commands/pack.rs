use anyhow::{Context, Result};
use std::{fs, io::Write, path::PathBuf};

pub fn run(
    manifest: PathBuf,
    output: Option<PathBuf>,
    report: Option<PathBuf>,
    entries: Option<PathBuf>,
    summaries: Option<PathBuf>,
    json: bool,
    out: &mut impl Write,
) -> Result<()> {
    let result = inkdict_pack::build(inkdict_pack::Options {
        manifest,
        output,
        entries,
        summaries,
    })?;
    if let Some(path) = report {
        let mut data = serde_json::to_vec_pretty(&result)?;
        data.push(b'\n');
        fs::write(&path, data).with_context(|| format!("writing {}", path.display()))?;
    }
    if json {
        serde_json::to_writer_pretty(&mut *out, &result)?;
        writeln!(out)?;
    } else {
        writeln!(
            out,
            "{} entries, {} lookup rows, {} summaries",
            result.entries, result.lookup_rows, result.summaries
        )?;
        writeln!(
            out,
            "{} resources, {} links, {} resource references",
            result.resources, result.links, result.resource_refs
        )?;
        for (kind, problem) in &result.problems.0 {
            writeln!(out, "  {kind}: {}", problem.count)?;
        }
        writeln!(out, "{} ({} bytes)", result.output, result.bytes)?;
    }
    Ok(())
}
