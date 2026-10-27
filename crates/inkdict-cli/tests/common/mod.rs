use std::{path::PathBuf, sync::OnceLock};

pub fn sample_pack() -> PathBuf {
    static PACK: OnceLock<PathBuf> = OnceLock::new();
    PACK.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("inkdict-cli-sample");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.inkdict");
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../spec/samples/en-zh/manifest.toml");
        let mut options = inkdict_pack::Options::new(manifest);
        options.output = Some(path.clone());
        inkdict_pack::build(options).unwrap();
        path
    })
    .clone()
}
