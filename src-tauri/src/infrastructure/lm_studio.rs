use crate::domain::{
    models::{Frame, Model, ModelMessage},
    ports::{ModelGateway, ModelStream},
};
use futures_util::{future::BoxFuture, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::time::Duration;

pub const ENDPOINT: &str = "http://127.0.0.1:1234/v1";
pub struct LmStudioGateway {
    client: reqwest::Client,
    endpoint: String,
}
impl LmStudioGateway {
    pub fn local() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: client()?,
            endpoint: ENDPOINT.into(),
        })
    }
    #[cfg(test)]
    pub(crate) fn at(endpoint: &str) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: client()?,
            endpoint: endpoint.into(),
        })
    }
}
impl ModelGateway for LmStudioGateway {
    fn models(&self) -> BoxFuture<'_, Result<Vec<Model>, String>> {
        Box::pin(models_at(&self.client, &self.endpoint))
    }
    fn stream(
        &self,
        model: String,
        messages: Vec<ModelMessage>,
    ) -> BoxFuture<'_, Result<ModelStream, String>> {
        Box::pin(async move {
            let response=tokio::time::timeout(Duration::from_secs(120),self.client.post(format!("{}/chat/completions",self.endpoint))
                .json(&serde_json::json!({"model":model,"messages":messages,"stream":true,"temperature":0.6,"max_tokens":4096})).send())
                .await.map_err(|_|"LM Studio timed out before sending a response")?.map_err(|_|"Connection to LM Studio failed")?;
            if !response.status().is_success() {
                return Err(format!(
                    "LM Studio returned HTTP {}. Check the loaded model and server settings.",
                    response.status()
                ));
            }
            let bytes = response.bytes_stream().boxed();
            let stream = futures_util::stream::try_unfold(
                (bytes, Decoder::default(), VecDeque::new()),
                |(mut bytes, mut decoder, mut pending)| async move {
                    loop {
                        if let Some(frame) = pending.pop_front() {
                            return Ok(Some((frame, (bytes, decoder, pending))));
                        }
                        let chunk = tokio::time::timeout(Duration::from_secs(120), bytes.next())
                            .await
                            .map_err(|_| "No model data received for 120 seconds")?;
                        let Some(chunk) = chunk else { return Ok(None) };
                        let chunk = chunk.map_err(|_| "LM Studio stream disconnected")?;
                        pending.extend(decoder.push(&chunk)?);
                    }
                },
            );
            Ok(stream.boxed() as ModelStream)
        })
    }
}
pub fn client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .build()
}
#[derive(Deserialize)]
struct Models {
    data: Vec<Model>,
}
pub(crate) async fn models_at(
    client: &reqwest::Client,
    endpoint: &str,
) -> Result<Vec<Model>, String> {
    let response = client
        .get(format!("{endpoint}/models"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| "Cannot reach LM Studio. Start its local server on port 1234.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "LM Studio returned HTTP {}. Check server authentication/settings.",
            response.status()
        ));
    }
    let data = response
        .json::<Models>()
        .await
        .map_err(|_| "Invalid model discovery response".to_string())?;
    Ok(data.data)
}

// Accumulate bytes, not strings: a UTF-8 code point or SSE delimiter can cross HTTP chunks.
#[derive(Default)]
pub struct Decoder {
    pending: Vec<u8>,
}
impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Frame>, String> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 2 * 1024 * 1024 {
            return Err("LM Studio stream frame exceeded 2 MiB".into());
        }
        let mut frames = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| "Invalid UTF-8 in model stream")?
                .trim();
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                frames.push(Frame::Done);
                continue;
            }
            let value: Value =
                serde_json::from_str(data).map_err(|_| "Invalid JSON in model stream")?;
            if value.get("error").is_some() {
                return Err("LM Studio reported an inference error. Check its server logs.".into());
            }
            if let Some(delta) = value.pointer("/choices/0/delta") {
                if delta.get("tool_calls").is_some() {
                    return Err(
                        "Model requested tool execution; tools are disabled in this MVP.".into(),
                    );
                }
                if let Some(text) = delta.get("content").and_then(Value::as_str) {
                    frames.push(Frame::Content(text.into()));
                }
                if let Some(text) = delta
                    .get("reasoning_content")
                    .or_else(|| delta.get("reasoning"))
                    .and_then(Value::as_str)
                {
                    frames.push(Frame::Reasoning(text.into()));
                }
            }
        }
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_every_byte_split_utf8_crlf_and_reasoning() {
        let source="data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"hmm\"}}]}\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"Hi 🦀\"}}]}\n\ndata: [DONE]\n\n";
        let mut decoder = Decoder::default();
        let mut output = Vec::new();
        for byte in source.as_bytes() {
            output.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(
            output,
            vec![
                Frame::Reasoning("hmm".into()),
                Frame::Content("Hi 🦀".into()),
                Frame::Done
            ]
        );
    }
    #[test]
    fn rejects_errors_malformed_and_tool_calls() {
        for value in [
            "bad json",
            "{\"error\":\"bad\"}",
            "{\"choices\":[{\"delta\":{\"tool_calls\":[]}}]}",
        ] {
            assert!(Decoder::default()
                .push(format!("data: {value}\n\n").as_bytes())
                .is_err());
        }
    }
}
