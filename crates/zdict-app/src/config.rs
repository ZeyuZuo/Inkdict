//! What the app remembers between runs: the pack path and the settings,
//! as `config.json` in the app's config directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AppError, Result, Settings};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub pack: Option<PathBuf>,
    pub settings: Settings,
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
