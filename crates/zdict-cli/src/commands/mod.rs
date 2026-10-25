//! Human-readable and JSON output for the public pack API.

mod bench;
mod show;

use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Result;
use serde_json::{Value, json};
use zdict_core::Pack;

use crate::args::Command;

pub fn run(pack: &Pack, command: Command, json: bool, out: &mut impl Write) -> Result<()> {
    match command {
        Command::Info => {
            let meta: BTreeMap<_, _> = pack.meta().iter().collect();
            let files = pack.file_names()?;
            if json {
                return print_json(out, &json!({ "meta": meta, "files": files }));
            }
            for (key, value) in meta {
                writeln!(out, "{key}\t{value}")?;
            }
            writeln!(out, "files:")?;
            for name in files {
                writeln!(out, "  {name}")?;
            }
        }
        Command::Suggest { text, limit } => {
            let hits = pack.suggest(&text, limit)?;
            if json {
                return print_json(out, &serde_json::to_value(hits)?);
            }
            for hit in hits {
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}#{}",
                    hit.form,
                    hit.kind,
                    headword(&hit.headword, hit.homograph),
                    hit.entry,
                    hit.anchor
                )?;
            }
        }
        Command::Search {
            text,
            limit,
            column,
        } => {
            let hits = pack.search(&text, limit, column.map(Into::into))?;
            if json {
                return print_json(out, &serde_json::to_value(hits)?);
            }
            for hit in hits {
                writeln!(
                    out,
                    "{:.4}\t{}\t{}#{}\t{}",
                    hit.score,
                    headword(&hit.headword, hit.homograph),
                    hit.entry,
                    hit.anchor,
                    hit.lang
                )?;
            }
        }
        Command::Show { key_or_word } => return show::run(pack, &key_or_word, json, out),
        Command::Bench {
            kind,
            queries_file,
            repeat,
        } => return bench::run(pack, kind, &queries_file, repeat.get(), json, out),
    }
    Ok(())
}

fn headword(word: &str, homograph: Option<i64>) -> String {
    match homograph {
        Some(number) => format!("{word} {number}"),
        None => word.to_owned(),
    }
}

fn print_json(out: &mut impl Write, value: &Value) -> Result<()> {
    serde_json::to_writer_pretty(&mut *out, value)?;
    writeln!(out)?;
    Ok(())
}
