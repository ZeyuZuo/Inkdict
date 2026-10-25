//! Resource references to URLs (spec/v0.md §5, spec/pack-v0.md §6).

use std::collections::HashMap;
use std::fmt::Write;

use rusqlite::Connection;

use crate::error::Result;
use crate::pack::Pack;

/// URL templates per resource kind (`wa`, `xa`, `img`…), in fallback order.
pub(crate) type Sources = HashMap<String, Vec<String>>;

pub(crate) fn read_sources(db: &Connection) -> Result<Sources> {
    let mut statement =
        db.prepare("SELECT kind, template FROM resource_sources ORDER BY kind, priority")?;
    let mut sources = Sources::new();
    for row in statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
        let (kind, template): (String, String) = row?;
        sources.entry(kind).or_default().push(template);
    }
    Ok(sources)
}

impl Pack {
    /// URLs for a resource reference `kind:key`, best first. Empty when the
    /// kind has no source or the reference has no `:`.
    pub fn resource_urls(&self, reference: &str) -> Vec<String> {
        let Some((kind, key)) = reference.split_once(':') else {
            return Vec::new();
        };
        let Some(templates) = self.sources.get(kind) else {
            return Vec::new();
        };
        let encoded = encode_uri_component(key);
        templates
            .iter()
            .map(|template| {
                template
                    .replace("{key:uri}", &encoded)
                    .replace("{key}", key)
            })
            .collect()
    }
}

/// JavaScript `encodeURIComponent`: UTF-8 percent-encoding of everything
/// except `A–Z a–z 0–9 - _ . ! ~ * ' ( )`.
pub fn encode_uri_component(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            encoded.push(byte as char);
        } else {
            write!(encoded, "%{byte:02X}").unwrap();
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_pack::TestPack;

    #[test]
    fn encodes_like_encode_uri_component() {
        assert_eq!(encode_uri_component("apple#_gb_5"), "apple%23_gb_5");
        assert_eq!(encode_uri_component("a b/c?d%"), "a%20b%2Fc%3Fd%25");
        assert_eq!(encode_uri_component("!~*'()-_."), "!~*'()-_.");
        assert_eq!(encode_uri_component("中"), "%E4%B8%AD");
    }

    #[test]
    fn fills_templates_in_priority_order() {
        let test = TestPack::new();
        test.sql(
            "INSERT INTO resource_sources VALUES
               ('wa', 2, 'https://b/{key}.mp3'), ('wa', 1, 'https://a/{key:uri}.mp3');",
        );
        let pack = test.open();
        assert_eq!(
            pack.resource_urls("wa:a#b"),
            ["https://a/a%23b.mp3", "https://b/a#b.mp3"]
        );
        assert!(pack.resource_urls("img:x").is_empty());
        assert!(pack.resource_urls("no-kind").is_empty());
    }
}
