//! Inkdict desktop: Tauri commands over `inkdict-app` (apps/desktop/README.md).

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bench;
mod commands;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use inkdict_app::Session;
use tauri::Manager;

/// Shared by every command: configuration, library, and the open dictionary.
pub struct AppState {
    pub session: Arc<RwLock<Session>>,
}

impl AppState {
    /// Prepare the current dictionary's word index under a read lock.
    pub fn warm_up(&self) {
        let shared = Arc::clone(&self.session);
        std::thread::spawn(move || {
            if let Some(dictionary) = shared
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .dictionary()
            {
                let _ = dictionary.warm_up();
            }
        });
    }
}

/// `--pack <path>`, then `INKDICT_PACK`; Session loads the remembered active pack.
fn pack_path() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--pack" {
            return args.next().map(PathBuf::from);
        }
        if let Some(path) = arg.strip_prefix("--pack=") {
            return Some(PathBuf::from(path));
        }
    }
    std::env::var_os("INKDICT_PACK").map(PathBuf::from)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("config.json");
            let (session, error) = Session::start(Some(config_path), pack_path());
            if let Some(error) = error {
                eprintln!("inkdict: {error}");
            }
            let state = AppState {
                session: Arc::new(RwLock::new(session)),
            };
            state.warm_up();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pack_info,
            commands::library,
            commands::add_packs,
            commands::remove_pack,
            commands::move_pack,
            commands::use_pack,
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
        .expect("error while running the Inkdict desktop app");
}
