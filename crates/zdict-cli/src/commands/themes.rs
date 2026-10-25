//! List every shipped theme and its validation problems.

use std::io::Write;

use anyhow::{Result, ensure};
use serde_json::json;
use zdict_core::Pack;

use super::print_json;

pub fn run(pack: &Pack, json: bool, out: &mut impl Write) -> Result<()> {
    let registry = pack.registry()?;
    let mut themes = Vec::new();
    let mut valid = true;
    for name in pack.theme_names()? {
        let problems = match pack.theme(&name) {
            Ok(theme) => theme.validate(&registry),
            Err(error) => vec![error.to_string()],
        };
        valid &= problems.is_empty();
        if !json {
            writeln!(out, "{name}")?;
            for problem in &problems {
                writeln!(out, "  {problem}")?;
            }
        }
        themes.push(json!({ "name": name, "problems": problems }));
    }
    if json {
        print_json(out, &json!(themes))?;
    }
    ensure!(valid, "pack themes have validation problems");
    Ok(())
}
