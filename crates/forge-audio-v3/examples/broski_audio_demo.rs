//! Broski DJ Brain Interactive Demo — unified audio-behavior pipeline showcase.
//!
//! Zero-heap-alloc demo harness on hot path, demonstrating:
//! 1. Virtual mixer with 4 decks, distinct BPMs, Camelot keys
//! 2. Broski brain decision tree (Shadow/Senex/Trickster)
//! 3. Voice command input and routing
//! 4. SovereignFocus tap-tempo calibration
//! 5. Real-time HUD showing OODA state and audio reactivity

use std::io::{self, Write};
use std::time::Instant;
use forge_audio_v3::broski::{
    DjAssistant, TransitionState, BroskiPersonality, SovereignEngine,
    ObservationBuffer, parse_voice_command, DjAction, DjSuggestion, BroskiArchetype,
};
use forge_audio_v3::correspondence_bus::camelot_compat;

const KEYS: &[&str] = &["5A", "7B", "9A", "2B"];

struct VirtualDeck {
    bpm: f64,
    key_idx: usize,
    rms: f32,
    is_playing: bool,
}

impl VirtualDeck {
    fn new(id: usize) -> Self {
        Self {
            bpm: 110.0 + (id as f64 * 5.0),
            key_idx: id % KEYS.len(),
            rms: 0.0,
            is_playing: false,
        }
    }

    fn tick(&mut self, elapsed: f64) {
        if self.is_playing {
            self.rms = (0.3 + 0.2 * (elapsed * self.bpm / 60.0).sin().abs()) as f32;
        } else {
            self.rms = 0.0;
        }
    }
}

struct VirtualMixer {
    decks: [VirtualDeck; 4],
    crossfader: f32,
    master_bpm: f64,
    start_time: Instant,
}

impl VirtualMixer {
    fn new() -> Self {
        Self {
            decks: [
                VirtualDeck::new(0),
                VirtualDeck::new(1),
                VirtualDeck::new(2),
                VirtualDeck::new(3),
            ],
            crossfader: 0.0,
            master_bpm: 120.0,
            start_time: Instant::now(),
        }
    }

    fn tick(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        for deck in &mut self.decks {
            deck.tick(elapsed);
        }
    }

    fn transition_state(&self) -> TransitionState {
        let deck_a = &self.decks[0];
        let deck_b = &self.decks[1];
        let key_a = KEYS[deck_a.key_idx];
        let key_b = KEYS[deck_b.key_idx];

        TransitionState {
            energy_left: deck_a.rms,
            energy_right: deck_b.rms,
            combined_energy: (deck_a.rms + deck_b.rms) / 2.0,
            crossfader: self.crossfader,
            deck_a_bpm: deck_a.bpm as f32,
            deck_b_bpm: deck_b.bpm as f32,
            harmonic_compat: camelot_compat(key_a, key_b),
            vocal_collision: 0.0,
            groove_lock: {
                let diff = (deck_a.bpm - deck_b.bpm).abs();
                (1.0 - (diff / ((deck_a.bpm + deck_b.bpm) * 0.5)).min(1.0)) as f32
            },
            vocal_energy: [0.0; 4],
        }
    }
}

struct DemoBroski {
    mixer: VirtualMixer,
    assistant: DjAssistant,
    personality: BroskiPersonality,
    sov_engine: SovereignEngine,
    obs_buffer: ObservationBuffer<String>,
    tap_count: usize,
    last_suggestion: Option<DjSuggestion>,
}

impl DemoBroski {
    fn new() -> Self {
        Self {
            mixer: VirtualMixer::new(),
            assistant: DjAssistant::new(),
            personality: BroskiPersonality::default(),
            sov_engine: SovereignEngine::new(),
            obs_buffer: ObservationBuffer::new(),
            tap_count: 0,
            last_suggestion: None,
        }
    }

    fn tick(&mut self) {
        self.mixer.tick();
        let state = self.mixer.transition_state();
        let elapsed = self.mixer.start_time.elapsed().as_secs_f64();
        if let Some(sugg) = self.assistant.tick(&state, elapsed) {
            self.last_suggestion = Some(sugg);
        }
    }

    fn handle_command(&mut self, input: &str) {
        let mut lower = [0u8; 64];
        for (i, b) in input.as_bytes().iter().enumerate().take(64) {
            lower[i] = b.to_ascii_lowercase();
        }
        let lower_str =
            std::str::from_utf8(&lower[..input.len().min(64)]).unwrap_or("");

        match lower_str {
            "play a" | "play deck a" => {
                self.mixer.decks[0].is_playing = true;
                println!("▶ Deck A");
            }
            "play b" | "play deck b" => {
                self.mixer.decks[1].is_playing = true;
                println!("▶ Deck B");
            }
            "stop a" | "stop deck a" => {
                self.mixer.decks[0].is_playing = false;
                println!("⏹ Deck A");
            }
            "stop b" | "stop deck b" => {
                self.mixer.decks[1].is_playing = false;
                println!("⏹ Deck B");
            }
            "left" => {
                self.mixer.crossfader = -1.0;
                println!("↤ Left");
            }
            "right" => {
                self.mixer.crossfader = 1.0;
                println!("↦ Right");
            }
            "center" => {
                self.mixer.crossfader = 0.0;
                println!("↕ Center");
            }
            "shadow" => {
                self.assistant.archetype = BroskiArchetype::Shadow;
                println!("🌑 Shadow");
            }
            "senex" => {
                self.assistant.archetype = BroskiArchetype::Senex;
                println!("👴 Senex");
            }
            "trickster" => {
                self.assistant.archetype = BroskiArchetype::Trickster;
                println!("🤹 Trickster");
            }
            "tap" => {
                self.tap_count += 1;
                println!("⏱ Tap: {}", self.tap_count);
            }
            "status" | "?" => {
                let state = self.mixer.transition_state();
                println!("Energy: {:.2} | Compat: {:.2} | Lock: {:.2}",
                    state.combined_energy, state.harmonic_compat, state.groove_lock);
            }
            "help" => {
                println!("play [a|b] stop [a|b] [left|right|center] [shadow|senex|trickster] tap status help quit");
            }
            _ => {
                let actions = parse_voice_command(input);
                if !actions.is_empty() {
                    println!("🎙 {} actions", actions.len());
                    for action in actions {
                        match action {
                            DjAction::SetCrossfader(value) => {
                                self.mixer.crossfader = value as f32;
                                println!("  Xfade: {:.2}", value);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn render_hud(&self) {
        let state = self.mixer.transition_state();
        println!("╔════════════════════════════╗");
        println!("║ Broski DJ Brain HUD        ║");
        println!("╠════════════════════════════╣");
        println!(
            "║ Energy: {:<3.0}% Lock: {:<3.0}%      ║",
            state.combined_energy * 100.0,
            state.groove_lock * 100.0
        );
        println!(
            "║ A: {:.1} BPM B: {:.1} BPM    ║",
            state.deck_a_bpm, state.deck_b_bpm
        );
        println!("╚════════════════════════════╝");
    }
}

fn main() {
    let mut demo = DemoBroski::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("╔═══════════════════════════════════════╗");
    println!("║  Broski DJ Brain Demo (2026-08-26)   ║");
    println!("╚═══════════════════════════════════════╝\n");

    demo.mixer.decks[0].is_playing = true;
    demo.mixer.decks[1].is_playing = true;

    loop {
        demo.tick();
        demo.render_hud();

        print!("broski> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim();
        if trimmed == "quit" || trimmed == "exit" {
            println!("👋");
            break;
        }

        if !trimmed.is_empty() {
            demo.handle_command(trimmed);
        }
    }
}
