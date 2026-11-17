use super::assets::{Asset, VerifiedPack};
use crate::domain::voice::{SpeechModel, SpeechModelInfo};
use futures_util::future::BoxFuture;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc},
};
use tokio_util::sync::CancellationToken;
const ASSETS: &[Asset] = &[
    Asset {
        name: "model.int8.onnx",
        url: concat!("https://huggingface.co/csukuangfj/kokoro-int8-multi-lang-v1_0/resolve/57adb37fc0e8d4bd430cfc1b8f8c2294945b9af2/", "model.int8.onnx"),
        bytes: 114_202_993,
        sha256: "75635ea3b85a32d00094dcdd61a258aba87c160f7c264c304a7b3ae9d734195e",
    },
    Asset {
        name: "voices.bin",
        url: concat!("https://huggingface.co/csukuangfj/kokoro-int8-multi-lang-v1_0/resolve/57adb37fc0e8d4bd430cfc1b8f8c2294945b9af2/", "voices.bin"),
        bytes: 28_200_960,
        sha256: "1c5a5b983d3d50d8586d437a51f3faa2da7919ce76a013c081e65671a3447c29",
    },
    Asset {
        name: "lexicon-gb-en.txt",
        url: concat!("https://huggingface.co/csukuangfj/kokoro-int8-multi-lang-v1_0/resolve/57adb37fc0e8d4bd430cfc1b8f8c2294945b9af2/", "lexicon-gb-en.txt"),
        bytes: 6_366_635,
        sha256: "c4cbb37316f62210dff52718a7afcaae24f50c032cc75ab47ae67b831d1049e7",
    },
    Asset {
        name: "tokens.txt",
        url: concat!("https://huggingface.co/csukuangfj/kokoro-int8-multi-lang-v1_0/resolve/57adb37fc0e8d4bd430cfc1b8f8c2294945b9af2/", "tokens.txt"),
        bytes: 687,
        sha256: "6ebb6bb288f20f3ae8d004d3c2ca27697da27c037d75e81a60e2a6a663f95425",
    },
    Asset {
        name: "LICENSE",
        url: concat!("https://huggingface.co/csukuangfj/kokoro-int8-multi-lang-v1_0/resolve/57adb37fc0e8d4bd430cfc1b8f8c2294945b9af2/", "LICENSE"),
        bytes: 11_358,
        sha256: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
    },
];
pub struct KokoroModel(pub PathBuf);
impl KokoroModel {
    pub fn verify(directory: &Path) -> Result<(), String> {
        VerifiedPack::verify(directory, ASSETS)
    }
}
impl SpeechModel for KokoroModel {
    fn info(&self) -> SpeechModelInfo {
        VerifiedPack(self.0.clone(), ASSETS).info()
    }
    fn download(
        &self,
        cancel: CancellationToken,
        progress: Arc<AtomicU64>,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            VerifiedPack(self.0.clone(), ASSETS)
                .download(cancel, progress)
                .await
        })
    }
}
