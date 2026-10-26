//! Command-line arguments shared by the command modules.

use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Look up words and measure dictionary pack performance.
#[derive(Parser)]
#[command(name = "zdict", version, about)]
pub struct Args {
    /// Dictionary pack; defaults to ZDICT_PACK.
    #[arg(long, global = true, env = "ZDICT_PACK", value_name = "PATH")]
    pub pack: Option<PathBuf>,
    /// Print machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List metadata and embedded file names.
    Info,
    /// Serve the desktop app's commands over HTTP on localhost, so its UI
    /// can be developed in a browser with real data (VITE_ZDICT_BRIDGE).
    Serve {
        #[arg(long, default_value_t = 1430)]
        port: u16,
    },
    /// List prefix matches, contained forms and close spellings.
    Words {
        text: String,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },
    /// Show an entry by key, or the first word-list row.
    Show { key_or_word: String },
    /// Print a themed render tree as pretty JSON.
    Render {
        key_or_word: String,
        #[arg(long, default_value = "default")]
        theme: String,
        #[arg(long = "overlay", value_name = "FILE")]
        overlays: Vec<PathBuf>,
    },
    /// List pack themes and validate each against the type registry.
    Themes,
    /// Evaluate spelling correction against expected headwords.
    Eval {
        #[command(subcommand)]
        command: EvalCommand,
    },
    /// Time queries after warming up on the first 100 lines.
    Bench {
        #[arg(value_enum)]
        kind: BenchKind,
        queries_file: PathBuf,
        #[arg(long, default_value = "1")]
        repeat: NonZeroUsize,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum BenchKind {
    Words,
    Entry,
    Render,
}

#[derive(Subcommand)]
pub enum EvalCommand {
    /// Read typo<TAB>expected_headword cases and measure top-1/top-5 recall.
    Correct { cases_file: PathBuf },
}
