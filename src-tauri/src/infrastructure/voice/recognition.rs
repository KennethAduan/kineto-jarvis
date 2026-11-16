use super::model::WhisperModel;
use crate::domain::voice::SpeechRecognizer;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct LocalWhisper {
    path: PathBuf,
    context: Mutex<Option<WhisperContext>>,
}
struct AbortContext {
    cancel: CancellationToken,
    deadline: Instant,
}
unsafe extern "C" fn should_abort(data: *mut std::ffi::c_void) -> bool {
    // The boxed context remains alive until synchronous state.full has joined its workers.
    // Callback access is read-only, so parallel calls cannot mutate shared Rust state.
    let context = unsafe { &*(data as *const AbortContext) };
    context.cancel.is_cancelled() || Instant::now() > context.deadline
}
impl LocalWhisper {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            context: Mutex::new(None),
        }
    }
}
impl SpeechRecognizer for LocalWhisper {
    fn transcribe(&self, samples: Vec<f32>, cancel: CancellationToken) -> Result<String, String> {
        // Default Whisper/GGML diagnostics can contain transcript tokens. No logging backend
        // is enabled: these hooks keep recognition content out of stdout/stderr as well.
        whisper_rs::install_logging_hooks();
        if cancel.is_cancelled() {
            return Err("Transcription cancelled".into());
        }
        let mut context = self
            .context
            .lock()
            .map_err(|_| "Speech recognition state unavailable")?;
        if context.is_none() {
            WhisperModel::verify(&self.path)?;
            let mut parameters = WhisperContextParameters::default();
            parameters.use_gpu(false);
            *context = Some(
                WhisperContext::new_with_params(&self.path, parameters).map_err(|_| {
                    "Could not load the offline speech model. Reinstall it in Voice settings."
                })?,
            );
        }
        let context = context.as_ref().ok_or("Speech model unavailable")?;
        let mut state = context
            .create_state()
            .map_err(|_| "Could not prepare speech recognition")?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(
            std::thread::available_parallelism().map_or(2, |n| n.get().min(4)) as i32,
        );
        params.set_language(None);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_print_realtime(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        let mut abort = Box::new(AbortContext {
            cancel: cancel.clone(),
            deadline: Instant::now() + Duration::from_secs(120),
        });
        // whisper-rs 0.16's convenience callback miscasts its double-boxed closure.
        // Supply a concrete, owned context to the C callback instead; it outlives full().
        unsafe {
            params.set_abort_callback(Some(should_abort));
            params.set_abort_callback_user_data((&mut *abort as *mut AbortContext).cast());
        }
        state.full(params, &samples).map_err(|_| {
            if cancel.is_cancelled() {
                "Transcription cancelled"
            } else {
                "Local speech recognition failed or timed out. Try a shorter recording."
            }
        })?;
        if cancel.is_cancelled() {
            return Err("Transcription cancelled".into());
        }
        let mut text = String::new();
        for segment in state.as_iter() {
            text.push_str(
                &segment
                    .to_str_lossy()
                    .map_err(|_| "Could not decode the recognized speech")?,
            );
        }
        let text = text.trim().to_owned();
        if text.is_empty() {
            return Err("No speech was recognized. Try again closer to the microphone.".into());
        }
        if text.chars().count() > 16_000 {
            return Err("The transcript is too long for one message.".into());
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_callback_observes_cancellation_and_deadline() {
        let mut context = Box::new(AbortContext {
            cancel: CancellationToken::new(),
            deadline: Instant::now() + Duration::from_secs(120),
        });
        let pointer = (&mut *context as *mut AbortContext).cast();
        assert!(!unsafe { should_abort(pointer) });
        context.cancel.cancel();
        assert!(unsafe { should_abort(pointer) });
        context.cancel = CancellationToken::new();
        context.deadline = Instant::now() - Duration::from_secs(1);
        assert!(unsafe { should_abort(pointer) });
    }
    #[test]
    #[ignore = "requires WHISPER_TEST_MODEL and WHISPER_TEST_WAV; never captures a microphone"]
    fn recognizes_public_sample_offline() {
        let model = std::env::var_os("WHISPER_TEST_MODEL").expect("model path");
        let wav = std::env::var_os("WHISPER_TEST_WAV").expect("WAV path");
        let mut reader = hound::WavReader::open(wav).unwrap();
        assert_eq!(reader.spec().sample_rate, 16_000);
        assert_eq!(reader.spec().channels, 1);
        let samples = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.)
            .collect();
        let text = LocalWhisper::new(model.into())
            .transcribe(samples, CancellationToken::new())
            .unwrap()
            .to_lowercase();
        assert!(text.contains("country"), "unexpected transcript: {text}");
        assert!(text.contains("ask"), "unexpected transcript: {text}");
    }
}
