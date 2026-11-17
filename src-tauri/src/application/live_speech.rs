use super::{
    speech::{SpeechService, SpeechStream},
    voice::prepare_speech_fragment,
};
use crate::domain::{models::StreamEvent, ports::EventSink, voice::SpeechOutput};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct Pending {
    markdown: String,
    spoken: usize,
    stream: Option<SpeechStream>,
    disabled: bool,
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
                disabled: false,
            }),
        }
    }
    fn disable_speech(&self, pending: &mut Pending, message: String) -> Result<(), String> {
        pending.disabled = true;
        pending.markdown.clear();
        if let Some(stream) = pending.stream.take() {
            stream.abort();
        }
        self.speech.note_warning(message.clone());
        // Speech is an optional consumer of answer deltas, never a chat transport failure.
        self.downstream.send(StreamEvent::SpeechWarning { message })
    }
    fn enqueue(&self, pending: &mut Pending, markdown: &str) -> Result<(), String> {
        if pending.spoken >= 8_000 || self.cancel.is_cancelled() {
            return Ok(());
        }
        let text: String = prepare_speech_fragment(markdown)
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
        if success && !pending.disabled && !self.cancel.is_cancelled() {
            if let Err(error) = self.enqueue(&mut pending, &tail) {
                let error = self.speech.error().unwrap_or(error);
                self.disable_speech(&mut pending, error)?;
            }
        } else if let Some(stream) = pending.stream.as_ref() {
            stream.abort();
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
            if self.cancel.is_cancelled() || pending.disabled || pending.spoken >= 8_000 {
                return Ok(());
            }
            if pending.stream.is_some() {
                if let Some(error) = self.speech.error() {
                    return self.disable_speech(&mut pending, error);
                }
            }
            pending.markdown.push_str(&text);
            if pending.markdown.len() > 32_768 {
                return self.disable_speech(
                    &mut pending,
                    "This answer is too complex for live speech. The text reply will continue."
                        .into(),
                );
            }
            while let Some(boundary) =
                phrase_boundary(&pending.markdown, if pending.spoken == 0 { 32 } else { 72 })
            {
                let sentence: String = pending.markdown.drain(..boundary).collect();
                if let Err(error) = self.enqueue(&mut pending, &sentence) {
                    let error = self.speech.error().unwrap_or(error);
                    self.disable_speech(&mut pending, error)?;
                    break;
                }
            }
        }
        Ok(())
    }
}

/// Only cut in plain Markdown text outside code blocks, inline code, links and images.
/// Keeping the untouched suffix preserves parser context across arbitrary UTF-8 deltas.
fn phrase_boundary(markdown: &str, target_chars: usize) -> Option<usize> {
    let mut code = false;
    let mut link_depth = 0;
    let mut text_chars = 0;
    let (mut scanned, mut brackets, mut parentheses, mut ticks) = (0, 0_i32, 0_i32, 0_u32);
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code = true,
            Event::End(TagEnd::CodeBlock) => code = false,
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => link_depth += 1,
            Event::End(TagEnd::Link | TagEnd::Image) => link_depth -= 1,
            Event::Text(_) if !code && link_depth == 0 => {
                for (offset, c) in markdown[range.clone()].char_indices() {
                    text_chars += 1;
                    let end = range.start + offset + c.len_utf8();
                    let next = markdown[end..].chars().next();
                    let punctuation =
                        matches!(c, '.' | '?' | '!') && next.is_some_and(char::is_whitespace);
                    let clause = text_chars >= target_chars && c.is_whitespace();
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
    fn first_phrase_is_released_without_sentence_punctuation_or_completion() {
        let words = "I can help you plan the rest of your day and choose what to focus on next";
        let end = phrase_boundary(words, 32).unwrap();
        assert!(end < words.len());
        assert!(words[..end].ends_with(' '));
        assert_eq!(prepare_speech_fragment(&words[..end]), words[..end].trim());
        assert!(!prepare_speech_fragment(&words[..end]).ends_with('.'));
        assert!(phrase_boundary("an unfinished wor", 32).is_none());
        assert!(phrase_boundary("```rust\n", 32).is_none());
    }
    #[test]
    fn sentences_start_before_completion_and_unicode_survives_delta_splits() {
        let answer = "Hello there! Here is a café suggestion. A final unfinished clause";
        let mut pending = String::new();
        let mut sentences = Vec::new();
        for c in answer.chars() {
            pending.push(c);
            while let Some(end) = phrase_boundary(&pending, 220) {
                sentences.push(pending.drain(..end).collect::<String>());
            }
        }
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences.join("") + &pending, answer);
        assert_eq!(pending.trim(), "A final unfinished clause");
    }
    #[test]
    fn never_speaks_fenced_code_or_link_destinations_as_sentences() {
        assert_eq!(
            phrase_boundary("```rust\nsecret. do_not_read. \n", 220),
            None
        );
        let raw = "```rust\nsecret. do_not_read. \n```\nA safe sentence. More text";
        let end = phrase_boundary(raw, 220).unwrap();
        let spoken = prepare_speech_fragment(&raw[..end]);
        assert!(!spoken.contains("secret"));
        assert!(spoken.contains("A safe sentence"));
        assert_eq!(
            phrase_boundary("See [this. label](https://private.invalid/", 220),
            None
        );
        let linked = "See [this label](https://private.invalid/a.b). Next sentence";
        let end = phrase_boundary(linked, 220).unwrap();
        assert!(!prepare_speech_fragment(&linked[..end]).contains("private.invalid"));
    }
}
