//! Voice command parsing for DJ control.
//!
//! Translates natural speech into `DjAction` enums. Command parsing logic
//! is unconditional; STT capture itself (if added in future) may be feature-gated.
//!
//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\voice_commands.rs (195 LOC).

use super::{DeckId, DjAction};

/// Parse natural speech into DJ actions.
///
/// Returns an empty vec for unrecognized commands.
pub fn parse_voice_command(text: &str) -> Vec<DjAction> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.is_empty() {
        return Vec::new();
    }

    // Crossfader commands
    if words.contains(&"crossfader") {
        if words.contains(&"left") {
            return vec![DjAction::SetCrossfader(-1.0)];
        } else if words.contains(&"right") {
            return vec![DjAction::SetCrossfader(1.0)];
        } else if words.contains(&"center") || words.contains(&"centre") || words.contains(&"middle") {
            return vec![DjAction::SetCrossfader(0.0)];
        }
        return Vec::new();
    }

    // Loop off (check before loop on)
    if words.contains(&"loop") && words.contains(&"off") {
        if let Some(deck) = extract_deck(&words) {
            return vec![DjAction::DeactivateLoop(deck)];
        }
        return Vec::new();
    }

    // Loop on
    if words.contains(&"loop") {
        if let Some(deck) = extract_deck(&words) {
            return vec![DjAction::ActivateLoop(deck)];
        }
        return Vec::new();
    }

    // Load track: "load <query> on deck X"
    if words.first() == Some(&"load") {
        let deck = extract_deck(&words).unwrap_or(DeckId::C);
        // Extract the query between "load" and "on/deck"
        let query = extract_load_query(&words);
        return vec![DjAction::LoadTrack {
            deck,
            path: query,
        }];
    }

    // Play / go
    if words.contains(&"play") || words.contains(&"go") {
        if let Some(deck) = extract_deck(&words) {
            return vec![DjAction::Play(deck)];
        }
        return Vec::new();
    }

    // Stop / pause / kill
    if words.contains(&"stop") || words.contains(&"pause") || words.contains(&"kill") {
        if let Some(deck) = extract_deck(&words) {
            return vec![DjAction::Stop(deck)];
        }
        return Vec::new();
    }

    // Yield / take / mine
    if words.contains(&"yield") || words.contains(&"take") || words.contains(&"mine") {
        if let Some(deck) = extract_deck(&words) {
            return vec![DjAction::Yield(deck)];
        }
        return Vec::new();
    }

    Vec::new()
}

/// Extract deck reference from words like "deck a", "deck b", etc.
fn extract_deck(words: &[&str]) -> Option<DeckId> {
    for (i, word) in words.iter().enumerate() {
        if *word == "deck" {
            if let Some(next) = words.get(i + 1) {
                match *next {
                    "a" => return Some(DeckId::A),
                    "b" => return Some(DeckId::B),
                    "c" => return Some(DeckId::C),
                    "d" => return Some(DeckId::D),
                    _ => {}
                }
            }
        }
    }
    None
}

/// Extract the query portion from a load command.
/// "load some track on deck a" → "some track"
/// "load some track deck a" → "some track"
/// "load some track" → "some track"
fn extract_load_query(words: &[&str]) -> String {
    let mut end = words.len();

    // Find "on deck X" or "deck X" at the end
    for i in 1..words.len() {
        if words[i] == "deck" {
            // Check if preceded by "on"
            if i > 0 && words[i - 1] == "on" {
                end = i - 1;
            } else {
                end = i;
            }
            break;
        }
    }

    // Skip the "load" keyword at index 0
    let query_words = &words[1..end];
    query_words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_deck_a() {
        let actions = parse_voice_command("play deck a");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::Play(DeckId::A)));
    }

    #[test]
    fn test_stop_deck_b() {
        let actions = parse_voice_command("stop deck b");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::Stop(DeckId::B)));
    }

    #[test]
    fn test_load_track() {
        let actions = parse_voice_command("load my favorite song on deck a");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            DjAction::LoadTrack { deck, path } => {
                assert_eq!(*deck, DeckId::A);
                assert_eq!(path, "my favorite song");
            }
            other => panic!("Expected LoadTrack, got {:?}", other),
        }
    }

    #[test]
    fn test_load_defaults_to_deck_c() {
        let actions = parse_voice_command("load some banger");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            DjAction::LoadTrack { deck, path } => {
                assert_eq!(*deck, DeckId::C);
                assert_eq!(path, "some banger");
            }
            other => panic!("Expected LoadTrack, got {:?}", other),
        }
    }

    #[test]
    fn test_crossfader_left() {
        let actions = parse_voice_command("crossfader left");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::SetCrossfader(v) if (*v - (-1.0)).abs() < f64::EPSILON));
    }

    #[test]
    fn test_crossfader_center() {
        let actions = parse_voice_command("crossfader center");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::SetCrossfader(v) if v.abs() < f64::EPSILON));
    }

    #[test]
    fn test_loop_on() {
        let actions = parse_voice_command("loop deck a");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::ActivateLoop(DeckId::A)));
    }

    #[test]
    fn test_loop_off() {
        let actions = parse_voice_command("loop off deck b");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::DeactivateLoop(DeckId::B)));
    }

    #[test]
    fn test_yield() {
        let actions = parse_voice_command("yield deck c");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::Yield(DeckId::C)));
    }

    #[test]
    fn test_no_match() {
        let actions = parse_voice_command("what is the meaning of life");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_natural_speech() {
        let actions = parse_voice_command("go deck b");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::Play(DeckId::B)));
    }

    #[test]
    fn test_kill_variant() {
        let actions = parse_voice_command("kill deck d");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], DjAction::Stop(DeckId::D)));
    }
}
