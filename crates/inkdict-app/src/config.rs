//! What the app remembers between runs: the library and the settings,
//! as `config.json` in the app's config directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AppError, Result, Settings};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "SavedConfig")]
pub struct Config {
    pub folder: Option<PathBuf>,
    pub packs: Vec<PathBuf>,
    pub active: Option<PathBuf>,
    pub settings: Settings,
    pub themes: BTreeMap<String, String>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SavedConfig {
    folder: Option<PathBuf>,
    packs: Option<Vec<PathBuf>>,
    active: Option<PathBuf>,
    pack: Option<PathBuf>,
    settings: Settings,
    themes: BTreeMap<String, String>,
}

impl From<SavedConfig> for Config {
    fn from(saved: SavedConfig) -> Self {
        let (packs, active) = match saved.packs {
            Some(packs) => (packs, saved.active),
            None => (saved.pack.iter().cloned().collect(), saved.pack),
        };
        let active = active.filter(|path| packs.contains(path));
        Self {
            folder: saved.folder,
            packs,
            active,
            settings: saved.settings,
            themes: saved.themes,
        }
    }
}

impl Config {
    /// The saved configuration, or the default when there is none or it
    /// cannot be read (a broken file must not keep the app from starting).
    pub fn load(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write atomically: a temporary file next to it, then rename.
    pub fn save(&self, path: &Path) -> Result<()> {
        let fail = |err: std::io::Error| {
            AppError::new("internal", format!("saving {}: {err}", path.display()))
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(fail)?;
        }
        let temporary = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).expect("config serializes");
        std::fs::write(&temporary, text).map_err(fail)?;
        std::fs::rename(&temporary, path).map_err(fail)
    }
}
