//! ZDict core: reads `.zdpack` dictionary packs (spec/pack-v0.md).
//!
//! Every app embeds this crate: Android through UniFFI, the desktop app
//! through Tauri, the web app and server through WebAssembly.

mod error;
mod meta;
mod pack;

pub use error::{Error, MetaProblem, Result};
pub use meta::Meta;
pub use pack::Pack;

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
