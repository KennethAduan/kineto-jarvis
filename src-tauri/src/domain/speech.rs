use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechVoice {
    #[serde(rename = "supertonic_m2")]
    Supertonic2,
    #[serde(rename = "supertonic_m1")]
    Supertonic1,
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
            Self::Supertonic1 | Self::Supertonic2 => None,
        }
    }
    pub fn is_supertonic(self) -> bool {
        matches!(self, Self::Supertonic1 | Self::Supertonic2)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPreferences {
    pub voice: SpeechVoice,
    pub speed: f32,
    #[serde(default = "default_volume")]
    pub volume: f32,
}
fn default_volume() -> f32 {
    1.0
}
impl Default for SpeechPreferences {
    fn default() -> Self {
        Self {
            voice: SpeechVoice::Supertonic2,
            speed: 1.15,
            volume: default_volume(),
        }
    }
}
impl SpeechPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.volume.is_finite() || !(0.0..=1.5).contains(&self.volume) {
            return Err("Choose an output volume between 0 and 150%.".into());
        }
        if !self.speed.is_finite() || !(0.75..=1.5).contains(&self.speed) {
            return Err("Choose a speaking speed between 0.75 and 1.50.".into());
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

// Decorative emoji are not words. Keep unsupported letters intact so language errors stay visible.
pub fn spoken_text(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{1f000}'..='\u{1faff}'
            | '\u{2600}'..='\u{27bf}'
            | '\u{fe0e}'
            | '\u{fe0f}'
            | '\u{200d}'
            | '\u{20e3}' => ' ',
            '‘' | '’' => '\'',
            '“' | '”' => '"',
            '—' | '–' => ',',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversational_speed_defaults_faster_and_accepts_the_supported_range() {
        assert_eq!(SpeechPreferences::default().speed, 1.15);
        for speed in [0.75, 1.15, 1.5] {
            assert!(SpeechPreferences {
                speed,
                ..Default::default()
            }
            .validate()
            .is_ok());
        }
        for speed in [0.74, 1.51, f32::NAN, f32::INFINITY] {
            assert!(SpeechPreferences {
                speed,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }
    #[test]
    fn existing_preferences_gain_default_volume_and_invalid_gain_is_rejected() {
        let prefs: SpeechPreferences =
            serde_json::from_str(r#"{"voice":"bm_fable","speed":1.1}"#).unwrap();
        assert_eq!(prefs.volume, 1.);
        assert_eq!(prefs.voice, SpeechVoice::Fable);
        for volume in [-1., 1.51, f32::NAN, f32::INFINITY] {
            assert!(SpeechPreferences { volume, ..prefs }.validate().is_err());
        }
    }
}
