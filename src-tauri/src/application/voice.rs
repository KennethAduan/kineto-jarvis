use crate::domain::voice::*;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio_util::sync::CancellationToken;

struct Task {
    stage: VoiceStage,
    cancel: CancellationToken,
}
pub struct VoiceService {
    recorder: Box<dyn AudioRecorder>,
    recognizer: Box<dyn SpeechRecognizer>,
    model: Box<dyn SpeechModel>,
    speaker: Box<dyn SpeechOutput>,
    task: Mutex<Task>,
    progress: Arc<AtomicU64>,
}
struct VoiceJob<'a>(&'a Mutex<Task>);
impl Drop for VoiceJob<'_> {
    fn drop(&mut self) {
        if let Ok(mut task) = self.0.lock() {
            task.stage = VoiceStage::Idle;
        }
    }
}
impl VoiceService {
    pub fn new(
        recorder: Box<dyn AudioRecorder>,
        recognizer: Box<dyn SpeechRecognizer>,
        model: Box<dyn SpeechModel>,
        speaker: Box<dyn SpeechOutput>,
    ) -> Self {
        Self {
            recorder,
            recognizer,
            model,
            speaker,
            task: Mutex::new(Task {
                stage: VoiceStage::Idle,
                cancel: CancellationToken::new(),
            }),
            progress: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn status(&self) -> Result<VoiceStatus, String> {
        Ok(VoiceStatus {
            stage: self
                .task
                .lock()
                .map_err(|_| "Voice state unavailable")?
                .stage,
            model: self.model.info(),
            downloaded_bytes: self.progress.load(Ordering::Relaxed),
            recording: self.recorder.info(),
            speaking: self.speaker.is_speaking(),
            playback_error: self.speaker.error(),
        })
    }
    pub fn start(&self) -> Result<(), String> {
        let mut task = self.task.lock().map_err(|_| "Voice state unavailable")?;
        if task.stage != VoiceStage::Idle {
            return Err("Finish or cancel the current voice action first.".into());
        }
        if !self.model.info().ready {
            return Err("Set up offline voice before recording.".into());
        }
        self.speaker.stop()?;
        self.recorder.start()?;
        task.stage = VoiceStage::Recording;
        task.cancel = CancellationToken::new();
        Ok(())
    }
    pub fn finish(&self) -> Result<String, String> {
        let cancel = {
            let mut task = self.task.lock().map_err(|_| "Voice state unavailable")?;
            if task.stage != VoiceStage::Recording {
                return Err("No recording is active".into());
            }
            task.stage = VoiceStage::Transcribing;
            task.cancel.clone()
        };
        let _job = VoiceJob(&self.task);
        let audio = self.recorder.finish()?;
        let samples = prepare_audio(audio)?;
        if cancel.is_cancelled() {
            return Err("Transcription cancelled".into());
        }
        self.recognizer.transcribe(samples, cancel)
    }
    pub async fn download(&self) -> Result<(), String> {
        let cancel = {
            let mut task = self.task.lock().map_err(|_| "Voice state unavailable")?;
            if task.stage != VoiceStage::Idle {
                return Err("Finish or cancel the current voice action first.".into());
            }
            task.stage = VoiceStage::Downloading;
            task.cancel = CancellationToken::new();
            task.cancel.clone()
        };
        let _job = VoiceJob(&self.task);
        self.progress.store(0, Ordering::Relaxed);
        self.model.download(cancel, self.progress.clone()).await
    }
    pub fn cancel(&self) -> Result<(), String> {
        let mut task = self.task.lock().map_err(|_| "Voice state unavailable")?;
        task.cancel.cancel();
        if task.stage == VoiceStage::Recording {
            self.recorder.cancel()?;
            task.stage = VoiceStage::Idle;
        }
        Ok(())
    }
    pub fn speak(&self, markdown: &str) -> Result<(), String> {
        let task = self.task.lock().map_err(|_| "Voice state unavailable")?;
        if task.stage != VoiceStage::Idle {
            return Err("Finish or cancel dictation before playing a reply.".into());
        }
        let text = prepare_speech(markdown);
        if text.trim().is_empty() {
            return Err("This reply has no speakable text.".into());
        }
        self.speaker.speak(&text)
    }
    pub fn stop_speaking(&self) -> Result<(), String> {
        self.speaker.stop()
    }
}

pub fn prepare_audio(audio: CapturedAudio) -> Result<Vec<f32>, String> {
    if !(8_000..=192_000).contains(&audio.sample_rate)
        || audio.samples.len() > audio.sample_rate as usize * MAX_RECORDING_SECONDS as usize
    {
        return Err("Recording exceeded the supported audio limits.".into());
    }
    if audio.samples.len() < audio.sample_rate as usize / 2 {
        return Err("Record for at least half a second before transcribing.".into());
    }
    if audio.samples.iter().any(|s| !s.is_finite()) {
        return Err("The microphone returned invalid audio.".into());
    }
    let energy = audio.samples.iter().map(|x| x * x).sum::<f32>() / audio.samples.len() as f32;
    if energy < 0.000004 {
        return Err(
            "The recording is silent or too quiet. Try again closer to the microphone.".into(),
        );
    }
    let length = audio.samples.len() * SPEECH_SAMPLE_RATE as usize / audio.sample_rate as usize;
    let ratio = audio.sample_rate as f64 / SPEECH_SAMPLE_RATE as f64;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        let position = i as f64 * ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(audio.samples.len() - 1);
        let fraction = (position - left as f64) as f32;
        result.push(
            (audio.samples[left] * (1. - fraction) + audio.samples[right] * fraction)
                .clamp(-1., 1.),
        );
    }
    Ok(result)
}

pub fn prepare_speech(markdown: &str) -> String {
    let mut result = String::new();
    let mut code_block = false;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block = true,
            Event::End(TagEnd::CodeBlock) => {
                code_block = false;
                result.push_str(" Code block omitted. ");
            }
            Event::Text(text) | Event::Code(text) if !code_block => result.push_str(&text),
            Event::SoftBreak | Event::HardBreak => result.push(' '),
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item) => {
                result.push_str(". ")
            }
            _ => {}
        }
    }
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(8_000)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    struct Recorder(AtomicBool);
    impl AudioRecorder for Recorder {
        fn start(&self) -> Result<(), String> {
            self.0.store(true, Ordering::Relaxed);
            Ok(())
        }
        fn finish(&self) -> Result<CapturedAudio, String> {
            self.0.store(false, Ordering::Relaxed);
            Ok(CapturedAudio {
                samples: vec![0.1; 16_000],
                sample_rate: 16_000,
            })
        }
        fn cancel(&self) -> Result<(), String> {
            self.0.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn info(&self) -> RecordingInfo {
            RecordingInfo {
                active: self.0.load(Ordering::Relaxed),
                seconds: 1.,
            }
        }
    }
    struct Recognizer(bool);
    impl SpeechRecognizer for Recognizer {
        fn transcribe(&self, samples: Vec<f32>, _: CancellationToken) -> Result<String, String> {
            assert_eq!(samples.len(), 16_000);
            if self.0 {
                Ok("A draft to review".into())
            } else {
                Err("Recognition failed".into())
            }
        }
    }
    struct Model(bool);
    impl SpeechModel for Model {
        fn info(&self) -> SpeechModelInfo {
            SpeechModelInfo {
                ready: self.0,
                bytes: 100,
            }
        }
        fn download(
            &self,
            cancel: CancellationToken,
            _: Arc<AtomicU64>,
        ) -> BoxFuture<'_, Result<(), String>> {
            Box::pin(async move {
                cancel.cancelled().await;
                Err("cancelled".into())
            })
        }
    }
    struct Speaker;
    impl SpeechOutput for Speaker {
        fn speak(&self, text: &str) -> Result<(), String> {
            assert!(!text.is_empty());
            Ok(())
        }
        fn stop(&self) -> Result<(), String> {
            Ok(())
        }
        fn is_speaking(&self) -> bool {
            false
        }
    }
    fn service(ready: bool, recognize: bool) -> VoiceService {
        VoiceService::new(
            Box::<Recorder>::default(),
            Box::new(Recognizer(recognize)),
            Box::new(Model(ready)),
            Box::new(Speaker),
        )
    }
    #[test]
    fn voice_requires_setup_and_rejects_overlapping_actions() {
        assert!(service(false, true).start().is_err());
        let voice = service(true, true);
        voice.start().unwrap();
        assert!(voice.status().unwrap().recording.active);
        assert!(voice.start().is_err());
        assert!(voice.speak("A reply").is_err());
        assert_eq!(voice.finish().unwrap(), "A draft to review");
        assert!(voice.status().unwrap().stage == VoiceStage::Idle);
        assert!(!voice.status().unwrap().recording.active);
        assert!(voice.finish().is_err());
    }
    #[test]
    fn cancellation_closes_capture_and_errors_restore_idle() {
        let voice = service(true, false);
        voice.start().unwrap();
        voice.cancel().unwrap();
        assert!(!voice.status().unwrap().recording.active);
        assert!(voice.finish().is_err());
        voice.start().unwrap();
        assert!(voice.finish().is_err());
        assert!(voice.status().unwrap().stage == VoiceStage::Idle);
        voice.start().unwrap();
        voice.cancel().unwrap();
    }
    #[tokio::test]
    async fn cancelling_download_resets_state_and_blocks_recording_until_settled() {
        let voice = service(true, true);
        let job = voice.download();
        tokio::pin!(job);
        tokio::select! {
            result = &mut job => panic!("download unexpectedly finished: {result:?}"),
            _ = tokio::task::yield_now() => {}
        }
        assert!(voice.status().unwrap().stage == VoiceStage::Downloading);
        assert!(voice.start().is_err());
        voice.cancel().unwrap();
        assert!(job.await.is_err());
        assert!(voice.status().unwrap().stage == VoiceStage::Idle);
    }
    #[test]
    fn audio_bounds_silence_and_resampling() {
        let audio = |samples, sample_rate| CapturedAudio {
            samples,
            sample_rate,
        };
        assert!(prepare_audio(audio(vec![0.; 16_000], 16_000)).is_err());
        assert!(prepare_audio(audio(vec![0.1; 100], 16_000)).is_err());
        assert!(prepare_audio(audio(vec![f32::NAN; 16_000], 16_000)).is_err());
        assert!(prepare_audio(audio(vec![0.1; 16_000 * 31], 16_000)).is_err());
        assert!(prepare_audio(audio(vec![0.1; 16_000], 0)).is_err());
        let normalized = prepare_audio(audio(vec![2.; 48_000], 48_000)).unwrap();
        assert_eq!(normalized.len(), 16_000);
        assert!(normalized.iter().all(|sample| *sample == 1.));
    }
    #[test]
    fn speech_uses_readable_text_without_code_or_link_targets() {
        let text = prepare_speech("# Hello\n\nRead **this** [label](https://private.invalid).\n\n```sh\nsecret_command\n```\n");
        assert!(text.contains("Read this label"));
        assert!(text.contains("Code block omitted"));
        assert!(!text.contains("secret_command"));
        assert!(!text.contains("https://"));
        assert_eq!(prepare_speech(&"é".repeat(10_000)).chars().count(), 8_000);
    }
}
