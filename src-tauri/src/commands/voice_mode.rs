use super::DesktopState;
use crate::domain::{voice::SpeechOutput, voice::VoiceStage, voice_mode::*};
use std::sync::Arc;
use tauri::{ipc::Channel, State};
struct Sink(Channel<VoiceModeEvent>);
impl VoiceModeSink for Sink {
    fn emit(&self, event: VoiceModeEvent) -> Result<(), String> {
        self.0
            .send(event)
            .map_err(|_| "Voice window disconnected".into())
    }
}
#[tauri::command]
pub(crate) fn voice_mode_status(state: State<DesktopState>) -> Result<VoiceModeStatus, String> {
    state.voice_mode.status()
}
#[tauri::command]
pub(crate) async fn voice_mode_download(state: State<'_, DesktopState>) -> Result<(), String> {
    state.voice_mode.download().await
}
#[tauri::command]
pub(crate) fn voice_mode_start(
    state: State<DesktopState>,
    on_event: Channel<VoiceModeEvent>,
) -> Result<String, String> {
    let _gate = state
        .audio_gate
        .lock()
        .map_err(|_| "Microphone state unavailable")?;
    let dictation = state.voice.status()?;
    if dictation.stage != VoiceStage::Idle
        || state.assistant.is_generating()?
        || state.speech.is_speaking()
    {
        return Err("Finish the current chat or voice action before starting voice mode.".into());
    }
    if !dictation.model.ready {
        return Err("Download the Whisper dictation model in Voice settings first.".into());
    }
    let speech = state.speech.status()?;
    if speech.preferences.voice != crate::domain::speech::SpeechVoice::System && !speech.model.ready
    {
        return Err("Download the Kokoro voice pack in Voice settings first.".into());
    }
    state.voice_mode.start(Arc::new(Sink(on_event)))
}
#[tauri::command]
pub(crate) fn voice_mode_heartbeat(
    state: State<DesktopState>,
    session_id: String,
) -> Result<(), String> {
    state.voice_mode.heartbeat(&session_id)
}
#[tauri::command]
pub(crate) fn voice_mode_resume(
    state: State<DesktopState>,
    session_id: String,
) -> Result<(), String> {
    if state.assistant.is_generating()? || state.speech.is_speaking() {
        return Err("Wait for the reply and speech playback to finish before listening.".into());
    }
    state.voice_mode.resume(&session_id)
}
#[tauri::command]
pub(crate) fn voice_mode_stop(state: State<DesktopState>) -> Result<(), String> {
    let active = state.voice_mode.status()?.enabled;
    state.voice_mode.stop()?;
    if active {
        state.assistant.cancel()?;
        state.speech.stop()?;
    }
    Ok(())
}
