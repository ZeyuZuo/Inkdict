use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use inkdict_core::{Registry, text::Scheme, theme::Theme};
use serde_json::Value;

use crate::Result;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SourceError(pub String);

pub(crate) fn bad(message: impl Into<String>) -> SourceError {
    SourceError(message.into())
}

pub(crate) fn read(path: &Path) -> std::result::Result<Vec<u8>, SourceError> {
    fs::read(path).map_err(|e| bad(format!("{}: {e}", path.display())))
}

pub(crate) fn read_text(path: &Path) -> std::result::Result<String, SourceError> {
    String::from_utf8(read(path)?).map_err(|e| bad(format!("{}: {e}", path.display())))
}

pub(crate) struct Source {
    pub pack: toml::Table,
    pub source_lang: String,
    pub targets: Vec<String>,
    pub scheme: Scheme,
    pub boost: Vec<String>,
    pub sources: Vec<(String, String)>,
    pub types: PathBuf,
    pub themes: Vec<PathBuf>,
    pub entries: PathBuf,
    pub summaries: Option<PathBuf>,
    pub resources: Option<PathBuf>,
}

pub(crate) fn valid_lang(text: &str) -> bool {
    let mut parts = text.split('-');
    let primary = parts.next().unwrap_or_default();
    (2..=3).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_lowercase())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

pub(crate) fn valid_res(text: &str) -> bool {
    let Some((kind, key)) = text.split_once(':') else {
        return false;
    };
    kind.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && kind
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
        && !key.is_empty()
        && !key.contains(['\r', '\n'])
}

fn strings(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect()
}

pub(crate) fn load_source(path: &Path) -> std::result::Result<Source, SourceError> {
    let data: toml::Table = read_text(path)?
        .parse()
        .map_err(|e| bad(format!("{}: {e}", path.display())))?;
    let empty = toml::Table::new();
    let table = |name: &str| {
        data.get(name)
            .and_then(toml::Value::as_table)
            .unwrap_or(&empty)
    };
    let (meta, pack, languages, files, lookup) = (
        table("meta"),
        table("pack"),
        table("languages"),
        table("files"),
        table("lookup"),
    );
    let error = |s: &str| bad(format!("{}: {s}", path.display()));
    if meta.get("format").and_then(toml::Value::as_str) != Some("inkdict-source")
        || meta.get("version").and_then(toml::Value::as_integer) != Some(1)
    {
        return Err(error("not an inkdict-source v1 manifest"));
    }
    let id = pack
        .get("id")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    if !id.split('-').all(|p| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    }) || pack
        .get("title")
        .and_then(toml::Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(error(
            "pack.id (lowercase letters, digits, -) and pack.title are required",
        ));
    }
    let source_lang = languages
        .get("source")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let targets = strings(languages.get("targets"));
    if !valid_lang(source_lang)
        || targets
            .as_ref()
            .is_none_or(|t| !t.iter().all(|s| valid_lang(s)))
    {
        return Err(error(
            "languages.source and languages.targets must be BCP 47 tags",
        ));
    }
    let types = files
        .get("types")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty());
    let entries = files
        .get("entries")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty());
    let themes = strings(files.get("themes")).filter(|t| !t.is_empty());
    let (Some(types), Some(entries), Some(themes)) = (types, entries, themes) else {
        return Err(error(
            "files.types, files.themes and files.entries are required",
        ));
    };
    let Some(scheme) = lookup
        .get("norm")
        .and_then(toml::Value::as_str)
        .and_then(Scheme::parse)
    else {
        return Err(error("lookup.norm must be fold-1 or exact-1"));
    };
    let boost = if lookup.contains_key("boost") {
        strings(lookup.get("boost"))
            .ok_or_else(|| error("lookup.boost must be a list of selectors"))?
    } else {
        Vec::new()
    };
    let mut sources = Vec::new();
    if let Some(items) = data.get("source") {
        let msg = "each [[source]] needs a kind and a template with {key}";
        for item in items.as_array().ok_or_else(|| error(msg))? {
            let kind = item
                .get("kind")
                .and_then(toml::Value::as_str)
                .unwrap_or_default();
            let template = item
                .get("template")
                .and_then(toml::Value::as_str)
                .unwrap_or_default();
            if !valid_res(&format!("{kind}:x")) || !template.contains("{key") {
                return Err(error(msg));
            }
            sources.push((kind.to_owned(), template.to_owned()));
        }
    }
    let base = fs::canonicalize(path)
        .map_err(|e| error(&e.to_string()))?
        .parent()
        .unwrap()
        .to_owned();
    let optional = |name| {
        files
            .get(name)
            .and_then(toml::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|s| base.join(s))
    };
    Ok(Source {
        pack: pack.clone(),
        source_lang: source_lang.to_owned(),
        targets: targets.unwrap(),
        scheme,
        boost,
        sources,
        types: base.join(types),
        themes: themes.iter().map(|t| base.join(t)).collect(),
        entries: base.join(entries),
        summaries: optional("summaries"),
        resources: optional("resources"),
    })
}

impl Source {
    pub fn registry(&self) -> Result<Registry> {
        for (name, path) in [
            ("entries", Some(&self.entries)),
            ("summaries", self.summaries.as_ref()),
        ] {
            if let Some(path) = path
                && !path.is_file()
            {
                return Err(bad(format!("{}: {name} file not found", path.display())).into());
            }
        }
        let registry = Registry::parse(&read_text(&self.types)?)
            .map_err(|e| bad(format!("{}: {e}", self.types.display())))?;
        for path in &self.themes {
            let theme = Theme::parse(&path.to_string_lossy(), &read_text(path)?)
                .map_err(|e| bad(format!("{}: {e}", path.display())))?;
            let errors = theme.validate(&registry);
            if let Some(first) = errors.first() {
                return Err(bad(format!(
                    "{}: {first} ({} problems)",
                    path.display(),
                    errors.len()
                ))
                .into());
            }
        }
        Ok(registry)
    }
}

pub(crate) struct Entries {
    pub raw: Vec<u8>,
    pub trees: Vec<Value>,
}

pub(crate) fn read_entries(path: &Path) -> Result<Entries> {
    let raw = read(path)?;
    let mut trees = Vec::new();
    let mut keys = HashSet::new();
    for (i, line) in jsonl_lines(&raw).enumerate() {
        let entry = parse_json(path, i + 1, line)?;
        let Some(key) = entry
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return Err(bad(format!(
                "{}:{}: the root needs an id and a lang",
                path.display(),
                i + 1
            ))
            .into());
        };
        if !entry.get("lang").is_some_and(Value::is_string) {
            return Err(bad(format!(
                "{}:{}: the root needs an id and a lang",
                path.display(),
                i + 1
            ))
            .into());
        }
        if !keys.insert(key.to_owned()) {
            return Err(bad(format!(
                "{}:{}: duplicate entry key {key}",
                path.display(),
                i + 1
            ))
            .into());
        }
        trees.push(entry);
    }
    Ok(Entries { raw, trees })
}

/// JSONL records end at LF; drop a CR only when it immediately precedes that LF.
pub(crate) fn jsonl_lines(raw: &[u8]) -> impl Iterator<Item = &[u8]> {
    raw.split_inclusive(|&byte| byte == b'\n')
        .map(|line| match line.strip_suffix(b"\n") {
            Some(line) => line.strip_suffix(b"\r").unwrap_or(line),
            None => line,
        })
}

pub(crate) fn parse_json(
    path: &Path,
    number: usize,
    line: &[u8],
) -> std::result::Result<Value, SourceError> {
    serde_json::from_slice(line)
        .map_err(|e| bad(format!("{}:{number}: not JSON ({e})", path.display())))
}

pub(crate) type Resources = BTreeMap<(String, String), (&'static str, PathBuf)>;

pub(crate) fn read_resources(directory: Option<&Path>) -> Result<Resources> {
    let mut found = BTreeMap::new();
    let Some(directory) = directory else {
        return Ok(found);
    };
    let sorted = |path: &Path| -> std::result::Result<Vec<PathBuf>, SourceError> {
        let mut paths = fs::read_dir(path)
            .and_then(|items| {
                items
                    .map(|i| i.map(|i| i.path()))
                    .collect::<std::io::Result<Vec<_>>>()
            })
            .map_err(|e| bad(format!("{}: {e}", path.display())))?;
        paths.sort();
        Ok(paths)
    };
    for kind_dir in sorted(directory)? {
        if !kind_dir.is_dir() {
            continue;
        }
        let kind = kind_dir.file_name().unwrap().to_string_lossy();
        for path in sorted(&kind_dir)? {
            let name = path.file_name().unwrap().to_string_lossy();
            let (key, ext) = name.rsplit_once('.').unwrap_or(("", ""));
            let mime = match ext.to_ascii_lowercase().as_str() {
                "mp3" => "audio/mpeg",
                "wav" => "audio/wav",
                "ogg" => "audio/ogg",
                "m4a" => "audio/mp4",
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                _ => {
                    return Err(
                        bad(format!("{}: unknown resource extension", path.display())).into(),
                    );
                }
            };
            if key.is_empty() {
                return Err(bad(format!("{}: unknown resource extension", path.display())).into());
            }
            if !valid_res(&format!("{kind}:{key}")) {
                return Err(bad(format!(
                    "{}: invalid resource kind {}",
                    path.display(),
                    serde_json::to_string(&kind).expect("strings serialize")
                ))
                .into());
            }
            let resource = (kind.to_string(), key.to_string());
            if found.contains_key(&resource) {
                return Err(bad(format!(
                    "{}: duplicate resource {kind}:{key}",
                    path.display()
                ))
                .into());
            }
            found.insert(resource, (mime, path));
        }
    }
    Ok(found)
}
