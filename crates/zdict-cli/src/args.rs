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
    /// Suggest forms starting with TEXT.
    Suggest {
        text: String,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },
    /// Search definitions, translations and examples.
    Search {
        text: String,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
        #[arg(long, value_enum)]
        column: Option<SearchColumn>,
    },
    /// Show an entry by key, or the first suggestion for a word.
    Show { key_or_word: String },
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
pub enum SearchColumn {
    Text,
    Example,
}

impl From<SearchColumn> for zdict_core::Column {
    fn from(column: SearchColumn) -> Self {
        match column {
            SearchColumn::Text => Self::Text,
            SearchColumn::Example => Self::Example,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum BenchKind {
    Suggest,
    Search,
    Entry,
}
