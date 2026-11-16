use crate::domain::voice::SpeechOutput;
use std::sync::Mutex;

#[derive(Default)]
pub struct SystemSpeech {
    engine: Mutex<Option<tts::Tts>>,
}
impl SpeechOutput for SystemSpeech {
    fn speak(&self, text: &str) -> Result<(), String> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| "Speech output is unavailable")?;
        if engine.is_none() {
            *engine = Some(tts::Tts::default().map_err(|_| {
                "No system speech voice is available. Install a voice in system settings."
            })?);
        }
        engine
            .as_mut()
            .ok_or("Speech output unavailable")?
            .speak(text, true)
            .map_err(|_| "The system could not speak this reply")?;
        Ok(())
    }
    fn stop(&self) -> Result<(), String> {
        if let Some(engine) = self
            .engine
            .lock()
            .map_err(|_| "Speech output unavailable")?
            .as_mut()
        {
            engine.stop().map_err(|_| "Could not stop speech output")?;
        }
        Ok(())
    }
    fn is_speaking(&self) -> bool {
        self.engine
            .lock()
            .ok()
            .and_then(|mut e| e.as_mut().map(|e| e.is_speaking().unwrap_or(false)))
            .unwrap_or(false)
    }
}
impl Drop for SystemSpeech {
    fn drop(&mut self) {
        if let Ok(engine) = self.engine.get_mut() {
            if let Some(engine) = engine.as_mut() {
                let _ = engine.stop();
            }
        }
    }
}
