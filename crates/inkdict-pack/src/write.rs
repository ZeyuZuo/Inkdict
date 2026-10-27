use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use inkdict_core::{APPLICATION_ID, FORMAT_VERSION};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    Options, Problems, Report, Result,
    content::validate_entry,
    index::Indexer,
    source::{self, Source, jsonl_lines, read},
    summaries::{self, Target},
};

const ZSTD_LEVEL: i32 = 19;
const SCHEMA: &str = include_str!("schema.sql");

pub(crate) fn train_dictionary(lines: &[&[u8]]) -> std::io::Result<Option<Vec<u8>>> {
    if lines.len() < 1000 {
        return Ok(None);
    }
    let samples: Vec<_> = lines.iter().step_by(4).copied().collect();
    zstd::dict::from_samples(&samples, 256 * 1024).map(Some)
}

pub(crate) fn build(options: Options) -> Result<Report> {
    let mut source = source::load_source(&options.manifest)?;
    if let Some(path) = options.entries {
        source.entries = path;
    }
    if let Some(path) = options.summaries {
        source.summaries = Some(path);
    }
    let registry = source.registry()?;
    let indexer = Indexer::new(&registry, source.scheme, &source.boost)?;
    let entries = source::read_entries(&source.entries)?;
    let lines: Vec<_> = jsonl_lines(&entries.raw).collect();
    let resources = source::read_resources(source.resources.as_deref())?;
    let dictionary = train_dictionary(&lines)?;
    let output = options.output.unwrap_or_else(|| {
        options
            .manifest
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{}.inkdict", source.pack["id"].as_str().unwrap()))
    });
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .ok_or_else(|| source::bad("output needs a file name"))?;
    let temporary = parent.join(format!(".{}.tmp", name.to_string_lossy()));
    match fs::remove_file(&temporary) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    let result = (|| {
        let mut db = Connection::open(&temporary)?;
        db.execute_batch(&format!("PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA application_id={APPLICATION_ID}; PRAGMA user_version={FORMAT_VERSION};"))?;
        db.execute_batch(SCHEMA)?;
        let transaction = db.transaction()?;
        let mut compressor = zstd::bulk::Compressor::with_dictionary(
            ZSTD_LEVEL,
            dictionary.as_deref().unwrap_or_default(),
        )?;
        let mut problems = Problems::default();
        let mut targets = HashMap::new();
        let mut links = Vec::new();
        let source_kinds: HashSet<_> = source.sources.iter().map(|(k, _)| k.as_str()).collect();
        let mut body_bytes = 0;
        let mut lookup_rows = 0;
        let mut resource_refs = 0;
        {
            let mut insert_entry =
                transaction.prepare("INSERT INTO entries VALUES (?, ?, ?, ?, ?)")?;
            let mut insert_lookup =
                transaction.prepare("INSERT INTO lookup VALUES (?, ?, ?, ?, ?, ?)")?;
            for (i, (line, entry)) in lines.iter().zip(&entries.trees).enumerate() {
                let key = entry["id"].as_str().unwrap();
                let number = i + 1;
                for message in validate_entry(&registry, entry) {
                    problems.add("invalid content", Some(key), message);
                }
                let index = indexer.index(entry);
                for (kind, detail) in index.problems {
                    problems.add(kind, Some(key), detail);
                }
                let mut anchors = HashMap::new();
                for (position, id) in index.anchors.into_iter().enumerate() {
                    anchors.entry(id).or_insert(position);
                }
                targets.insert(
                    key.to_owned(),
                    Target {
                        number,
                        anchors,
                        lang: entry["lang"].as_str().unwrap().to_owned(),
                    },
                );
                links.push((key.to_owned(), index.links));
                let body = compressor.compress(line)?;
                body_bytes += body.len();
                insert_entry.execute(params![
                    number as i64,
                    key,
                    index.headword,
                    index.homograph,
                    body
                ])?;
                lookup_rows += index.lookup.len();
                for ((norm, anchor), (class, form, kind)) in index.lookup {
                    let rank = class * indexer.rank_width() + index.importance;
                    insert_lookup.execute(params![
                        norm,
                        rank as i64,
                        number as i64,
                        anchor as i64,
                        form,
                        kind.name()
                    ])?;
                }
                for reference in index.resources {
                    resource_refs += 1;
                    let (kind, key_part) = reference.split_once(':').unwrap_or((&reference, ""));
                    if resources.contains_key(&(kind.to_owned(), key_part.to_owned())) {
                        continue;
                    }
                    if !source_kinds.contains(kind) {
                        problems.add("resource without source", Some(key), reference);
                    }
                }
            }
        }
        let links = summaries::check_links(&targets, &links, &mut problems);
        let summaries = summaries::write_summaries(
            &transaction,
            source.summaries.as_deref(),
            &targets,
            &mut problems,
        )?;
        for ((kind, key), (mime, path)) in &resources {
            transaction.execute(
                "INSERT INTO resources VALUES (?, ?, ?, ?)",
                params![kind, key, mime, read(path)?],
            )?;
        }
        let mut priorities = HashMap::<&str, usize>::new();
        for (kind, template) in &source.sources {
            let priority = priorities.entry(kind).or_default();
            *priority += 1;
            transaction.execute(
                "INSERT INTO resource_sources VALUES (?, ?, ?)",
                params![kind, *priority as i64, template],
            )?;
        }
        // Repeated theme basenames use the last file.
        let mut files = BTreeMap::from([("types.toml".to_owned(), read(&source.types)?)]);
        for path in &source.themes {
            files.insert(
                format!("themes/{}", path.file_name().unwrap().to_string_lossy()),
                read(path)?,
            );
        }
        if let Some(dict) = &dictionary {
            files.insert("zstd.dict".to_owned(), dict.clone());
        }
        for (name, body) in files {
            transaction.execute("INSERT INTO files VALUES (?, ?)", params![name, body])?;
        }
        let meta = metadata(
            &source,
            &entries.raw,
            entries.trees.len(),
            resources.len(),
            dictionary.is_some(),
        )?;
        for (key, value) in &meta {
            transaction.execute("INSERT INTO meta VALUES (?, ?)", params![key, value])?;
        }
        transaction.commit()?;
        db.execute_batch("VACUUM")?;
        db.close().map_err(|(_, error)| error)?;
        fs::rename(&temporary, &output).map_err(|err| {
            source::bad(format!(
                "{}: cannot replace it ({err}); on Windows, close programs that have it open",
                output.display()
            ))
        })?;
        Ok(Report {
            output: output.to_string_lossy().into_owned(),
            bytes: fs::metadata(&output)?.len(),
            entries: entries.trees.len(),
            body_bytes,
            lookup_rows,
            summaries,
            resources: resources.len(),
            links,
            resource_refs,
            meta,
            problems,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Metadata arrays retain the pack data format's spaces between items.
fn json_array(items: &[String]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|s| serde_json::to_string(s).expect("strings serialize"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn metadata(
    source: &Source,
    raw: &[u8],
    entries: usize,
    resources: usize,
    dictionary: bool,
) -> Result<BTreeMap<String, String>> {
    let (major, minor, patch) = unicode_normalization::UNICODE_VERSION;
    let themes = source
        .themes
        .iter()
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let built_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanoseconds")
        .format(&Rfc3339)?
        .replace('Z', "+00:00");
    let mut meta = BTreeMap::from([
        ("format".into(), "inkdict-pack".into()),
        ("format_version".into(), FORMAT_VERSION.to_string()),
        ("id".into(), source.pack["id"].as_str().unwrap().into()),
        (
            "title".into(),
            source.pack["title"].as_str().unwrap().into(),
        ),
        ("source_lang".into(), source.source_lang.clone()),
        ("target_langs".into(), json_array(&source.targets)),
        ("themes".into(), json_array(&themes)),
        ("norm".into(), source.scheme.name().into()),
        ("unicode_version".into(), format!("{major}.{minor}.{patch}")),
        ("compression".into(), "zstd".into()),
        ("zstd_level".into(), ZSTD_LEVEL.to_string()),
        ("entries".into(), entries.to_string()),
        ("resources".into(), resources.to_string()),
        ("built_at".into(), built_at),
        ("source_sha256".into(), format!("{:x}", Sha256::digest(raw))),
        (
            "builder".into(),
            format!("inkdict-pack {}", env!("CARGO_PKG_VERSION")),
        ),
    ]);
    for name in ["edition", "license"] {
        if let Some(value) = source
            .pack
            .get(name)
            .and_then(toml::Value::as_str)
            .filter(|s| !s.is_empty())
        {
            meta.insert(name.into(), value.into());
        }
    }
    if dictionary {
        meta.insert("zstd_dict".into(), "zstd.dict".into());
    }
    Ok(meta)
}
