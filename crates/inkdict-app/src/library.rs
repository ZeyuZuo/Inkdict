//! The dictionary library and the open dictionary shared by app commands.

use std::path::{Path, PathBuf};

use inkdict_core::Pack;
use serde::Serialize;

use crate::{AppError, Config, Dictionary, PackInfo, Result, Settings};

#[derive(Debug, Serialize)]
pub struct LibraryItem {
    pub path: PathBuf,
    pub title: Option<String>,
    pub entries: Option<u64>,
    pub bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Library {
    pub active: Option<PathBuf>,
    pub packs: Vec<LibraryItem>,
}

#[derive(Debug, Serialize)]
pub struct Rejected {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Added {
    pub library: Library,
    pub rejected: Vec<Rejected>,
}

/// The app's state across commands, optionally persisted as config.json.
pub struct Session {
    config_path: Option<PathBuf>,
    config: Config,
    dictionary: Option<Dictionary>,
    resource_base: Option<String>,
}

impl Session {
    /// Load configuration and try to open the active pack. A command-line
    /// pack takes precedence, even when it can no longer be opened.
    pub fn start(config_path: Option<PathBuf>, pack: Option<PathBuf>) -> (Self, Option<AppError>) {
        let config = config_path.as_deref().map(Config::load).unwrap_or_default();
        let mut session = Self {
            config_path,
            config,
            dictionary: None,
            resource_base: None,
        };
        if let Some(path) = pack {
            let path = canonical_or_original(&path);
            let path = match session.position(&path) {
                Ok(index) => session.config.packs[index].clone(),
                Err(_) => {
                    session.config.packs.push(path.clone());
                    path
                }
            };
            session.config.active = Some(path);
        }
        let active = session.config.active.clone();
        let error = active
            .as_deref()
            .and_then(|path| match session.open_dictionary(path) {
                Ok(dictionary) => {
                    session.dictionary = Some(dictionary);
                    None
                }
                Err(error) => Some(error),
            });
        let save_error = session.save().err();
        (session, error.or(save_error))
    }

    /// Where the host serves the files packs store themselves, e.g.
    /// `inkdict-res://localhost/`; entry HTML then tries
    /// `<base><pack id>/<kind>/<key>` before online sources, and the host
    /// answers it with [`Dictionary::resource`].
    pub fn set_resource_base(&mut self, base: impl Into<String>) {
        self.resource_base = Some(base.into());
        if let Some(dictionary) = &mut self.dictionary {
            dictionary.set_resource_base(self.resource_base.clone());
        }
    }

    fn install(&mut self, mut dictionary: Dictionary) {
        dictionary.set_resource_base(self.resource_base.clone());
        self.dictionary = Some(dictionary);
    }

    pub fn dictionary(&self) -> Option<&Dictionary> {
        self.dictionary.as_ref()
    }

    fn open_dictionary(&self, path: &Path) -> Result<Dictionary> {
        let pack = Pack::open(path)?;
        let theme = self.config.themes.get(pack.meta().id()).map(String::as_str);
        Dictionary::from_pack(pack, self.config.settings.clone(), theme)
    }

    pub fn library(&self) -> Library {
        Library {
            active: self.config.active.clone(),
            packs: self
                .config
                .packs
                .iter()
                .map(|path| {
                    let bytes = std::fs::metadata(path).ok().map(|meta| meta.len());
                    let (title, entries, error) = match Pack::open(path) {
                        Ok(pack) => (
                            Some(pack.meta().title().to_owned()),
                            pack.meta().entries(),
                            None,
                        ),
                        Err(error) => (None, None, Some(error.to_string())),
                    };
                    LibraryItem {
                        path: path.clone(),
                        title,
                        entries,
                        bytes,
                        error,
                    }
                })
                .collect(),
        }
    }

    /// Append valid packs, rejecting duplicates by canonical path as well
    /// as files that cannot be opened with the current settings.
    pub fn add_packs(&mut self, paths: Vec<PathBuf>) -> Result<Added> {
        let mut rejected = Vec::new();
        for path in paths {
            let add = || -> Result<(PathBuf, Dictionary)> {
                let canonical = dunce::canonicalize(&path).map_err(|error| {
                    AppError::new("bad_input", format!("{}: {error}", path.display()))
                })?;
                if self.position(&canonical).is_ok() {
                    return Err(AppError::new("bad_input", "pack is already in the library"));
                }
                let dictionary = self.open_dictionary(&canonical)?;
                Ok((canonical, dictionary))
            };
            match add() {
                Ok((path, dictionary)) => {
                    self.config.packs.push(path.clone());
                    if self.config.active.is_none() {
                        self.config.active = Some(path);
                        self.install(dictionary);
                    }
                }
                Err(error) => rejected.push(Rejected {
                    path,
                    message: error.to_string(),
                }),
            }
        }
        self.save()?;
        Ok(Added {
            library: self.library(),
            rejected,
        })
    }

    /// Remove only the library reference, selecting the first remaining
    /// pack that can be opened when the active reference is removed.
    pub fn remove_pack(&mut self, path: &Path) -> Result<Library> {
        let index = self.position(path)?;
        let removed = self.config.packs.remove(index);
        if self.config.active.as_ref() == Some(&removed) {
            self.dictionary = None;
            self.config.active = None;
            let opened = self.config.packs.iter().find_map(|path| {
                self.open_dictionary(path)
                    .ok()
                    .map(|dictionary| (path.clone(), dictionary))
            });
            if let Some((path, dictionary)) = opened {
                self.install(dictionary);
                self.config.active = Some(path);
            }
        }
        self.save()?;
        Ok(self.library())
    }

    pub fn move_pack(&mut self, path: &Path, index: usize) -> Result<Library> {
        let old = self.position(path)?;
        let path = self.config.packs.remove(old);
        self.config
            .packs
            .insert(index.min(self.config.packs.len()), path);
        self.save()?;
        Ok(self.library())
    }

    pub fn use_pack(&mut self, path: &Path) -> Result<PackInfo> {
        let index = self.position(path)?;
        let path = &self.config.packs[index];
        let dictionary = self.open_dictionary(path)?;
        let info = dictionary.info();
        self.config.active = Some(path.clone());
        self.install(dictionary);
        self.save()?;
        Ok(info)
    }

    pub fn settings(&self) -> &Settings {
        &self.config.settings
    }

    /// Apply first, then remember only non-default choices by pack id.
    pub fn set_theme(&mut self, name: &str) -> Result<PackInfo> {
        let dictionary = self.dictionary.as_mut().ok_or_else(AppError::no_pack)?;
        dictionary.set_theme(name)?;
        let info = dictionary.info();
        if name == dictionary.pack.meta().default_theme() {
            self.config.themes.remove(&info.id);
        } else {
            self.config.themes.insert(info.id.clone(), name.to_owned());
        }
        self.save()?;
        Ok(info)
    }

    /// Apply first: a failed settings overlay must never reach config.json.
    pub fn set_settings(&mut self, settings: Settings) -> Result<Settings> {
        if let Some(dictionary) = &mut self.dictionary {
            dictionary.set_settings(settings.clone())?;
        }
        self.config.settings = settings.clone();
        self.save()?;
        Ok(settings)
    }

    fn save(&self) -> Result<()> {
        match self.config_path.as_deref() {
            Some(path) => self.config.save(path),
            None => Ok(()),
        }
    }

    fn position(&self, path: &Path) -> Result<usize> {
        let canonical = canonical_or_original(path);
        self.config
            .packs
            .iter()
            .position(|saved| canonical_or_original(saved) == canonical)
            .ok_or_else(|| {
                AppError::new(
                    "not_found",
                    format!("{} is not in the library", path.display()),
                )
            })
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;
