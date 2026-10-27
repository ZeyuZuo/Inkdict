use std::fmt;

/// Why a pack could not be opened or read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// `PRAGMA application_id` is not `INKD`: some other SQLite file.
    #[error("not an Inkdict pack (application_id {0:#010x})")]
    NotAPack(u32),

    /// `PRAGMA user_version` is a pack format this reader does not know.
    #[error("unsupported pack format version {0}")]
    UnsupportedVersion(u32),

    /// A `meta` key is missing or has a value this reader cannot use.
    #[error("meta {key}: {problem}")]
    Meta { key: String, problem: MetaProblem },

    /// No row with this name in `files`.
    #[error("file not in pack: {0}")]
    MissingFile(String),

    /// A file in `files` (type registry, theme) is not valid.
    #[error("{name}: {reason}")]
    BadFile { name: String, reason: String },

    /// No entry with this key (or row id).
    #[error("entry not in pack: {0}")]
    MissingEntry(String),

    /// An entry body does not decompress or is not a valid node tree.
    #[error("entry {key}: {reason}")]
    BadEntry { key: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetaProblem {
    Missing,
    Invalid(String),
}

impl fmt::Display for MetaProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaProblem::Missing => f.write_str("missing"),
            MetaProblem::Invalid(why) => write!(f, "invalid ({why})"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
