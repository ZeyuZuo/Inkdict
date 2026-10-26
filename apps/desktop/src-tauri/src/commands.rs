//! One function per command of apps/desktop/README.md. Each runs on
//! Tauri's thread pool and reads the shared dictionary.

use std::path::PathBuf;

use tauri::State;
use zdict_app::{
    AppError, Config, Dictionary, EntryView, PackInfo, Result, Settings, Target, WordView,
};
use zdict_core::Link;

use crate::AppState;

/// Run `f` on the open dictionary, or fail with `no_pack`.
fn with<T>(state: &State<'_, AppState>, f: impl FnOnce(&Dictionary) -> Result<T>) -> Result<T> {
    let guard = state.dictionary.read().unwrap_or_else(|e| e.into_inner());
    f(guard.as_ref().ok_or_else(AppError::no_pack)?)
}

#[tauri::command]
pub fn pack_info(state: State<'_, AppState>) -> Result<PackInfo> {
    with(&state, |dictionary| Ok(dictionary.info()))
}

#[tauri::command]
pub fn open_pack(state: State<'_, AppState>, path: PathBuf) -> Result<PackInfo> {
    state.open(path)?;
    pack_info(state)
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
pub fn settings(state: State<'_, AppState>) -> Result<Settings> {
    let guard = state.dictionary.read().unwrap_or_else(|e| e.into_inner());
    Ok(match guard.as_ref() {
        Some(dictionary) => dictionary.settings().clone(),
        None => Config::load(&state.config_path).settings,
    })
}

/// Apply and then remember new settings; later `entry` calls render with
/// them. Settings that fail to apply are not saved.
#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    {
        let mut guard = state.dictionary.write().unwrap_or_else(|e| e.into_inner());
        if let Some(dictionary) = guard.as_mut() {
            dictionary.set_settings(settings.clone())?;
        }
    }
    let mut config = Config::load(&state.config_path);
    config.settings = settings.clone();
    config.save(&state.config_path)?;
    Ok(settings)
}

#[tauri::command]
pub fn theme_css(state: State<'_, AppState>) -> Result<String> {
    with(&state, |dictionary| Ok(dictionary.theme_css()))
}
