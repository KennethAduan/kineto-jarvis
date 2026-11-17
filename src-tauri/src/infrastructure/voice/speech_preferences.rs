use crate::domain::speech::{SpeechPreferences, SpeechPreferencesStore};
use std::{io::Write, path::PathBuf};

pub struct JsonSpeechPreferences(pub PathBuf);
impl SpeechPreferencesStore for JsonSpeechPreferences {
    fn load(&self) -> Result<SpeechPreferences, String> {
        match std::fs::read(&self.0) {
            Ok(bytes) => {
                if bytes.len() > 1024 {
                    return Err("Voice preferences are invalid.".into());
                }
                let preferences: SpeechPreferences =
                    serde_json::from_slice(&bytes).map_err(|_| "Voice preferences are invalid.")?;
                preferences.validate()?;
                Ok(preferences)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SpeechPreferences::default()),
            Err(_) => Err("Could not read voice preferences.".into()),
        }
    }
    fn save(&self, preferences: SpeechPreferences) -> Result<(), String> {
        preferences.validate()?;
        let parent = self.0.parent().ok_or("Invalid preferences location")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temporary
            .write_all(&serde_json::to_vec(&preferences).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary.persist(&self.0).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::speech::SpeechVoice;
    #[test]
    fn saved_voice_survives_reopening_and_rejects_invalid_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        assert_eq!(
            JsonSpeechPreferences(path.clone()).load().unwrap().voice,
            SpeechVoice::George
        );
        let preferences = SpeechPreferences {
            voice: SpeechVoice::Fable,
            speed: 1.1,
            volume: 1.,
        };
        JsonSpeechPreferences(path.clone())
            .save(preferences)
            .unwrap();
        assert_eq!(
            JsonSpeechPreferences(path.clone()).load().unwrap(),
            preferences
        );
        std::fs::write(&path, r#"{"voice":"untrusted-model-path","speed":1}"#).unwrap();
        assert!(JsonSpeechPreferences(path.clone()).load().is_err());
        std::fs::write(&path, r#"{"voice":"bm_george","speed":99}"#).unwrap();
        assert!(JsonSpeechPreferences(path).load().is_err());
    }
}
