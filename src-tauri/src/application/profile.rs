use crate::domain::profile::Profile;
use std::collections::HashSet;

/// Small personal profiles use bounded lexical retrieval, without an embedding service.
/// Identity is available across chats; only the most relevant profile sections enter the prompt.
pub fn reference(profile: &Profile, query: &str) -> Option<String> {
    if !profile.enabled || (profile.display_name.is_empty() && profile.content.is_empty()) {
        return None;
    }
    let terms: HashSet<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|term| {
            term.len() >= 3
                && !matches!(
                    term.as_str(),
                    "the"
                        | "and"
                        | "you"
                        | "your"
                        | "what"
                        | "who"
                        | "how"
                        | "are"
                        | "can"
                        | "tell"
                        | "about"
                        | "for"
                        | "with"
                        | "that"
                        | "this"
                )
        })
        .collect();
    let mut chunks = Vec::new();
    for paragraph in profile
        .content
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
    {
        let chars: Vec<char> = paragraph.chars().collect();
        for chunk in chars.chunks(900) {
            chunks.push(chunk.iter().collect::<String>());
        }
    }
    let mut ranked: Vec<_> = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let words: HashSet<String> = chunk
                .split(|c: char| !c.is_alphanumeric())
                .map(str::to_lowercase)
                .collect();
            let score = terms.intersection(&words).count();
            (index, score)
        })
        .collect();
    ranked.sort_by_key(|(index, score)| (std::cmp::Reverse(*score), *index));
    // The overview is useful even when a question such as "who am I?" has no keyword match.
    let mut selected = if chunks.is_empty() { vec![] } else { vec![0] };
    for (index, score) in ranked {
        if score > 0 && !selected.contains(&index) && selected.len() < 4 {
            selected.push(index);
        }
    }
    selected.sort_unstable();
    let snippets: Vec<_> = selected.into_iter().map(|index| &chunks[index]).collect();
    Some(
        serde_json::json!({"source":"User-maintained local personal profile",
        "display_name":profile.display_name,"snippets":snippets})
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_and_relevant_sections_are_bounded_and_disabled_profiles_are_absent() {
        let mut profile = Profile { display_name: "Avery Example".into(),
            content: "Engineer based in Example City.\n\nEDUCATION\nExample University, computer science.\n\nCONTACT\navery@example.invalid".into(), enabled: true };
        let context = reference(&profile, "Where did I study education?").unwrap();
        assert!(context.contains("Example University"));
        assert!(context.contains("Avery Example"));
        assert!(!context.contains("avery@example.invalid"));
        assert!(reference(&profile, "Who am I?")
            .unwrap()
            .contains("Engineer"));
        profile.enabled = false;
        assert!(reference(&profile, "education").is_none());
        profile.enabled = true;
        profile.content = "é".repeat(20_000);
        assert!(reference(&profile, "é").unwrap().chars().count() < 4_000);
        profile.content.push('é');
        assert!(profile.validate().is_err());
    }
}
