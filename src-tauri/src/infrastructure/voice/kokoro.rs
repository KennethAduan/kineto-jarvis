use super::kokoro_model::KokoroModel;
use super::phonemes::BritishPhonemizer;
use crate::domain::speech::{SpeechAudio, SpeechPreferences, SpeechSynthesizer};
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig,
    OfflineTtsModelConfig,
};
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
// Keep the Windows native staging dependency tied to the same public FFI layout.
#[cfg(target_os = "windows")]
const _: usize = std::mem::size_of::<sherpa_onnx_sys::tts::OfflineTtsConfig>();

pub struct Kokoro {
    directory: PathBuf,
    engine: Mutex<Option<Engine>>,
}
struct Engine {
    tts: OfflineTts,
    phonemizer: BritishPhonemizer,
}
impl Kokoro {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            engine: Mutex::new(None),
        }
    }
}
impl SpeechSynthesizer for Kokoro {
    fn synthesize(
        &self,
        text: &str,
        preferences: SpeechPreferences,
        cancel: CancellationToken,
    ) -> Result<SpeechAudio, String> {
        preferences.validate()?;
        let sid = preferences
            .voice
            .speaker_id()
            .ok_or("Choose a Kokoro voice")?;
        if cancel.is_cancelled() {
            return Err("Speech cancelled".into());
        }
        // Bound each native inference call and reject embedded NUL before the C API.
        if text.contains('\0') || text.chars().count() > 240 {
            return Err("Invalid speech segment".into());
        }
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| "Speech engine unavailable")?;
        if engine.is_none() {
            KokoroModel::verify(&self.directory)?;
            let path = |file: &str| self.directory.join(file).to_string_lossy().into_owned();
            let config = OfflineTtsConfig {
                model: OfflineTtsModelConfig {
                    kokoro: OfflineTtsKokoroModelConfig {
                        model: Some(path("model.int8.onnx")),
                        voices: Some(path("voices.bin")),
                        tokens: Some(path("tokens.txt")),
                        lexicon: Some(path("lexicon-gb-en.txt")),
                        ..Default::default()
                    },
                    provider: Some("cpu".into()),
                    num_threads: 2,
                    debug: false,
                    ..Default::default()
                },
                max_num_sentences: 1,
                ..Default::default()
            };
            *engine = Some(Engine {
                tts: OfflineTts::create(&config)
                    .ok_or("Could not load Kokoro. Reinstall its voice pack in Voice settings.")?,
                phonemizer: BritishPhonemizer::load(&self.directory.join("lexicon-gb-en.txt"))?,
            });
        }
        let engine = engine.as_ref().ok_or("Speech engine unavailable")?;
        let deadline = Instant::now() + Duration::from_secs(60);
        let token = cancel.clone();
        let phonemes = engine.phonemizer.phonemes(text)?;
        if cancel.is_cancelled() {
            return Err("Speech cancelled".into());
        }
        let generated = engine
            .tts
            .generate_with_config(
                "",
                &GenerationConfig {
                    sid,
                    speed: preferences.speed,
                    phoneme_codepoints: Some(vec![phonemes]),
                    ..Default::default()
                },
                Some(move |_: &[f32], _| !token.is_cancelled() && Instant::now() < deadline),
            )
            .ok_or("Kokoro could not synthesize this text")?;
        if cancel.is_cancelled() {
            return Err("Speech cancelled".into());
        }
        let samples = generated.samples();
        if generated.sample_rate() != 24_000
            || samples.is_empty()
            || samples.len() > 24_000 * 60
            || samples.iter().any(|s| !s.is_finite())
        {
            return Err("Kokoro returned invalid or oversized audio".into());
        }
        Ok(SpeechAudio {
            samples: super::loudness::normalize(samples, preferences.volume),
            sample_rate: 24_000,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{speech::SpeechVoice, voice::SpeechModel};
    use std::sync::{atomic::AtomicU64, Arc};
    #[tokio::test]
    #[ignore = "downloads a 149 MB public voice pack and saves generated public-text WAVs to KOKORO_TEST_DIR; no playback"]
    async fn kokoro_download_and_synthesize_british_voices() {
        let directory = PathBuf::from(std::env::var_os("KOKORO_TEST_DIR").expect("test directory"));
        let model = KokoroModel(directory.clone());
        if !model.info().ready {
            model
                .download(CancellationToken::new(), Arc::new(AtomicU64::new(0)))
                .await
                .unwrap();
        }
        KokoroModel::verify(&directory).unwrap();
        let engine = Kokoro::new(directory.clone());
        let mut lengths = Vec::new();
        for (name, voice, speed) in [
            ("george", SpeechVoice::George, 0.95),
            ("fable", SpeechVoice::Fable, 0.95),
            ("george-fast", SpeechVoice::George, 1.25),
        ] {
            let audio = engine
                .synthesize(
                    "Hey there! How can I assist you today, Ito? 😊",
                    SpeechPreferences {
                        voice,
                        speed,
                        ..Default::default()
                    },
                    CancellationToken::new(),
                )
                .unwrap();
            assert!(audio.samples.len() > 24_000);
            assert!(audio.samples.iter().any(|sample| sample.abs() > 0.01));
            assert!(audio
                .samples
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 0.95001));
            lengths.push(audio.samples.len());
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: audio.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer =
                hound::WavWriter::create(directory.join(format!("{name}.wav")), spec).unwrap();
            for sample in audio.samples {
                writer
                    .write_sample((sample.clamp(-1., 1.) * 32767.) as i16)
                    .unwrap();
            }
            writer.finalize().unwrap();
        }
        assert!(lengths[2] < lengths[0], "speed must change audio duration");
    }
}
