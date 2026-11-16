use super::DesktopState;
use crate::domain::voice::VoiceStatus;
use tauri::State;

#[tauri::command]
pub(crate) fn voice_status(state: State<DesktopState>) -> Result<VoiceStatus, String> {
    state.voice.status()
}
#[tauri::command]
pub(crate) async fn voice_download_model(state: State<'_, DesktopState>) -> Result<(), String> {
    state.voice.download().await
}
#[tauri::command]
pub(crate) async fn voice_start_recording(state: State<'_, DesktopState>) -> Result<(), String> {
    if state.assistant.is_generating()? {
        return Err("Wait for the current reply or stop it before dictating.".into());
    }
    let voice = state.voice.clone();
    tauri::async_runtime::spawn_blocking(move || voice.start())
        .await
        .map_err(|_| "Could not start microphone worker")?
}
#[tauri::command]
pub(crate) async fn voice_finish_recording(
    state: State<'_, DesktopState>,
) -> Result<String, String> {
    let voice = state.voice.clone();
    tauri::async_runtime::spawn_blocking(move || voice.finish())
        .await
        .map_err(|_| "Speech recognition worker stopped unexpectedly")?
}
#[tauri::command]
pub(crate) fn voice_cancel(state: State<DesktopState>) -> Result<(), String> {
    state.voice.cancel()
}
#[tauri::command]
pub(crate) async fn voice_speak_message(
    state: State<'_, DesktopState>,
    conversation_id: String,
    message_id: String,
) -> Result<(), String> {
    let message = state
        .assistant
        .messages(&conversation_id)?
        .into_iter()
        .find(|m| m.id == message_id && m.role == "assistant" && m.status == "complete")
        .ok_or("Choose a completed assistant reply to read aloud.")?;
    let voice = state.voice.clone();
    tauri::async_runtime::spawn_blocking(move || voice.speak(&message.content))
        .await
        .map_err(|_| "Speech output worker stopped unexpectedly")?
}
#[tauri::command]
pub(crate) fn voice_stop_speaking(state: State<DesktopState>) -> Result<(), String> {
    state.voice.stop_speaking()
}
