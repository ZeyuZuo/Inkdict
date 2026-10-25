//! Display settings, applied as an overlay theme (spec v0 §7.2).

use serde::{Deserialize, Serialize};
use zdict_core::theme::{Theme, ThemeStack};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hide_chinese: bool,
    pub hide_pronunciation: bool,
    pub hide_examples: bool,
    /// Text size factor; the UI applies it with CSS.
    pub font_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hide_chinese: false,
            hide_pronunciation: false,
            hide_examples: false,
            font_scale: 1.0,
        }
    }
}

impl Settings {
    /// The overlay rules these settings add over the pack theme.
    pub fn overlay(&self) -> String {
        let mut style =
            String::from("[meta]\nformat = \"zdict-theme\"\nversion = 0\nname = \"Settings\"\n");
        let mut hide = |selector: &str| {
            style.push_str(&format!("[style.\"{selector}\"]\ndisplay = \"none\"\n"));
        };
        if self.hide_chinese {
            hide("*[lang=zh]");
        }
        if self.hide_pronunciation {
            hide("pron_list");
        }
        if self.hide_examples {
            hide("example");
        }
        style
    }
}

/// The desktop app's own theme (themes/desktop.toml): the application
/// layer between the pack theme and the settings (spec v0 §7.2).
pub const DESKTOP_THEME: &str = include_str!("../themes/desktop.toml");

/// Pack theme, then the desktop theme, then the settings layer.
pub(crate) fn stack(base: &Theme, settings: &Settings) -> ThemeStack {
    let mut stack = ThemeStack::new(base.clone());
    stack.push(
        Theme::parse("desktop.toml", DESKTOP_THEME).expect("the bundled desktop theme parses"),
    );
    let overlay =
        Theme::parse("settings", &settings.overlay()).expect("generated overlay is valid TOML");
    stack.push(overlay);
    stack
}
