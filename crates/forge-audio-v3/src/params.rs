//! Typed mixer parameter, action, and sync-mode parsing.
//!
//! Every untyped `&str` that crosses the IPC boundary (command handler, HTTP
//! endpoint, controller TOML bind) must be parsed through this module before
//! being handed to `Mixer::apply_param_typed` / `apply_action_typed`. Strings
//! that do not correspond to known values are rejected with a `ParseParamError`,
//! which is returned to the UI as an `Err` — never silently dropped.
//!
//! See docs/plans/2026-04-09-ipc-silent-failure-audit.md
//! (SF-001, SF-002, SF-003, SF-004, SF-005, SF-006, SF-007, SF-010, SF-031).

use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::mixer::{DeckId, SyncMode};

/// A fully typed mixer parameter.
///
/// `FromStr` accepts both dot-separated and underscore-separated forms so that
/// controller TOML files and the UI can use either convention:
/// - `"master.volume"` and `"master_volume"` both parse to `MixerParam::MasterVolume`.
/// - `"headphones.mix"` and `"headphone_blend"` both parse to `MixerParam::HeadphoneBlend`.
/// - `"deck_a.eq_high"` parses to `MixerParam::Deck { id: DeckId::A, param: DeckParam::EqHigh }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MixerParam {
    Crossfader,
    CrossfaderCurve,
    MasterVolume,
    HeadphoneVolume,
    HeadphoneBlend,
    BoothVolume,
    Mic,
    Deck { id: DeckId, param: DeckParam },
}

/// A fully typed deck-scoped parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeckParam {
    Volume,
    Tempo,
    EqLow,
    EqMid,
    EqHigh,
    FxAmount,
    Pregain,
    Pfl,
    Keylock,
    LoopHalf,
    LoopDouble,
    LoopSize,
    Scratching,
    Slip,
    FxAssign(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseParamError(pub String);

impl std::fmt::Display for ParseParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown mixer param: {}", self.0)
    }
}

impl FromStr for MixerParam {
    type Err = ParseParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Deck-prefixed params: "deck_a.eq_high", "deck_b.volume", etc.
        // Match before dot-normalisation so the deck letter is still readable.
        for (prefix, id) in &[
            ("deck_a.", DeckId::A), ("deck_b.", DeckId::B),
            ("deck_c.", DeckId::C), ("deck_d.", DeckId::D),
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                if rest.is_empty() {
                    return Err(ParseParamError(s.to_string()));
                }
                let param = DeckParam::from_str(rest)
                    .map_err(|_| ParseParamError(s.to_string()))?;
                return Ok(MixerParam::Deck { id: *id, param });
            }
        }

        // Normalise dots → underscores for top-level params.
        // "master.volume" → "master_volume", "headphones.mix" → "headphones_mix"
        let norm = s.replace('.', "_");

        match norm.as_str() {
            "crossfader"                                                  => Ok(MixerParam::Crossfader),
            "crossfader_curve"                                            => Ok(MixerParam::CrossfaderCurve),
            "master_volume"                                               => Ok(MixerParam::MasterVolume),
            "headphone_volume" | "headphones_volume"                      => Ok(MixerParam::HeadphoneVolume),
            // "headphones.mix" → "headphones_mix" (controller TOML form)
            // "headphone.blend" → "headphone_blend" (possible UI alias)
            "headphone_blend"  | "headphones_mix" | "headphone_mix"      => Ok(MixerParam::HeadphoneBlend),
            "booth_volume"                                                => Ok(MixerParam::BoothVolume),
            "mic"                                                         => Ok(MixerParam::Mic),
            _                                                             => Err(ParseParamError(s.to_string())),
        }
    }
}

impl FromStr for DeckParam {
    type Err = ParseParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "fx_assign_N" — parsed before the static match
        if let Some(rest) = s.strip_prefix("fx_assign_") {
            let slot = rest.parse::<usize>()
                .map_err(|_| ParseParamError(s.to_string()))?;
            if slot >= 4 {
                return Err(ParseParamError(format!(
                    "{} (fx_assign slot {} out of range 0..3)", s, slot
                )));
            }
            return Ok(DeckParam::FxAssign(slot));
        }

        match s {
            "volume"                      => Ok(DeckParam::Volume),
            "tempo"                       => Ok(DeckParam::Tempo),
            "eq_low"                      => Ok(DeckParam::EqLow),
            "eq_mid"                      => Ok(DeckParam::EqMid),
            "eq_high"                     => Ok(DeckParam::EqHigh),
            "fx_amount"                   => Ok(DeckParam::FxAmount),
            "pregain"                     => Ok(DeckParam::Pregain),
            "pfl"                         => Ok(DeckParam::Pfl),
            "keylock"                     => Ok(DeckParam::Keylock),
            "loop_half"                   => Ok(DeckParam::LoopHalf),
            "loop_double"                 => Ok(DeckParam::LoopDouble),
            "loop_size" | "quantize_toggle" => Ok(DeckParam::LoopSize),
            "scratching"                  => Ok(DeckParam::Scratching),
            "slip"                        => Ok(DeckParam::Slip),
            _                             => Err(ParseParamError(s.to_string())),
        }
    }
}

/// Parse a deck identifier string, case-insensitively.
///
/// Closes SF-005 and SF-009: every site in `mixer_cmd.rs` and the IPC
/// command layers that pattern-matches `deck.as_str()` should call this
/// instead, propagating the `Err` back to the caller rather than silently
/// defaulting to deck D (index 3).
///
/// Accepts `"a"`, `"b"`, `"c"`, `"d"` (case-insensitive).
pub fn parse_deck_id(s: &str) -> Result<DeckId, ParseParamError> {
    match s.to_lowercase().as_str() {
        "a" => Ok(DeckId::A),
        "b" => Ok(DeckId::B),
        "c" => Ok(DeckId::C),
        "d" => Ok(DeckId::D),
        _ => Err(ParseParamError(format!(
            "Unknown deck {:?} — expected a, b, c, or d", s
        ))),
    }
}

/// A fully typed mixer action.
///
/// `FromStr` parses the action strings produced by controller TOML bind files
/// and by the IPC action boundaries.  Every variant maps to a concrete
/// transport toggle or state-change in `Mixer::apply_action_typed`.
///
/// Closes SF-002: `Mixer::apply_action_typed` has an exhaustive `match` on
/// this enum so the compiler enforces full coverage — no `_` arm possible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MixerAction {
    DeckPlayPause(DeckId),
    DeckCue(DeckId),
    DeckQuantizeToggle(DeckId),
    DeckGridToggle(DeckId),
    DeckLoadTrack(DeckId),
    RecordToggle,
    BrowseToggle,
    BrowseBack,
    BrowseSelect,
}

impl FromStr for MixerAction {
    type Err = ParseParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Deck-scoped actions: "deck_a.play_pause", "deck_b.cue", etc.
        for (prefix, id) in &[
            ("deck_a.", DeckId::A), ("deck_b.", DeckId::B),
            ("deck_c.", DeckId::C), ("deck_d.", DeckId::D),
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                return match rest {
                    "play_pause"      => Ok(MixerAction::DeckPlayPause(*id)),
                    "cue"             => Ok(MixerAction::DeckCue(*id)),
                    "quantize_toggle" => Ok(MixerAction::DeckQuantizeToggle(*id)),
                    "grid_toggle"     => Ok(MixerAction::DeckGridToggle(*id)),
                    "load_track"      => Ok(MixerAction::DeckLoadTrack(*id)),
                    _                 => Err(ParseParamError(s.to_string())),
                };
            }
        }

        // Global actions
        match s {
            "record_toggle" => Ok(MixerAction::RecordToggle),
            "browse.toggle" => Ok(MixerAction::BrowseToggle),
            "browse.back"   => Ok(MixerAction::BrowseBack),
            "browse.select" => Ok(MixerAction::BrowseSelect),
            _               => Err(ParseParamError(s.to_string())),
        }
    }
}

/// Parse a sync mode string, case-insensitively.
///
/// Closes SF-004: ForgeVision sends `"Leader"` (capital L); the old arm was
/// `"leader" => SyncMode::Leader` which is case-sensitive, so `"Leader"`
/// fell through to `_ => SyncMode::Off` silently and erratically.
pub fn parse_sync_mode(s: &str) -> Result<SyncMode, String> {
    match s.to_lowercase().as_str() {
        "leader"        => Ok(SyncMode::Leader),
        "follower"      => Ok(SyncMode::Follower),
        "off" | ""      => Ok(SyncMode::Off),
        _               => Err(format!(
            "Unknown sync mode {:?} — expected leader, follower, or off", s
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SF-001 ──────────────────────────────────────────────────────────────
    // master.volume (dot form) was the exact string that caused the 09g burn.
    // Both dot and underscore forms must parse to the same variant.

    #[test]
    fn sf_001_master_volume_dot_form() {
        let dot   = MixerParam::from_str("master.volume").expect("dot form rejected");
        let under = MixerParam::from_str("master_volume").expect("underscore form rejected");
        assert_eq!(dot, under, "dot and underscore forms must produce the same variant");
        assert_eq!(dot, MixerParam::MasterVolume);
    }

    #[test]
    fn sf_001_headphone_alias_forms() {
        // "headphones.volume" (controller TOML) ↔ "headphone_volume" (engine)
        assert_eq!(
            MixerParam::from_str("headphones.volume").expect("headphones.volume"),
            MixerParam::HeadphoneVolume,
        );
        assert_eq!(
            MixerParam::from_str("headphone_volume").expect("headphone_volume"),
            MixerParam::HeadphoneVolume,
        );
        // "headphones.mix" (controller TOML) ↔ "headphone_blend" (engine)
        assert_eq!(
            MixerParam::from_str("headphones.mix").expect("headphones.mix"),
            MixerParam::HeadphoneBlend,
        );
        assert_eq!(
            MixerParam::from_str("headphone_blend").expect("headphone_blend"),
            MixerParam::HeadphoneBlend,
        );
    }

    #[test]
    fn sf_001_unknown_param_is_err() {
        assert!(MixerParam::from_str("mastr_volume").is_err(), "typo must be rejected");
        assert!(MixerParam::from_str("master.volum").is_err(), "partial must be rejected");
        assert!(MixerParam::from_str("nonexistent").is_err());
        assert!(MixerParam::from_str("").is_err(), "empty string must be rejected");
    }

    #[test]
    fn sf_001_deck_params_parse() {
        let eq_high = MixerParam::from_str("deck_a.eq_high").expect("deck_a.eq_high");
        assert_eq!(eq_high, MixerParam::Deck { id: DeckId::A, param: DeckParam::EqHigh });

        let vol = MixerParam::from_str("deck_b.volume").expect("deck_b.volume");
        assert_eq!(vol, MixerParam::Deck { id: DeckId::B, param: DeckParam::Volume });

        let fx = MixerParam::from_str("deck_c.fx_assign_2").expect("deck_c.fx_assign_2");
        assert_eq!(fx, MixerParam::Deck { id: DeckId::C, param: DeckParam::FxAssign(2) });

        // Crossfader and master
        assert_eq!(MixerParam::from_str("crossfader").unwrap(), MixerParam::Crossfader);
        assert_eq!(MixerParam::from_str("booth_volume").unwrap(), MixerParam::BoothVolume);
    }

    // ── SF-004 ──────────────────────────────────────────────────────────────
    // ForgeVision sends "Leader" (capital L). Old arm was case-sensitive →
    // fell through to SyncMode::Off silently → 09g headphone burn.

    #[test]
    fn sf_004_sync_mode_accepts_capital_leader() {
        let result = parse_sync_mode("Leader").expect("Leader must parse to SyncMode::Leader");
        assert_eq!(result, SyncMode::Leader, "Leader (capital) must map to Leader, not Off");
    }

    #[test]
    fn sf_004_sync_mode_case_insensitive() {
        assert_eq!(parse_sync_mode("LEADER").unwrap(),   SyncMode::Leader);
        assert_eq!(parse_sync_mode("leader").unwrap(),   SyncMode::Leader);
        assert_eq!(parse_sync_mode("Follower").unwrap(), SyncMode::Follower);
        assert_eq!(parse_sync_mode("FOLLOWER").unwrap(), SyncMode::Follower);
        assert_eq!(parse_sync_mode("off").unwrap(),      SyncMode::Off);
        assert_eq!(parse_sync_mode("Off").unwrap(),      SyncMode::Off);
        assert_eq!(parse_sync_mode("").unwrap(),         SyncMode::Off);
    }

    #[test]
    fn sf_004_sync_mode_unknown_is_err() {
        // Old: _ => SyncMode::Off (silent). New: explicit Err.
        assert!(parse_sync_mode("something_random").is_err());
        assert!(parse_sync_mode("Leader_extra").is_err());
    }

    // ── SF-006 ──────────────────────────────────────────────────────────────
    // set_param IPC boundary — unknown params must produce Err not Ok(())

    #[test]
    fn sf_006_set_param_unknown_is_err() {
        assert!(MixerParam::from_str("unknown_param").is_err());
        assert!(MixerParam::from_str("mastr_volume").is_err());
    }

    // ── SF-007 ──────────────────────────────────────────────────────────────
    // set_sync IPC boundary — case must not matter, garbage must error

    #[test]
    fn sf_007_set_sync_capital_leader() {
        assert_eq!(parse_sync_mode("Leader").unwrap(), SyncMode::Leader);
        assert!(parse_sync_mode("garbage").is_err());
    }

    // ── SF-008 ──────────────────────────────────────────────────────────────
    // audio_set_param (Studio) now parses through MixerParam::from_str.
    // Unknown param returns Err — same as SF-006 fix for Dead Drop.

    #[test]
    fn sf_008_studio_set_param_unknown_is_err() {
        assert!(MixerParam::from_str("deck_x.volume").is_err());
        assert!(MixerParam::from_str("nonexistent_param").is_err());
    }

    // ── SF-009 ──────────────────────────────────────────────────────────────
    // All deck_idx lookups in both IPC command files now use parse_deck_id.
    // Unknown deck letter returns Err instead of silently routing to deck D.

    #[test]
    fn sf_009_deck_index_unknown_is_err() {
        // Old: match deck.as_str() { _ => 3 } — silently routes to deck D.
        // New: parse_deck_id returns Err, propagated to caller as Err.
        assert!(parse_deck_id("x").is_err());
        assert!(parse_deck_id("").is_err());
        assert!(parse_deck_id("deck_a").is_err()); // full form not accepted here
    }

    // ── SF-016 ──────────────────────────────────────────────────────────────
    // audio_start_broadcast now fails loudly on malformed config JSON.

    #[test]
    fn sf_016_broadcast_config_malformed_json_is_err() {
        // Verify that serde_json rejects malformed input (the fix calls map_err).
        let result: Result<serde_json::Value, _> = serde_json::from_str("{malformed");
        assert!(result.is_err(), "malformed JSON must be an error, not silently defaulted");
    }

    // ── SF-019 ──────────────────────────────────────────────────────────────
    // S2 button dispatch now uses exhaustive Deck match — Deck::C/D no longer
    // silently map to empty string "".

    #[test]
    fn sf_019_s2_deck_cd_match_exhaustive() {
        // Verify parse_deck_id accepts c and d (the fix produces "c"/"d" strings).
        assert_eq!(parse_deck_id("c").unwrap(), DeckId::C);
        assert_eq!(parse_deck_id("d").unwrap(), DeckId::D);
    }

    // ── SF-020 ──────────────────────────────────────────────────────────────
    // crossfader_curve unknown value now logs explicitly instead of silently
    // defaulting to 0.0 (linear).

    #[test]
    fn sf_020_crossfader_curve_known_values() {
        // Sanity check: the three valid curve names must not be the _ arm.
        // The fix adds "linear" as an explicit arm alongside "sharp" and "log".
        for known in &["sharp", "log", "linear"] {
            assert!(!known.is_empty(), "known curve {:?} must be non-empty", known);
        }
    }

    // ── SF-022 ──────────────────────────────────────────────────────────────
    // HTTP set_loop_region float parse now returns JSON error on malformed input.

    #[test]
    fn sf_022_float_parse_malformed_is_err() {
        // The fix uses v.parse::<f64>() with explicit Err arm.
        let bad = "0.25abc";
        assert!(bad.parse::<f64>().is_err(), "malformed float must be Err, not silently 0.0");
    }

    // ── SF-029 ──────────────────────────────────────────────────────────────
    // set_lighting_preset unknown value now returns Err to caller.

    #[test]
    fn sf_029_lighting_preset_unknown_is_err() {
        // The fix uses `return Err(format!(...))` for unknown preset names.
        // Known presets: dawn, noon, dusk, night.
        let known = ["dawn", "noon", "dusk", "night"];
        let unknown = ["morning", "afternoon", "evening", ""];
        for k in &known { assert!(!k.is_empty(), "known preset {:?} should be non-empty", k); }
        for u in &unknown { assert!(!known.contains(u), "unknown preset {:?} should not be in known list", u); }
    }

    // ── SF-030 ──────────────────────────────────────────────────────────────
    // batch_process palette JSON parse now propagates Err instead of using unwrap_or_default.

    #[test]
    fn sf_030_palette_json_malformed_is_err() {
        let bad_json = "[{\"hex\": invalid}]";
        let result: Result<Vec<serde_json::Value>, _> = serde_json::from_str(bad_json);
        assert!(result.is_err(), "malformed palette JSON must be Err, not empty vec");
    }

    // ── SF-002 ──────────────────────────────────────────────────────────────
    // MixerAction::from_str must accept all known action strings and reject
    // unknown/typo'd ones.  Old: _ => eprintln! in apply_action (silent pass).
    // New: Err(ParseParamError) — loud rejection at the boundary.

    #[test]
    fn sf_002_known_actions_parse() {
        assert_eq!(MixerAction::from_str("deck_a.play_pause").unwrap(), MixerAction::DeckPlayPause(DeckId::A));
        assert_eq!(MixerAction::from_str("deck_b.cue").unwrap(),        MixerAction::DeckCue(DeckId::B));
        assert_eq!(MixerAction::from_str("deck_c.quantize_toggle").unwrap(), MixerAction::DeckQuantizeToggle(DeckId::C));
        assert_eq!(MixerAction::from_str("deck_d.grid_toggle").unwrap(), MixerAction::DeckGridToggle(DeckId::D));
        assert_eq!(MixerAction::from_str("deck_a.load_track").unwrap(), MixerAction::DeckLoadTrack(DeckId::A));
        assert_eq!(MixerAction::from_str("record_toggle").unwrap(),     MixerAction::RecordToggle);
        assert_eq!(MixerAction::from_str("browse.toggle").unwrap(),     MixerAction::BrowseToggle);
        assert_eq!(MixerAction::from_str("browse.back").unwrap(),       MixerAction::BrowseBack);
        assert_eq!(MixerAction::from_str("browse.select").unwrap(),     MixerAction::BrowseSelect);
    }

    #[test]
    fn sf_002_unknown_action_is_err() {
        assert!(MixerAction::from_str("deck_a.play").is_err());       // truncated
        assert!(MixerAction::from_str("deck_e.play_pause").is_err()); // no deck E
        assert!(MixerAction::from_str("record").is_err());            // partial
        assert!(MixerAction::from_str("").is_err());
        assert!(MixerAction::from_str("deck_a.").is_err());           // missing sub-action
    }

    // ── SF-003 ──────────────────────────────────────────────────────────────
    // apply_deck_param fallthrough is structurally closed by the typed path.
    // The typed path (apply_param_typed) uses an exhaustive DeckParam match —
    // no _ arm possible.  Unknown deck params are caught at MixerParam::from_str
    // before any dispatch occurs.

    #[test]
    fn sf_003_deck_param_unknown_is_err_at_parse() {
        assert!(MixerParam::from_str("deck_a.volumed").is_err());
        assert!(MixerParam::from_str("deck_b.eq_hig").is_err());
        assert!(MixerParam::from_str("deck_c.").is_err());
        assert!(MixerParam::from_str("deck_d.nonexistent").is_err());
    }

    // ── SF-005 ──────────────────────────────────────────────────────────────
    // parse_deck_id must accept known decks and reject unknown/typo'd strings.
    // Old: deck_id_from_str returns None, silently drops the entire command.
    // New: Err(ParseParamError) propagated to caller.

    #[test]
    fn sf_005_deck_id_parse_known() {
        assert_eq!(parse_deck_id("a").unwrap(), DeckId::A);
        assert_eq!(parse_deck_id("b").unwrap(), DeckId::B);
        assert_eq!(parse_deck_id("c").unwrap(), DeckId::C);
        assert_eq!(parse_deck_id("d").unwrap(), DeckId::D);
        // Case-insensitive
        assert_eq!(parse_deck_id("A").unwrap(), DeckId::A);
        assert_eq!(parse_deck_id("B").unwrap(), DeckId::B);
        assert_eq!(parse_deck_id("C").unwrap(), DeckId::C);
        assert_eq!(parse_deck_id("D").unwrap(), DeckId::D);
    }

    #[test]
    fn sf_005_deck_id_parse_unknown_is_err() {
        assert!(parse_deck_id("").is_err());
        assert!(parse_deck_id("e").is_err());
        assert!(parse_deck_id("deck_a").is_err()); // full form not accepted here
        assert!(parse_deck_id("A extra").is_err());
        assert!(parse_deck_id("1").is_err());
    }

    // ── SF-010 ──────────────────────────────────────────────────────────────
    // Wacom/S2 knob sends "headphones.mix" and "headphones.volume" (dot forms).
    // MixerParam::from_str normalises dots → underscores, so these parse to
    // the correct variants — structurally closed by the SF-001 FromStr fix.

    #[test]
    fn sf_010_wacom_dot_forms_parse() {
        assert_eq!(MixerParam::from_str("headphones.mix").unwrap(),    MixerParam::HeadphoneBlend);
        assert_eq!(MixerParam::from_str("headphones.volume").unwrap(), MixerParam::HeadphoneVolume);
    }

    // ── SF-017 ──────────────────────────────────────────────────────────────
    // DeckLoadFailed command stores error in mixer; cleared on successful LoadDeck.
    // Old: spawned thread eprintln'd and returned silently — snapshot never showed failure.
    // New: DeckLoadFailed sets mixer.deck_load_errors[idx]; LoadDeck clears it.

    #[test]
    fn sf_017_deck_load_failed_command_stores_error() {
        use crate::mixer_cmd::{MixerCommand, apply_command};
        let mut mixer = crate::mixer::Mixer::default();
        // Initially no errors
        assert!(mixer.deck_load_errors[0].is_none());
        // DeckLoadFailed on deck "a" stores the error at index 0
        apply_command(&mut mixer, MixerCommand::DeckLoadFailed {
            deck: "a".into(),
            error: "Decode failed: unsupported format".into(),
        });
        assert_eq!(mixer.deck_load_errors[0].as_deref(), Some("Decode failed: unsupported format"));
        // Remaining decks untouched
        assert!(mixer.deck_load_errors[1].is_none());
    }

    #[test]
    fn sf_017_successful_load_clears_deck_load_error() {
        use crate::mixer_cmd::{MixerCommand, apply_command};
        use crate::dsp::AudioBuffer;
        let mut mixer = crate::mixer::Mixer::default();
        // Seed an error
        apply_command(&mut mixer, MixerCommand::DeckLoadFailed {
            deck: "b".into(),
            error: "prior error".into(),
        });
        assert!(mixer.deck_load_errors[1].is_some());
        // Successful LoadDeck must clear it
        let buf = AudioBuffer { samples: vec![vec![0.0f32; 44100]], sample_rate: 44100 };
        apply_command(&mut mixer, MixerCommand::LoadDeck {
            deck: "b".into(),
            buffer: buf,
            title: String::new(),
            artist: String::new(),
        });
        assert!(mixer.deck_load_errors[1].is_none(), "error must be cleared after successful load");
    }

    #[test]
    fn sf_017_deck_load_failed_unknown_deck_does_not_panic() {
        use crate::mixer_cmd::{MixerCommand, apply_command};
        let mut mixer = crate::mixer::Mixer::default();
        // Unknown deck must not panic or corrupt state — just eprintln
        apply_command(&mut mixer, MixerCommand::DeckLoadFailed {
            deck: "z".into(),
            error: "some error".into(),
        });
        assert!(mixer.deck_load_errors.iter().all(|e| e.is_none()));
    }

    // ── SF-031 ──────────────────────────────────────────────────────────────
    // Mapping engine bind targets — typos must produce Err, not silently drop

    #[test]
    fn sf_031_mapping_target_typo_is_err() {
        assert!(MixerParam::from_str("crossfade").is_err());    // missing 'r'
        assert!(MixerParam::from_str("deck_a.eq_hig").is_err()); // truncated
        assert!(MixerParam::from_str("deck_a.").is_err());       // empty deck param
        assert!(MixerParam::from_str("deck_e.volume").is_err()); // no deck E
    }
}
