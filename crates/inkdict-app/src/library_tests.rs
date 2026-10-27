use super::*;

use crate::tests::sample_pack;

fn copy_pack(source: &Path, dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::copy(source, &path).unwrap();
    dunce::canonicalize(path).unwrap()
}

#[test]
fn legacy_pack_migrates_and_only_writes_the_library_shape() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    std::fs::write(
        &config_path,
        r#"{"pack":"/x.inkdict","settings":{"font_scale":1.3}}"#,
    )
    .unwrap();
    let config = Config::load(&config_path);
    assert_eq!(config.packs, vec![PathBuf::from("/x.inkdict")]);
    assert_eq!(config.active, Some(PathBuf::from("/x.inkdict")));
    assert_eq!(config.settings.font_scale, 1.3);
    let (session, error) = Session::start(Some(config_path.clone()), None);
    assert!(error.is_some());
    assert!(session.dictionary().is_none());
    let library = session.library();
    assert_eq!(library.active, config.active);
    assert!(library.packs[0].title.is_none());
    assert!(library.packs[0].entries.is_none());
    assert!(library.packs[0].bytes.is_none());
    assert!(library.packs[0].error.is_some());
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
    assert!(saved.get("pack").is_none());
    assert_eq!(saved["packs"], serde_json::json!(["/x.inkdict"]));
    assert_eq!(saved["active"], "/x.inkdict");
}

#[test]
fn config_discards_an_active_path_outside_the_library() {
    let config: Config =
        serde_json::from_str(r#"{"packs":["/a.inkdict"],"active":"/b.inkdict"}"#).unwrap();
    assert_eq!(config.active, None);
    // The new format takes precedence if a legacy field also remains.
    let config: Config =
        serde_json::from_str(r#"{"packs":[],"active":null,"pack":"/legacy.inkdict"}"#).unwrap();
    assert_eq!(config, Config::default());
}

#[test]
fn add_activates_first_valid_pack_and_rejects_duplicates_and_invalid_files() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let invalid = dir.path().join("text.inkdict");
    std::fs::write(&invalid, "not a dictionary pack").unwrap();
    let missing = dir.path().join("missing.inkdict");
    let (mut session, error) = Session::start(Some(config_path.clone()), None);
    assert!(error.is_none());
    let alias = pack
        .parent()
        .unwrap()
        .join(".")
        .join(pack.file_name().unwrap());
    let added = session
        .add_packs(vec![
            invalid.clone(),
            pack.clone(),
            alias.clone(),
            missing.clone(),
        ])
        .unwrap();
    assert_eq!(added.library.active.as_ref(), Some(&pack));
    assert_eq!(added.library.packs.len(), 1);
    assert_eq!(added.library.packs[0].path, pack);
    assert!(added.library.packs[0].title.is_some());
    assert_eq!(added.library.packs[0].entries, Some(6));
    assert_eq!(
        added.library.packs[0].bytes,
        Some(std::fs::metadata(&pack).unwrap().len())
    );
    assert!(added.library.packs[0].error.is_none());
    assert_eq!(added.rejected.len(), 3);
    for (rejected, expected) in added.rejected.iter().zip([invalid, alias, missing]) {
        assert_eq!(rejected.path, expected);
        assert!(!rejected.message.is_empty());
    }
    assert!(added.rejected[1].message.contains("already in the library"));
    assert!(session.dictionary().is_some());
    assert_eq!(Config::load(&config_path).active, Some(pack));
}

#[cfg(unix)]
#[test]
fn add_and_use_compare_symlink_targets() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let alias = dir.path().join("alias.inkdict");
    std::os::unix::fs::symlink(&pack, &alias).unwrap();
    let (mut session, _) = Session::start(None, None);
    let added = session
        .add_packs(vec![alias.clone(), pack.clone()])
        .unwrap();
    assert_eq!(added.library.packs[0].path, pack);
    assert_eq!(added.rejected.len(), 1);
    session.use_pack(&alias).unwrap();
    assert_eq!(session.library().active, Some(pack));
}

#[test]
fn move_changes_display_order_only_and_clamps_the_index() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let second = copy_pack(&pack, dir.path(), "second.inkdict");
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()));
    session.add_packs(vec![second.clone()]).unwrap();
    let library = session.move_pack(&pack, usize::MAX).unwrap();
    assert_eq!(library.packs[0].path, second);
    assert_eq!(library.packs[1].path, pack);
    assert_eq!(library.active, Some(pack.clone()));
    assert_eq!(Config::load(&config_path).packs, vec![second, pack.clone()]);
    assert_eq!(session.move_pack(&pack, 0).unwrap().packs[0].path, pack);
}

#[test]
fn remove_active_skips_broken_packs_and_opens_the_next_usable_pack() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let broken = copy_pack(&pack, dir.path(), "broken.inkdict");
    let next = copy_pack(&pack, dir.path(), "next.inkdict");
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()));
    session
        .add_packs(vec![broken.clone(), next.clone()])
        .unwrap();
    std::fs::write(&broken, "no longer a pack").unwrap();
    let library = session.remove_pack(&pack).unwrap();
    assert_eq!(library.active, Some(next.clone()));
    assert!(library.packs[0].error.is_some());
    assert!(library.packs[0].title.is_none());
    assert_eq!(library.packs[0].bytes, Some(16));
    assert!(session.dictionary().is_some());
    assert!(pack.is_file(), "remove must leave the file on disk");
    assert_eq!(Config::load(&config_path).active, Some(next.clone()));
    let library = session.remove_pack(&next).unwrap();
    assert_eq!(library.active, None);
    assert!(session.dictionary().is_none());
    assert!(next.is_file());
    session.remove_pack(&broken).unwrap();
    assert!(session.library().packs.is_empty());
}

#[test]
fn remove_inactive_preserves_the_open_dictionary() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let other = copy_pack(&pack, dir.path(), "other.inkdict");
    let (mut session, _) = Session::start(None, Some(pack.clone()));
    session.add_packs(vec![other.clone()]).unwrap();
    let before = session.dictionary().unwrap() as *const Dictionary;
    let library = session.remove_pack(&other).unwrap();
    assert_eq!(library.active, Some(pack));
    assert_eq!(session.dictionary().unwrap() as *const Dictionary, before);
}

#[test]
fn use_requires_membership_and_failed_open_preserves_the_active_pack() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let other = copy_pack(&pack, dir.path(), "other.inkdict");
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()));
    // Even a valid pack must be explicitly added before it can be used.
    assert_eq!(session.use_pack(&other).unwrap_err().code, "not_found");
    assert_eq!(session.move_pack(&other, 0).unwrap_err().code, "not_found");
    assert_eq!(session.remove_pack(&other).unwrap_err().code, "not_found");
    session.add_packs(vec![other.clone()]).unwrap();
    let settings = Settings {
        hide_examples: true,
        ..Settings::default()
    };
    session.set_settings(settings.clone()).unwrap();
    assert_eq!(session.use_pack(&other).unwrap().entries, Some(6));
    assert_eq!(session.dictionary().unwrap().settings(), &settings);
    assert_eq!(Config::load(&config_path).active, Some(other.clone()));
    session.use_pack(&pack).unwrap();
    std::fs::remove_file(&other).unwrap();
    assert!(session.use_pack(&other).is_err());
    assert_eq!(session.library().active, Some(pack));
    assert!(session.dictionary().is_some());
}

#[test]
fn startup_override_adds_or_selects_a_pack_and_preserves_settings() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let other = copy_pack(&pack, dir.path(), "other.inkdict");
    let config_path = dir.path().join("config.json");
    let settings = Settings {
        hide_pronunciation: true,
        ..Settings::default()
    };
    Config {
        packs: vec![pack.clone()],
        active: Some(pack.clone()),
        settings: settings.clone(),
    }
    .save(&config_path)
    .unwrap();
    let (session, error) = Session::start(Some(config_path.clone()), Some(other.clone()));
    assert!(error.is_none());
    assert_eq!(session.library().active, Some(other));
    assert_eq!(session.library().packs.len(), 2);
    assert_eq!(session.dictionary().unwrap().settings(), &settings);
    let (session, error) = Session::start(Some(config_path.clone()), Some(pack.clone()));
    assert!(error.is_none());
    assert_eq!(session.library().packs.len(), 2);
    assert_eq!(Config::load(&config_path).active, Some(pack));
    let (restored, error) = Session::start(Some(config_path), None);
    assert!(error.is_none());
    assert_eq!(restored.library().active, session.library().active);
    assert_eq!(restored.dictionary().unwrap().settings(), &settings);
}

#[test]
fn settings_are_saved_only_after_the_open_dictionary_accepts_them() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack));
    let accepted = Settings {
        hide_examples: true,
        ..Settings::default()
    };
    assert_eq!(session.set_settings(accepted.clone()).unwrap(), accepted);
    assert_eq!(Config::load(&config_path).settings, accepted);
    assert_eq!(session.dictionary().unwrap().settings(), &accepted);
    // A type with both roles is hidden once, not as two equal tables.
    session.dictionary.as_mut().unwrap().registry = inkdict_core::Registry::parse(
        "[meta]\nformat = \"inkdict-types\"\nversion = 1\n[types.both]\nkind = \"inline\"\nroles = [\"pronunciation\", \"example\"]\n",
    )
    .unwrap();
    let both = Settings {
        hide_pronunciation: true,
        hide_examples: true,
        font_scale: 1.5,
    };
    assert_eq!(session.set_settings(both.clone()).unwrap(), both);
    assert_eq!(Config::load(&config_path).settings, both);
}

#[test]
fn settings_without_an_open_pack_are_saved() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), None);
    let settings = Settings {
        font_scale: 1.2,
        ..Settings::default()
    };
    session.set_settings(settings.clone()).unwrap();
    assert_eq!(Config::load(&config_path).settings, settings);
}

#[test]
fn in_memory_session_writes_nothing() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let local = copy_pack(&pack, dir.path(), "local.inkdict");
    let before = std::fs::read(&local).unwrap();
    let (mut session, error) = Session::start(None, Some(local.clone()));
    assert!(error.is_none());
    assert!(session.config_path.is_none());
    session.add_packs(vec![pack]).unwrap();
    session.move_pack(&local, 1).unwrap();
    session.use_pack(&local).unwrap();
    session
        .set_settings(Settings {
            hide_examples: true,
            ..Settings::default()
        })
        .unwrap();
    session.remove_pack(&local).unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(local).unwrap(), before);
}

#[test]
fn sample_legacy_config_migrates_a_valid_pack_and_restores_settings() {
    let dir = tempfile::tempdir().unwrap();
    let pack = copy_pack(&sample_pack(), dir.path(), "legacy.inkdict");
    let config_path = dir.path().join("config.json");
    std::fs::write(
        &config_path,
        serde_json::to_vec(&serde_json::json!({
            "pack": pack, "settings": {"hide_examples": true, "font_scale": 1.3}
        }))
        .unwrap(),
    )
    .unwrap();
    let (session, error) = Session::start(Some(config_path.clone()), None);
    assert!(error.is_none());
    assert_eq!(session.library().active, Some(pack.clone()));
    assert_eq!(session.library().packs.len(), 1);
    assert_eq!(session.dictionary().unwrap().info().entries, Some(6));
    assert!(session.dictionary().unwrap().settings().hide_examples);
    assert_eq!(session.dictionary().unwrap().settings().font_scale, 1.3);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config_path).unwrap()).unwrap();
    assert!(saved.get("pack").is_none());
    assert_eq!(saved["packs"], serde_json::json!([pack]));
    assert_eq!(saved["active"], serde_json::json!(pack));
}

#[test]
fn sample_startup_keeps_a_broken_pack_with_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let pack = copy_pack(&sample_pack(), dir.path(), "broken.inkdict");
    let config_path = dir.path().join("config.json");
    let (session, error) = Session::start(Some(config_path.clone()), Some(pack.clone()));
    assert!(error.is_none());
    drop(session);
    std::fs::write(&pack, "broken").unwrap();
    let (session, error) = Session::start(Some(config_path.clone()), None);
    assert!(error.is_some());
    assert!(session.dictionary().is_none());
    let library = session.library();
    assert_eq!(library.active, Some(pack.clone()));
    assert_eq!(library.packs.len(), 1);
    assert_eq!(library.packs[0].path, pack);
    assert!(library.packs[0].error.is_some());
    assert_eq!(library.packs[0].title, None);
    assert_eq!(library.packs[0].entries, None);
    assert_eq!(library.packs[0].bytes, Some(6));
    assert_eq!(Config::load(&config_path).packs, [pack]);
}
