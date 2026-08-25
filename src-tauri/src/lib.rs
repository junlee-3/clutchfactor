mod coach;
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
            coach::key::load_dev_env_local();
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
            commands::get_match_report,
            commands::get_habits,
            commands::get_trends,
            commands::get_round_ticks,
            commands::get_round_review,
            commands::import_corpus_demo,
            commands::build_corpus,
            commands::corpus_status,
            commands::get_grid,
            commands::analyze_positioning,
            commands::get_app_settings,
            commands::set_tracked_override,
            commands::delete_match,
            commands::re_analyze_match,
            commands::coach_status,
            commands::set_gemini_key,
            commands::set_coach_models,
            commands::set_coach_enabled,
            commands::test_gemini_key,
            commands::get_coach_rounds,
            commands::regenerate_coach_round,
            commands::get_coach_synthesis,
            commands::regenerate_coach_synthesis
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
