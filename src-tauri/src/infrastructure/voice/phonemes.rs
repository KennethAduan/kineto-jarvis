use misaki_rs::{fallback::FallbackError, Fallback, Language, G2P};
use std::{collections::HashMap, path::Path};

struct BritishFallback(HashMap<String, String>);
impl Fallback for BritishFallback {
    fn phonemize(&self, word: &str) -> Result<String, FallbackError> {
        if word.eq_ignore_ascii_case("jarvis") {
            return Ok("ʤˈɑːvɪs".into());
        }
        if let Some(phonemes) = self.0.get(&word.to_lowercase()) {
            return Ok(phonemes.clone());
        }
        // Spell unknown ASCII names/technical identifiers rather than silently dropping them.
        let mut phones = Vec::new();
        const LETTERS: [&str; 26] = [
            "ˈA",
            "bˈiː",
            "sˈiː",
            "dˈiː",
            "ˈiː",
            "ˈɛf",
            "ʤˈiː",
            "ˈAʧ",
            "ˈI",
            "ʤˈA",
            "kˈA",
            "ˈɛl",
            "ˈɛm",
            "ˈɛn",
            "ˈQ",
            "pˈiː",
            "kjˈuː",
            "ˈɑː",
            "ˈɛs",
            "tˈiː",
            "jˈuː",
            "vˈiː",
            "dˈʌbᵊljˌuː",
            "ˈɛks",
            "wˈI",
            "zˈɛd",
        ];
        for character in word.chars() {
            if character.is_ascii_alphabetic() {
                phones.push(LETTERS[(character.to_ascii_lowercase() as u8 - b'a') as usize]);
            } else if character != '-' && character != '\'' {
                return Err(FallbackError::NoPhonemes {
                    word: String::new(),
                });
            }
        }
        if phones.is_empty() {
            return Err(FallbackError::NoPhonemes {
                word: String::new(),
            });
        }
        Ok(phones.join(" "))
    }
}
pub struct BritishPhonemizer(G2P);
impl BritishPhonemizer {
    pub fn load(lexicon: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(lexicon)
            .map_err(|_| "Could not read British pronunciation data")?;
        let words = text
            .lines()
            .filter_map(|line| {
                let mut columns = line.split_whitespace();
                let word = columns.next()?.to_owned();
                Some((word, columns.collect::<String>()))
            })
            .collect();
        Ok(Self(G2P::with_fallback(
            Language::EnglishGB,
            Some(Box::new(BritishFallback(words))),
        )))
    }
    pub fn phonemes(&self, text: &str) -> Result<Vec<i32>, String> {
        let (phonemes, _) = self
            .0
            .g2p(text)
            .map_err(|_| "Could not prepare English pronunciation")?;
        if phonemes.contains('❓') {
            return Err("These Kokoro voices support English. Try the system voice for unsupported words or languages.".into());
        }
        let result: Vec<i32> = phonemes.chars().map(|c| c as i32).collect();
        if result.is_empty() || result.len() > 480 {
            return Err(
                "This speech segment is too complex. Shorten it or choose the system voice.".into(),
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn british_pronunciation_handles_plurals_numbers_and_unknown_ascii_words() {
        let engine = BritishPhonemizer(G2P::with_fallback(
            Language::EnglishGB,
            Some(Box::new(BritishFallback(HashMap::new()))),
        ));
        let phones = engine
            .phonemes("All systems are ready. You have 3 tasks.")
            .unwrap();
        let readable: String = phones
            .iter()
            .filter_map(|p| char::from_u32(*p as u32))
            .collect();
        assert!(
            readable.contains('z'),
            "plural systems must remain speakable"
        );
        assert!(!readable.contains('3'));
        assert!(engine.phonemes("qzxwv").unwrap().len() > 5);
    }
}
