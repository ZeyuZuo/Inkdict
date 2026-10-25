//! ZDict core: reads `.zdpack` dictionary packs (spec/pack-v0.md).
//!
//! Both clients embed this crate: Android through UniFFI, the desktop app
//! (Tauri 2) directly.

mod entry;
mod error;
mod fuzzy;
mod link;
mod lookup;
mod meta;
mod node;
mod pack;
mod registry;
mod render;
mod resource;
mod search;
#[cfg(test)]
mod test_pack;
pub mod text;
pub mod theme;

pub use error::{Error, MetaProblem, Result};
pub use fuzzy::{Correction, max_distance};
pub use link::{Focus, Link, LinkTarget, plain};
pub use lookup::Suggestion;
pub use meta::Meta;
pub use node::{Attr, Child, Entry, Node};
pub use pack::Pack;
pub use registry::{Kind, Registry, Role, TypeSpec};
pub use render::{
    Anchor, Audio, Collapse, Generated, Icon, Image, Marker, RenderChild, RenderNode, Renderer,
    TextStyle, interpolate,
};
pub use resource::encode_uri_component;
pub use search::{Column, Hit, split_rowid};

/// `PRAGMA application_id` of a pack: ASCII `ZDPK`.
pub const APPLICATION_ID: u32 = 0x5A44_504B;

/// Pack format version, stored as `PRAGMA user_version`.
pub const FORMAT_VERSION: u32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_id_spells_zdpk() {
        assert_eq!(&APPLICATION_ID.to_be_bytes(), b"ZDPK");
    }
}
