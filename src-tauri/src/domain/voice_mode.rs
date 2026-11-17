use super::voice::SpeechModelInfo;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceModePhase {
    Off,
    Downloading,
    Starting,
    Waiting,
    Listening,
    Transcribing,
    Paused,
    Stopping,
}
#[derive(Serialize)]
pub struct VoiceModeStatus {
    pub enabled: bool,
    pub phase: VoiceModePhase,
    pub model: SpeechModelInfo,
    pub downloaded_bytes: u64,
    pub error: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VoiceModeEvent {
    InputLevel { level: f32 },
    Phase { phase: VoiceModePhase },
    Transcript { text: String },
    Error { message: String },
    Stopped,
}
pub trait VoiceModeSink: Send + Sync {
    fn emit(&self, event: VoiceModeEvent) -> Result<(), String>;
}
pub trait InputFrames: Send {
    fn next(&self, timeout: Duration) -> Result<Option<Vec<f32>>, String>;
}
pub trait VoiceInput: Send + Sync {
    fn open(&self) -> Result<Box<dyn InputFrames>, String>;
}
pub trait WakeDetector: Send {
    fn feed(&self, samples: &[f32]) -> bool;
    fn reset(&self);
}
pub trait WakeFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn WakeDetector>, String>;
}
pub trait TurnDetector: Send {
    fn feed(&self, samples: &[f32]) -> Option<Vec<f32>>;
    fn reset(&self);
}
pub trait TurnFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn TurnDetector>, String>;
}
