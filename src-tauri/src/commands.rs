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
    execute_send(
        &state.assistant,
        conversation_id,
        content,
        model,
        &ChannelSink(on_event),
    )
    .await
}

pub(crate) struct DesktopState {
    pub assistant: AssistantService,
    pub platform: PlatformInfo,
    pub endpoint: &'static str,
}
