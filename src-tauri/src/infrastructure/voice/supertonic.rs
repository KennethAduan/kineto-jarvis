use super::{kokoro::Kokoro, supertonic_model::SupertonicModel};
use crate::domain::speech::{
    spoken_text, SpeechAudio, SpeechPreferences, SpeechSynthesizer, SpeechVoice,
};
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsModelConfig,
    OfflineTtsSupertonicModelConfig,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub struct Supertonic {
    directory: PathBuf,
    engine: Mutex<Option<OfflineTts>>,
}
impl Supertonic {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            engine: Mutex::new(None),
        }
    }
}
impl SpeechSynthesizer for Supertonic {
    fn synthesize(
        &self,
        text: &str,
        preferences: SpeechPreferences,
        cancel: CancellationToken,
    ) -> Result<SpeechAudio, String> {
        preferences.validate()?;
        if cancel.is_cancelled() {
            return Err("Speech cancelled".into());
        }
        if !preferences.voice.is_supertonic() || text.contains('\0') || text.chars().count() > 240 {
            return Err("Invalid Supertonic speech segment".into());
        }
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| "Speech engine unavailable")?;
        if engine.is_none() {
            SupertonicModel::verify(&self.directory)?;
            let path = |file: &str| Some(self.directory.join(file).to_string_lossy().into_owned());
            let config = OfflineTtsConfig {
                model: OfflineTtsModelConfig {
                    supertonic: OfflineTtsSupertonicModelConfig {
                        duration_predictor: path("duration_predictor.int8.onnx"),
                        text_encoder: path("text_encoder.int8.onnx"),
                        vector_estimator: path("vector_estimator.int8.onnx"),
                        vocoder: path("vocoder.int8.onnx"),
                        tts_json: path("tts.json"),
                        unicode_indexer: path("unicode_indexer.bin"),
                        voice_style: path("voice.bin"),
                    },
                    num_threads: 2,
                    provider: Some("cpu".into()),
                    debug: false,
                    ..Default::default()
                },
                max_num_sentences: 1,
                ..Default::default()
            };
            *engine = Some(
                OfflineTts::create(&config)
                    .ok_or("Could not load Supertonic. Reinstall its voice pack.")?,
            );
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        let token = cancel.clone();
        let text = spoken_text(text);
        // Proper case prevents all-caps assistant names being treated as initialisms.
        let text = text
            .split_whitespace()
            .map(|word| {
                if word
                    .trim_matches(|c: char| !c.is_alphabetic())
                    .eq_ignore_ascii_case("jarvis")
                {
                    word.replace("JARVIS", "Jarvis")
                } else if word
                    .trim_matches(|c: char| !c.is_alphabetic())
                    .eq_ignore_ascii_case("ultron")
                {
                    word.replace("ULTRON", "Ultron")
                } else {
                    word.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut extra = HashMap::new();
        extra.insert("lang".into(), serde_json::json!("en"));
        let generated = engine
            .as_ref()
            .ok_or("Speech engine unavailable")?
            .generate_with_config(
                &text,
                &GenerationConfig {
                    sid: if preferences.voice == SpeechVoice::Supertonic1 {
                        5
                    } else {
                        6
                    },
                    speed: preferences.speed,
                    num_steps: 5,
                    silence_scale: 0.05,
                    extra: Some(extra),
                    ..Default::default()
                },
                Some(move |_: &[f32], _| !token.is_cancelled() && Instant::now() < deadline),
            )
            .ok_or("Supertonic could not synthesize this text")?;
        if cancel.is_cancelled() {
            return Err("Speech cancelled".into());
        }
        let rate = generated.sample_rate();
        let samples = generated.samples();
        if !(8_000..=48_000).contains(&rate)
            || samples.is_empty()
            || samples.len() > rate as usize * 60
            || samples.iter().any(|s| !s.is_finite())
        {
            return Err("Supertonic returned invalid audio".into());
        }
        Ok(SpeechAudio {
            samples: super::loudness::normalize(
                super::loudness::speech_edges(samples, rate as u32),
                preferences.volume,
            ),
            sample_rate: rate as u32,
        })
    }
}
pub struct VoiceEngines {
    pub kokoro: Kokoro,
    pub supertonic: Supertonic,
}
impl SpeechSynthesizer for VoiceEngines {
    fn synthesize(
        &self,
        text: &str,
        preferences: SpeechPreferences,
        cancel: CancellationToken,
    ) -> Result<SpeechAudio, String> {
        if preferences.voice.is_supertonic() {
            self.supertonic.synthesize(text, preferences, cancel)
        } else {
            self.kokoro.synthesize(text, preferences, cancel)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires SUPERTONIC_TEST_DIR; synthesizes public text without microphone or playback"]
    fn synthesizes_names_with_the_new_local_engine() {
        let engine = Supertonic::new(PathBuf::from(
            std::env::var_os("SUPERTONIC_TEST_DIR").unwrap(),
        ));
        let started = Instant::now();
        let audio = engine
            .synthesize(
                "I am Jarvis. Ultron is a fictional character. How can I help you?",
                SpeechPreferences::default(),
                CancellationToken::new(),
            )
            .unwrap();
        assert!(!audio.samples.is_empty());
        assert!(audio
            .samples
            .iter()
            .all(|s| s.is_finite() && s.abs() <= 0.95001));
        eprintln!(
            "Supertonic: {:.2}s synthesis, {:.2}s audio at {} Hz",
            started.elapsed().as_secs_f32(),
            audio.samples.len() as f32 / audio.sample_rate as f32,
            audio.sample_rate
        );
        let path =
            PathBuf::from(std::env::var_os("SUPERTONIC_TEST_DIR").unwrap()).join("preview.wav");
        let mut writer = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: audio.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in audio.samples {
            writer.write_sample((sample * 32767.) as i16).unwrap();
        }
        writer.finalize().unwrap();
    }
}
