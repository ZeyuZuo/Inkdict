//! The dictionary library and the open dictionary shared by app commands.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use inkdict_core::Pack;
use serde::Serialize;

use crate::{AppError, Config, Dictionary, PackInfo, Result, Settings};

#[derive(Debug, Serialize)]
pub struct LibraryItem {
    pub path: PathBuf,
    pub in_folder: bool,
    pub title: Option<String>,
    pub entries: Option<u64>,
    pub bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Library {
    pub folder: Option<PathBuf>,
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
    folder: Option<PathBuf>,
    dictionary: Option<Dictionary>,
    resource_base: Option<String>,
}

impl Session {
    /// Load configuration and try to open the active pack. A command-line
    /// pack takes precedence, even when it can no longer be opened.
    pub fn start(
        config_path: Option<PathBuf>,
        pack: Option<PathBuf>,
        default_folder: Option<PathBuf>,
    ) -> (Self, Option<AppError>) {
        let config = config_path.as_deref().map(Config::load).unwrap_or_default();
        let mut session = Self {
            config_path,
            folder: config.folder.clone().or(default_folder),
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
        let folder_error = session.prepare_folder().and_then(|()| session.scan()).err();
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
        if session.folder.is_some() && session.config.active.is_none() {
            session.select_first();
        }
        let save_error = session.save().err();
        (session, folder_error.or(error).or(save_error))
    }

    fn prepare_folder(&mut self) -> Result<()> {
        if let Some(folder) = &self.folder {
            std::fs::create_dir_all(folder).map_err(|error| {
                AppError::new(
                    "internal",
                    format!("creating {}: {error}", folder.display()),
                )
            })?;
            self.folder = Some(dunce::canonicalize(folder).map_err(|error| {
                AppError::new("internal", format!("opening {}: {error}", folder.display()))
            })?);
        }
        Ok(())
    }

    /// The effective dictionary folder, if the host supplies one.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Reconcile folder files without reopening an existing active dictionary.
    pub fn rescan(&mut self) -> Result<Library> {
        let packs = self.config.packs.clone();
        let active = self.config.active.clone();
        self.scan()?;
        if self.folder.is_some() && self.config.active.is_none() {
            self.select_first();
        }
        if packs != self.config.packs || active != self.config.active {
            self.save()?;
        }
        Ok(self.library())
    }

    fn scan(&mut self) -> Result<()> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let fail = |error| {
            AppError::new(
                "internal",
                format!("scanning {}: {error}", folder.display()),
            )
        };
        let mut found = Vec::new();
        for entry in std::fs::read_dir(folder).map_err(fail)? {
            let entry = entry.map_err(fail)?;
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.')
                || !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("inkdict"))
            {
                continue;
            }
            let meta = entry.metadata().map_err(fail)?;
            if !meta.is_file()
                || SystemTime::now()
                    .duration_since(meta.modified().map_err(fail)?)
                    .unwrap_or_default()
                    < Duration::from_secs(2)
            {
                continue;
            }
            found.push((entry.file_name(), dunce::canonicalize(&path).map_err(fail)?));
        }
        found.sort_by(|a, b| a.0.cmp(&b.0));
        self.config
            .packs
            .retain(|path| !inside(path, folder) || path.exists());
        if self
            .config
            .active
            .as_ref()
            .is_some_and(|path| !self.config.packs.contains(path))
        {
            self.config.active = None;
            self.dictionary = None;
        }
        for (_, path) in found {
            if self.position(&path).is_err() {
                self.config.packs.push(path);
            }
        }
        Ok(())
    }

    /// Use an existing directory, leaving the old folder's files in place.
    pub fn set_folder(&mut self, path: PathBuf) -> Result<Library> {
        if !path.is_dir() {
            return Err(AppError::new(
                "bad_input",
                "dictionary folder must be an existing directory",
            ));
        }
        let folder = dunce::canonicalize(&path)
            .map_err(|error| AppError::new("bad_input", format!("{}: {error}", path.display())))?;
        if self.folder.as_ref() != Some(&folder)
            && let Some(old) = &self.folder
        {
            self.config.packs.retain(|path| !inside(path, old));
            if self
                .config
                .active
                .as_ref()
                .is_some_and(|path| !self.config.packs.contains(path))
            {
                self.config.active = None;
                self.dictionary = None;
            }
        }
        self.config.folder = Some(folder.clone());
        self.folder = Some(folder);
        let library = self.rescan()?;
        self.save()?;
        Ok(library)
    }

    fn select_first(&mut self) {
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
            folder: self.folder.clone(),
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
                        in_folder: self
                            .folder
                            .as_ref()
                            .is_some_and(|folder| inside(path, folder)),
                        title,
                        entries,
                        bytes,
                        error,
                    }
                })
                .collect(),
        }
    }

    /// Import valid packs into the folder, or append their paths without one.
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
                if let Some(folder) = &self.folder
                    && !inside(&canonical, folder)
                {
                    let destination = copy_into(&canonical, folder)?;
                    drop(dictionary);
                    let dictionary = self.open_dictionary(&destination)?;
                    return Ok((destination, dictionary));
                }
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

    /// Trash folder files, or forget outside paths, selecting the next usable pack.
    pub fn remove_pack(&mut self, path: &Path) -> Result<Library> {
        let index = self.position(path)?;
        let removed = self.config.packs[index].clone();
        if self
            .folder
            .as_ref()
            .is_some_and(|folder| inside(&removed, folder))
        {
            let active = self.config.active.as_ref() == Some(&removed);
            if active {
                self.dictionary = None;
            }
            if let Err(error) = discard(&removed) {
                if active && let Ok(dictionary) = self.open_dictionary(&removed) {
                    self.install(dictionary);
                }
                return Err(AppError::new(
                    "internal",
                    format!("trashing {}: {error}", removed.display()),
                ));
            }
        }
        self.config.packs.remove(index);
        if self.config.active.as_ref() == Some(&removed) {
            self.dictionary = None;
            self.config.active = None;
            self.select_first();
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

fn inside(path: &Path, folder: &Path) -> bool {
    canonical_or_original(path)
        .parent()
        .is_some_and(|parent| canonical_or_original(parent) == canonical_or_original(folder))
}

fn copy_into(source: &Path, folder: &Path) -> Result<PathBuf> {
    let name = source
        .file_name()
        .ok_or_else(|| AppError::new("bad_input", "pack has no file name"))?;
    let destination = folder.join(name);
    let clash = || {
        AppError::new(
            "bad_input",
            format!(
                "a file named {} is already in the dictionary folder",
                name.to_string_lossy()
            ),
        )
    };
    if destination.symlink_metadata().is_ok() {
        return Err(clash());
    }
    let mut temporary_name = std::ffi::OsString::from(".");
    temporary_name.push(name);
    temporary_name.push(".part");
    let temporary = folder.join(temporary_name);
    let fail = |error| AppError::new("internal", format!("copying {}: {error}", source.display()));
    let mut output = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(fail)?;
    let copy = || -> Result<()> {
        let mut input = std::fs::File::open(source).map_err(fail)?;
        std::io::copy(&mut input, &mut output).map_err(fail)?;
        drop(output);
        if destination.symlink_metadata().is_ok() {
            return Err(clash());
        }
        std::fs::rename(&temporary, &destination).map_err(fail)
    };
    if let Err(error) = copy() {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    dunce::canonicalize(&destination).map_err(fail)
}

#[cfg(not(test))]
fn discard(path: &Path) -> std::result::Result<(), trash::Error> {
    trash::delete(path)
}

#[cfg(test)]
fn discard(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;
