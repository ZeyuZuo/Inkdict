//! What the desktop app does with an open dictionary, independent of
//! Tauri: the commands of apps/desktop/README.md as plain methods, the
//! settings overlay and the saved configuration. The Tauri layer
//! (apps/desktop/src-tauri) only wraps these.

mod config;
mod settings;

use std::path::Path;

use serde::Serialize;
use zdict_core::theme::{Theme, ThemeStack};
use zdict_core::{Error as CoreError, Focus, Link, LinkTarget, Pack, Registry, Word};
use zdict_html::{Markup, entry_html, theme_css};

pub use config::Config;
pub use settings::{Settings, Switches};

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
    pub target_langs: Vec<String>,
    /// Display switches the settings show for this pack.
    pub switches: Switches,
}

/// A word list row, with the headword a variant or inflection points to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WordView {
    #[serde(flatten)]
    pub word: Word,
    pub target: Option<String>,
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
    /// Open a pack and render with its default theme.
    pub fn open(path: &Path, settings: Settings) -> Result<Self> {
        let pack = Pack::open(path)?;
        let registry = pack.registry()?;
        let base = pack.theme(pack.meta().default_theme())?;
        let stack = settings::stack(&base, &registry, &settings)?;
        Ok(Dictionary {
            pack,
            registry,
            base,
            settings,
            stack,
        })
    }

    /// Read the word index now rather than on the first lookup.
    pub fn warm_up(&self) -> Result<()> {
        self.pack.prepare_words()?;
        Ok(())
    }

    pub fn info(&self) -> PackInfo {
        let meta = self.pack.meta();
        PackInfo {
            id: meta.id().to_owned(),
            title: meta.title().to_owned(),
            entries: meta.entries(),
            source_lang: meta.source_lang().to_owned(),
            target_langs: meta.target_langs().to_vec(),
            switches: Switches::of(&self.registry),
        }
    }

    /// The word list: at most `limit` rows per group (spec/pack-v1.md §5).
    pub fn words(&self, text: &str, limit: usize) -> Result<Vec<WordView>> {
        let words = self.pack.words(text, limit)?;
        Ok(words
            .into_iter()
            .map(|word| WordView {
                target: word.target().map(str::to_owned),
                word,
            })
            .collect())
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

    /// Follow a link of the entry `current` (spec/v1.md §6).
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

    /// Apply new settings; on an error the old ones stay in force.
    pub fn set_settings(&mut self, settings: Settings) -> Result<&Settings> {
        self.stack = settings::stack(&self.base, &self.registry, &settings)?;
        self.settings = settings;
        Ok(&self.settings)
    }

    pub fn theme_css(&self) -> String {
        theme_css(&self.stack)
    }
}

#[cfg(test)]
mod tests;
