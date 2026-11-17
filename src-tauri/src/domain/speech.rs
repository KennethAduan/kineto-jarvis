use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechVoice {
    #[serde(rename = "bm_george")]
    George,
    #[serde(rename = "bm_fable")]
    Fable,
    #[serde(rename = "bm_daniel")]
    Daniel,
    #[serde(rename = "bm_lewis")]
    Lewis,
    #[serde(rename = "system")]
    System,
}
impl SpeechVoice {
    pub fn speaker_id(self) -> Option<i32> {
        match self {
            Self::George => Some(26),
            Self::Fable => Some(25),
            Self::Daniel => Some(24),
            Self::Lewis => Some(27),
            Self::System => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPreferences {
    pub voice: SpeechVoice,
    pub speed: f32,
}
impl Default for SpeechPreferences {
    fn default() -> Self {
        Self {
            voice: SpeechVoice::George,
            speed: 0.95,
        }
    }
}
impl SpeechPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.speed.is_finite() || !(0.75..=1.25).contains(&self.speed) {
            return Err("Choose a speaking speed between 0.75 and 1.25.".into());
        }
        Ok(())
    }
}
pub struct SpeechAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
pub trait SpeechSynthesizer: Send + Sync {
    fn synthesize(
        &self,
        text: &str,
        preferences: SpeechPreferences,
        cancel: CancellationToken,
    ) -> Result<SpeechAudio, String>;
}
pub trait AudioPlayback: Send + Sync {
    /// Blocks until the buffer is played or cancelled. Never saves audio to disk.
    fn play(&self, audio: SpeechAudio, cancel: CancellationToken) -> Result<(), String>;
}
pub trait SpeechPreferencesStore: Send + Sync {
    fn load(&self) -> Result<SpeechPreferences, String>;
    fn save(&self, preferences: SpeechPreferences) -> Result<(), String>;
}
