//! One function per command of apps/desktop/README.md, over the shared session.

use std::path::PathBuf;
use std::sync::Arc;

use inkdict_app::{
    Added, AppError, Dictionary, EntryView, Library, PackInfo, Result, Settings, Target, WordView,
};
use inkdict_core::Link;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::AppState;

/// Run `f` on the open dictionary, or fail with `no_pack`.
fn with<T>(state: &State<'_, AppState>, f: impl FnOnce(&Dictionary) -> Result<T>) -> Result<T> {
    let guard = state.session.read().unwrap_or_else(|e| e.into_inner());
    f(guard.dictionary().ok_or_else(AppError::no_pack)?)
}

#[tauri::command]
pub fn pack_info(state: State<'_, AppState>) -> Result<PackInfo> {
    with(&state, |dictionary| Ok(dictionary.info()))
}

#[tauri::command]
pub fn library(state: State<'_, AppState>) -> Library {
    state
        .session
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .library()
}

#[tauri::command]
pub async fn add_packs(state: State<'_, AppState>, paths: Vec<PathBuf>) -> Result<Added> {
    let shared = Arc::clone(&state.session);
    let (result, opened) = tauri::async_runtime::spawn_blocking(move || {
        let mut session = shared.write().unwrap_or_else(|e| e.into_inner());
        let had_dictionary = session.dictionary().is_some();
        let result = session.add_packs(paths);
        let opened = !had_dictionary && session.dictionary().is_some();
        (result, opened)
    })
    .await
    .map_err(|error| AppError::new("internal", error.to_string()))?;
    if opened {
        state.warm_up();
    }
    result
}

#[tauri::command]
pub fn rescan(state: State<'_, AppState>) -> Result<Library> {
    let mut session = state.session.write().unwrap_or_else(|e| e.into_inner());
    let active = session.library().active;
    let result = session.rescan();
    let opened = session.dictionary().is_some() && session.library().active != active;
    drop(session);
    if opened {
        state.warm_up();
    }
    result
}

#[tauri::command]
pub fn set_folder(state: State<'_, AppState>, path: PathBuf) -> Result<Library> {
    let mut session = state.session.write().unwrap_or_else(|e| e.into_inner());
    let active = session.library().active;
    let result = session.set_folder(path);
    let opened = session.dictionary().is_some() && session.library().active != active;
    drop(session);
    if opened {
        state.warm_up();
    }
    result
}

#[tauri::command]
pub fn show_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    let folder = state
        .session
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .folder()
        .map(PathBuf::from)
        .ok_or_else(|| AppError::new("bad_input", "no dictionary folder is configured"))?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| AppError::new("internal", error.to_string()))
}

#[tauri::command]
pub fn remove_pack(state: State<'_, AppState>, path: PathBuf) -> Result<Library> {
    let mut session = state.session.write().unwrap_or_else(|e| e.into_inner());
    let active = session.library().active;
    let result = session.remove_pack(&path);
    let opened = session.dictionary().is_some() && session.library().active != active;
    drop(session);
    if opened {
        state.warm_up();
    }
    result
}

#[tauri::command]
pub fn move_pack(state: State<'_, AppState>, path: PathBuf, index: usize) -> Result<Library> {
    state
        .session
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .move_pack(&path, index)
}

#[tauri::command]
pub fn use_pack(state: State<'_, AppState>, path: PathBuf) -> Result<PackInfo> {
    let result = state
        .session
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .use_pack(&path);
    if result.is_ok() {
        state.warm_up();
    }
    result
}

#[tauri::command]
pub fn words(state: State<'_, AppState>, text: String, limit: usize) -> Result<Vec<WordView>> {
    with(&state, |dictionary| dictionary.words(&text, limit))
}

#[tauri::command]
pub fn entry(state: State<'_, AppState>, key: String) -> Result<EntryView> {
    with(&state, |dictionary| dictionary.entry(&key))
}

#[tauri::command]
pub fn follow(state: State<'_, AppState>, current: String, link: Link) -> Result<Target> {
    with(&state, |dictionary| dictionary.follow(&current, &link))
}

#[tauri::command]
pub fn settings(state: State<'_, AppState>) -> Settings {
    state
        .session
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .settings()
        .clone()
}

#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    state
        .session
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .set_settings(settings)
}

#[tauri::command]
pub fn set_theme(state: State<'_, AppState>, name: String) -> Result<PackInfo> {
    state
        .session
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .set_theme(&name)
}

#[tauri::command]
pub fn theme_css(state: State<'_, AppState>) -> Result<String> {
    with(&state, |dictionary| Ok(dictionary.theme_css()))
}
