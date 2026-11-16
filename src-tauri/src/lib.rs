mod application;
mod commands;
mod domain;
mod infrastructure;
#[cfg(test)]
mod integration_tests;
use application::assistant::AssistantService;
use commands::*;
use infrastructure::{
    lm_studio::{self, LmStudioGateway},
    platform,
    sqlite::Store as SqliteStore,
};
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let store = SqliteStore::open(&dir.join("jarvis.db")).map_err(std::io::Error::other)?;
            app.manage(DesktopState {
                assistant: AssistantService::new(
                    Box::new(store),
                    Box::new(LmStudioGateway::local()?),
                ),
                platform: platform::info(),
                endpoint: lm_studio::ENDPOINT,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            discover_models,
            create_conversation,
            get_messages,
            delete_conversation,
            send_message,
            cancel_generation,
            get_activity,
            search_conversations
        ])
        .run(tauri::generate_context!())
        .expect("Failed to start Kineto Jarvis");
}
