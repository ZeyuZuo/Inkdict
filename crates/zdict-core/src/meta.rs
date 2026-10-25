//! The `meta` table (spec/pack-v0.md §2.1).

use std::collections::BTreeMap;

use crate::FORMAT_VERSION;
use crate::error::{Error, MetaProblem, Result};

/// Value of `meta.format` in every pack.
pub const FORMAT_NAME: &str = "zdict-pack";

/// The only body compression of format v0.
pub const COMPRESSION: &str = "zstd";

/// FTS rowids give the language 8 bits (spec/pack-v0.md §4).
const MAX_FTS_LANGS: usize = 256;

/// Keys the reader needs; the rest are informational.
const REQUIRED: [&str; 8] = [
    "format",
    "format_version",
    "compression",
    "fts_langs",
    "id",
    "title",
    "source_lang",
    "target_lang",
];

/// Pack metadata: every key as stored, plus the parsed `fts_langs`.
#[derive(Debug, Clone)]
pub struct Meta {
    values: BTreeMap<String, String>,
    fts_langs: Vec<String>,
}

impl Meta {
    /// Check the keys a reader depends on and parse `fts_langs`.
    pub fn new(values: BTreeMap<String, String>) -> Result<Self> {
        if let Some(key) = REQUIRED.iter().find(|key| !values.contains_key(**key)) {
            return Err(meta_error(key, MetaProblem::Missing));
        }
        expect(&values, "format", FORMAT_NAME)?;
        expect(&values, "format_version", &FORMAT_VERSION.to_string())?;
        expect(&values, "compression", COMPRESSION)?;
        let fts_langs: Vec<String> = serde_json::from_str(&values["fts_langs"]).map_err(|err| {
            meta_error(
                "fts_langs",
                MetaProblem::Invalid(format!("not a JSON array of strings: {err}")),
            )
        })?;
        if !(1..=MAX_FTS_LANGS).contains(&fts_langs.len()) {
            let problem = format!(
                "{} languages, expected 1 to {MAX_FTS_LANGS}",
                fts_langs.len()
            );
            return Err(meta_error("fts_langs", MetaProblem::Invalid(problem)));
        }
        Ok(Meta { values, fts_langs })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// All keys and values, sorted by key.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    pub fn id(&self) -> &str {
        &self.values["id"]
    }

    pub fn title(&self) -> &str {
        &self.values["title"]
    }

    pub fn source_lang(&self) -> &str {
        &self.values["source_lang"]
    }

    pub fn target_lang(&self) -> &str {
        &self.values["target_lang"]
    }

    /// BCP-47 tag of each language number in FTS rowids (spec/pack-v0.md §4).
    pub fn fts_langs(&self) -> &[String] {
        &self.fts_langs
    }

    /// Number of entries as recorded at build time, if present and numeric.
    pub fn entries(&self) -> Option<u64> {
        self.get("entries")?.parse().ok()
    }

    /// Name of the zstd dictionary in `files`; absent for small packs.
    /// An empty name means no dictionary, as in the Python reader.
    pub fn zstd_dict(&self) -> Option<&str> {
        self.get("zstd_dict").filter(|name| !name.is_empty())
    }
}

fn expect(values: &BTreeMap<String, String>, key: &str, wanted: &str) -> Result<()> {
    let value = &values[key];
    if value == wanted {
        Ok(())
    } else {
        Err(meta_error(
            key,
            MetaProblem::Invalid(format!("{value:?}, expected {wanted:?}")),
        ))
    }
}

fn meta_error(key: &str, problem: MetaProblem) -> Error {
    Error::Meta {
        key: key.to_owned(),
        problem,
    }
}
