use crate::domain::voice::{SpeechModel, SpeechModelInfo};
use futures_util::{future::BoxFuture, StreamExt};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub const MODEL_BYTES: u64 = 147_951_465;
pub const MODEL_SHA256: &str = "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe";
const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
pub struct WhisperModel {
    path: PathBuf,
}
impl WhisperModel {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn verify(path: &Path) -> Result<(), String> {
        let mut file = File::open(path).map_err(|_| "Set up the offline speech model first.")?;
        if file.metadata().map_err(|e| e.to_string())?.len() != MODEL_BYTES {
            return Err("Speech model is incomplete. Reinstall it in Voice settings.".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != MODEL_SHA256 {
            return Err(
                "Speech model failed its integrity check. Reinstall it in Voice settings.".into(),
            );
        }
        Ok(())
    }
}
impl SpeechModel for WhisperModel {
    fn info(&self) -> SpeechModelInfo {
        SpeechModelInfo {
            ready: self
                .path
                .metadata()
                .is_ok_and(|m| m.is_file() && m.len() == MODEL_BYTES),
            bytes: MODEL_BYTES,
        }
    }
    fn download(
        &self,
        cancel: CancellationToken,
        progress: Arc<AtomicU64>,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            let parent = self.path.parent().ok_or("Invalid speech model location")?;
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let mut temporary =
                tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
            // This distinct client downloads only a public model; inference never uses it.
            let client = reqwest::Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::limited(5))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(900))
                .build()
                .map_err(|e| e.to_string())?;
            let response = tokio::select! {
                _=cancel.cancelled()=>return Err("Model download cancelled".into()),
                response=client.get(MODEL_URL).send()=>response.map_err(|_|"Could not download the speech model. Check your connection.")?,
            };
            if !response.status().is_success() {
                return Err(format!(
                    "Speech model download returned HTTP {}",
                    response.status()
                ));
            }
            let mut stream = response.bytes_stream();
            let mut count = 0u64;
            let mut hash = Sha256::new();
            loop {
                let chunk = tokio::select! {_=cancel.cancelled()=>return Err("Model download cancelled".into()),chunk=stream.next()=>chunk};
                let Some(chunk) = chunk else { break };
                let chunk = chunk.map_err(|_| "Speech model download was interrupted")?;
                count += chunk.len() as u64;
                if count > MODEL_BYTES {
                    return Err("Speech model exceeded its expected size".into());
                }
                temporary.write_all(&chunk).map_err(|e| e.to_string())?;
                hash.update(&chunk);
                progress.store(count, Ordering::Relaxed);
            }
            if count != MODEL_BYTES || format!("{:x}", hash.finalize()) != MODEL_SHA256 {
                return Err(
                    "Speech model checksum did not match. The download was discarded.".into(),
                );
            }
            if cancel.is_cancelled() {
                return Err("Model download cancelled".into());
            }
            temporary.as_file().sync_all().map_err(|e| e.to_string())?;
            temporary.persist(&self.path).map_err(|e| e.to_string())?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "downloads the fixed 148 MB public model; no microphone or app data access"]
    async fn downloads_and_verifies_public_model() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        let model = WhisperModel::new(path.clone());
        let progress = Arc::new(AtomicU64::new(0));
        model
            .download(CancellationToken::new(), progress.clone())
            .await
            .unwrap();
        assert_eq!(progress.load(Ordering::Relaxed), MODEL_BYTES);
        assert!(model.info().ready);
        WhisperModel::verify(&path).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn missing_and_truncated_models_are_not_ready() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        let model = WhisperModel::new(path.clone());
        assert!(!model.info().ready);
        assert!(WhisperModel::verify(&path).is_err());
        std::fs::write(&path, b"incomplete model").unwrap();
        assert!(!model.info().ready);
        assert!(WhisperModel::verify(&path).is_err());
    }
}
