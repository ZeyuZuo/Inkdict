//! Dictionary lookup and benchmarks for `.zdpack` files.

mod args;
mod commands;
mod serve;

use std::io;

use anyhow::{Context, Result};
use clap::Parser;
use zdict_core::Pack;

use args::{Args, Command};

fn main() -> Result<()> {
    let args = Args::parse();
    let path = args
        .pack
        .context("no pack specified: pass --pack <PATH> or set ZDICT_PACK")?;
    if let Command::Serve { port } = args.command {
        return serve::run(&path, port);
    }
    let pack = Pack::open(&path).with_context(|| format!("opening {}", path.display()))?;
    commands::run(&pack, args.command, args.json, &mut io::stdout().lock())
}
