use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Debug)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub updated_at: i64,
}
#[derive(Clone, Serialize, Debug)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: i64,
}
#[derive(Serialize, Debug)]
pub struct Activity {
    pub id: String,
    pub conversation_id: Option<String>,
    pub model: String,
    pub status: String,
    pub detail: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Model {
    pub id: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    Started { message: Message },
    Delta { text: String },
    Thinking,
}
#[derive(Serialize)]
pub struct SendResult {
    pub message: Message,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Frame {
    Content(String),
    Reasoning(String),
    Done,
}

#[derive(Serialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Serialize)]
pub struct PlatformInfo {
    pub name: &'static str,
    pub automation_available: bool,
}

#[derive(Serialize)]
pub struct Permission {
    pub capability: &'static str,
    pub state: &'static str,
    pub description: &'static str,
}
