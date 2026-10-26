//! The `meta` table (spec/pack-v1.md §2.2).

use std::collections::BTreeMap;

use crate::FORMAT_VERSION;
use crate::error::{Error, MetaProblem, Result};
use crate::text::Scheme;

/// Value of `meta.format` in every pack.
pub const FORMAT_NAME: &str = "zdict-pack";

/// The only body compression of format v1.
pub const COMPRESSION: &str = "zstd";

/// Keys the reader needs; the rest are informational.
const REQUIRED: [&str; 10] = [
    "format",
    "format_version",
    "compression",
    "id",
    "title",
    "source_lang",
    "target_langs",
    "themes",
    "norm",
    "unicode_version",
];

/// Pack metadata: every key as stored, plus the parsed JSON lists and lookup scheme.
#[derive(Debug, Clone)]
pub struct Meta {
    values: BTreeMap<String, String>,
    target_langs: Vec<String>,
    themes: Vec<String>,
    norm: Scheme,
}

impl Meta {
    /// Check the keys a reader depends on and parse the lists and `norm`.
    pub fn new(values: BTreeMap<String, String>) -> Result<Self> {
        if let Some(key) = REQUIRED.iter().find(|key| !values.contains_key(**key)) {
            return Err(meta_error(key, MetaProblem::Missing));
        }
        expect(&values, "format", FORMAT_NAME)?;
        expect(&values, "format_version", &FORMAT_VERSION.to_string())?;
        expect(&values, "compression", COMPRESSION)?;
        let target_langs = strings(&values, "target_langs")?;
        let themes = strings(&values, "themes")?;
        if themes.is_empty() {
            let problem = MetaProblem::Invalid("no theme".to_owned());
            return Err(meta_error("themes", problem));
        }
        let norm = Scheme::parse(&values["norm"]).ok_or_else(|| {
            let problem = format!("{:?}, expected \"fold-1\" or \"exact-1\"", values["norm"]);
            meta_error("norm", MetaProblem::Invalid(problem))
        })?;
        Ok(Meta {
            values,
            target_langs,
            themes,
            norm,
        })
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

    pub fn edition(&self) -> Option<&str> {
        self.get("edition")
    }

    pub fn license(&self) -> Option<&str> {
        self.get("license")
    }

    /// Language of headwords and source text (BCP 47).
    pub fn source_lang(&self) -> &str {
        &self.values["source_lang"]
    }

    /// Languages of translations; empty for a monolingual dictionary.
    pub fn target_langs(&self) -> &[String] {
        &self.target_langs
    }

    /// Theme names, `themes/<name>.toml` in `files`; the first is the default.
    pub fn themes(&self) -> &[String] {
        &self.themes
    }

    pub fn default_theme(&self) -> &str {
        &self.themes[0]
    }

    /// Lookup key scheme the pack was built with; queries use the same one.
    pub fn norm(&self) -> Scheme {
        self.norm
    }

    /// Unicode version of the tables the lookup keys were built with.
    pub fn unicode_version(&self) -> &str {
        &self.values["unicode_version"]
    }

    /// Number of entries as recorded at build time, if present and numeric.
    pub fn entries(&self) -> Option<u64> {
        self.get("entries")?.parse().ok()
    }

    /// Number of resources stored in the pack, if present and numeric.
    pub fn resources(&self) -> Option<u64> {
        self.get("resources")?.parse().ok()
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

/// A JSON array of strings.
fn strings(values: &BTreeMap<String, String>, key: &str) -> Result<Vec<String>> {
    serde_json::from_str(&values[key]).map_err(|err| {
        meta_error(
            key,
            MetaProblem::Invalid(format!("not a JSON array of strings: {err}")),
        )
    })
}

fn meta_error(key: &str, problem: MetaProblem) -> Error {
    Error::Meta {
        key: key.to_owned(),
        problem,
    }
}
