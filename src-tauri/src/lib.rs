mod analysis;
mod commands;

use commands::{AnalysisState, AppState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialise app state
            app.manage(AppState {
                analysis_state: AnalysisState::new(),
            });

            // Start background analysis worker
            commands::start_background_worker(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ingest_audio_file,
            commands::ingest_youtube,
            commands::get_analysis_status,
            commands::get_analysis_result,
            commands::list_analysed_tracks,
            commands::delete_analysis,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
