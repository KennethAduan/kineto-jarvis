use super::models::{Activity, Conversation, Frame, Message, Model, ModelMessage, StreamEvent};
use futures_util::{future::BoxFuture, stream::BoxStream};

/// Ports contain no Tauri, SQLite, reqwest, filesystem or platform dependencies.
pub trait ConversationRepository: super::profile::ProfileRepository + Send {
    fn conversations(&self) -> Result<Vec<Conversation>, String>;
    fn search(&self, query: &str) -> Result<Vec<Conversation>, String>;
    fn create(&self) -> Result<Conversation, String>;
    fn messages(&self, id: &str) -> Result<Vec<Message>, String>;
    fn begin(
        &mut self,
        conversation: &str,
        content: &str,
        model: &str,
        task: &str,
    ) -> Result<String, String>;
    fn checkpoint(&self, id: &str, content: &str) -> Result<(), String>;
    fn finish(
        &mut self,
        id: &str,
        task: &str,
        content: &str,
        status: &str,
        detail: &str,
    ) -> Result<Message, String>;
    fn delete(&self, id: &str) -> Result<(), String>;
    fn activity(&self) -> Result<Vec<Activity>, String>;
}

pub type ModelStream = BoxStream<'static, Result<Frame, String>>;
pub trait ModelGateway: Send + Sync {
    fn models(&self) -> BoxFuture<'_, Result<Vec<Model>, String>>;
    fn stream(
        &self,
        model: String,
        messages: Vec<ModelMessage>,
    ) -> BoxFuture<'_, Result<ModelStream, String>>;
}

pub trait EventSink: Send + Sync {
    fn send(&self, event: StreamEvent) -> Result<(), String>;
}

pub trait NativePlatform {
    fn name(&self) -> &'static str;
    fn automation_available(&self) -> bool {
        false
    }
}
