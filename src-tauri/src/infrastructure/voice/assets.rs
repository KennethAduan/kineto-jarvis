use crate::domain::voice::{SpeechModel, SpeechModelInfo};
use futures_util::{future::BoxFuture, StreamExt};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub struct Asset {
    pub name: &'static str,
    pub url: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}
pub struct VerifiedPack(pub PathBuf, pub &'static [Asset]);
impl VerifiedPack {
    pub fn verify(directory: &Path, assets: &[Asset]) -> Result<(), String> {
        for asset in assets {
            let mut file = std::fs::File::open(directory.join(asset.name))
                .map_err(|_| "Download the Voice pack in Voice settings first.")?;
            if file.metadata().map_err(|e| e.to_string())?.len() != asset.bytes {
                return Err("Voice pack is incomplete. Reinstall it in Voice settings.".into());
            }
            let mut hash = Sha256::new();
            let mut buffer = [0; 65536];
            loop {
                let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            if format!("{:x}", hash.finalize()) != asset.sha256 {
                return Err(
                    "Voice pack integrity check failed. Reinstall the voice pack in Voice settings."
                        .into(),
                );
            }
        }
        Ok(())
    }
}
impl SpeechModel for VerifiedPack {
    fn info(&self) -> SpeechModelInfo {
        SpeechModelInfo {
            ready: self.1.iter().all(|asset| {
                self.0
                    .join(asset.name)
                    .metadata()
                    .is_ok_and(|m| m.is_file() && m.len() == asset.bytes)
            }),
            bytes: self.1.iter().map(|asset| asset.bytes).sum(),
        }
    }
    fn download(
        &self,
        cancel: CancellationToken,
        progress: Arc<AtomicU64>,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            std::fs::create_dir_all(&self.0).map_err(|e| e.to_string())?;
            let client = reqwest::Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::limited(5))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(900))
                .build()
                .map_err(|e| e.to_string())?;
            let mut staged = Vec::new();
            let mut total = 0;
            // All files are staged and verified before publishing. Cancellation drops partial files.
            for asset in self.1 {
                let mut file =
                    tempfile::NamedTempFile::new_in(&self.0).map_err(|e| e.to_string())?;
                let response = tokio::select! {
                    _ = cancel.cancelled() => return Err("Voice pack download cancelled".into()),
                    result = client.get(asset.url).send() => result.map_err(|_| "Could not download the voice pack. Check your connection.")?,
                }.error_for_status().map_err(|_| "Voice pack server returned an error")?;
                let mut stream = response.bytes_stream();
                let mut count = 0u64;
                let mut hash = Sha256::new();
                loop {
                    let chunk = tokio::select! {
                        _ = cancel.cancelled() => return Err("Voice pack download cancelled".into()),
                        chunk = stream.next() => chunk,
                    };
                    let Some(chunk) = chunk else {
                        break;
                    };
                    let chunk = chunk.map_err(|_| "Voice pack download was interrupted")?;
                    count += chunk.len() as u64;
                    if count > asset.bytes {
                        return Err("Voice pack exceeded its expected size".into());
                    }
                    hash.update(&chunk);
                    file.write_all(&chunk).map_err(|e| e.to_string())?;
                    progress.store(total + count, Ordering::Relaxed);
                }
                if count != asset.bytes || format!("{:x}", hash.finalize()) != asset.sha256 {
                    return Err("Voice pack checksum did not match. Download discarded.".into());
                }
                file.as_file().sync_all().map_err(|e| e.to_string())?;
                total += count;
                staged.push((asset.name, file));
            }
            if cancel.is_cancelled() {
                return Err("Voice pack download cancelled".into());
            }
            for (name, file) in staged {
                file.persist(self.0.join(name)).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }
}
