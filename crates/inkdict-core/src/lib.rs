//! Inkdict core: reads `.inkdict` dictionary packs (spec/pack-v1.md).
//!
//! Both clients embed this crate: Android through UniFFI, the desktop app
//! (Tauri 2) directly.

mod entry;
mod error;
mod fuzzy;
mod link;
mod meta;
mod node;
mod pack;
mod registry;
mod render;
mod resource;
#[cfg(test)]
mod test_pack;
pub mod text;
pub mod theme;
mod words;

pub use error::{Error, MetaProblem, Result};
pub use fuzzy::max_distance;
pub use link::{Focus, Link, LinkTarget, plain};
pub use meta::Meta;
pub use node::{Attr, Child, Entry, Node};
pub use pack::Pack;
pub use registry::{IndexKind, Kind, Registry, Role, TypeSpec};
pub use render::{
    Anchor, Audio, Collapse, Font, Generated, Icon, Image, Marker, RenderChild, RenderNode,
    Renderer, Tab, TextStyle, interpolate,
};
pub use resource::encode_uri_component;
pub use words::{Group, Word};

/// `PRAGMA application_id` of a pack: ASCII `INKD`.
pub const APPLICATION_ID: u32 = 0x494E_4B44;

/// Pack format version, stored as `PRAGMA user_version`.
pub const FORMAT_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_id_spells_inkd() {
        assert_eq!(&APPLICATION_ID.to_be_bytes(), b"INKD");
    }
}
