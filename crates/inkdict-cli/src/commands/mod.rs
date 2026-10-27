//! Human-readable and JSON output for the public pack API.

mod bench;
mod eval;
mod render;
mod show;
mod themes;

use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Result;
use serde_json::{Value, json};
use inkdict_core::{Group, Pack};

use crate::args::{Command, EvalCommand};

pub fn run(pack: &Pack, command: Command, json: bool, out: &mut impl Write) -> Result<()> {
    match command {
        Command::Serve { .. } => unreachable!("handled in main before opening the pack"),
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
        Command::Words { text, limit } => {
            let words = pack.words(&text, limit)?;
            if json {
                return print_json(out, &serde_json::to_value(words)?);
            }
            for word in words {
                let group = match word.group {
                    Group::Prefix => "prefix",
                    Group::Contains => "contains",
                    Group::Close => "close",
                };
                write!(
                    out,
                    "{}\t{}\t{}\t{}\t{}#{}\t{}",
                    group,
                    word.form,
                    word.kind.name(),
                    headword(&word.headword, word.homograph),
                    word.entry,
                    word.anchor,
                    word.summary.as_deref().unwrap_or_default()
                )?;
                if let Some(distance) = word.distance {
                    write!(out, "\t{distance}")?;
                }
                writeln!(out)?;
            }
        }
        Command::Show { key_or_word } => return show::run(pack, &key_or_word, json, out),
        Command::Render {
            key_or_word,
            theme,
            overlays,
        } => return render::run(pack, &key_or_word, &theme, &overlays, out),
        Command::Themes => return themes::run(pack, json, out),
        Command::Eval {
            command: EvalCommand::Correct { cases_file },
        } => return eval::run(pack, &cases_file, json, out),
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
