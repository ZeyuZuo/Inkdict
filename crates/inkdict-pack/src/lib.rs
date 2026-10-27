//! Build Inkdict packs from source manifests, preserving source JSON bodies.
mod content;
mod index;
mod readings;
mod report;
mod source;
mod summaries;
mod write;

use std::path::PathBuf;

pub use report::{Problem, Problems, Report, Sample};
pub use source::SourceError;

/// A manifest and optional output/input overrides. Relative overrides are relative
/// to the process's working directory.
#[derive(Debug, Clone)]
pub struct Options {
    pub manifest: PathBuf,
    pub output: Option<PathBuf>,
    pub entries: Option<PathBuf>,
    pub summaries: Option<PathBuf>,
}

impl Options {
    pub fn new(manifest: impl Into<PathBuf>) -> Self {
        Self {
            manifest: manifest.into(),
            output: None,
            entries: None,
            summaries: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Time(#[from] time::error::Format),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Build atomically, returning counts, metadata, and nonfatal content problems.
pub fn build(options: Options) -> Result<Report> {
    write::build(options)
}
