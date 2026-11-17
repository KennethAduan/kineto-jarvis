use super::{
    speech::{SpeechService, SpeechStream},
    voice::prepare_speech,
};
use crate::domain::{models::StreamEvent, ports::EventSink, voice::SpeechOutput};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct Pending {
    markdown: String,
    spoken: usize,
    stream: Option<SpeechStream>,
}
pub struct SpeakingSink<'a> {
    downstream: &'a dyn EventSink,
    speech: SpeechService,
    cancel: CancellationToken,
    pending: Mutex<Pending>,
}
impl<'a> SpeakingSink<'a> {
    pub fn new(
        downstream: &'a dyn EventSink,
        speech: SpeechService,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            downstream,
            speech,
            cancel,
            pending: Mutex::new(Pending {
                markdown: String::new(),
                spoken: 0,
                stream: None,
            }),
        }
    }
    fn enqueue(&self, pending: &mut Pending, markdown: &str) -> Result<(), String> {
        if pending.spoken >= 8_000 || self.cancel.is_cancelled() {
            return Ok(());
        }
        let text: String = prepare_speech(markdown)
            .chars()
            .take(8_000 - pending.spoken)
            .collect();
        if text
            .trim_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .is_empty()
        {
            return Ok(());
        }
        if pending.stream.is_none() {
            pending.stream = Some(self.speech.stream(&self.cancel)?);
        }
        pending.spoken += text.chars().count();
        pending
            .stream
            .as_ref()
            .ok_or("Live speech unavailable")?
            .push(text)
    }
    pub fn finish(&self, success: bool) -> Result<(), String> {
        let mut pending = self.pending.lock().map_err(|_| "Live speech unavailable")?;
        let tail = std::mem::take(&mut pending.markdown);
        if success {
            self.enqueue(&mut pending, &tail)?;
        } else if pending.stream.is_some() {
            self.speech.stop()?;
        }
        pending.stream.take(); // Close the queue; successful playback drains in order.
        Ok(())
    }
}
impl EventSink for SpeakingSink<'_> {
    fn send(&self, event: StreamEvent) -> Result<(), String> {
        let text = match &event {
            StreamEvent::Delta { text } => Some(text.clone()),
            _ => None,
        };
        self.downstream.send(event)?;
        if let Some(text) = text {
            let mut pending = self.pending.lock().map_err(|_| "Live speech unavailable")?;
            if self.cancel.is_cancelled() || pending.spoken >= 8_000 {
                return Ok(());
            }
            pending.markdown.push_str(&text);
            if pending.markdown.len() > 32_768 {
                return Err("Live speech segment exceeded its limit. Try a shorter answer.".into());
            }
            while let Some(boundary) = sentence_boundary(&pending.markdown) {
                let sentence: String = pending.markdown.drain(..boundary).collect();
                self.enqueue(&mut pending, &sentence)?;
            }
        }
        Ok(())
    }
}

/// Only cut in plain Markdown text outside code blocks, inline code, links and images.
/// Keeping the untouched suffix preserves parser context across arbitrary UTF-8 deltas.
fn sentence_boundary(markdown: &str) -> Option<usize> {
    let mut code = false;
    let mut link_depth = 0;
    let (mut scanned, mut brackets, mut parentheses, mut ticks) = (0, 0_i32, 0_i32, 0_u32);
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code = true,
            Event::End(TagEnd::CodeBlock) => code = false,
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => link_depth += 1,
            Event::End(TagEnd::Link | TagEnd::Image) => link_depth -= 1,
            Event::Text(_) if !code && link_depth == 0 => {
                for (offset, c) in markdown[range.clone()].char_indices() {
                    let end = range.start + offset + c.len_utf8();
                    let next = markdown[end..].chars().next();
                    let punctuation =
                        matches!(c, '.' | '?' | '!') && next.is_some_and(char::is_whitespace);
                    let clause = end >= 220 && c.is_whitespace();
                    if punctuation || clause {
                        for delimiter in markdown[scanned..end].chars() {
                            match delimiter {
                                '[' => brackets += 1,
                                ']' => brackets -= 1,
                                '(' => parentheses += 1,
                                ')' => parentheses -= 1,
                                '`' => ticks += 1,
                                _ => {}
                            }
                        }
                        scanned = end;
                        if brackets == 0 && parentheses == 0 && ticks.is_multiple_of(2) {
                            return Some(end);
                        }
                    }
                }
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item)
                if !code
                    && link_depth == 0
                    && range.end < markdown.len()
                    && markdown[..range.end].ends_with('\n') =>
            {
                return Some(range.end);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sentences_start_before_completion_and_unicode_survives_delta_splits() {
        let answer = "Hello there! Here is a café suggestion. A final unfinished clause";
        let mut pending = String::new();
        let mut sentences = Vec::new();
        for c in answer.chars() {
            pending.push(c);
            while let Some(end) = sentence_boundary(&pending) {
                sentences.push(pending.drain(..end).collect::<String>());
            }
        }
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences.join("") + &pending, answer);
        assert_eq!(pending.trim(), "A final unfinished clause");
    }
    #[test]
    fn never_speaks_fenced_code_or_link_destinations_as_sentences() {
        assert_eq!(sentence_boundary("```rust\nsecret. do_not_read. \n"), None);
        let raw = "```rust\nsecret. do_not_read. \n```\nA safe sentence. More text";
        let end = sentence_boundary(raw).unwrap();
        let spoken = prepare_speech(&raw[..end]);
        assert!(!spoken.contains("secret"));
        assert!(spoken.contains("A safe sentence"));
        assert_eq!(
            sentence_boundary("See [this. label](https://private.invalid/"),
            None
        );
        let linked = "See [this label](https://private.invalid/a.b). Next sentence";
        let end = sentence_boundary(linked).unwrap();
        assert!(!prepare_speech(&linked[..end]).contains("private.invalid"));
    }
}
