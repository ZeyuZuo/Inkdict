//! What the desktop app does with an open dictionary, independent of
//! Tauri: the commands of apps/desktop/README.md as plain methods, the
//! settings overlay and the saved configuration. The Tauri layer
//! (apps/desktop/src-tauri) only wraps these.

mod config;
mod settings;

use std::path::Path;

use serde::Serialize;
use zdict_core::theme::{Theme, ThemeStack};
use zdict_core::{
    Column, Correction, Error as CoreError, Focus, Hit, Link, LinkTarget, Pack, Registry,
    Suggestion,
};
use zdict_html::{Markup, entry_html, theme_css};

pub use config::Config;
pub use settings::{DESKTOP_THEME, Settings};

/// A failed command: `code` is one of `no_pack`, `not_found`, `bad_input`,
/// `internal`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        AppError {
            code,
            message: message.into(),
        }
    }

    pub fn no_pack() -> Self {
        AppError::new("no_pack", "no dictionary pack is open")
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<CoreError> for AppError {
    fn from(err: CoreError) -> Self {
        let code = match err {
            CoreError::MissingEntry(_) => "not_found",
            CoreError::NotAPack(_) | CoreError::UnsupportedVersion(_) | CoreError::Meta { .. } => {
                "bad_input"
            }
            _ => "internal",
        };
        AppError::new(code, err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackInfo {
    pub id: String,
    pub title: String,
    pub entries: Option<u64>,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryView {
    pub key: String,
    pub headword: String,
    pub homograph: Option<i64>,
    pub html: String,
}

/// Where a followed link leads (`kind` tags the JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Entry { key: String, focus: FocusView },
    Lookup { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FocusView {
    Top,
    Anchor { number: u32 },
    WordFamily,
}

/// An open pack with its registry and the theme stack for the settings.
pub struct Dictionary {
    pack: Pack,
    registry: Registry,
    base: Theme,
    settings: Settings,
    stack: ThemeStack,
}

impl Dictionary {
    /// Open a pack and render with its `default` theme (or its first one).
    pub fn open(path: &Path, settings: Settings) -> Result<Self> {
        let pack = Pack::open(path)?;
        let registry = pack.registry()?;
        let names = pack.theme_names()?;
        let name = if names.iter().any(|name| name == "default") {
            "default"
        } else {
            names
                .first()
                .ok_or_else(|| AppError::new("bad_input", "the pack has no theme"))?
        };
        let base = pack.theme(name)?;
        let stack = settings::stack(&base, &settings);
        Ok(Dictionary {
            pack,
            registry,
            base,
            settings,
            stack,
        })
    }

    /// Build the spelling-correction index now rather than on first use.
    pub fn warm_up(&self) -> Result<()> {
        self.pack.prepare_correction()?;
        Ok(())
    }

    pub fn info(&self) -> PackInfo {
        let meta = self.pack.meta();
        PackInfo {
            id: meta.id().to_owned(),
            title: meta.title().to_owned(),
            entries: meta.entries(),
            source_lang: meta.source_lang().to_owned(),
            target_lang: meta.target_lang().to_owned(),
        }
    }

    pub fn suggest(&self, text: &str, limit: usize) -> Result<Vec<Suggestion>> {
        Ok(self.pack.suggest(text, limit)?)
    }

    pub fn correct(&self, text: &str, limit: usize) -> Result<Vec<Correction>> {
        Ok(self.pack.correct(text, limit)?)
    }

    /// `column`: None, `"text"` or `"example"`.
    pub fn search(&self, text: &str, limit: usize, column: Option<&str>) -> Result<Vec<Hit>> {
        let column = match column {
            None => None,
            Some("text") => Some(Column::Text),
            Some("example") => Some(Column::Example),
            Some(other) => {
                return Err(AppError::new(
                    "bad_input",
                    format!("unknown column {other}"),
                ));
            }
        };
        Ok(self.pack.search(text, limit, column)?)
    }

    pub fn entry(&self, key: &str) -> Result<EntryView> {
        let entry = self.pack.entry(key)?;
        let tree = self
            .pack
            .renderer(&self.registry, &self.stack)
            .render(&entry);
        Ok(EntryView {
            html: entry_html(&tree, Markup::App),
            key: entry.key,
            headword: entry.headword,
            homograph: entry.homograph,
        })
    }

    /// Follow a link of the entry `current` (spec v0 §6).
    pub fn follow(&self, current: &str, link: &Link) -> Result<Target> {
        let current = self.pack.entry(current)?;
        Ok(match self.pack.resolve(&current, link)? {
            LinkTarget::Entry { key, focus } => Target::Entry {
                key,
                focus: match focus {
                    Focus::Top => FocusView::Top,
                    Focus::Anchor(number) => FocusView::Anchor { number },
                    Focus::WordFamily => FocusView::WordFamily,
                },
            },
            LinkTarget::Lookup(text) => Target::Lookup { text },
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: Settings) -> &Settings {
        self.stack = settings::stack(&self.base, &settings);
        self.settings = settings;
        &self.settings
    }

    pub fn theme_css(&self) -> String {
        theme_css(&self.stack)
    }
}

#[cfg(test)]
mod tests;
