use super::wake_model::WakeModel;
use crate::domain::voice_mode::*;
use sherpa_onnx::{
    KeywordSpotter, KeywordSpotterConfig, OnlineStream, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};
use std::path::PathBuf;
pub struct NativeWakeFactory(pub PathBuf);
struct Wake {
    detector: KeywordSpotter,
    stream: OnlineStream,
}
impl WakeFactory for NativeWakeFactory {
    fn create(&self) -> Result<Box<dyn WakeDetector>, String> {
        WakeModel::verify(&self.0)?;
        let path = |name: &str| self.0.join(name).to_string_lossy().into_owned();
        let mut config = KeywordSpotterConfig::default();
        config.model_config.transducer.encoder = Some(path("encoder.onnx"));
        config.model_config.transducer.decoder = Some(path("decoder.onnx"));
        config.model_config.transducer.joiner = Some(path("joiner.onnx"));
        config.model_config.tokens = Some(path("tokens.txt"));
        config.model_config.num_threads = 1;
        config.keywords_threshold = 0.25;
        config.keywords_buf = Some("▁HE Y ▁JA R VI S :1.5 #0.25 @Hey_Jarvis".into());
        let detector =
            KeywordSpotter::create(&config).ok_or("Could not load Hey Jarvis detection")?;
        let stream = detector.create_stream();
        Ok(Box::new(Wake { detector, stream }))
    }
}
impl WakeDetector for Wake {
    fn feed(&self, samples: &[f32]) -> bool {
        self.stream.accept_waveform(16_000, samples);
        while self.detector.is_ready(&self.stream) {
            self.detector.decode(&self.stream);
            if self
                .detector
                .get_result(&self.stream)
                .is_some_and(|result| !result.keyword.is_empty())
            {
                self.detector.reset(&self.stream);
                return true;
            }
        }
        false
    }
    fn reset(&self) {
        self.detector.reset(&self.stream);
    }
}
pub struct NativeTurnFactory(pub PathBuf);
struct Turn(VoiceActivityDetector);
impl TurnFactory for NativeTurnFactory {
    fn create(&self) -> Result<Box<dyn TurnDetector>, String> {
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(
                    self.0
                        .join("silero_vad.onnx")
                        .to_string_lossy()
                        .into_owned(),
                ),
                threshold: 0.5,
                min_silence_duration: 0.45,
                min_speech_duration: 0.25,
                window_size: 512,
                max_speech_duration: 25.,
            },
            sample_rate: 16_000,
            num_threads: 1,
            provider: Some("cpu".into()),
            debug: false,
            ..Default::default()
        };
        Ok(Box::new(Turn(
            VoiceActivityDetector::create(&config, 30.).ok_or("Could not load speech detection")?,
        )))
    }
}
impl TurnDetector for Turn {
    fn detected(&self) -> bool {
        self.0.detected()
    }
    fn feed(&self, samples: &[f32]) -> Option<Vec<f32>> {
        self.0.accept_waveform(samples);
        if self.0.is_empty() {
            return None;
        }
        let segment = self.0.front()?;
        let audio = segment.samples().to_vec();
        self.0.pop();
        Some(audio)
    }
    fn reset(&self) {
        self.0.reset();
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        speech::{SpeechPreferences, SpeechSynthesizer},
        voice::{CapturedAudio, SpeechModel, SpeechRecognizer},
    };
    use std::sync::{atomic::AtomicU64, Arc};
    use tokio_util::sync::CancellationToken;
    #[tokio::test]
    #[ignore = "requires WAKE_TEST_DIR and KOKORO_TEST_DIR; downloads the 7 MB listening pack and synthesizes public text; no microphone"]
    async fn detects_synthesized_hey_jarvis_and_speech_endpoint() {
        let directory =
            PathBuf::from(std::env::var_os("WAKE_TEST_DIR").expect("wake test directory"));
        let model = WakeModel(directory.clone());
        if !model.info().ready {
            model
                .download(CancellationToken::new(), Arc::new(AtomicU64::new(0)))
                .await
                .unwrap();
        }
        WakeModel::verify(&directory).unwrap();
        let synth = super::super::kokoro::Kokoro::new(PathBuf::from(
            std::env::var_os("KOKORO_TEST_DIR").expect("Kokoro test directory"),
        ));
        let audio = synth
            .synthesize(
                "Hey Jarvis.",
                SpeechPreferences {
                    voice: crate::domain::speech::SpeechVoice::George,
                    ..Default::default()
                },
                CancellationToken::new(),
            )
            .unwrap();
        let mut samples = crate::application::voice::prepare_audio(CapturedAudio {
            samples: audio.samples,
            sample_rate: audio.sample_rate,
        })
        .unwrap();
        samples.extend(vec![0.; 16_000]);
        let detector = NativeWakeFactory(directory.clone()).create().unwrap();
        let mut detected = false;
        for chunk in samples.chunks(512) {
            detected |= detector.feed(chunk);
        }
        assert!(detected, "Hey Jarvis must trigger the fixed wake phrase");
        detector.reset();
        let audio = synth
            .synthesize(
                "What is two plus two?",
                SpeechPreferences {
                    voice: crate::domain::speech::SpeechVoice::George,
                    ..Default::default()
                },
                CancellationToken::new(),
            )
            .unwrap();
        let mut samples = crate::application::voice::prepare_audio(CapturedAudio {
            samples: audio.samples,
            sample_rate: audio.sample_rate,
        })
        .unwrap();
        samples.extend(vec![0.; 32_000]);
        let vad = NativeTurnFactory(directory).create().unwrap();
        let mut endpoint = false;
        for chunk in samples.chunks(512) {
            assert!(
                !detector.feed(chunk),
                "ordinary question must not trigger wake phrase"
            );
            if let Some(turn) = vad.feed(chunk) {
                assert!(turn.len() > 8_000);
                if let Some(path) = std::env::var_os("WHISPER_TEST_MODEL") {
                    let recognizer = super::super::recognition::LocalWhisper::new(path.into());
                    let text = recognizer
                        .transcribe(turn, CancellationToken::new())
                        .unwrap()
                        .to_lowercase();
                    assert!(
                        text.contains("plus") || text.contains('+'),
                        "public question must remain intelligible: {text}"
                    );
                }
                endpoint = true;
            }
        }
        assert!(endpoint, "silence must end the spoken question");
    }
}
