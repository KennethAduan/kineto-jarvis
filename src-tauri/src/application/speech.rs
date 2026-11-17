use crate::domain::{
    speech::*,
    voice::{SpeechModel, SpeechModelInfo, SpeechOutput},
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

const PREVIEW_TEXT: &str = "Good evening. All systems are ready. How may I assist you today?";
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechPhase {
    Idle,
    Downloading,
    Preparing,
    Playing,
}
#[derive(Serialize)]
pub struct SpeechStatus {
    pub preferences: SpeechPreferences,
    pub model: SpeechModelInfo,
    pub phase: SpeechPhase,
    pub downloaded_bytes: u64,
    pub error: Option<String>,
    pub packs: SpeechPacks,
}
#[derive(Serialize)]
pub struct SpeechPacks {
    pub kokoro: SpeechModelInfo,
    pub supertonic: SpeechModelInfo,
}
struct Task {
    phase: SpeechPhase,
    cancel: CancellationToken,
    error: Option<String>,
    download_voice: Option<SpeechVoice>,
}
struct Inner {
    synthesizer: Box<dyn SpeechSynthesizer>,
    player: Box<dyn AudioPlayback>,
    store: Box<dyn SpeechPreferencesStore>,
    model: Box<dyn SpeechModel>,
    supertonic: Box<dyn SpeechModel>,
    system: Box<dyn SpeechOutput>,
    preferences: Mutex<SpeechPreferences>,
    task: Mutex<Task>,
    progress: Arc<AtomicU64>,
}
#[derive(Clone)]
pub struct SpeechService(Arc<Inner>);
pub struct SpeechStream {
    sender: std::sync::mpsc::SyncSender<String>,
    cancel: CancellationToken,
}
impl SpeechStream {
    pub fn push(&self, text: String) -> Result<(), String> {
        if self.cancel.is_cancelled() {
            return Err("Voice reply stopped".into());
        }
        self.sender.try_send(text).map_err(|_| {
            "Live speech queue is full or stopped. Stop voice mode and try a shorter answer.".into()
        })
    }
}
struct Job(Arc<Inner>);
impl Drop for Job {
    fn drop(&mut self) {
        if let Ok(mut task) = self.0.task.lock() {
            task.phase = SpeechPhase::Idle;
            task.download_voice = None;
        }
    }
}
impl SpeechService {
    pub fn new(
        synthesizer: Box<dyn SpeechSynthesizer>,
        player: Box<dyn AudioPlayback>,
        store: Box<dyn SpeechPreferencesStore>,
        model: Box<dyn SpeechModel>,
        supertonic: Box<dyn SpeechModel>,
        system: Box<dyn SpeechOutput>,
    ) -> Result<Self, String> {
        let (preferences, error) = match store.load().and_then(|preferences| { preferences.validate()?; Ok(preferences) }) {
            Ok(preferences) => (preferences, None),
            Err(_) => (SpeechPreferences::default(), Some("Saved voice preferences could not be read. Using George until you save a new preference.".into())),
        };
        Ok(Self(Arc::new(Inner {
            synthesizer,
            player,
            store,
            model,
            supertonic,
            system,
            preferences: Mutex::new(preferences),
            task: Mutex::new(Task {
                phase: SpeechPhase::Idle,
                cancel: CancellationToken::new(),
                error,
                download_voice: None,
            }),
            progress: Arc::new(AtomicU64::new(0)),
        })))
    }
    pub fn status(&self) -> Result<SpeechStatus, String> {
        let task = self.0.task.lock().map_err(|_| "Speech state unavailable")?;
        let preferences = *self
            .0
            .preferences
            .lock()
            .map_err(|_| "Voice preferences unavailable")?;
        Ok(SpeechStatus {
            preferences,
            model: pack(&self.0, task.download_voice.unwrap_or(preferences.voice)).info(),
            packs: SpeechPacks {
                kokoro: self.0.model.info(),
                supertonic: self.0.supertonic.info(),
            },
            phase: task.phase,
            downloaded_bytes: self.0.progress.load(Ordering::Relaxed),
            error: task.error.clone(),
        })
    }
    pub fn save(&self, preferences: SpeechPreferences) -> Result<(), String> {
        preferences.validate()?;
        let mut task = self.0.task.lock().map_err(|_| "Speech state unavailable")?;
        if task.phase != SpeechPhase::Idle {
            return Err("Stop playback or finish setup before changing the voice.".into());
        }
        self.0.store.save(preferences)?;
        *self
            .0
            .preferences
            .lock()
            .map_err(|_| "Voice preferences unavailable")? = preferences;
        task.error = None;
        Ok(())
    }
    pub fn preview(&self, preferences: SpeechPreferences) -> Result<(), String> {
        preferences.validate()?;
        self.start(PREVIEW_TEXT, Some(preferences))
    }
    pub async fn download(&self, voice: SpeechVoice) -> Result<(), String> {
        let cancel = self.begin(SpeechPhase::Downloading)?;
        let _job = Job(self.0.clone());
        self.0
            .task
            .lock()
            .map_err(|_| "Speech state unavailable")?
            .download_voice = Some(voice);
        self.0.progress.store(0, Ordering::Relaxed);
        pack(&self.0, voice)
            .download(cancel, self.0.progress.clone())
            .await
    }
    fn begin(&self, phase: SpeechPhase) -> Result<CancellationToken, String> {
        let mut task = self.0.task.lock().map_err(|_| "Speech state unavailable")?;
        if task.phase != SpeechPhase::Idle {
            return Err("Stop the current speech action first.".into());
        }
        task.phase = phase;
        task.cancel = CancellationToken::new();
        task.error = None;
        Ok(task.cancel.clone())
    }
}
impl SpeechService {
    /// Reserves one cancellable playback worker for a bounded queue of sentences.
    pub fn stream(&self, parent: &CancellationToken) -> Result<SpeechStream, String> {
        let mut task = self.0.task.lock().map_err(|_| "Speech state unavailable")?;
        if task.phase != SpeechPhase::Idle {
            return Err("Stop the current speech action first.".into());
        }
        let preferences = *self
            .0
            .preferences
            .lock()
            .map_err(|_| "Voice preferences unavailable")?;
        if preferences.voice != SpeechVoice::System
            && !pack(&self.0, preferences.voice).info().ready
        {
            return Err("Download the selected voice pack first.".into());
        }
        let cancel = parent.child_token();
        if cancel.is_cancelled() {
            return Err("Voice reply stopped".into());
        }
        task.phase = SpeechPhase::Preparing;
        task.cancel = cancel.clone();
        task.error = None;
        let (sender, receiver) = std::sync::mpsc::sync_channel::<String>(256);
        let inner = self.0.clone();
        let token = cancel.clone();
        std::thread::Builder::new()
            .name("live-local-speech".into())
            .spawn(move || {
                let _job = Job(inner.clone());
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_stream(&inner, receiver, preferences, token.clone())
                }))
                .unwrap_or_else(|_| Err("Speech worker stopped unexpectedly".into()));
                if let Err(error) = result {
                    if !token.is_cancelled() {
                        if let Ok(mut task) = inner.task.lock() {
                            task.error = Some(error);
                        }
                    }
                }
            })
            .map_err(|_| {
                task.phase = SpeechPhase::Idle;
                "Could not start live speech".to_owned()
            })?;
        Ok(SpeechStream { sender, cancel })
    }
    fn start(&self, text: &str, preview: Option<SpeechPreferences>) -> Result<(), String> {
        if text.trim().is_empty() || text.chars().count() > 8_000 {
            return Err("Choose a reply with up to 8,000 speakable characters.".into());
        }
        // Keep the task lock while checking preferences and reserving playback.
        let mut task = self.0.task.lock().map_err(|_| "Speech state unavailable")?;
        if task.phase != SpeechPhase::Idle {
            return Err("Stop the current speech action first.".into());
        }
        let preferences = preview.unwrap_or(
            *self
                .0
                .preferences
                .lock()
                .map_err(|_| "Voice preferences unavailable")?,
        );
        if preferences.voice != SpeechVoice::System
            && !pack(&self.0, preferences.voice).info().ready
        {
            return Err(
                "Download the selected voice pack in Voice settings before reading aloud.".into(),
            );
        }
        task.phase = SpeechPhase::Preparing;
        task.cancel = CancellationToken::new();
        task.error = None;
        let cancel = task.cancel.clone();
        let inner = self.0.clone();
        let text = text.replace('\0', " ");
        std::thread::Builder::new()
            .name("local-speech".into())
            .spawn(move || {
                let _job = Job(inner.clone());
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(&inner, &text, preferences, cancel.clone())
                }))
                .unwrap_or_else(|_| Err("Speech worker stopped unexpectedly".into()));
                if let Err(error) = result {
                    if !cancel.is_cancelled() {
                        if let Ok(mut task) = inner.task.lock() {
                            task.error = Some(error);
                        }
                    }
                }
            })
            .map_err(|_| {
                task.phase = SpeechPhase::Idle;
                "Could not start the speech worker".to_owned()
            })?;
        Ok(())
    }
}
fn pack(inner: &Inner, voice: SpeechVoice) -> &dyn SpeechModel {
    if voice.is_supertonic() {
        &*inner.supertonic
    } else {
        &*inner.model
    }
}
impl SpeechOutput for SpeechService {
    fn speak(&self, text: &str) -> Result<(), String> {
        self.start(text, None)
    }
    fn stop(&self) -> Result<(), String> {
        self.0
            .task
            .lock()
            .map_err(|_| "Speech state unavailable")?
            .cancel
            .cancel();
        self.0.system.stop()
    }
    fn is_speaking(&self) -> bool {
        self.0
            .task
            .lock()
            .is_ok_and(|task| matches!(task.phase, SpeechPhase::Preparing | SpeechPhase::Playing))
    }
    fn error(&self) -> Option<String> {
        self.0.task.lock().ok().and_then(|task| task.error.clone())
    }
}
fn run(
    inner: &Inner,
    text: &str,
    preferences: SpeechPreferences,
    cancel: CancellationToken,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Ok(());
    }
    if preferences.voice == SpeechVoice::System {
        inner.system.speak(text)?;
        let deadline = Instant::now() + Duration::from_secs(600);
        while inner.system.is_speaking() {
            if cancel.is_cancelled() || Instant::now() > deadline {
                inner.system.stop()?;
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        return Ok(());
    }
    let deadline = Instant::now() + Duration::from_secs(600);
    for chunk in speech_chunks(text) {
        if cancel.is_cancelled() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err("Speech playback reached its time limit".into());
        }
        if let Ok(mut task) = inner.task.lock() {
            task.phase = SpeechPhase::Preparing;
        }
        let audio = inner
            .synthesizer
            .synthesize(&chunk, preferences, cancel.clone())?;
        if cancel.is_cancelled() {
            return Ok(());
        }
        if let Ok(mut task) = inner.task.lock() {
            task.phase = SpeechPhase::Playing;
        }
        inner.player.play(audio, cancel.clone())?;
    }
    Ok(())
}

fn run_stream(
    inner: &Inner,
    receiver: std::sync::mpsc::Receiver<String>,
    preferences: SpeechPreferences,
    cancel: CancellationToken,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(600);
    if preferences.voice == SpeechVoice::System {
        while !cancel.is_cancelled() {
            if Instant::now() >= deadline {
                return Err("Live speech reached its time limit".into());
            }
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(text) => run(inner, &text, preferences, cancel.clone())?,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        return Ok(());
    }
    // Prepare upcoming speech while the previous buffer plays. Keep only one ready buffer.
    let producer_cancel = cancel.child_token();
    std::thread::scope(|scope| {
        let (sender, audio) = std::sync::mpsc::sync_channel::<SpeechAudio>(1);
        let token = producer_cancel.clone();
        let producer = scope.spawn(move || -> Result<(), String> {
            while !token.is_cancelled() {
                if Instant::now() >= deadline {
                    return Err("Live speech reached its time limit".into());
                }
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(text) => {
                        for chunk in speech_chunks(&text) {
                            if token.is_cancelled() {
                                return Ok(());
                            }
                            let buffer =
                                inner
                                    .synthesizer
                                    .synthesize(&chunk, preferences, token.clone())?;
                            if token.is_cancelled() {
                                return Ok(());
                            }
                            if sender.send(buffer).is_err() {
                                return Ok(());
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            Ok(())
        });
        let playback = (|| {
            while !cancel.is_cancelled() {
                if Instant::now() >= deadline {
                    return Err("Live speech reached its time limit".into());
                }
                match audio.recv_timeout(Duration::from_millis(100)) {
                    Ok(buffer) => {
                        if cancel.is_cancelled() {
                            break;
                        }
                        if let Ok(mut task) = inner.task.lock() {
                            task.phase = SpeechPhase::Playing;
                        }
                        inner.player.play(buffer, cancel.clone())?;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            Ok(())
        })();
        // Drop the receiver before joining so cancellation also unblocks a full audio queue.
        drop(audio);
        producer_cancel.cancel();
        let generated = producer
            .join()
            .unwrap_or_else(|_| Err("Speech preparation stopped unexpectedly".into()));
        playback.and(generated)
    })
}
pub fn speech_chunks(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = text.trim();
    while !remaining.is_empty() {
        let limit = remaining
            .char_indices()
            .nth(220)
            .map_or(remaining.len(), |(index, _)| index);
        let boundary = if limit == remaining.len() {
            limit
        } else {
            remaining[..limit]
                .rfind(char::is_whitespace)
                .filter(|index| *index > 0)
                .unwrap_or(limit)
        };
        result.push(remaining[..boundary].trim().to_owned());
        remaining = remaining[boundary..].trim_start();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    #[derive(Clone, Default)]
    struct Seen(Arc<Mutex<Vec<SpeechPreferences>>>);
    struct Synth {
        seen: Seen,
        wait: bool,
        fail: bool,
    }
    impl SpeechSynthesizer for Synth {
        fn synthesize(
            &self,
            _: &str,
            preferences: SpeechPreferences,
            cancel: CancellationToken,
        ) -> Result<SpeechAudio, String> {
            self.seen.0.lock().unwrap().push(preferences);
            if self.fail {
                return Err("Synthesis failed".into());
            }
            if self.wait {
                while !cancel.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                return Err("cancelled".into());
            }
            Ok(SpeechAudio {
                samples: vec![0.1; 240],
                sample_rate: 24_000,
            })
        }
    }
    struct Player;
    impl AudioPlayback for Player {
        fn play(&self, audio: SpeechAudio, _: CancellationToken) -> Result<(), String> {
            assert_eq!(audio.samples.len(), 240);
            Ok(())
        }
    }
    struct Store(Arc<Mutex<SpeechPreferences>>);
    impl SpeechPreferencesStore for Store {
        fn load(&self) -> Result<SpeechPreferences, String> {
            Ok(*self.0.lock().unwrap())
        }
        fn save(&self, preferences: SpeechPreferences) -> Result<(), String> {
            *self.0.lock().unwrap() = preferences;
            Ok(())
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
            _: CancellationToken,
            _: Arc<AtomicU64>,
        ) -> BoxFuture<'_, Result<(), String>> {
            Box::pin(async { Ok(()) })
        }
    }
    struct System;
    impl SpeechOutput for System {
        fn speak(&self, _: &str) -> Result<(), String> {
            panic!("must never silently fall back to system speech")
        }
        fn stop(&self) -> Result<(), String> {
            Ok(())
        }
        fn is_speaking(&self) -> bool {
            false
        }
    }
    fn fixture(
        ready: bool,
        wait: bool,
        fail: bool,
    ) -> (SpeechService, Seen, Arc<Mutex<SpeechPreferences>>) {
        fixture_with_player(ready, wait, fail, Box::new(Player))
    }
    fn fixture_with_player(
        ready: bool,
        wait: bool,
        fail: bool,
        player: Box<dyn AudioPlayback>,
    ) -> (SpeechService, Seen, Arc<Mutex<SpeechPreferences>>) {
        let seen = Seen::default();
        let preferences = Arc::new(Mutex::new(SpeechPreferences::default()));
        (
            SpeechService::new(
                Box::new(Synth {
                    seen: seen.clone(),
                    wait,
                    fail,
                }),
                player,
                Box::new(Store(preferences.clone())),
                Box::new(Model(ready)),
                Box::new(Model(ready)),
                Box::new(System),
            )
            .unwrap(),
            seen,
            preferences,
        )
    }
    #[test]
    fn prepares_next_sentence_during_playback_and_cancels_a_full_audio_queue() {
        use std::sync::atomic::AtomicBool;
        struct HeldPlayer(Arc<AtomicBool>);
        impl AudioPlayback for HeldPlayer {
            fn play(&self, _: SpeechAudio, cancel: CancellationToken) -> Result<(), String> {
                self.0.store(true, Ordering::Relaxed);
                while !cancel.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(())
            }
        }
        let playing = Arc::new(AtomicBool::new(false));
        let (speech, seen, _) =
            fixture_with_player(true, false, false, Box::new(HeldPlayer(playing.clone())));
        let parent = CancellationToken::new();
        let stream = speech.stream(&parent).unwrap();
        for sentence in [
            "First sentence.",
            "Second sentence.",
            "Third sentence.",
            "Fourth sentence.",
        ] {
            stream.push(sentence.into()).unwrap();
        }
        until(|| playing.load(Ordering::Relaxed));
        until(|| seen.0.lock().unwrap().len() == 3);
        // First is playing, second is ready, third is blocked at the bounded handoff.
        assert!(speech.is_speaking());
        parent.cancel();
        until(|| !speech.is_speaking());
        assert_eq!(seen.0.lock().unwrap().len(), 3);
    }
    fn until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() {
            assert!(Instant::now() < deadline, "speech task did not settle");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn streamed_model_sentences_reach_speech_before_completion_without_replaying_or_reading_emoji()
    {
        struct Sink;
        impl crate::domain::ports::EventSink for Sink {
            fn send(&self, _: crate::domain::models::StreamEvent) -> Result<(), String> {
                Ok(())
            }
        }
        use crate::{
            application::live_speech::SpeakingSink,
            domain::{models::StreamEvent, ports::EventSink},
        };
        let (speech, seen, _) = fixture(true, false, false);
        let sink = SpeakingSink::new(&Sink, speech.clone(), CancellationToken::new());
        sink.send(StreamEvent::Thinking).unwrap();
        assert!(seen.0.lock().unwrap().is_empty());
        sink.send(StreamEvent::Delta {
            text: "Hey there! ".into(),
        })
        .unwrap();
        until(|| seen.0.lock().unwrap().len() == 1);
        assert!(speech.is_speaking());
        sink.send(StreamEvent::Delta {
            text: "How can I assist you today, Ito? 😊".into(),
        })
        .unwrap();
        until(|| seen.0.lock().unwrap().len() == 2);
        sink.finish(true).unwrap();
        until(|| !speech.is_speaking());
        assert_eq!(
            seen.0.lock().unwrap().len(),
            2,
            "emoji tail must not create another utterance"
        );
    }
    #[test]
    fn live_queue_plays_before_input_finishes_and_cancels_pending_sentences() {
        let (speech, seen, _) = fixture(true, false, false);
        let parent = CancellationToken::new();
        let stream = speech.stream(&parent).unwrap();
        stream.push("The first sentence.".into()).unwrap();
        until(|| seen.0.lock().unwrap().len() == 1);
        // The model can still be generating; an open queue holds playback ownership.
        assert!(speech.is_speaking());
        stream.push("The next sentence.".into()).unwrap();
        drop(stream);
        until(|| !speech.is_speaking());
        assert_eq!(seen.0.lock().unwrap().len(), 2);

        let (speech, seen, _) = fixture(true, true, false);
        let parent = CancellationToken::new();
        let stream = speech.stream(&parent).unwrap();
        stream.push("A slow first sentence.".into()).unwrap();
        until(|| seen.0.lock().unwrap().len() == 1);
        stream.push("This must never be spoken.".into()).unwrap();
        parent.cancel();
        until(|| !speech.is_speaking());
        assert_eq!(seen.0.lock().unwrap().len(), 1);
        assert!(stream.push("Late text".into()).is_err());
    }
    #[test]
    fn preview_does_not_save_and_read_aloud_uses_saved_voice() {
        let (speech, seen, store) = fixture(true, false, false);
        let preview = SpeechPreferences {
            voice: SpeechVoice::Fable,
            speed: 1.1,
            volume: 1.,
        };
        speech.preview(preview).unwrap();
        until(|| speech.status().unwrap().phase == SpeechPhase::Idle);
        assert_eq!(seen.0.lock().unwrap()[0], preview);
        assert_eq!(store.lock().unwrap().voice, SpeechVoice::George);
        let saved = SpeechPreferences {
            voice: SpeechVoice::Daniel,
            speed: 1.2,
            volume: 1.,
        };
        speech.save(saved).unwrap();
        speech.speak("A completed reply").unwrap();
        until(|| speech.status().unwrap().phase == SpeechPhase::Idle);
        assert_eq!(seen.0.lock().unwrap()[1], saved);
        assert_eq!(*store.lock().unwrap(), saved);
    }
    #[test]
    fn cancellation_drops_late_audio_and_rejects_overlapping_actions() {
        let (speech, seen, _) = fixture(true, true, false);
        speech.speak("A reply").unwrap();
        until(|| !seen.0.lock().unwrap().is_empty());
        assert!(speech.save(SpeechPreferences::default()).is_err());
        assert!(speech.preview(SpeechPreferences::default()).is_err());
        speech.stop().unwrap();
        until(|| speech.status().unwrap().phase == SpeechPhase::Idle);
        assert!(speech.error().is_none());
    }
    #[test]
    fn setup_is_required_and_worker_errors_are_visible() {
        let (missing, _, store) = fixture(false, false, false);
        assert!(missing.speak("A reply").is_err());
        assert!(missing
            .save(SpeechPreferences {
                voice: SpeechVoice::George,
                speed: 5.,
                volume: 1.,
            })
            .is_err());
        assert_eq!(store.lock().unwrap().speed, 1.15);
        let (speech, _, _) = fixture(true, false, true);
        speech.speak("A reply").unwrap();
        until(|| speech.status().unwrap().phase == SpeechPhase::Idle);
        assert_eq!(speech.error().as_deref(), Some("Synthesis failed"));
    }
    #[test]
    fn chunks_preserve_words_and_bound_unicode_segments() {
        let text = "A useful sentence with words. ".repeat(50);
        let chunks = speech_chunks(&text);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 220));
        assert_eq!(chunks.join(" "), text.trim());
        assert!(speech_chunks(&"界".repeat(1000))
            .iter()
            .all(|chunk| chunk.chars().count() <= 220));
    }
}
