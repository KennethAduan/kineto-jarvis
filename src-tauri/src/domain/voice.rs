use futures_util::future::BoxFuture;
use serde::Serialize;
use std::sync::{atomic::AtomicU64, Arc};
use tokio_util::sync::CancellationToken;

pub const MAX_RECORDING_SECONDS: u32 = 30;
pub const SPEECH_SAMPLE_RATE: u32 = 16_000;

pub struct CapturedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
#[derive(Clone, Copy, Serialize)]
pub struct RecordingInfo {
    pub active: bool,
    pub seconds: f32,
}
#[derive(Clone, Copy, Serialize)]
pub struct SpeechModelInfo {
    pub ready: bool,
    pub bytes: u64,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceStage {
    Idle,
    Recording,
    Transcribing,
    Downloading,
}
#[derive(Serialize)]
pub struct VoiceStatus {
    pub stage: VoiceStage,
    pub model: SpeechModelInfo,
    pub downloaded_bytes: u64,
    pub recording: RecordingInfo,
    pub speaking: bool,
}

pub trait AudioRecorder: Send + Sync {
    fn start(&self) -> Result<(), String>;
    fn finish(&self) -> Result<CapturedAudio, String>;
    fn cancel(&self) -> Result<(), String>;
    fn info(&self) -> RecordingInfo;
}
pub trait SpeechRecognizer: Send + Sync {
    fn transcribe(&self, samples: Vec<f32>, cancel: CancellationToken) -> Result<String, String>;
}
pub trait SpeechModel: Send + Sync {
    fn info(&self) -> SpeechModelInfo;
    fn download(
        &self,
        cancel: CancellationToken,
        progress: Arc<AtomicU64>,
    ) -> BoxFuture<'_, Result<(), String>>;
}
pub trait SpeechOutput: Send + Sync {
    fn speak(&self, text: &str) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
    fn is_speaking(&self) -> bool;
}
