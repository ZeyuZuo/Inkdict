//! The type registry, `types.toml` in a pack (spec/v1.md §3).

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
    /// Playable through the first declared resource attribute.
    Audio,
    /// A picture through the first declared resource attribute.
    Image,
    /// Navigates to `attrs.entry` / `attrs.anchor`, or looks up its text.
    Link,
    /// Can be the target of a link (has an `id`).
    Anchor,
    /// A positive homograph number displayed beside the headword.
    Homograph,
    /// Controlled by the pronunciation display switch.
    Pronunciation,
    /// Controlled by the examples display switch.
    Example,
    /// May be omitted when expanding index readings.
    Optional,
    /// Adjacent nodes of this type offer alternative index readings.
    Choice,
}

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub kind: Kind,
    /// Allowed child types with groups expanded; `#text` for strings.
    pub children: HashSet<String>,
    /// Attribute name -> declared type (`string`, `int`, `res?`, `enum(a|b)`…).
    pub attrs: BTreeMap<String, String>,
    pub roles: Vec<Role>,
    /// Lookup kind, present exactly when this type has the index role.
    pub index: Option<String>,
    /// Resource attribute names, in declaration order.
    pub res_attrs: Vec<String>,
}

/// Every node type a pack's entries may contain.
#[derive(Debug, Clone)]
pub struct Registry {
    types: HashMap<String, TypeSpec>,
}

impl Registry {
    pub fn parse(text: &str) -> Result<Self> {
        let data: toml::Table = text.parse().map_err(|err| bad_types(&err))?;
        let meta = table(&data, "meta")?;
        if meta.get("format").and_then(toml::Value::as_str) != Some("zdict-types")
            || meta.get("version").and_then(toml::Value::as_integer) != Some(1)
        {
            return Err(bad_types("not a zdict-types v1 type registry"));
        }
        if !data.contains_key("types") {
            return Err(bad_types("missing [types]"));
        }
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
                expand(groups, &child, name, &mut Vec::new(), &mut children)?;
            }
            let mut attrs = BTreeMap::new();
            let mut res_attrs = Vec::new();
            for (attr, kind) in table(spec, "attrs")? {
                let kind = kind
                    .as_str()
                    .ok_or_else(|| bad_types(format!("{name}.{attr} is not a string")))?;
                if !valid_attr_kind(kind) {
                    return Err(bad_types(format!(
                        "{name}.{attr}: invalid attribute type {kind}"
                    )));
                }
                if kind.trim_end_matches('?') == "res" {
                    res_attrs.push(attr.clone());
                }
                attrs.insert(attr.clone(), kind.to_owned());
            }
            let roles: Vec<Role> = strings(spec, "roles", name)?
                .iter()
                .map(|role| match role.as_str() {
                    "index" => Ok(Role::Index),
                    "audio" => Ok(Role::Audio),
                    "image" => Ok(Role::Image),
                    "link" => Ok(Role::Link),
                    "anchor" => Ok(Role::Anchor),
                    "homograph" => Ok(Role::Homograph),
                    "pronunciation" => Ok(Role::Pronunciation),
                    "example" => Ok(Role::Example),
                    "optional" => Ok(Role::Optional),
                    "choice" => Ok(Role::Choice),
                    other => Err(bad_types(format!("{name}: unknown role {other}"))),
                })
                .collect::<Result<_>>()?;
            let index = spec.get("index").and_then(toml::Value::as_str);
            if roles.contains(&Role::Index) != spec.contains_key("index")
                || (spec.contains_key("index")
                    && !matches!(
                        index,
                        Some(
                            "headword" | "variant" | "inflection" | "phrase" | "reading" | "hidden"
                        )
                    ))
            {
                return Err(bad_types(format!(
                    "{name}: an index role needs an index kind, and only then"
                )));
            }
            types.insert(
                name.clone(),
                TypeSpec {
                    kind,
                    children,
                    attrs,
                    roles,
                    index: index.map(String::from),
                    res_attrs,
                },
            );
        }
        for (name, spec) in &types {
            for child in &spec.children {
                if child != "#text" && !types.contains_key(child) {
                    return Err(bad_types(format!("{name}: unknown child type {child}")));
                }
            }
        }
        Ok(Registry { types })
    }

    pub fn get(&self, kind: &str) -> Option<&TypeSpec> {
        self.types.get(kind)
    }

    pub fn contains(&self, kind: &str) -> bool {
        self.types.contains_key(kind)
    }

    /// Default layout; types the registry does not know display inline (spec v1 §3).
    pub fn kind(&self, kind: &str) -> Kind {
        self.get(kind).map_or(Kind::Inline, |spec| spec.kind)
    }

    /// Lookup kind declared by an index-role type.
    pub fn index_kind(&self, kind: &str) -> Option<&str> {
        self.get(kind).and_then(|spec| spec.index.as_deref())
    }

    /// Resource attributes in declaration order; unknown types have none.
    pub fn res_attrs(&self, kind: &str) -> &[String] {
        self.get(kind).map_or(&[], |spec| &spec.res_attrs)
    }

    /// Behaviour roles; unknown types have none.
    pub fn roles(&self, kind: &str) -> &[Role] {
        self.get(kind).map_or(&[], |spec| &spec.roles)
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

/// Add a child type, or every member of a `@group` (groups may name groups).
fn expand(
    groups: &toml::Table,
    child: &str,
    owner: &str,
    open: &mut Vec<String>,
    children: &mut HashSet<String>,
) -> Result<()> {
    let Some(group) = child.strip_prefix('@') else {
        children.insert(child.to_owned());
        return Ok(());
    };
    if open.iter().any(|name| name == group) {
        return Err(bad_types(format!(
            "{owner}: group @{group} contains itself"
        )));
    }
    let members = groups
        .get(group)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| bad_types(format!("{owner}: unknown group @{group}")))?;
    open.push(group.to_owned());
    for member in members {
        let member = member
            .as_str()
            .ok_or_else(|| bad_types(format!("group @{group} has a non-string member")))?;
        expand(groups, member, owner, open, children)?;
    }
    open.pop();
    Ok(())
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

/// Attribute declarations use the protocol's deliberately small grammar.
fn valid_attr_kind(kind: &str) -> bool {
    let kind = kind.strip_suffix('?').unwrap_or(kind);
    if matches!(kind, "string" | "int" | "bool" | "res" | "ref") {
        return true;
    }
    kind.strip_prefix("enum(")
        .and_then(|kind| kind.strip_suffix(')'))
        .is_some_and(|values| {
            values.split('|').all(|value| {
                !value.is_empty()
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            })
        })
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
version = 1
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
    fn expands_groups_inside_groups() {
        let text = "[groups]\nouter = [\"@inner\", \"b\"]\ninner = [\"#text\"]\n\
                    [types.a]\nkind = \"block\"\nchildren = [\"@outer\"]\n";
        let registry = Registry::parse(&format!(
            "[meta]\nformat = \"zdict-types\"\nversion = 1\n{text}[types.b]\nkind = \"inline\"\n"
        ))
        .unwrap();
        let children = &registry.get("a").unwrap().children;
        assert_eq!(
            children,
            &HashSet::from(["#text".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn v1_roles_index_kinds_and_resource_order() {
        let registry = Registry::parse(
            r#"
[meta]
format = "zdict-types"
version = 1
[types.word]
kind = "inline"
roles = ["index", "homograph", "pronunciation", "example", "optional", "choice"]
index = "reading"
attrs = { z_sound = "res?", a_picture = "res", n = "int", flag = "bool?", e = "enum(a|B_2.x-3)?" }
"#,
        )
        .unwrap();
        assert_eq!(registry.index_kind("word"), Some("reading"));
        assert_eq!(registry.res_attrs("word"), ["z_sound", "a_picture"]);
        assert_eq!(
            registry.roles("word"),
            [
                Role::Index,
                Role::Homograph,
                Role::Pronunciation,
                Role::Example,
                Role::Optional,
                Role::Choice
            ]
        );
        assert!(registry.roles("unknown").is_empty());
        assert!(registry.res_attrs("unknown").is_empty());
        assert_eq!(registry.index_kind("unknown"), None);
    }

    #[test]
    fn rejects_invalid_v1_declarations() {
        let prefix = "[meta]\nformat = \"zdict-types\"\nversion = 1\n";
        for declaration in [
            "kind = \"grid\"",
            "kind = \"inline\"\nchildren = [\"missing\"]",
            "kind = \"inline\"\nroles = [\"fly\"]",
            "kind = \"inline\"\nroles = [\"index\"]",
            "kind = \"inline\"\nindex = \"headword\"",
            "kind = \"inline\"\nroles = [\"index\"]\nindex = \"word\"",
            "kind = \"inline\"\nroles = [\"index\"]\nindex = 1",
        ] {
            assert!(
                Registry::parse(&format!("{prefix}[types.a]\n{declaration}")).is_err(),
                "{declaration}"
            );
        }
        for attr in [
            "float",
            "res??",
            "enum()",
            "enum(a|)",
            "enum(a b)",
            "string[]",
        ] {
            assert!(
                Registry::parse(&format!(
                    "{prefix}[types.a]\nkind = \"inline\"\nattrs = {{ a = {attr:?} }}"
                ))
                .is_err(),
                "{attr}"
            );
        }
        for meta in [
            "",
            "[meta]\nformat = \"zdict-types\"\nversion = 0\n",
            "[meta]\nformat = \"zdict-theme\"\nversion = 1\n",
        ] {
            assert!(Registry::parse(&format!("{meta}[types.a]\nkind = \"inline\"")).is_err());
        }
    }

    #[test]
    fn rejects_bad_registries() {
        for bad in [
            "[types.a]\nkind = \"grid\"",
            "[types.a]\nkind = \"block\"\nchildren = [\"@nope\"]",
            "[types.a]\nkind = \"block\"\nroles = [\"fly\"]",
            "[groups]\ng = [\"@g\"]\n[types.a]\nkind = \"block\"\nchildren = [\"@g\"]",
            "not toml =",
        ] {
            assert!(
                matches!(Registry::parse(bad), Err(Error::BadFile { .. })),
                "{bad}"
            );
        }
    }
}
