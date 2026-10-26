//! Display settings, applied as an overlay theme over the pack's (spec/v1.md §3.2, §4.3).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use zdict_core::theme::{Theme, ThemeStack};
use zdict_core::{Registry, Role};

use crate::{AppError, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Hide nodes with the `pronunciation` role.
    pub hide_pronunciation: bool,
    /// Hide nodes with the `example` role.
    pub hide_examples: bool,
    /// Text size factor; the UI applies it with CSS.
    pub font_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hide_pronunciation: false,
            hide_examples: false,
            font_scale: 1.0,
        }
    }
}

/// Which display switches a pack offers: only those whose role some type has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Switches {
    pub pronunciation: bool,
    pub example: bool,
}

impl Switches {
    pub fn of(registry: &Registry) -> Self {
        Switches {
            pronunciation: !registry.types_with_role(Role::Pronunciation).is_empty(),
            example: !registry.types_with_role(Role::Example).is_empty(),
        }
    }
}

impl Settings {
    /// The overlay rules these settings add over the pack theme: the types
    /// carrying a hidden role, by name, since themes select types.
    pub fn overlay(&self, registry: &Registry) -> String {
        let mut style =
            String::from("[meta]\nformat = \"zdict-theme\"\nversion = 1\nname = \"Settings\"\n");
        let hidden = [
            (self.hide_pronunciation, Role::Pronunciation),
            (self.hide_examples, Role::Example),
        ];
        // Each type once: a type with both roles must not repeat its table.
        let names: BTreeSet<&str> = hidden
            .into_iter()
            .filter(|(hide, _)| *hide)
            .flat_map(|(_, role)| registry.types_with_role(role))
            .collect();
        for name in names {
            // Quoted as a string key (a JSON string is a TOML basic string),
            // so no name can end the table header.
            let key = serde_json::Value::from(name);
            style.push_str(&format!("[style.{key}]\ndisplay = \"none\"\n"));
        }
        style
    }
}

/// The pack theme, then the settings layer.
pub(crate) fn stack(base: &Theme, registry: &Registry, settings: &Settings) -> Result<ThemeStack> {
    let overlay = Theme::parse("settings", &settings.overlay(registry))
        .map_err(|err| AppError::new("internal", format!("settings overlay: {err}")))?;
    let mut stack = ThemeStack::new(base.clone());
    stack.push(overlay);
    Ok(stack)
}
