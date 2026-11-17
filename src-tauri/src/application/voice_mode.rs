use crate::domain::{
    voice::{SpeechModel, SpeechRecognizer},
    voice_mode::*,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

struct Task {
    id: String,
    enabled: bool,
    phase: VoiceModePhase,
    cancel: CancellationToken,
    heartbeat: Instant,
    resume: bool,
    error: Option<String>,
}
struct Inner {
    input: Box<dyn VoiceInput>,
    wake: Box<dyn WakeFactory>,
    turn: Box<dyn TurnFactory>,
    recognizer: Arc<dyn SpeechRecognizer>,
    model: Box<dyn SpeechModel>,
    task: Mutex<Task>,
    progress: Arc<AtomicU64>,
}
#[derive(Clone)]
pub struct VoiceModeService(Arc<Inner>);
struct Job(Arc<Inner>);
impl Drop for Job {
    fn drop(&mut self) {
        if let Ok(mut task) = self.0.task.lock() {
            task.cancel.cancel();
            task.enabled = false;
            task.phase = VoiceModePhase::Off;
        }
    }
}
impl VoiceModeService {
    pub fn new(
        input: Box<dyn VoiceInput>,
        wake: Box<dyn WakeFactory>,
        turn: Box<dyn TurnFactory>,
        recognizer: Arc<dyn SpeechRecognizer>,
        model: Box<dyn SpeechModel>,
    ) -> Self {
        Self(Arc::new(Inner {
            input,
            wake,
            turn,
            recognizer,
            model,
            task: Mutex::new(Task {
                id: String::new(),
                enabled: false,
                phase: VoiceModePhase::Off,
                cancel: CancellationToken::new(),
                heartbeat: Instant::now(),
                resume: false,
                error: None,
            }),
            progress: Arc::new(AtomicU64::new(0)),
        }))
    }
    pub fn status(&self) -> Result<VoiceModeStatus, String> {
        let task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        Ok(VoiceModeStatus {
            enabled: task.enabled,
            phase: task.phase,
            model: self.0.model.info(),
            downloaded_bytes: self.0.progress.load(Ordering::Relaxed),
            error: task.error.clone(),
        })
    }
    pub async fn download(&self) -> Result<(), String> {
        let cancel = {
            let mut task = self
                .0
                .task
                .lock()
                .map_err(|_| "Voice mode state unavailable")?;
            if task.phase != VoiceModePhase::Off {
                return Err("Stop voice mode before installing its listening pack.".into());
            }
            task.phase = VoiceModePhase::Downloading;
            task.cancel = CancellationToken::new();
            task.error = None;
            task.cancel.clone()
        };
        let _job = Job(self.0.clone());
        self.0.progress.store(0, Ordering::Relaxed);
        self.0.model.download(cancel, self.0.progress.clone()).await
    }
    pub fn start(&self, sink: Arc<dyn VoiceModeSink>) -> Result<String, String> {
        let mut task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        if task.phase != VoiceModePhase::Off {
            return Err("Voice mode is already active or finishing setup.".into());
        }
        if !self.0.model.info().ready {
            return Err("Download the Hey Jarvis listening pack in Voice settings first.".into());
        }
        task.id = uuid::Uuid::new_v4().to_string();
        task.enabled = true;
        task.phase = VoiceModePhase::Starting;
        task.cancel = CancellationToken::new();
        task.heartbeat = Instant::now();
        task.resume = false;
        task.error = None;
        let id = task.id.clone();
        let cancel = task.cancel.clone();
        let inner = self.0.clone();
        std::thread::Builder::new()
            .name("voice-conversation".into())
            .spawn(move || {
                let _job = Job(inner.clone());
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(&inner, &*sink, cancel.clone())
                }))
                .unwrap_or_else(|_| Err("Voice conversation worker stopped unexpectedly".into()));
                if let Err(message) = result {
                    if !cancel.is_cancelled() {
                        if let Ok(mut task) = inner.task.lock() {
                            task.error = Some(message.clone());
                        }
                        let _ = sink.emit(VoiceModeEvent::Error { message });
                    }
                }
                let _ = sink.emit(VoiceModeEvent::Stopped);
            })
            .map_err(|_| {
                task.enabled = false;
                task.phase = VoiceModePhase::Off;
                "Could not start voice conversation".to_owned()
            })?;
        Ok(id)
    }
    pub fn heartbeat(&self, id: &str) -> Result<(), String> {
        let mut task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        if !task.enabled || task.id != id {
            return Err("Voice session ended".into());
        }
        task.heartbeat = Instant::now();
        Ok(())
    }
    pub fn resume(&self, id: &str) -> Result<(), String> {
        let mut task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        if !task.enabled || task.id != id || task.phase != VoiceModePhase::Paused {
            return Err("Voice session is not waiting for a reply".into());
        }
        task.resume = true;
        Ok(())
    }
    pub fn stop(&self) -> Result<(), String> {
        let mut task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        task.cancel.cancel();
        if task.enabled {
            task.phase = VoiceModePhase::Stopping;
        }
        Ok(())
    }
    pub fn reply_token(&self) -> Result<Option<CancellationToken>, String> {
        let task = self
            .0
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?;
        Ok(
            (task.enabled && task.phase == VoiceModePhase::Paused && !task.cancel.is_cancelled())
                .then(|| task.cancel.clone()),
        )
    }
}
fn phase(inner: &Inner, sink: &dyn VoiceModeSink, value: VoiceModePhase) -> Result<(), String> {
    inner
        .task
        .lock()
        .map_err(|_| "Voice mode state unavailable")?
        .phase = value;
    sink.emit(VoiceModeEvent::Phase { phase: value })
}
fn live(inner: &Inner, cancel: &CancellationToken) -> bool {
    if cancel.is_cancelled() {
        return false;
    }
    let valid = inner
        .task
        .lock()
        .is_ok_and(|task| task.heartbeat.elapsed() < Duration::from_secs(8));
    if !valid {
        cancel.cancel();
    }
    valid
}
fn run(inner: &Inner, sink: &dyn VoiceModeSink, cancel: CancellationToken) -> Result<(), String> {
    let wake = inner.wake.create()?;
    let turn = inner.turn.create()?;
    if !live(inner, &cancel) {
        return Ok(());
    }
    let mut microphone = Some(inner.input.open()?);
    let mut listening = false;
    let mut started = Instant::now();
    let mut meter_updated = Instant::now() - Duration::from_secs(1);
    phase(inner, sink, VoiceModePhase::Waiting)?;
    while live(inner, &cancel) {
        let paused = inner
            .task
            .lock()
            .map_err(|_| "Voice mode state unavailable")?
            .phase
            == VoiceModePhase::Paused;
        if paused {
            let resume = {
                let mut task = inner
                    .task
                    .lock()
                    .map_err(|_| "Voice mode state unavailable")?;
                std::mem::take(&mut task.resume)
            };
            if !resume {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            microphone = Some(inner.input.open()?);
            turn.reset();
            listening = true;
            started = Instant::now();
            phase(inner, sink, VoiceModePhase::Listening)?;
        }
        let frame = microphone
            .as_ref()
            .ok_or("Microphone unavailable")?
            .next(Duration::from_millis(100))?;
        let Some(frame) = frame else {
            continue;
        };
        if meter_updated.elapsed() >= Duration::from_millis(100) {
            sink.emit(VoiceModeEvent::InputLevel {
                level: input_level(&frame),
            })?;
            meter_updated = Instant::now();
        }
        if !listening {
            if wake.feed(&frame) {
                turn.reset();
                listening = true;
                started = Instant::now();
                phase(inner, sink, VoiceModePhase::Listening)?;
            }
            continue;
        }
        if let Some(samples) = turn.feed(&frame) {
            microphone.take(); // Close the device before transcription, inference, and playback.
            phase(inner, sink, VoiceModePhase::Transcribing)?;
            let transcript = inner.recognizer.transcribe(samples, cancel.clone())?;
            if !live(inner, &cancel) {
                break;
            }
            let transcript = strip_wake_phrase(&transcript);
            if transcript.is_empty() {
                microphone = Some(inner.input.open()?);
                turn.reset();
                started = Instant::now();
                phase(inner, sink, VoiceModePhase::Listening)?;
                continue;
            }
            if matches!(
                transcript
                    .to_lowercase()
                    .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace()),
                "stop listening" | "stop voice mode" | "goodbye jarvis" | "goodbye"
            ) {
                break;
            }
            phase(inner, sink, VoiceModePhase::Paused)?;
            sink.emit(VoiceModeEvent::Transcript { text: transcript })?;
        } else if started.elapsed() > Duration::from_secs(30) {
            listening = false;
            turn.reset();
            wake.reset();
            phase(inner, sink, VoiceModePhase::Waiting)?;
        }
    }
    drop(microphone);
    Ok(())
}
pub fn strip_wake_phrase(text: &str) -> String {
    let text = text.trim();
    let lower = text.to_lowercase();
    for prefix in ["hey jarvis", "hey, jarvis", "jarvis"] {
        if lower.starts_with(prefix) {
            return text[prefix.len()..]
                .trim_start_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
                .to_owned();
        }
    }
    text.to_owned()
}

fn input_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.;
    }
    let energy = samples
        .iter()
        .filter(|s| s.is_finite())
        .map(|s| f64::from(*s).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    // Map -60 dBFS through 0 dBFS to the visible meter range.
    ((20. * energy.sqrt().max(0.000001).log10() + 60.) / 60.).clamp(0., 1.) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meter_distinguishes_silence_from_captured_sound() {
        assert_eq!(input_level(&[0.; 512]), 0.);
        assert!(input_level(&[0.1; 512]) > 0.6);
        assert_eq!(input_level(&[1.; 512]), 1.);
        assert_eq!(input_level(&[f32::NAN; 512]), 0.);
    }
    use crate::domain::voice::SpeechModelInfo;
    use futures_util::future::BoxFuture;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    struct Frames(Arc<AtomicUsize>);
    impl Drop for Frames {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::Relaxed);
        }
    }
    impl InputFrames for Frames {
        fn next(&self, _: Duration) -> Result<Option<Vec<f32>>, String> {
            std::thread::sleep(Duration::from_millis(1));
            Ok(Some(vec![0.1; 512]))
        }
    }
    struct Input(Arc<AtomicUsize>);
    impl VoiceInput for Input {
        fn open(&self) -> Result<Box<dyn InputFrames>, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Box::new(Frames(self.0.clone())))
        }
    }
    struct Wake(AtomicBool);
    impl WakeDetector for Wake {
        fn feed(&self, _: &[f32]) -> bool {
            !self.0.swap(true, Ordering::Relaxed)
        }
        fn reset(&self) {
            self.0.store(false, Ordering::Relaxed);
        }
    }
    struct WakeMaker;
    impl WakeFactory for WakeMaker {
        fn create(&self) -> Result<Box<dyn WakeDetector>, String> {
            Ok(Box::new(Wake(AtomicBool::new(false))))
        }
    }
    struct Turn(AtomicUsize);
    impl TurnDetector for Turn {
        fn feed(&self, _: &[f32]) -> Option<Vec<f32>> {
            if self.0.fetch_add(1, Ordering::Relaxed) == 1 {
                Some(vec![0.1; 16_000])
            } else {
                None
            }
        }
        fn reset(&self) {
            self.0.store(0, Ordering::Relaxed);
        }
    }
    struct TurnMaker;
    impl TurnFactory for TurnMaker {
        fn create(&self) -> Result<Box<dyn TurnDetector>, String> {
            Ok(Box::new(Turn(AtomicUsize::new(0))))
        }
    }
    struct Recognizer(Arc<AtomicUsize>);
    impl SpeechRecognizer for Recognizer {
        fn transcribe(&self, _: Vec<f32>, _: CancellationToken) -> Result<String, String> {
            assert_eq!(
                self.0.load(Ordering::Relaxed),
                0,
                "microphone must close before transcription"
            );
            Ok("What is two plus two?".into())
        }
    }
    struct Model;
    impl SpeechModel for Model {
        fn info(&self) -> SpeechModelInfo {
            SpeechModelInfo {
                ready: true,
                bytes: 10,
            }
        }
        fn download(
            &self,
            _: CancellationToken,
            _: Arc<AtomicU64>,
        ) -> BoxFuture<'_, Result<(), String>> {
            Box::pin(async { Ok(()) })
        }
    }
    #[derive(Default)]
    struct Sink(Mutex<Vec<VoiceModeEvent>>);
    impl VoiceModeSink for Sink {
        fn emit(&self, event: VoiceModeEvent) -> Result<(), String> {
            self.0.lock().unwrap().push(event);
            Ok(())
        }
    }
    fn fixture() -> (VoiceModeService, Arc<AtomicUsize>, Arc<Sink>) {
        let open = Arc::new(AtomicUsize::new(0));
        let mode = VoiceModeService::new(
            Box::new(Input(open.clone())),
            Box::new(WakeMaker),
            Box::new(TurnMaker),
            Arc::new(Recognizer(open.clone())),
            Box::new(Model),
        );
        (mode, open, Arc::new(Sink::default()))
    }
    fn until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() {
            assert!(Instant::now() < deadline, "voice mode did not settle");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn hands_free_turn_closes_mic_then_requires_explicit_resume_after_reply() {
        let (mode, open, sink) = fixture();
        let id = mode.start(sink.clone()).unwrap();
        until(|| {
            sink.0
                .lock()
                .unwrap()
                .iter()
                .any(|event| matches!(event, VoiceModeEvent::Transcript { .. }))
        });
        assert_eq!(open.load(Ordering::Relaxed), 0);
        assert!(mode.status().unwrap().phase == VoiceModePhase::Paused);
        assert!(mode.resume("stale-session").is_err());
        assert!(mode.start(sink.clone()).is_err());
        mode.resume(&id).unwrap();
        until(|| {
            sink.0
                .lock()
                .unwrap()
                .iter()
                .filter(|event| matches!(event, VoiceModeEvent::Transcript { .. }))
                .count()
                == 2
        });
        mode.stop().unwrap();
        until(|| !mode.status().unwrap().enabled);
        assert_eq!(open.load(Ordering::Relaxed), 0);
    }
    #[test]
    fn missing_heartbeat_closes_capture_and_invalidates_session() {
        let (mode, open, sink) = fixture();
        let id = mode.start(sink).unwrap();
        mode.0.task.lock().unwrap().heartbeat = Instant::now() - Duration::from_secs(9);
        until(|| !mode.status().unwrap().enabled);
        assert_eq!(open.load(Ordering::Relaxed), 0);
        assert!(mode.heartbeat(&id).is_err());
    }
    #[test]
    fn removes_only_a_leading_wake_phrase() {
        assert_eq!(
            strip_wake_phrase("Hey Jarvis, plan my day."),
            "plan my day."
        );
        assert_eq!(strip_wake_phrase("Jarvis!"), "");
        assert_eq!(
            strip_wake_phrase("Tell me about Jarvis."),
            "Tell me about Jarvis."
        );
    }
}
