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
            let recognizer = std::sync::Arc::new(
                infrastructure::voice::recognition::LocalWhisper::new(speech_model.clone()),
            );
            let wake_directory = dir.join("voice").join("hey-jarvis-v1");
            let voice_mode = application::voice_mode::VoiceModeService::new(
                Box::new(infrastructure::voice::voice_input::NativeVoiceInput),
                Box::new(infrastructure::voice::wake::NativeWakeFactory(
                    wake_directory.clone(),
                )),
                Box::new(infrastructure::voice::wake::NativeTurnFactory(
                    wake_directory.clone(),
                )),
                recognizer.clone(),
                Box::new(infrastructure::voice::wake_model::WakeModel(wake_directory)),
            );
            let kokoro = dir.join("voice").join("kokoro-int8-v1");
            let speech = application::speech::SpeechService::new(
                Box::new(infrastructure::voice::kokoro::Kokoro::new(kokoro.clone())),
                Box::new(infrastructure::voice::playback::SystemAudioPlayback),
                Box::new(
                    infrastructure::voice::speech_preferences::JsonSpeechPreferences(
                        dir.join("voice").join("preferences.json"),
                    ),
                ),
                Box::new(infrastructure::voice::kokoro_model::KokoroModel(kokoro)),
                Box::<infrastructure::voice::speech::SystemSpeech>::default(),
            )
            .map_err(std::io::Error::other)?;
            let voice = std::sync::Arc::new(application::voice::VoiceService::new(
                Box::<infrastructure::voice::microphone::Microphone>::default(),
                Box::new(recognizer),
                Box::new(infrastructure::voice::model::WhisperModel::new(
                    speech_model,
                )),
                Box::new(speech.clone()),
            ));
            app.manage(DesktopState {
                assistant: AssistantService::new(
                    Box::new(store),
                    Box::new(LmStudioGateway::local()?),
                ),
                platform: platform::info(),
                endpoint: lm_studio::ENDPOINT,
                voice,
                speech,
                voice_mode,
                audio_gate: std::sync::Arc::new(std::sync::Mutex::new(())),
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
            speech_status,
            speech_save_preferences,
            speech_preview,
            speech_download_model,
            voice_mode_status,
            voice_mode_download,
            voice_mode_start,
            voice_mode_heartbeat,
            voice_mode_resume,
            voice_mode_stop,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to start Kineto Jarvis");
}
