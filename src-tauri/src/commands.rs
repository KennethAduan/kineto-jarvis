use crate::{
    application::assistant::{execute_send, AssistantService},
    application::policy,
    domain::models::{Permission, PlatformInfo},
    domain::{
        models::{Activity, Conversation, Message, Model, SendResult, StreamEvent},
        ports::EventSink,
    },
};
use serde::Serialize;
use tauri::{ipc::Channel, State};
mod voice;
pub(crate) use voice::*;
mod speech;
pub(crate) use speech::*;
mod voice_mode;
pub(crate) use voice_mode::*;
#[derive(Serialize)]
pub(crate) struct Bootstrap {
    conversations: Vec<Conversation>,
    activity: Vec<Activity>,
    platform: PlatformInfo,
    permissions: Vec<Permission>,
    endpoint: &'static str,
}
#[tauri::command]
pub(crate) fn bootstrap(state: State<DesktopState>) -> Result<Bootstrap, String> {
    Ok(Bootstrap {
        conversations: state.assistant.conversations()?,
        activity: state.assistant.activity()?,
        platform: state.platform.clone(),
        permissions: policy::permissions(),
        endpoint: state.endpoint,
    })
}
#[tauri::command]
pub(crate) async fn search_conversations(
    state: State<'_, DesktopState>,
    query: String,
) -> Result<Vec<Conversation>, String> {
    state.assistant.search(&query)
}
#[tauri::command]
pub(crate) async fn discover_models(state: State<'_, DesktopState>) -> Result<Vec<Model>, String> {
    state.assistant.models().await
}
#[tauri::command]
pub(crate) fn create_conversation(state: State<DesktopState>) -> Result<Conversation, String> {
    state.assistant.create()
}
#[tauri::command]
pub(crate) fn get_messages(
    state: State<DesktopState>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    state.assistant.messages(&conversation_id)
}
#[tauri::command]
pub(crate) fn get_activity(state: State<DesktopState>) -> Result<Vec<Activity>, String> {
    state.assistant.activity()
}
#[tauri::command]
pub(crate) fn delete_conversation(
    state: State<DesktopState>,
    conversation_id: String,
) -> Result<(), String> {
    state.assistant.delete(&conversation_id)
}
#[tauri::command]
pub(crate) fn cancel_generation(state: State<DesktopState>) -> Result<(), String> {
    state.assistant.cancel()
}
#[tauri::command]
pub(crate) fn profile_get(
    state: State<DesktopState>,
) -> Result<crate::domain::profile::Profile, String> {
    state.assistant.profile()
}
#[tauri::command]
pub(crate) fn profile_save(
    state: State<DesktopState>,
    profile: crate::domain::profile::Profile,
) -> Result<(), String> {
    state.assistant.save_profile(&profile)
}

struct ChannelSink(Channel<StreamEvent>);
impl EventSink for ChannelSink {
    fn send(&self, event: StreamEvent) -> Result<(), String> {
        self.0
            .send(event)
            .map_err(|_| "Chat window disconnected".into())
    }
}
#[tauri::command]
pub(crate) async fn send_message(
    state: State<'_, DesktopState>,
    conversation_id: String,
    content: String,
    model: String,
    on_event: Channel<StreamEvent>,
) -> Result<SendResult, String> {
    let channel = ChannelSink(on_event);
    let speaking = state.voice_mode.reply_token()?.map(|token| {
        crate::application::live_speech::SpeakingSink::new(&channel, state.speech.clone(), token)
    });
    let sink: &dyn EventSink = speaking
        .as_ref()
        .map_or(&channel as &dyn EventSink, |sink| sink);
    let result = execute_send(&state.assistant, conversation_id, content, model, sink).await;
    if let Some(speaking) = speaking {
        speaking.finish(
            result
                .as_ref()
                .is_ok_and(|reply| reply.error.is_none() && reply.message.status == "complete"),
        )?;
    }
    result
}

pub(crate) struct DesktopState {
    pub assistant: std::sync::Arc<AssistantService>,
    pub platform: PlatformInfo,
    pub endpoint: &'static str,
    pub voice: std::sync::Arc<crate::application::voice::VoiceService>,
    pub speech: crate::application::speech::SpeechService,
    pub voice_mode: crate::application::voice_mode::VoiceModeService,
    pub audio_gate: std::sync::Arc<std::sync::Mutex<()>>,
}
