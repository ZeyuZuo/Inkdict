//! Resolve an entry and apply pack and user theme layers.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use zdict_core::Pack;
use zdict_core::theme::{Theme, ThemeStack};

use super::{print_json, show};

pub fn run(
    pack: &Pack,
    key_or_word: &str,
    theme: &str,
    overlays: &[PathBuf],
    out: &mut impl Write,
) -> Result<()> {
    let entry = show::resolve(pack, key_or_word)?;
    let registry = pack.registry()?;
    let mut stack = ThemeStack::new(pack.theme(theme)?);
    for path in overlays {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading overlay {}", path.display()))?;
        stack.push(Theme::parse(&path.display().to_string(), &text)?);
    }
    let tree = pack.renderer(&registry, &stack).render(&entry);
    print_json(out, &serde_json::to_value(tree)?)
}
