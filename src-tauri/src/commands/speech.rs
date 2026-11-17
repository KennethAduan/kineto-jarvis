use super::DesktopState;
use crate::{application::speech::SpeechStatus, domain::speech::SpeechPreferences};
use tauri::State;

#[tauri::command]
pub(crate) fn speech_status(state: State<DesktopState>) -> Result<SpeechStatus, String> {
    state.speech.status()
}
#[tauri::command]
pub(crate) async fn speech_save_preferences(
    state: State<'_, DesktopState>,
    preferences: SpeechPreferences,
) -> Result<(), String> {
    let speech = state.speech.clone();
    tauri::async_runtime::spawn_blocking(move || speech.save(preferences))
        .await
        .map_err(|_| "Could not save voice preferences")?
}
#[tauri::command]
pub(crate) fn speech_preview(
    state: State<DesktopState>,
    preferences: SpeechPreferences,
) -> Result<(), String> {
    if state.voice.status()?.recording.active {
        return Err("Finish dictation before previewing a voice.".into());
    }
    state.speech.preview(preferences)
}
#[tauri::command]
pub(crate) async fn speech_download_model(state: State<'_, DesktopState>) -> Result<(), String> {
    state.speech.download().await
}
