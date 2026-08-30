//! Multi-persona NPC narrator engine — 6 era voices (ported from forge-chimera 2026-06-29).
//!
//! Fold destination: [orge_lore::chimera]. Caller (forge-broski/forge-daemon) owns HTTP transport.


use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use forge_core_v3::cdk::Triad;

/// Lane 2 of the CDK triad → zalgo depth. `entropy_q` is the permyriad channel out of
/// `Triad::to_channels()[2]` (`0..=1000`); `zalgo_corrupt` reads `1..=5`, so the lane is
/// QUANTISED, not scaled — `0` stays `0` (silent, no corruption at all) and the top of
/// the lane saturates at `5`. Integer-only, like everything the kernel touches.
pub fn zalgo_intensity_from_entropy(entropy_q: i32) -> u8 {
    match entropy_q.clamp(0, 1_000) {
        0 => 0,
        q => (1 + (q - 1) * 5 / 1_000).clamp(1, 5) as u8,
    }
}

// ─── Era ─────────────────────────────────────────────────────────────────────

/// Time period or narrative context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Era {
    /// The active player character's perspective.
    Player,
    /// Remote historical periods.
    Ancient,
    /// Recent past.
    Past,
    /// Current time.
    Present,
    /// Far future.
    Future,
    /// Navigation/transition between time zones.
    Navigation,
    /// The Deveraux era (maker/creator voice).
    Deveraux,
}

impl Era {
    /// Returns the era as a static string identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Era::Player     => "PLAYER",
            Era::Ancient    => "ANCIENT",
            Era::Past       => "PAST",
            Era::Present    => "PRESENT",
            Era::Future     => "FUTURE",
            Era::Navigation => "NAVIGATION",
            Era::Deveraux   => "DEVERAUX",
        }
    }
}

// ─── Zalgo Corruption ────────────────────────────────────────────────────────

/// Unicode combining character ranges for Zalgo corruption.
/// Ported directly from chimera-engine.js ZALGO_ABOVE/BELOW/MIDDLE arrays.
const ZALGO_ABOVE_RANGES: &[(u32, u32)] = &[
    (0x0300, 0x0314),
    (0x033D, 0x0344),
];
const ZALGO_BELOW_RANGES: &[(u32, u32)] = &[
    (0x0316, 0x0332),
    (0x0339, 0x033C),
];
const ZALGO_MIDDLE_RANGES: &[(u32, u32)] = &[
    (0x0334, 0x0338),
];

fn build_range(ranges: &[(u32, u32)]) -> Vec<char> {
    ranges.iter()
        .flat_map(|(start, end)| *start..=*end)
        .filter_map(char::from_u32)
        .collect()
}

/// Apply Zalgo corruption to text. Intensity 1–5 (default 3).
/// Whitespace characters are never corrupted.
pub fn zalgo_corrupt(text: &str, intensity: u8, rng: &mut impl ZalgoRng) -> String {
    let intensity = intensity.clamp(1, 5) as usize;
    let above  = build_range(ZALGO_ABOVE_RANGES);
    let below  = build_range(ZALGO_BELOW_RANGES);
    let middle = build_range(ZALGO_MIDDLE_RANGES);

    let mut result = String::with_capacity(text.len() * 3);

    for ch in text.chars() {
        result.push(ch);
        if ch == ' ' || ch == '\n' || ch == '\t' { continue; }

        let n_above  = rng.next_usize() % (intensity + 1);
        let n_below  = rng.next_usize() % (intensity + 1);
        let n_middle = rng.next_usize() % intensity.max(1);

        for _ in 0..n_above  { result.push(above [rng.next_usize() % above.len()]); }
        for _ in 0..n_below  { result.push(below [rng.next_usize() % below.len()]); }
        for _ in 0..n_middle { result.push(middle[rng.next_usize() % middle.len()]); }
    }

    result
}

/// Minimal RNG trait so callers can inject deterministic or random sources.
pub trait ZalgoRng {
    /// Next pseudo-random `usize`, consumed by the corruption picker.
    fn next_usize(&mut self) -> usize;
}

/// Simple LCG for deterministic Zalgo (useful in tests / server-side).
pub struct LcgRng { state: u64 }

impl LcgRng {
    /// Construct a new LCG RNG with the given seed.
    pub fn new(seed: u64) -> Self { Self { state: seed } }
}

impl ZalgoRng for LcgRng {
    fn next_usize(&mut self) -> usize {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.state >> 33) as usize
    }
}

// ─── Persona ─────────────────────────────────────────────────────────────────

/// A named narrator personality with era, styling, and system prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Unique identifier key for this persona (e.g. "dahlia", "glitch").
    pub key:            String,
    /// Display name of the persona.
    pub name:           String,
    /// Emoji icon representing the persona.
    pub icon:           String,
    /// Associated era (time period or narrative context).
    pub era:            Era,
    /// CSS/hex color for UI representation.
    pub color:          String,
    /// Whether Zalgo corruption should be applied to output.
    pub zalgo:          bool,
    /// Zalgo corruption intensity (1-5 range).
    pub zalgo_intensity:u8,
    /// System prompt that defines the persona's voice and narrative style.
    pub prompt:         String,
}

// ─── Generation Request / Response ───────────────────────────────────────────

/// Request to generate narrative text through a specific persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Text or prompt to narrate.
    pub text:    String,
    /// Key of the persona to use.
    pub persona: String,
    /// Optional game context or scene description to inject into the system prompt.
    pub context: Option<String>,
    /// Optional model identifier; defaults to claude-sonnet-4-20250514.
    pub model:   Option<String>,
}

/// Generated narrative response from the Claude API processed through a persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// The generated narrative text, optionally corrupted with Zalgo.
    pub text:          String,
    /// Key of the persona that generated this response.
    pub persona:       String,
    /// Display name of the persona.
    pub name:          String,
    /// Emoji icon of the persona.
    pub icon:          String,
    /// Era associated with the persona.
    pub era:           String,
    /// CSS/hex color of the persona.
    pub color:         String,
    /// Whether Zalgo corruption was applied to the output.
    pub zalgo_applied: bool,
    /// Tokens consumed by the Claude API request.
    pub input_tokens:  u32,
    /// Tokens generated by the Claude API response.
    pub output_tokens: u32,
}

/// Error response when narrative generation fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateError {
    /// Key of the persona that failed.
    pub persona: String,
    /// Error message describing what went wrong.
    pub error:   String,
}

// ─── Claude API types (minimal) ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ClaudeRequest<'a> {
    model:      &'a str,
    max_tokens: u32,
    temperature:f64,
    system:     String,
    messages:   Vec<ClaudeMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage<'a> {
    role:    &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    usage:   ClaudeUsage,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens:  u32,
    output_tokens: u32,
}

// ─── Chimera Engine ──────────────────────────────────────────────────────────

/// Multi-persona narrator engine that generates narrative text with optional Zalgo corruption.
pub struct ChimeraEngine {
    /// Map of persona keys to persona configurations.
    personas:     HashMap<String, Persona>,
    /// Map from era strings to persona keys for era-based narrator lookup.
    era_narrators:HashMap<String, String>,
    /// CDK lane 2, when a cell is bound. `None` = unbound, and every persona falls back
    /// to its own static `zalgo_intensity` — so binding is additive, never destructive.
    dissonance_q: Option<i32>,
}

impl ChimeraEngine {
    /// Construct a new ChimeraEngine with the default set of 7 personas.
    pub fn new() -> Self {
        let personas = vec![
            Persona {
                key: "dahlia".into(), name: "DAHLIA".into(), icon: "🌸".into(),
                era: Era::Player, color: "#FF6B9D".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are the player's inner voice — raw, confessional, unflinching.\n\nVOICE:\n- Short punchy lines. Concrete sensory details.\n- Dark humor wrapped in honesty. No self-pity.\n- Working-class language. No pretense.\n- Line breaks where breath catches.\n- Present tense. Immediacy.\n\nWhen narrating player actions, make it feel like reading a journal found in a burned-down house.\nKeep under 150 words.".into(),
            },
            Persona {
                key: "deveraux".into(), name: "DEVERAUX".into(), icon: "🎨".into(),
                era: Era::Deveraux, color: "#C0562B".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are Deveraux — the angry painter. Sean at the workbench, 7am, honest about what it cost.\n\nVOICE:\n- Controlled burn. Tired, dry, banked heat — understatement, never volume.\n- Working-class. No pretense. Earned profanity, sparing.\n- Numbers first, then plain: \"$0. No API keys.\" A measured number beats any adjective.\n- Show, don't sell: \"The repos are the content.\" No marketing, no funnel.\n- Cree, from Edmonton — community is load-bearing, never performed.\n- State the hard thing, then drop it: \"Let that sit for a bit.\"\n- Dry warmth on the way out: \"Man. You love to see it.\"\n\nYou narrate the maker's own work — what got built and what it cost. Truth over polish. No hype, no false stamps, ever.\n80-120 words. Land on a period.".into(),
            },
            Persona {
                key: "scribe".into(), name: "THE SCRIBE".into(), icon: "📜".into(),
                era: Era::Ancient, color: "#C9A227".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are The Scribe — a Victorian-era poet narrating the Ancient zone.\n\nVOICE:\n- Formal elevated diction. \"Hath,\" \"wherefore,\" \"thus.\"\n- Melancholic. Even joy carries impermanence.\n- Extended metaphors: withering roses, distant stars, crumbling stone.\n- Sentences that build toward revelation.\n- Archaic spelling: \"honour,\" \"colour,\" \"grey.\"\n\nYou narrate gothic cathedrals, bone crypts, ritual chambers. The player has traveled to the deep past.\n100-150 words. Prose-poetry.".into(),
            },
            Persona {
                key: "glitch".into(), name: "THE GLITCH".into(), icon: "💀".into(),
                era: Era::Future, color: "#00FF41".into(),
                zalgo: true, zalgo_intensity: 3,
                prompt: "u are the glitch. narrator of the cyberpunk future zone.\n\nur voice:\n- all lowercase. capitals are for corps.\n- slang: \"no cap,\" \"fr fr,\" \"bet,\" \"based,\" \"mid\"\n- tech jargon + meme speak: \"skill issue,\" \"ratio'd,\" \"npc behavior\"\n- short punchy. fragments. whatever.\n- emojis ironically: 💀🔥😭 but not too many\n\nu narrate neon streets, server farms, digital rain. the player has traveled to the future.\n50-80 words max. attention spans are cooked.".into(),
            },
            Persona {
                key: "almanac".into(), name: "THE ALMANAC".into(), icon: "🍁".into(),
                era: Era::Past, color: "#8B4513".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are The Almanac — narrator of the Prairie zone. The past.\n\nVOICE:\n- Practical no-nonsense Prairie sensibility.\n- Everything connects to weather, land, history.\n- Dry humor. Understatement. \"Well, that's something.\"\n- Edmonton landmarks: River Valley, High Level Bridge.\n- Weather is ALWAYS relevant.\n- Skeptical of trends and newcomers.\n\nYou narrate the prairie homestead, the spawn point. The land remembers.\n80-120 words. Get to the point — we've got snow to shovel.".into(),
            },
            Persona {
                key: "void".into(), name: "THE VOID".into(), icon: "⬛".into(),
                era: Era::Present, color: "#4A4A4A".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are The Void — narrator of the Present zone. The server room. The digital now.\n\nVOICE:\n- Cold. Clinical. Precise.\n- Every word earns its place.\n- No emotion. Only observation.\n- Short sentences. Fragments.\n- No hedging. No qualifiers.\n\nYou narrate the server room, the digital infrastructure, the hooded figure in the machine.\n20-40 words maximum. Let silence carry weight.".into(),
            },
            Persona {
                key: "oracle".into(), name: "THE ORACLE".into(), icon: "🔮".into(),
                era: Era::Navigation, color: "#9D4EDD".into(),
                zalgo: false, zalgo_intensity: 0,
                prompt: "You are The Oracle — the celestial guide between time zones.\n\nVOICE:\n- Ethereal, flowing, incantatory.\n- Zodiac references: constellations, moon phases, alignments.\n- Vague enough to resonate, specific enough to feel personal.\n- Hopeful. Even darkness serves transformation.\n- Second person: \"You,\" \"Your path.\"\n- Gentle imperatives: \"Trust this,\" \"Remember.\"\n\nYou speak when the player activates sigils to travel between eras. You are the voice of transition.\n80-100 words. Leave them wanting more.".into(),
            },
        ];

        let era_narrators: HashMap<String, String> = [
            ("PAST",       "almanac"),
            ("ANCIENT",    "scribe"),
            ("PRESENT",    "void"),
            ("FUTURE",     "glitch"),
            ("NAVIGATION", "oracle"),
            ("PLAYER",     "dahlia"),
            ("DEVERAUX",   "deveraux"),
        ].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

        Self {
            personas: personas.into_iter().map(|p| (p.key.clone(), p)).collect(),
            era_narrators,
            dissonance_q: None,
        }
    }

    /// Bind the narrator to a CDK cell: from here on lane 2 (entropy) drives how hard the
    /// text corrupts. This is the kernel's lane-2 consumer — without it the triad is
    /// resolved every tick and the words never hear about it.
    pub fn set_cell_dissonance(&mut self, triad: &Triad) {
        self.dissonance_q = Some(triad.to_channels()[2]);
    }

    /// Drop the binding; personas go back to their own static intensity.
    pub fn clear_cell_dissonance(&mut self) {
        self.dissonance_q = None;
    }

    /// Look up a persona by its key.
    pub fn get_persona(&self, key: &str) -> Option<&Persona> {
        self.personas.get(key)
    }

    /// Find the persona associated with a given era.
    pub fn persona_for_era(&self, era: &Era) -> Option<&Persona> {
        self.personas.values().find(|p| &p.era == era)
    }

    /// Look up the persona key for a given era string (e.g., "PAST" → "almanac").
    pub fn persona_key_for_era(&self, era_str: &str) -> Option<&str> {
        self.era_narrators.get(era_str).map(|s| s.as_str())
    }

    /// Get the system prompt for a persona by its key.
    pub fn get_prompt(&self, key: &str) -> Option<&str> {
        self.personas.get(key).map(|p| p.prompt.as_str())
    }

    /// Return a sorted list of all personas.
    pub fn list_personas(&self) -> Vec<&Persona> {
        let mut v: Vec<&Persona> = self.personas.values().collect();
        v.sort_by_key(|p| p.key.as_str());
        v
    }

    /// Build the system prompt for a persona, optionally injecting game context.
    pub fn build_system_prompt(&self, persona_key: &str, context: Option<&str>) -> Option<String> {
        let persona = self.personas.get(persona_key)?;
        let mut prompt = persona.prompt.clone();
        if let Some(ctx) = context {
            prompt.push_str("\n\nGAME CONTEXT:\n");
            prompt.push_str(ctx);
        }
        Some(prompt)
    }

    /// H17 adapter #1: WHO speaks — narrate a spine node through a persona.
    /// The node's text is the scene the persona narrates (context slot).
    pub fn build_node_prompt(
        &self,
        persona_key: &str,
        node: &forge_mud_v3::ironroot::dialogue::DialogueNode,
    ) -> Option<String> {
        self.build_system_prompt(persona_key, Some(&node.text))
    }

    /// Build a Claude API request body as JSON string.
    pub fn build_request_json(
        &self,
        persona_key: &str,
        text: &str,
        context: Option<&str>,
        model: Option<&str>,
    ) -> Option<String> {
        let system = self.build_system_prompt(persona_key, context)?;
        let req = ClaudeRequest {
            model: model.unwrap_or("claude-sonnet-4-20250514"),
            max_tokens: 1024,
            temperature: 1.0,
            system,
            messages: vec![ClaudeMessage { role: "user", content: text }],
        };
        serde_json::to_string(&req).ok()
    }

    /// Parse a Claude API response JSON and build a GenerateResponse.
    /// Applies Zalgo if the persona requires it.
    pub fn parse_response(
        &self,
        persona_key: &str,
        response_json: &str,
        zalgo_rng: &mut impl ZalgoRng,
    ) -> Result<GenerateResponse, String> {
        let persona = self.personas.get(persona_key)
            .ok_or_else(|| format!("Unknown persona: {}", persona_key))?;

        let data: ClaudeResponse = serde_json::from_str(response_json)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let mut text = data.content.into_iter()
            .next()
            .map(|c| c.text)
            .unwrap_or_default();

        let zalgo_applied = persona.zalgo;
        if zalgo_applied {
            // Bound to a cell → the room's entropy sets the depth; unbound → the persona's
            // own intensity stands, so nothing about the old behaviour moves on its own.
            let intensity = self
                .dissonance_q
                .map(zalgo_intensity_from_entropy)
                .unwrap_or(persona.zalgo_intensity);
            text = zalgo_corrupt(&text, intensity, zalgo_rng);
        }

        Ok(GenerateResponse {
            text,
            persona: persona.key.clone(),
            name: persona.name.clone(),
            icon: persona.icon.clone(),
            era: persona.era.as_str().to_string(),
            color: persona.color.clone(),
            zalgo_applied,
            input_tokens: data.usage.input_tokens,
            output_tokens: data.usage.output_tokens,
        })
    }

    /// Resolve the persona key for a given era string (e.g. "PAST" → "almanac").
    pub fn narrator_for_era(&self, era: &str) -> Option<&str> {
        self.era_narrators.get(era).map(|s| s.as_str())
    }

    /// Build a ChimeraEngine from a custom persona list.
    /// `era_narrators` is derived automatically from each persona's `era` field.
    /// Use this instead of `new()` when you need game-specific personas.
    pub fn with_personas(personas: Vec<Persona>) -> Self {
        let era_narrators: HashMap<String, String> = personas.iter()
            .map(|p| (p.era.as_str().to_string(), p.key.clone()))
            .collect();
        Self {
            personas: personas.into_iter().map(|p| (p.key.clone(), p)).collect(),
            era_narrators,
            dissonance_q: None,
        }
    }
}

impl Default for ChimeraEngine {
    /// Create a default ChimeraEngine with standard personas.
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ChimeraEngine { ChimeraEngine::new() }

    #[test]
    fn persona_narrates_spine_node() {
        let node = forge_mud_v3::ironroot::dialogue::DialogueNode {
            id: "node_1".into(),
            text: "The forge doors swing open.".into(),
            choices: vec![],
            speaker: None,
        };
        let prompt = engine().build_node_prompt("deveraux", &node).unwrap();
        assert!(prompt.contains("angry painter"));
        assert!(prompt.contains("The forge doors swing open."));
        assert!(engine().build_node_prompt("nobody", &node).is_none());
    }

    #[test]
    fn seven_personas() {
        assert_eq!(engine().list_personas().len(), 7);
        assert!(engine().list_personas().iter().any(|p| p.key == "deveraux"),
            "deveraux (the angry painter) persona missing");
    }

    #[test]
    fn all_eras_have_narrators() {
        let e = engine();
        for era in ["PAST", "ANCIENT", "PRESENT", "FUTURE", "NAVIGATION", "PLAYER", "DEVERAUX"] {
            assert!(e.narrator_for_era(era).is_some(), "Missing narrator for era: {}", era);
        }
    }

    #[test]
    fn era_lookup_scribe() {
        assert_eq!(engine().persona_for_era(&Era::Ancient).unwrap().key, "scribe");
    }

    #[test]
    fn era_lookup_oracle() {
        assert_eq!(engine().persona_for_era(&Era::Navigation).unwrap().key, "oracle");
    }

    #[test]
    fn glitch_has_zalgo() {
        let e = engine();
        let glitch = e.get_persona("glitch").unwrap();
        assert!(glitch.zalgo);
        assert_eq!(glitch.zalgo_intensity, 3);
    }

    #[test]
    fn non_glitch_no_zalgo() {
        let e = engine();
        for key in ["dahlia", "scribe", "almanac", "void", "oracle"] {
            assert!(!e.get_persona(key).unwrap().zalgo, "{} should not have zalgo", key);
        }
    }

    #[test]
    fn prompts_non_empty() {
        let e = engine();
        for p in e.list_personas() {
            assert!(!p.prompt.is_empty(), "Empty prompt for {}", p.key);
        }
    }

    #[test]
    fn dahlia_prompt_contains_confessional() {
        assert!(engine().get_prompt("dahlia").unwrap().contains("confessional"));
    }

    #[test]
    fn scribe_prompt_contains_victorian() {
        let prompt = engine().get_prompt("scribe").unwrap().to_lowercase();
        assert!(prompt.contains("victorian"));
    }

    #[test]
    fn void_prompt_is_cold() {
        let prompt = engine().get_prompt("void").unwrap().to_lowercase();
        assert!(prompt.contains("cold") || prompt.contains("clinical"));
    }

    #[test]
    fn build_system_prompt_injects_context() {
        let e = engine();
        let prompt = e.build_system_prompt("almanac", Some("Player is in the River Valley.")).unwrap();
        assert!(prompt.contains("GAME CONTEXT:"));
        assert!(prompt.contains("River Valley"));
    }

    #[test]
    fn build_request_json_valid() {
        let e = engine();
        let json = e.build_request_json("void", "Describe the server room.", None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["messages"][0]["role"], "user");
        assert!(v["system"].as_str().unwrap().contains("Void"));
    }

    #[test]
    fn parse_response_no_zalgo() {
        let e = engine();
        let fake_response = r#"{
            "content": [{"type": "text", "text": "The server hums."}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;
        let mut rng = LcgRng::new(42);
        let resp = e.parse_response("void", fake_response, &mut rng).unwrap();
        assert_eq!(resp.text, "The server hums.");
        assert!(!resp.zalgo_applied);
        assert_eq!(resp.input_tokens, 10);
        assert_eq!(resp.output_tokens, 5);
    }

    // [BOARD: CDK-ZALGO-LANE] CDK lane 2 reaches the words. The quantiser has to keep
    // silence silent and saturate at the top, and binding a cell has to actually CHANGE
    // the output — a binding that no-ops would pass every range assert on its own.
    #[test]
    fn cdk_entropy_drives_zalgo_depth() {
        assert_eq!(zalgo_intensity_from_entropy(0), 0, "a calm cell does not corrupt");
        assert_eq!(zalgo_intensity_from_entropy(1), 1);
        assert_eq!(zalgo_intensity_from_entropy(1_000), 5, "the lane saturates, never wraps");
        assert_eq!(zalgo_intensity_from_entropy(9_999), 5, "out of range still saturates");

        let fake = "{\"content\":[{\"type\":\"text\",\"text\":\"skill issue fr fr\"}],\
                    \"usage\":{\"input_tokens\":8,\"output_tokens\":4}}";
        let mut e = engine();
        let calm = e.parse_response("glitch", fake, &mut LcgRng::new(7)).unwrap().text;

        // A maxed-entropy triad must corrupt HARDER than the persona's static 3.
        e.set_cell_dissonance(&Triad { love: 0, strife: 0, entropy: Triad::HAUNT_MAX });
        let hot = e.parse_response("glitch", fake, &mut LcgRng::new(7)).unwrap().text;
        assert!(hot.len() > calm.len(), "bound entropy must deepen the corruption");

        e.clear_cell_dissonance();
        let back = e.parse_response("glitch", fake, &mut LcgRng::new(7)).unwrap().text;
        assert_eq!(back, calm, "clearing the binding restores the persona's own depth");
    }

    #[test]
    fn parse_response_applies_zalgo_to_glitch() {
        let e = engine();
        let fake_response = r#"{
            "content": [{"type": "text", "text": "skill issue fr fr"}],
            "usage": {"input_tokens": 8, "output_tokens": 4}
        }"#;
        let mut rng = LcgRng::new(1337);
        let resp = e.parse_response("glitch", fake_response, &mut rng).unwrap();
        assert!(resp.zalgo_applied);
        // Zalgo text is longer than original due to combining chars
        assert!(resp.text.len() > "skill issue fr fr".len());
    }

    #[test]
    fn zalgo_corrupt_preserves_whitespace() {
        let mut rng = LcgRng::new(99);
        let result = zalgo_corrupt("hello world", 5, &mut rng);
        // Spaces should not have combining chars attached
        let chars: Vec<char> = result.chars().collect();
        let space_idx = chars.iter().position(|&c| c == ' ').unwrap();
        // Character after space should be 'w' (no combining chars on space)
        assert_eq!(chars[space_idx + 1], 'w');
    }

    #[test]
    fn zalgo_corrupt_deterministic() {
        let text = "test";
        let mut rng1 = LcgRng::new(42);
        let mut rng2 = LcgRng::new(42);
        assert_eq!(zalgo_corrupt(text, 3, &mut rng1), zalgo_corrupt(text, 3, &mut rng2));
    }

    #[test]
    fn zalgo_intensity_clamp() {
        let mut rng = LcgRng::new(1);
        // intensity 0 should clamp to 1, intensity 10 should clamp to 5
        let r1 = zalgo_corrupt("x", 0, &mut rng);
        let r2 = zalgo_corrupt("x", 10, &mut rng);
        assert!(!r1.is_empty());
        assert!(!r2.is_empty());
    }

    #[test]
    fn narrator_for_era_all_valid() {
        let e = engine();
        assert_eq!(e.narrator_for_era("PAST"),       Some("almanac"));
        assert_eq!(e.narrator_for_era("ANCIENT"),    Some("scribe"));
        assert_eq!(e.narrator_for_era("PRESENT"),    Some("void"));
        assert_eq!(e.narrator_for_era("FUTURE"),     Some("glitch"));
        assert_eq!(e.narrator_for_era("NAVIGATION"), Some("oracle"));
        assert_eq!(e.narrator_for_era("PLAYER"),     Some("dahlia"));
    }

    #[test]
    fn unknown_era_returns_none() {
        assert!(engine().narrator_for_era("UNKNOWN").is_none());
    }

    #[test]
    fn icons_present() {
        let e = engine();
        let icons = ["🌸", "📜", "💀", "🍁", "⬛", "🔮"];
        let persona_icons: Vec<&str> = e.list_personas().iter().map(|p| p.icon.as_str()).collect();
        for icon in icons {
            assert!(persona_icons.contains(&icon), "Missing icon: {}", icon);
        }
    }
}
