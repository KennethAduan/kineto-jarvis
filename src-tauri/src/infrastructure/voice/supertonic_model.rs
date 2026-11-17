use super::assets::{Asset, VerifiedPack};
use crate::domain::voice::{SpeechModel, SpeechModelInfo};
use futures_util::future::BoxFuture;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc},
};
use tokio_util::sync::CancellationToken;
const ASSETS: &[Asset] = &[
    Asset { name: "LICENSE", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/LICENSE", bytes: 1070, sha256: "0dfe0d0ba84416fe3879d9a34f4909d8d0137c78d1e95834177b0414ac096fa2" },
    Asset { name: "duration_predictor.int8.onnx", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/duration_predictor.int8.onnx", bytes: 3700147, sha256: "c3eb91414d5ff8a7a239b7fe9e34e7e2bf8a8140d8375ffb14718b1c639325db" },
    Asset { name: "text_encoder.int8.onnx", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/text_encoder.int8.onnx", bytes: 36416150, sha256: "c7befd5ea8c3119769e8a6c1486c4edc6a3bc8365c67621c881bbb774b9902ff" },
    Asset { name: "tts.json", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/tts.json", bytes: 8253, sha256: "42078d3aef1cd43ab43021f3c54f47d2d75ceb4e75f627f118890128b06a0d09" },
    Asset { name: "unicode_indexer.bin", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/unicode_indexer.bin", bytes: 262144, sha256: "8402ca48e5189a8950138580b0fff64db6f072f24ac07cd54ba8b2fbb9883b30" },
    Asset { name: "vector_estimator.int8.onnx", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/vector_estimator.int8.onnx", bytes: 78400833, sha256: "20cd86fa5c6effedfda0e7cffe5b0569ca401c440a0c3a1d72bf39286c0db3fd" },
    Asset { name: "vocoder.int8.onnx", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/vocoder.int8.onnx", bytes: 25991073, sha256: "e923d60f53f95eb1ce235f1dc33ec56d9c057823c96fa6f8acf98f32b0da6152" },
    Asset { name: "voice.bin", url: "https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-3-tts-int8-2026-05-11/resolve/cca5a0e6c96e1d2c720986bf7e75fcc81dee3ae4/voice.bin", bytes: 517168, sha256: "67d5209b0ee8ce6c74105ffbe12fe6a7628aea3b4ba2fcb308a4a67938a93ce8" },
    Asset { name: "MODEL_LICENSE", url: "https://huggingface.co/Supertone/supertonic-3/resolve/3cadd1ee6394adea1bd021217a0e650ede09a323/LICENSE", bytes: 15007, sha256: "0d944a9110fed9a9602d60e0423a272903e7bd21ab060490774efc77c2275e9f" },
];
pub struct SupertonicModel(pub PathBuf);
impl SupertonicModel {
    pub fn verify(directory: &Path) -> Result<(), String> {
        VerifiedPack::verify(directory, ASSETS)
    }
}
impl SpeechModel for SupertonicModel {
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
