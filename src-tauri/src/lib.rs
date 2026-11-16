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
            let speech_model = dir.join("voice").join("ggml-base.bin");
            let voice = std::sync::Arc::new(application::voice::VoiceService::new(
                Box::<infrastructure::voice::microphone::Microphone>::default(),
                Box::new(infrastructure::voice::recognition::LocalWhisper::new(
                    speech_model.clone(),
                )),
                Box::new(infrastructure::voice::model::WhisperModel::new(
                    speech_model,
                )),
                Box::<infrastructure::voice::speech::SystemSpeech>::default(),
            ));
            app.manage(DesktopState {
                assistant: AssistantService::new(
                    Box::new(store),
                    Box::new(LmStudioGateway::local()?),
                ),
                platform: platform::info(),
                endpoint: lm_studio::ENDPOINT,
                voice,
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
            search_conversations,
            voice_status,
            voice_download_model,
            voice_start_recording,
            voice_finish_recording,
            voice_cancel,
            voice_speak_message,
            voice_stop_speaking,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to start Kineto Jarvis");
}
