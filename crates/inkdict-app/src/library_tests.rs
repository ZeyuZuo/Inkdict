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
    let (session, error) = Session::start(Some(config_path.clone()), None, None);
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
    let (mut session, error) = Session::start(Some(config_path.clone()), None, None);
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
    let (mut session, _) = Session::start(None, None, None);
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
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
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
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
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
    let (mut session, _) = Session::start(None, Some(pack.clone()), None);
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
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
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
        ..Config::default()
    }
    .save(&config_path)
    .unwrap();
    let (session, error) = Session::start(Some(config_path.clone()), Some(other.clone()), None);
    assert!(error.is_none());
    assert_eq!(session.library().active, Some(other));
    assert_eq!(session.library().packs.len(), 2);
    assert_eq!(session.dictionary().unwrap().settings(), &settings);
    let (session, error) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
    assert!(error.is_none());
    assert_eq!(session.library().packs.len(), 2);
    assert_eq!(Config::load(&config_path).active, Some(pack));
    let (restored, error) = Session::start(Some(config_path), None, None);
    assert!(error.is_none());
    assert_eq!(restored.library().active, session.library().active);
    assert_eq!(restored.dictionary().unwrap().settings(), &settings);
}

#[test]
fn settings_are_saved_only_after_the_open_dictionary_accepts_them() {
    let pack = dunce::canonicalize(sample_pack()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), Some(pack), None);
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
    let (mut session, _) = Session::start(Some(config_path.clone()), None, None);
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
    let (mut session, error) = Session::start(None, Some(local.clone()), None);
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
fn theme_choices_persist_by_id_across_sessions_and_all_open_paths() {
    let dir = tempfile::tempdir().unwrap();
    let pack = copy_pack(&sample_pack(), dir.path(), "sample.inkdict");
    let other = copy_pack(&sample_pack(), dir.path(), "other.inkdict");
    let db = rusqlite::Connection::open(&other).unwrap();
    db.execute("UPDATE meta SET value = 'other' WHERE key = 'id'", [])
        .unwrap();
    drop(db);
    let config_path = dir.path().join("config.json");
    let (mut session, error) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
    assert!(error.is_none());
    let info = session.set_theme("plain").unwrap();
    assert_eq!(info.theme, "plain");
    assert_eq!(
        Config::load(&config_path).themes,
        [(info.id.clone(), "plain".into())].into()
    );
    let saved = std::fs::read(&config_path).unwrap();
    assert_eq!(session.set_theme("missing").unwrap_err().code, "bad_input");
    assert_eq!(session.dictionary().unwrap().info(), info);
    assert_eq!(std::fs::read(&config_path).unwrap(), saved);
    drop(session);

    let (mut session, error) = Session::start(Some(config_path.clone()), None, None);
    assert!(error.is_none());
    assert_eq!(session.dictionary().unwrap().info(), info);
    session.add_packs(vec![other.clone()]).unwrap();
    assert_eq!(session.use_pack(&other).unwrap().theme, "default");
    assert_eq!(session.use_pack(&pack).unwrap().theme, "plain");
    session.use_pack(&other).unwrap();
    session.remove_pack(&other).unwrap();
    assert_eq!(session.dictionary().unwrap().info(), info);

    // A moved/rebuilt pack has the same id, so even add_packs restores its choice.
    session.remove_pack(&pack).unwrap();
    assert_eq!(session.set_theme("plain").unwrap_err().code, "no_pack");
    let moved = copy_pack(&pack, dir.path(), "moved.inkdict");
    let added = session.add_packs(vec![moved.clone()]).unwrap();
    assert!(added.rejected.is_empty());
    assert_eq!(session.dictionary().unwrap().info(), info);
    drop(session);
    let (mut session, error) = Session::start(Some(config_path.clone()), Some(moved), None);
    assert!(error.is_none());
    assert_eq!(session.dictionary().unwrap().info(), info);
    assert_eq!(session.set_theme("default").unwrap().theme, "default");
    assert!(!Config::load(&config_path).themes.contains_key(&info.id));
    let (session, error) = Session::start(Some(config_path), None, None);
    assert!(error.is_none());
    assert_eq!(session.dictionary().unwrap().info().theme, "default");
}

#[test]
fn missing_saved_theme_falls_back_to_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let pack = sample_pack();
    Config {
        packs: vec![pack.clone()],
        active: Some(pack),
        themes: [("sample-en-zh".into(), "removed".into())].into(),
        ..Config::default()
    }
    .save(&config_path)
    .unwrap();
    let (session, error) = Session::start(Some(config_path), None, None);
    assert!(error.is_none());
    assert_eq!(session.dictionary().unwrap().info().theme, "default");
}

#[test]
fn config_without_themes_loads_library_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let pack = sample_pack();
    std::fs::write(&config_path, serde_json::to_vec(&serde_json::json!({
        "packs": [pack], "active": pack, "settings": { "hide_examples": true, "font_scale": 1.3 }
    })).unwrap()).unwrap();
    let config = Config::load(&config_path);
    assert_eq!(config.packs, vec![pack.clone()]);
    assert_eq!(config.active, Some(pack));
    assert!(config.settings.hide_examples);
    assert_eq!(config.settings.font_scale, 1.3);
    assert!(config.themes.is_empty());
    let (session, error) = Session::start(Some(config_path), None, None);
    assert!(error.is_none());
    assert_eq!(session.dictionary().unwrap().info().theme, "default");
    assert!(session.dictionary().unwrap().settings().hide_examples);
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
    let (session, error) = Session::start(Some(config_path.clone()), None, None);
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
    let (session, error) = Session::start(Some(config_path.clone()), Some(pack.clone()), None);
    assert!(error.is_none());
    drop(session);
    std::fs::write(&pack, "broken").unwrap();
    let (session, error) = Session::start(Some(config_path.clone()), None, None);
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

fn age(path: &Path) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(10))
        .unwrap();
}

fn folder_pack(dir: &Path, name: &str) -> PathBuf {
    let path = copy_pack(&sample_pack(), dir, name);
    age(&path);
    path
}

#[test]
fn start_scans_only_finished_visible_packs_in_name_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let z = folder_pack(root.as_path(), "z.inkdict");
    let a = folder_pack(root.as_path(), "a.INKDICT");
    for name in [".hidden.inkdict", "partial.inkdict.part", "notes.txt"] {
        folder_pack(root.as_path(), name);
    }
    let recent = copy_pack(&sample_pack(), root.as_path(), "recent.inkdict");
    std::fs::create_dir(root.as_path().join("sub.inkdict")).unwrap();
    folder_pack(&root.as_path().join("sub.inkdict"), "nested.inkdict");
    let (mut session, error) = Session::start(None, None, Some(root.as_path().to_owned()));
    assert!(error.is_none());
    let library = session.library();
    assert_eq!(
        library.folder,
        Some(dunce::canonicalize(root.as_path()).unwrap())
    );
    assert_eq!(library.active, Some(a.clone()));
    assert_eq!(session.config.packs, [a, z]);
    assert!(library.packs.iter().all(|item| item.in_folder));
    assert!(session.dictionary().is_some());
    age(&recent);
    session.rescan().unwrap();
    assert_eq!(session.config.packs.last(), Some(&recent));
}

#[test]
fn start_creates_the_folder_and_reports_creation_errors() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let folder = root.as_path().join("new/dictionaries");
    let (session, error) = Session::start(None, None, Some(folder.clone()));
    assert!(error.is_none());
    assert!(folder.is_dir());
    assert_eq!(session.folder(), Some(folder.as_path()));
    let file = root.as_path().join("file");
    std::fs::write(&file, "not a directory").unwrap();
    let (_, error) = Session::start(None, None, Some(file));
    assert_eq!(error.unwrap().code, "internal");
}

// Deletes a file the session holds open, which Windows does not allow
// (the app itself closes a dictionary before trashing it).
#[cfg(not(windows))]
#[test]
fn rescan_preserves_order_and_outside_entries_and_replaces_deleted_active() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let folder = root.as_path().join("dictionaries");
    std::fs::create_dir(&folder).unwrap();
    let a = folder_pack(&folder, "a.inkdict");
    let broken = folder_pack(&folder, "b.inkdict");
    let c = folder_pack(&folder, "c.inkdict");
    std::fs::write(&broken, "broken").unwrap();
    age(&broken);
    let outside = folder_pack(root.as_path(), "outside.inkdict");
    let config_path = root.as_path().join("config.json");
    Config {
        packs: vec![outside.clone()],
        ..Config::default()
    }
    .save(&config_path)
    .unwrap();
    let (mut session, error) =
        Session::start(Some(config_path.clone()), Some(a.clone()), Some(folder));
    assert!(error.is_none());
    assert!(!session.library().packs[0].in_folder);
    session.move_pack(&outside, usize::MAX).unwrap();
    let new = folder_pack(a.parent().unwrap(), "aa.inkdict");
    std::fs::remove_file(&a).unwrap();
    let library = session.rescan().unwrap();
    assert_eq!(library.active, Some(c.clone()));
    assert_eq!(
        session.config.packs,
        [broken.clone(), c, outside.clone(), new.clone()]
    );
    assert_eq!(Config::load(&config_path).packs, session.config.packs);
    // Outside missing files remain visible, while folder files are reconciled.
    std::fs::remove_file(&outside).unwrap();
    let before = session.dictionary().unwrap() as *const Dictionary;
    // A scan must preserve even an active file that can no longer be opened.
    let active = library.active.unwrap();
    std::fs::write(&active, "changed").unwrap();
    session.rescan().unwrap();
    assert_eq!(session.dictionary().unwrap() as *const Dictionary, before);
    assert_eq!(
        session.config.packs,
        [broken, active.clone(), outside, new.clone()]
    );
    std::fs::remove_file(active).unwrap();
    assert_eq!(session.rescan().unwrap().active, Some(new.clone()));
    std::fs::remove_file(new).unwrap();
    assert_eq!(session.rescan().unwrap().active, None);
    assert!(session.dictionary().is_none());
}

#[test]
fn add_copies_into_folder_and_rejects_clashes_and_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let folder = root.as_path().join("dictionaries");
    let source = folder_pack(root.as_path(), "source.inkdict");
    let (mut session, error) = Session::start(None, None, Some(folder.clone()));
    assert!(error.is_none());
    let added = session.add_packs(vec![source.clone()]).unwrap();
    let destination = folder.join("source.inkdict");
    assert!(added.rejected.is_empty());
    assert!(source.is_file());
    assert_eq!(
        std::fs::read(&source).unwrap(),
        std::fs::read(&destination).unwrap()
    );
    assert_eq!(added.library.active, Some(destination.clone()));
    assert!(added.library.packs[0].in_folder);
    assert!(session.dictionary().is_some());
    assert!(!folder.join(".source.inkdict.part").exists());
    let added = session
        .add_packs(vec![source, destination.clone()])
        .unwrap();
    assert_eq!(added.rejected.len(), 2);
    assert!(
        added.rejected[0]
            .message
            .contains("a file named source.inkdict is already in the dictionary folder")
    );
    assert!(added.rejected[1].message.contains("already in the library"));
    let existing = copy_pack(&sample_pack(), &folder, "existing.inkdict");
    let added = session.add_packs(vec![existing.clone()]).unwrap();
    assert!(added.rejected.is_empty());
    assert_eq!(added.library.packs[1].path, existing);
    assert_eq!(std::fs::read_dir(folder).unwrap().count(), 2);
}

#[test]
fn copy_failure_cleans_up_its_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let missing = root.as_path().join("missing.inkdict");
    assert_eq!(
        copy_into(&missing, root.as_path()).unwrap_err().code,
        "internal"
    );
    assert!(!root.as_path().join(".missing.inkdict.part").exists());
}

#[test]
fn remove_folder_file_discards_it_and_selects_next_pack() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let a = folder_pack(root.as_path(), "a.inkdict");
    let b = folder_pack(root.as_path(), "b.inkdict");
    let (mut session, _) = Session::start(None, None, Some(root.as_path().to_owned()));
    let library = session.remove_pack(&a).unwrap();
    assert!(!a.exists());
    assert_eq!(library.active, Some(b.clone()));
    assert!(session.dictionary().is_some());
    session.remove_pack(&b).unwrap();
    assert!(!b.exists());
    assert!(session.dictionary().is_none());
    assert!(session.config.packs.is_empty());
}

// Deletes a file the session holds open, which Windows does not allow
// (the app itself closes a dictionary before trashing it).
#[cfg(not(windows))]
#[test]
fn failed_discard_keeps_the_library_entry() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let pack = folder_pack(root.as_path(), "a.inkdict");
    let (mut session, _) = Session::start(None, None, Some(root.as_path().to_owned()));
    std::fs::remove_file(&pack).unwrap();
    assert_eq!(session.remove_pack(&pack).unwrap_err().code, "internal");
    assert_eq!(session.config.packs, std::slice::from_ref(&pack));
    assert_eq!(session.config.active, Some(pack));
}

#[test]
fn set_folder_forgets_old_entries_keeps_files_and_scans_new_folder() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let old = root.as_path().join("old");
    let new = root.as_path().join("new");
    std::fs::create_dir(&old).unwrap();
    std::fs::create_dir(&new).unwrap();
    let a = folder_pack(&old, "a.inkdict");
    let b = folder_pack(&new, "b.inkdict");
    let outside = folder_pack(root.as_path(), "outside.inkdict");
    let config_path = root.as_path().join("config.json");
    let (mut session, _) = Session::start(Some(config_path.clone()), None, Some(old.clone()));
    // Seed a legacy outside entry, which switching folders must preserve.
    session.config.packs.push(outside.clone());
    let library = session.set_folder(new.clone()).unwrap();
    assert_eq!(library.folder, Some(new.clone()));
    assert_eq!(library.active, Some(outside.clone()));
    assert_eq!(session.config.packs, [outside.clone(), b]);
    assert!(a.is_file());
    assert_eq!(Config::load(&config_path).folder, Some(new.clone()));
    let before = session.dictionary().unwrap() as *const Dictionary;
    session.set_folder(new.clone()).unwrap();
    assert_eq!(session.dictionary().unwrap() as *const Dictionary, before);
    for invalid in [a, root.as_path().join("missing")] {
        assert_eq!(session.set_folder(invalid).unwrap_err().code, "bad_input");
        assert_eq!(session.folder(), Some(new.as_path()));
    }
    drop(session);
    let (session, error) = Session::start(Some(config_path), None, Some(old));
    assert!(error.is_none());
    assert_eq!(session.folder(), Some(new.as_path()));
    assert_eq!(session.library().active, Some(outside));
}

#[test]
fn set_folder_activates_a_new_pack_when_no_outside_pack_remains() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let a = folder_pack(old.path(), "a.inkdict");
    let b = folder_pack(new.path(), "b.inkdict");
    let (mut session, _) = Session::start(None, None, Some(old.path().to_owned()));
    assert_eq!(
        session.set_folder(new.path().to_owned()).unwrap().active,
        Some(b)
    );
    assert!(a.is_file());
}

#[cfg(unix)]
#[test]
fn folder_membership_compares_canonical_parents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let folder = root.as_path().join("dictionaries");
    std::fs::create_dir(&folder).unwrap();
    let pack = folder_pack(&folder, "a.inkdict");
    let alias = root.as_path().join("alias");
    std::os::unix::fs::symlink(&folder, &alias).unwrap();
    let (mut session, error) = Session::start(None, Some(alias.join("a.inkdict")), Some(alias));
    assert!(error.is_none());
    assert_eq!(session.library().active, Some(pack.clone()));
    assert!(session.library().packs[0].in_folder);
    std::fs::remove_file(pack).unwrap();
    assert!(session.rescan().unwrap().packs.is_empty());
}

#[test]
fn old_config_without_folder_keeps_the_host_default() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let config_path = root.as_path().join("config.json");
    std::fs::write(&config_path, r#"{"packs":[],"active":null}"#).unwrap();
    assert!(Config::load(&config_path).folder.is_none());
    let folder = root.as_path().join("dictionaries");
    let (session, error) = Session::start(Some(config_path.clone()), None, Some(folder.clone()));
    assert!(error.is_none());
    assert_eq!(session.library().folder, Some(folder));
    assert!(Config::load(&config_path).folder.is_none());
}
