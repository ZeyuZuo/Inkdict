//! Shared by the oxdata tests: the Oxford pack and the Python reference
//! outputs in oxdata/ (scripts/sync-oxdata.sh, scripts/make-golden.py).
//! Tests skip when oxdata/ is absent: dictionary data is not part of this
//! repository.

use std::path::PathBuf;

use serde_json::Value;
use inkdict_core::Pack;

pub fn oxdata() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../oxdata")
}

/// The pack and one golden file; None (with a note) without oxdata/.
/// A synced pack without its golden file fails: the check would be empty.
pub fn load(golden: &str) -> Option<(Pack, Value)> {
    let pack = oxdata().join("oalecd10.inkdict");
    let golden = oxdata().join("golden").join(golden);
    if !pack.is_file() {
        eprintln!("skipped: {} missing", pack.display());
        return None;
    }
    assert!(
        golden.is_file(),
        "{} missing: run scripts/make-golden.py",
        golden.display()
    );
    let golden = serde_json::from_str(&std::fs::read_to_string(golden).unwrap()).unwrap();
    Some((Pack::open(pack).unwrap(), golden))
}
