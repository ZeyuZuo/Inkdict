//! The type registry, `types.toml` in a pack (spec/v0.md §3).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::{Error, Result};
use crate::pack::Pack;

/// Default layout of a type before any theme applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Block,
    Inline,
}

/// Behaviour of a type that does not depend on the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Its text goes into the lookup index.
    Index,
    /// Playable (`attrs.audio` or `attrs.res`).
    Audio,
    /// A picture (`attrs.res`).
    Image,
    /// Navigates to `attrs.entry` / `attrs.anchor`, or looks up its text.
    Link,
    /// Can be the target of a link (has an `id`).
    Anchor,
}

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub kind: Kind,
    /// Allowed child types with groups expanded; `#text` for strings.
    pub children: HashSet<String>,
    /// Attribute name -> declared type (`string`, `int`, `res?`, `enum(a|b)`…).
    pub attrs: BTreeMap<String, String>,
    pub roles: Vec<Role>,
}

/// Every node type a pack's entries may contain.
#[derive(Debug, Clone)]
pub struct Registry {
    types: HashMap<String, TypeSpec>,
}

impl Registry {
    pub fn parse(text: &str) -> Result<Self> {
        let data: toml::Table = text.parse().map_err(|err| bad_types(&err))?;
        let groups = table(&data, "groups")?;
        let mut types = HashMap::new();
        for (name, spec) in table(&data, "types")? {
            let spec = spec
                .as_table()
                .ok_or_else(|| bad_types(format!("{name} is not a table")))?;
            let kind = match spec.get("kind").and_then(toml::Value::as_str) {
                Some("block") => Kind::Block,
                Some("inline") => Kind::Inline,
                other => return Err(bad_types(format!("{name}: kind {other:?}"))),
            };
            let mut children = HashSet::new();
            for child in strings(spec, "children", name)? {
                match child.strip_prefix('@') {
                    Some(group) => {
                        let members = groups
                            .get(group)
                            .and_then(toml::Value::as_array)
                            .ok_or_else(|| bad_types(format!("{name}: unknown group @{group}")))?;
                        children
                            .extend(members.iter().filter_map(|m| m.as_str().map(String::from)));
                    }
                    None => {
                        children.insert(child);
                    }
                }
            }
            let mut attrs = BTreeMap::new();
            if let Some(declared) = spec.get("attrs").and_then(toml::Value::as_table) {
                for (attr, kind) in declared {
                    let kind = kind
                        .as_str()
                        .ok_or_else(|| bad_types(format!("{name}.{attr} is not a string")))?;
                    attrs.insert(attr.clone(), kind.to_owned());
                }
            }
            let roles = strings(spec, "roles", name)?
                .iter()
                .map(|role| match role.as_str() {
                    "index" => Ok(Role::Index),
                    "audio" => Ok(Role::Audio),
                    "image" => Ok(Role::Image),
                    "link" => Ok(Role::Link),
                    "anchor" => Ok(Role::Anchor),
                    other => Err(bad_types(format!("{name}: unknown role {other}"))),
                })
                .collect::<Result<_>>()?;
            types.insert(
                name.clone(),
                TypeSpec {
                    kind,
                    children,
                    attrs,
                    roles,
                },
            );
        }
        Ok(Registry { types })
    }

    pub fn get(&self, kind: &str) -> Option<&TypeSpec> {
        self.types.get(kind)
    }

    pub fn contains(&self, kind: &str) -> bool {
        self.types.contains_key(kind)
    }

    /// Default layout; types the registry does not know display inline (spec v0 §3).
    pub fn kind(&self, kind: &str) -> Kind {
        self.get(kind).map_or(Kind::Inline, |spec| spec.kind)
    }

    pub fn has_role(&self, kind: &str, role: Role) -> bool {
        self.get(kind)
            .is_some_and(|spec| spec.roles.contains(&role))
    }
}

impl Pack {
    /// The type registry the pack was built with.
    pub fn registry(&self) -> Result<Registry> {
        Registry::parse(&self.text_file("types.toml")?)
    }

    pub(crate) fn text_file(&self, name: &str) -> Result<String> {
        String::from_utf8(self.file(name)?).map_err(|err| Error::BadFile {
            name: name.to_owned(),
            reason: err.to_string(),
        })
    }
}

fn table<'a>(data: &'a toml::Table, key: &str) -> Result<&'a toml::Table> {
    static EMPTY: std::sync::OnceLock<toml::Table> = std::sync::OnceLock::new();
    match data.get(key) {
        None => Ok(EMPTY.get_or_init(toml::Table::new)),
        Some(value) => value
            .as_table()
            .ok_or_else(|| bad_types(format!("[{key}] is not a table"))),
    }
}

fn strings(spec: &toml::Table, key: &str, name: &str) -> Result<Vec<String>> {
    let Some(value) = spec.get(key) else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .and_then(|items| items.iter().map(|v| v.as_str().map(String::from)).collect())
        .ok_or_else(|| bad_types(format!("{name}.{key} is not a list of strings")))
}

fn bad_types(reason: impl ToString) -> Error {
    Error::BadFile {
        name: "types.toml".to_owned(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: &str = r##"
[meta]
format = "zdict-types"
version = 0
[groups]
inline = ["#text", "strong"]
[types.entry]
kind = "block"
children = ["sense"]
roles = ["anchor"]
[types.sense]
kind = "block"
attrs = { n = "int?" }
children = ["@inline"]
[types.strong]
kind = "inline"
children = ["#text"]
"##;

    #[test]
    fn parses_kinds_groups_attrs_and_roles() {
        let registry = Registry::parse(TYPES).unwrap();
        let sense = registry.get("sense").unwrap();
        assert_eq!(sense.kind, Kind::Block);
        assert!(sense.children.contains("#text") && sense.children.contains("strong"));
        assert_eq!(sense.attrs["n"], "int?");
        assert!(registry.has_role("entry", Role::Anchor));
        assert!(!registry.has_role("sense", Role::Anchor));
        assert_eq!(registry.kind("unknown"), Kind::Inline);
    }

    #[test]
    fn rejects_bad_registries() {
        for bad in [
            "[types.a]\nkind = \"grid\"",
            "[types.a]\nkind = \"block\"\nchildren = [\"@nope\"]",
            "[types.a]\nkind = \"block\"\nroles = [\"fly\"]",
            "not toml =",
        ] {
            assert!(
                matches!(Registry::parse(bad), Err(Error::BadFile { .. })),
                "{bad}"
            );
        }
    }
}
