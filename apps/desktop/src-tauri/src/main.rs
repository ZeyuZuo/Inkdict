//! ZDict desktop: Tauri commands over `zdict-app` (apps/desktop/README.md).

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bench;
mod commands;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::Manager;
use zdict_app::{Config, Dictionary};

/// Shared by every command: the open dictionary (if any) and where the
/// configuration lives.
pub struct AppState {
    pub dictionary: Arc<RwLock<Option<Dictionary>>>,
    pub config_path: PathBuf,
}

impl AppState {
    /// Open `path`, remember it, and read the word index in the background
    /// so the first keystroke is answered quickly too.
    pub fn open(&self, path: PathBuf) -> zdict_app::Result<()> {
        let mut config = Config::load(&self.config_path);
        let dictionary = Dictionary::open(&path, config.settings.clone())?;
        *self.dictionary.write().unwrap_or_else(|e| e.into_inner()) = Some(dictionary);
        config.pack = Some(path);
        config.save(&self.config_path)?;
        let shared = Arc::clone(&self.dictionary);
        std::thread::spawn(move || {
            if let Some(dictionary) = shared.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
                let _ = dictionary.warm_up();
            }
        });
        Ok(())
    }
}

/// `--pack <path>`, then `ZDICT_PACK`, then the remembered path.
fn pack_path(config: &Config) -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--pack" {
            return args.next().map(PathBuf::from);
        }
        if let Some(path) = arg.strip_prefix("--pack=") {
            return Some(PathBuf::from(path));
        }
    }
    std::env::var_os("ZDICT_PACK")
        .map(PathBuf::from)
        .or_else(|| config.pack.clone())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("config.json");
            let config = Config::load(&config_path);
            let state = AppState {
                dictionary: Arc::new(RwLock::new(None)),
                config_path,
            };
            if let Some(path) = pack_path(&config) {
                // A missing or broken pack is not fatal: the UI asks for one.
                if let Err(err) = state.open(path.clone()) {
                    eprintln!("zdict: cannot open {}: {err}", path.display());
                }
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pack_info,
            commands::open_pack,
            commands::words,
            commands::entry,
            commands::follow,
            commands::settings,
            commands::set_settings,
            commands::theme_css,
            bench::bench_plan,
            bench::bench_report,
            bench::bench_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the ZDict desktop app");
}
