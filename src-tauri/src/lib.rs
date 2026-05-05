mod commands;
mod extractor;
mod i18n;
mod models;
mod packer;
mod preview;
mod url_extractor;

use commands::AppState;
use extractor::learning;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Load or create default learning model
            let model = learning::load_model(app.handle());
            app.manage(AppState {
                model: std::sync::Mutex::new(model),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::extract_from_clipboard,
            commands::extract_from_url,
            commands::save_files,
            commands::create_archive,
            commands::extract_archive,
            commands::get_archive_info,
            commands::update_entry,
            commands::load_model,
            commands::save_model,
            commands::preview_file,
            commands::get_version,
            commands::get_platform_names,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeepSeek Extractor");
}
