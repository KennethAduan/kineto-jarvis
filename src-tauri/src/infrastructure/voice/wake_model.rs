use super::assets::{Asset, VerifiedPack};
use crate::domain::voice::{SpeechModel, SpeechModelInfo};
use futures_util::future::BoxFuture;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc},
};
use tokio_util::sync::CancellationToken;
// Mirror payloads are pinned to hashes verified against the original k2-fsa release.
const ASSETS:&[Asset]=&[
    Asset{name:"encoder.onnx",url:"https://huggingface.co/mobilebytesensei/sherpa-onnx-kws-zipformer-gigaspeech-3.3M/resolve/e8ca74a794ce66e38a100e7c60c0214189ff4100/encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",bytes:4_807_159,sha256:"1e721676515bcd42a186979733981213c66c80db680e1cc582dfedf3be76e678"},
    Asset{name:"decoder.onnx",url:"https://huggingface.co/mobilebytesensei/sherpa-onnx-kws-zipformer-gigaspeech-3.3M/resolve/e8ca74a794ce66e38a100e7c60c0214189ff4100/decoder-epoch-12-avg-2-chunk-16-left-64.onnx",bytes:1_063_189,sha256:"f61ebd3eed3773a44d088d53dfae92dbb6aec4839f4dcaee2d402414741663a3"},
    Asset{name:"joiner.onnx",url:"https://huggingface.co/mobilebytesensei/sherpa-onnx-kws-zipformer-gigaspeech-3.3M/resolve/e8ca74a794ce66e38a100e7c60c0214189ff4100/joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx",bytes:163_380,sha256:"eae9da0c7e1e6c6a3f4cc42d167899c388f6c6701b94cb96320e4f55df79624c"},
    Asset{name:"tokens.txt",url:"https://huggingface.co/mobilebytesensei/sherpa-onnx-kws-zipformer-gigaspeech-3.3M/resolve/e8ca74a794ce66e38a100e7c60c0214189ff4100/tokens.txt",bytes:5006,sha256:"fd2ded4050a55d2b1578870ba8697d02371980217806b7558bd0a5cc60f3ba53"},
    Asset{name:"silero_vad.onnx",url:"https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",bytes:643_854,sha256:"9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6"},
];
pub struct WakeModel(pub PathBuf);
impl WakeModel {
    pub fn verify(path: &Path) -> Result<(), String> {
        VerifiedPack::verify(path, ASSETS)
    }
}
impl SpeechModel for WakeModel {
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
