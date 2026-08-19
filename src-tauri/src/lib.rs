mod commands;

use std::sync::Mutex;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = cf_store::Store::open(&data_dir.join("clutchfactor.db"))
                .map_err(|e| format!("failed to open database: {e}"))?;
            app.manage(AppState {
                store: Mutex::new(store),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_demo,
            commands::list_matches,
            commands::tracked_player,
            commands::get_match_detail,
            commands::get_round_ticks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
