//! Entry resolution and an outline containing each node's own strings.

use std::io::Write;

use anyhow::{Context, Result};
use zdict_core::{Child, Entry, Error, Node, Pack};

use super::print_json;

pub fn run(pack: &Pack, key_or_word: &str, json: bool, out: &mut impl Write) -> Result<()> {
    let entry = resolve(pack, key_or_word)?;
    if json {
        let value = serde_json::from_str(&pack.entry_json(&entry.key)?)?;
        print_json(out, &value)
    } else {
        outline(&entry.root, 0, out)
    }
}

pub(super) fn resolve(pack: &Pack, key_or_word: &str) -> Result<Entry> {
    Ok(match pack.entry(key_or_word) {
        Ok(entry) => entry,
        Err(Error::MissingEntry(_)) => {
            let hit = pack
                .words(key_or_word, 1)?
                .into_iter()
                .next()
                .with_context(|| format!("no entry or word-list row for {key_or_word:?}"))?;
            pack.entry(&hit.entry)?
        }
        Err(error) => return Err(error.into()),
    })
}

fn outline(node: &Node, depth: usize, out: &mut impl Write) -> Result<()> {
    let mut strings: Vec<&str> = node.text.as_deref().into_iter().collect();
    strings.extend(node.children.iter().filter_map(|child| match child {
        Child::Text(text) => Some(text.as_str()),
        Child::Node(_) => None,
    }));
    if !strings.is_empty() {
        let text = strings.concat();
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        writeln!(out, "{}{}: {text}", "  ".repeat(depth), node.kind)?;
    }
    for child in node.child_nodes() {
        outline(child, depth + 1, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_preserves_depth_and_excludes_descendant_strings() {
        let root = serde_json::from_str(
            r#"{"type":"entry","children":[{"type":"head","children":[{"type":"headword","text":"apple"}]},{"type":"definition","children":["a ",{"type":"emphasis","text":"round"}," fruit\nwith seeds"]}]}"#,
        )
        .unwrap();
        let mut output = Vec::new();
        outline(&root, 0, &mut output).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "    headword: apple\n  definition: a fruit with seeds\n    emphasis: round\n"
        );
    }
}
