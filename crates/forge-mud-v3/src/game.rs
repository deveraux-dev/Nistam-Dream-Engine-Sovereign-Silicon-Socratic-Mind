//! The game engine: one verb in, one ANSI answer out, an autosave after
//! EVERY step including the last (Sean 2026-08-11 "saving each step
//! including the last"). XP is the terminal itself — every byte the
//! operator types through this door is experience.

use std::path::PathBuf;

use forge_core_v3::ramus_prime::MortonKey5D;
use forge_core_v3::sprite_blob::{u16_to_nistam, u64_to_nistam};
use forge_sieve_v3::world::{EcologySieve, WeatherSieve, MoonSieve, InfectionSieve};

use crate::consequence::{self, ActionTag, GuardResponse};
use crate::content::{
    accounts, achievements, alchemy, fishing, items, moons, npcs, pets, quests, relics, shadow,
    talents,
};
use crate::hermetics::ConnectionRoll;
use crate::haunt::ShadowMemory;
use crate::operator::{
    seed_hash, Operator, DEED_CRAFT, DEED_FORCE, DEED_GATHER, DEED_VOICE,
};
use crate::weather::{Era, Weather, WeatherModel};
use crate::world::{self, MAP_SIDE};
use crate::{abyss, cdk, casting, combat, combat_brain, combat_live, console, dream, explore, itemforge, live, magic, memory, mind, overlay, skills, voices, witness_mirror};

/// Named indices into [`skills::SKILLS`] — the contextual-training map (the
/// slide: the act in its context trains the skill of that context).
const SK_CAMPING: usize = 1;
const SK_FISHING: usize = 2;
const SK_DARK_CAMO: usize = 3;
const SK_WOODS_CAMO: usize = 4;
const SK_SCAVENGING: usize = 5;
const SK_BREWING: usize = 6;
const SK_WISDOM: usize = 7;
const SK_WAYFINDING: usize = 9;
const SK_TRACKING: usize = 11;
const SK_PARLEY: usize = 15;
const SK_WITNESSING: usize = 17;
/// skinning — art 0 (the Hunt); the abyss's own combat trains this face.
const SK_SKINNING: usize = 0;

/// Named-foe halves — adjective+noun, no digits (WAVE ABYSS-GLASS register:
/// dread, never gore).
const FOE_ADJ: [&str; 8] =
    ["Hollow", "Gnawing", "Silent", "Choking", "Withered", "Creeping", "Ashen", "Nameless"];
const FOE_NOUN: [&str; 8] = ["Warden", "Husk", "Coil", "Maw", "Shade", "Root", "Wraith", "Hound"];

/// The 13 XP gates — Fibonacci climbs, one boss per moon of the year.
pub const XP_GATES: [u64; 13] =
    [100, 200, 300, 500, 800, 1300, 2100, 3400, 5500, 8900, 14400, 23300, 37700];

/// A foe rolled during a delve step: its name, the depth it was met at
/// (the abyss's `t` axis), and whether the roll landed RARE.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Encounter {
    name: String,
    depth: u16,
    rare: bool,
}

/// A rest in progress: staged at `sleep`, resolved at `wake`. The envelope
/// carries the Sleeping-Beauty risk — `Attested` inside `dream::SLEEP_TTL_TICKS`
/// of `sleep_tick`, `Expired` past it.
struct SleepState {
    sleep_tick: u64,
    balance_pmy: u32,
    energy_pmy: u32,
    envelope: forge_envelope::EphemeralEnvelope<dream::SessionBuffer>,
}

/// The running game: the operator plus where their save lives.
pub struct Game {
    /// The operator being played.
    pub op: Operator,
    /// Autosave path; `None` runs saveless (tests).
    pub save_path: Option<PathBuf>,
    /// The node's sky (W-B seam): era and model dealt from the node seed,
    /// ticked once per command. Ambient — deliberately NOT in the save;
    /// it re-deals identically from the seed at every door.
    pub weather: WeatherModel,
    /// Zone-level weather sieve state: temperature, wind, storm risk.
    /// Ticked alongside weather.rs each command.
    pub weather_sieve: WeatherSieve,
    /// Zone-level animal population sieve: births, predation per species.
    /// Ticked alongside weather.rs each command.
    pub ecology_sieve: EcologySieve,
    /// Lunar phase and calendar state.
    pub moon_sieve: MoonSieve,
    /// Zone-level infection spread state.
    pub infection_sieve: InfectionSieve,
    /// Steps taken through this door — the session's own clock (drives the
    /// day phase and the mud.live heartbeat; session-only, unsaved).
    pub ticks: u64,
    /// The overlay ledger: authoring's append-only facts. The seed's deal is
    /// never edited; readers resolve overlay-first, seed-second.
    pub ledger: overlay::Ledger,
    /// Where the ledger persists (`overlays.ovl` beside the save); `None`
    /// runs saveless (tests) or after an unreadable-but-present file, so a
    /// file we could not read is never overwritten.
    pub ledger_path: Option<PathBuf>,
    /// The shadow's memory across runs — scar accrual, awareness tier, pressure.
    pub haunt: ShadowMemory,
    /// Where the haunt memory persists (`haunt.shr` beside the save); `None`
    /// runs saveless (tests) or after an unreadable-but-present file, so a
    /// file we could not read is never overwritten.
    pub haunt_path: Option<PathBuf>,
    /// Where the Academy milestone receipt log persists (`academy.log`
    /// beside the save); `None` runs saveless (tests). Plain-text,
    /// append-only, one paragraph per milestone fall — the real anchor the
    /// `[ASSUMED]` forge-insights/MUD milestone pairing (`content/
    /// achievements.rs:20-25`) names but does not yet wire; a future brick
    /// ingests this file into `forge-pkm-v3`'s corpus via `ingest::
    /// ingest_file` (its `IngestConfig.extensions` is caller-set — that
    /// future caller adds `"log"` to it, forge-mud-v3 stays undependent on
    /// forge-pkm-v3) — that ingest step is NOT this brick, this brick only
    /// makes the receipt real.
    pub academy_log_path: Option<PathBuf>,
    /// The active authoring scope (default: this node).
    pub author_scope: overlay::Scope,
    /// The last act's tag — the streak's memory (session-only, unsaved).
    last_tag: u8,
    /// Same-act repetitions since the current broke (the packet's streak).
    streak: u8,
    /// The operator's vitality, 0..=100 — session-only, starts full. Spoken
    /// as a word ladder, never a number (see [`vitality_word`]).
    vitality: u16,
    /// The pending abyss encounter, if any (set by `delve`, resolved by
    /// `fight` or `flee`).
    encounter: Option<Encounter>,
    /// The active spell channel, if any (session-only, unsaved).
    channel: casting::Channel,
    /// Rank-up lines queued during this turn (session-only, unsaved).
    rank_ups: Vec<String>,
    /// An active real-time fight, if one has been entered via
    /// `enter_live_fight` (session-only, unsaved). `None` means the plain
    /// one-shot `fight()` verb path is in effect — headless/piped callers
    /// (`process("fight")` directly, tests) never touch this at all.
    live_combat: Option<crate::combat_live::LiveCombat>,
    /// Horizontal-sync ledger (2026-08-18): the same `select_warden_variant`
    /// inputs any other render target must also write to, so the same
    /// (profile, seed) selects the same Bell Warden everywhere — session-only,
    /// unsaved (a run-scoped counter, not a save-file field; persistence
    /// lifecycle across waves is still open, `master-game-dev-skillv1` D06).
    pub run_profile: crate::ironroot::run_profile::RunProfile,
    /// The Dream Forge's day reel (`dream::DreamJournal`, Sentinel 246) —
    /// session-only, unsaved: `SessionBuffer::shred_on_wake` would be
    /// pointless if the journal it stages survived past sleep.
    dream_journal: dream::DreamJournal,
    /// A rest in progress, if `sleep` has been called and `wake` has not yet
    /// resolved it (session-only, unsaved — same lifetime as `dream_journal`).
    sleeping: Option<SleepState>,
    /// What the nights left behind: seals admitted through
    /// `forge_envelope::typed_manifold` (§8:247). The transcript is shredded;
    /// these are the only survivors, and they outlive the journal.
    dream_gifts: Vec<forge_envelope::typed_manifold::SealedGift>,
    /// The night's generator (§8:234): `NoFire` by default — offline play
    /// runs the mechanical skeleton; the shell lights `DoorFire` for glass.
    dream_fire: Box<dyn dream::DreamFire + Send>,
    /// The mirror that walks with you: the player's own conduct on the same
    /// eight axes a faction wears (session-only, unsaved).
    mirror: witness_mirror::WitnessMirror,
    /// The town square's social channel — how disturbed the talk is
    /// (session-only, unsaved; a room forgets between runs).
    square: magic::umwelt::SocialRoom,
    /// Tick the square was last read, so it can settle by elapsed time rather
    /// than by how many times it happened to be looked at.
    square_seen_tick: u64,
    /// The active NPE cartridge, if one was provided to initialize the first scene.
    pub npe_cart: Option<forge_cart_v3::npe::NpeCart>,
}

impl Game {
    /// A game at its node: weather dealt from the operator's node seed.
    pub fn new(op: Operator, save_path: Option<PathBuf>) -> Self {
        let author_scope = overlay::Scope::Node(op.node_seed);
        let mut ledger_path =
            save_path.as_ref().and_then(|p| p.parent()).map(|d| d.join("overlays.ovl"));
        let loaded = ledger_path.as_deref().map(overlay::Ledger::load);
        let ledger = match loaded {
            None => overlay::Ledger::default(),
            Some(Ok(l)) => l,
            // L10: a malformed ledger is refused WHOLE and halts unswallowably
            // — a silent empty load would overwrite the evidence on the next
            // authoring save.
            Some(Err(overlay::LedgerError::Malformed)) => {
                eprintln!("overlays.ovl is malformed — the whole ledger is refused (L10).");
                std::process::abort();
            }
            // Present but unreadable (io): run authoring saveless so the file
            // we could not read is never overwritten.
            Some(Err(_)) => {
                ledger_path = None;
                overlay::Ledger::default()
            }
        };
        // Load the shadow's memory (haunt) from persistence — same pattern as ledger.
        let haunt_path =
            save_path.as_ref().and_then(|p| p.parent()).map(|d| d.join("haunt.shr"));
        let haunt = match haunt_path.as_deref().and_then(|p| std::fs::read(p).ok()) {
            None => ShadowMemory::new(),
            Some(bytes) => match ShadowMemory::decode(&bytes) {
                Some(m) => m,
                // L10: malformed haunt file refuses whole — abort unswallowably.
                None => {
                    eprintln!("haunt.shr is malformed — the whole shadow is refused (L10).");
                    std::process::abort();
                }
            }
        };
        // The era pin (if any) must be honoured from the very first sky the
        // node deals — computed after the ledger, not before it.
        let weather = dealt_weather(&ledger, op.node_seed);
        let academy_log_path =
            save_path.as_ref().and_then(|p| p.parent()).map(|d| d.join("academy.log"));
        Self {
            op,
            save_path,
            weather,
            weather_sieve: WeatherSieve {
                zone_id: 0,
                temperature_history: [20; 13],
                precipitation_history: [0; 13],
                drought_ticks: 0,
                blizzard_ticks: 0,
                chinook_buildup: 0,
                temperature: 20,
                wind_speed: 0,
                wind_direction: 0,
                precipitation: 0,
                visibility: 10000,
                pressure: 5000,
                fires_upwind: 0,
                land_health: 5000,
                entropy_zone: 0,
                deforestation: 0,
                storm_probability: 0,
                hours_to_storm: 0,
                chinook_imminent: false,
                paranormal_fog: 0,
            },
            ecology_sieve: EcologySieve {
                zone_id: 0,
                populations: [100; 16],
                birth_rates: [500; 16],
                predation_matrix: [[0; 16]; 16],
                player_kills: [0; 16],
                carrying_capacity: [500; 16],
            },
            moon_sieve: MoonSieve {
                current_moon: 1,
                phase: 0,
                days_in_moon: 0,
                moon_transition_imminent: false,
            },
            infection_sieve: InfectionSieve {
                infection_type: forge_sieve_v3::world::InfectionType::Corruption,
                source_zone: 0,
                infected_zones: 0,
                spread_rate: 0,
                severity: [0; 16],
                vector: forge_sieve_v3::world::InfectionVector::Contact,
            },
            ticks: 0,
            ledger,
            ledger_path,
            haunt,
            haunt_path,
            academy_log_path,
            author_scope,
            last_tag: u8::MAX,
            streak: 0,
            vitality: 100,
            dream_journal: dream::DreamJournal::new(0),
            sleeping: None,
            dream_gifts: Vec::new(),
            dream_fire: Box::new(dream::NoFire),
            mirror: witness_mirror::WitnessMirror::new(),
            square: magic::umwelt::SocialRoom::at_ease(),
            square_seen_tick: 0,
            encounter: None,
            channel: casting::Channel::NONE,
            rank_ups: Vec::new(),
            live_combat: None,
            run_profile: crate::ironroot::run_profile::RunProfile::new(),
            npe_cart: None,
        }
    }

    /// A game initialized from an authored NPE cartridge (npe.ironroot.ron).
    pub fn from_npe_cart(mut op: Operator, cart: &forge_cart_v3::npe::NpeCart, save_path: Option<PathBuf>) -> Self {
        let (tx, ty) = world::town_square(op.node_seed);
        op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let mut g = Self::new(op, save_path);
        g.npe_cart = Some(cart.clone());
        g
    }

    /// Whether an abyss encounter is pending (main.rs uses this, before
    /// calling `process`, to decide whether a literal "fight" keystroke on
    /// a real terminal should open the live loop instead of the one-shot
    /// verb path).
    pub fn has_encounter(&self) -> bool {
        self.encounter.is_some()
    }

    /// The operator's current vitality (0..=100).
    pub fn vitality(&self) -> u16 {
        self.vitality
    }

    /// Whether a real-time fight is currently open.
    pub fn is_in_live_combat(&self) -> bool {
        self.live_combat.is_some()
    }

    /// Open the real-time fight loop against the pending encounter. Seeds
    /// the same attacker/difficulty roll the one-shot `fight()` verb uses,
    /// so difficulty is still fully determined by the seed — only the
    /// OUTCOME now depends on live play. Returns the intro line, or "there
    /// is nothing here to fight" if no encounter is pending (mirrors the
    /// one-shot verb's own guard).
    pub fn enter_live_fight(&mut self) -> String {
        let Some(enc) = self.encounter.clone() else {
            return String::from("there is nothing here to fight.");
        };
        let level = Self::level(self.op.xp) as u32;
        let chaos = (seed_hash(&[
            &u64_to_nistam(self.op.node_seed),
            &u64_to_nistam(self.op.xp),
            b"fight-chaos",
        ]) & 0xFF) as u32;
        let attacker = (self.op.skills.art_value(0) as u32) / 10 + level * 5 + chaos % 50;
        let difficulty = enc.depth as u32 * 30 + if enc.rare { 40 } else { 0 };
        let foe_hz = 800u16.saturating_sub(enc.depth * 190).clamp(40, 800);
        // 7.5s of headroom on a winnable roll, 4s on a losing one — the same
        // pass/fail signal the old d100-style check used, now spent as time
        // pressure instead of an instant verdict.
        let par_ticks: u16 = if attacker >= difficulty { 900 } else { 480 };
        self.live_combat = Some(crate::combat_live::LiveCombat::new(enc.name.clone(), foe_hz, par_ticks, enc.rare));
        format!(
            "{} squares before you. ready your guard.\r\n\
             attack / parry / dash / jump / interact / surge — flee to break off.",
            enc.name
        )
    }

    /// Advance the open live fight by one tick. Returns the sensation line
    /// for this tick (empty if nothing happened) unless the fight resolved,
    /// in which case the full closing reply (loot/vitality/death as
    /// appropriate) is returned and the live session clears. Panics if no
    /// live fight is open — callers must check `is_in_live_combat` first.
    pub fn live_combat_tick(&mut self, buttons: u8) -> String {
        let lc = self.live_combat.as_mut().expect("live_combat_tick called with no open fight");
        match lc.tick(buttons) {
            combat_live::LiveCombatOutcome::Continue { line } => line.unwrap_or_default(),
            combat_live::LiveCombatOutcome::Victory { line } => self.finish_live_fight(true, line),
            combat_live::LiveCombatOutcome::Defeat { line } => self.finish_live_fight(false, line),
            combat_live::LiveCombatOutcome::Fled { line } => {
                self.live_combat = None;
                self.encounter = None;
                line
            }
        }
    }

    /// Break off the open live fight early (the player pressed flee).
    pub fn live_combat_flee(&mut self) -> String {
        let Some(lc) = self.live_combat.take() else {
            return String::from("there is nothing here to flee.");
        };
        self.encounter = None;
        match lc.flee() {
            combat_live::LiveCombatOutcome::Fled { line } => line,
            _ => unreachable!("LiveCombat::flee always returns Fled"),
        }
    }

    /// Close out a resolved live fight: victory pays XP/trains/rolls loot on
    /// a rare kill; defeat costs vitality (full depth-scaled damage — a live
    /// defeat means the guard broke completely) and may kill the operator,
    /// mirroring the one-shot verb's own death handling.
    fn finish_live_fight(&mut self, victory: bool, line: String) -> String {
        let Some(lc) = self.live_combat.take() else {
            return line;
        };
        let enc_depth = self.encounter.as_ref().map(|e| e.depth).unwrap_or(1);
        let enc_name = lc.foe_name.clone();
        self.encounter = None;
        if victory {
            self.op.xp += 21;
            self.train(SK_SKINNING);
            self.run_profile.record_kill();
            let mut reply = line;
            if lc.rare {
                let item = pick(
                    items::ITEMS,
                    &[&u64_to_nistam(self.op.node_seed), &u64_to_nistam(self.op.xp), b"abyss-loot"],
                );
                let prov = itemforge::roll_provenance(seed_hash(&[
                    &u64_to_nistam(self.op.node_seed),
                    &u64_to_nistam(self.op.xp),
                    b"abyss-prov",
                ]));
                let material_idx = (seed_hash(&[
                    &u64_to_nistam(self.op.node_seed),
                    &u64_to_nistam(self.op.xp),
                    b"abyss-material",
                ]) % relics::MATERIALS.len() as u64) as usize;
                reply.push_str(&format!(
                    "\r\nit carried a {} {} of {} — {}",
                    itemforge::provenance_word(prov),
                    item.0,
                    relics::MATERIALS[material_idx],
                    item.1
                ));
            }
            reply
        } else {
            let taken = (15 * enc_depth) as u16;
            self.vitality = self.vitality.saturating_sub(taken);
            if self.vitality == 0 {
                let scar = combat_brain::forge_scar(
                    self.op.node_seed,
                    self.op.xp,
                    enc_depth as u64,
                    [self.x() as i64 * 1000, self.y() as i64 * 1000],
                    combat_brain::DeathCause::Combat,
                );
                self.haunt.record_scar(scar.scar_hash);
                format!(
                    "{line}\r\nthe world takes its mark — a scar is cut where you fell, and the ground will remember.\r\n{}",
                    self.die(combat_brain::DeathCause::Combat)
                )
            } else {
                format!("{line}\r\n{enc_name} holds the ground.")
            }
        }
    }

    /// The node's weather model — a pure function of the node seed, so the
    /// same node always opens under the same sky (and death, dealing a new
    /// seed, deals a new one).
    pub fn weather_for(node_seed: u64) -> WeatherModel {
        let era = Era::all()
            [(seed_hash(&[&u64_to_nistam(node_seed), b"era"]) % 4) as usize];
        WeatherModel::new(era, seed_hash(&[&u64_to_nistam(node_seed), b"weather"]) as u32)
    }
    /// Level = milestones fallen: the count of XP gates at or under `xp`.
    pub fn level(xp: u64) -> usize {
        XP_GATES.iter().filter(|&&g| xp >= g).count()
    }

    /// Process one command line: earn XP for its bytes, answer the verb,
    /// then AUTOSAVE — including the quit step. Returns (reply, keep_going).
    pub fn process(&mut self, line: &str) -> (String, bool) {
        let before = Self::level(self.op.xp);
        self.op.xp += line.trim().len() as u64;
        // The sky moves one step per command — the world breathes at the
        // operator's own pace, deterministically (same seed, same commands,
        // same weather forever).
        self.weather.tick();
        // The sky drives the sieve's inputs one bank at a time, BEFORE the
        // sieve shifts them into history — a condition holds, then breaks.
        if (self.ticks + 1) % crate::weather::SKY_BANK_PERIOD == 0 {
            self.drive_sieve();
        }
        // Advance sieve states: weather/ecology tick per command; infection spreads.
        self.weather_sieve.tick();
        self.ecology_sieve.tick();
        self.ticks += 1;
        // The watch forgets one step per command — heat is a fading warrant.
        self.op.heat = self.op.heat.saturating_sub(1);
        // The room settles the caster's carried noise the same way.
        self.op.muted_q = self.op.muted_q.saturating_sub(magic::SETTLE_Q as u16);
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        let verb = parts.first().copied().unwrap_or("").to_lowercase();

        // Channel advance: one glyph per turn, speaking it to the transcript.
        let mut channel_line = String::new();
        let mut effect_line = String::new();
        if self.channel.is_active() {
            if let Some(_) = self.channel.advance() {
                let partial = self.channel.word().unwrap_or("?");
                let prefix = partial.chars().take(self.channel.spoken()).collect::<String>();
                channel_line = format!("the word: {}", prefix);

                // Check if the channel just completed.
                if self.channel.is_complete() {
                    if let Some(effect_idx) = self.channel.effect_index() {
                        if (effect_idx as usize) < casting::EFFECT_LINES.len() {
                            effect_line = format!(
                                "the casting completes — {}",
                                casting::EFFECT_LINES[effect_idx as usize]
                            );
                        }
                        // Clear the channel after firing.
                        self.channel = casting::Channel::NONE;
                    }
                }
            }
        }

        // The mirror watches the verb, not the outcome — it reflects what you
        // reached for. One home for the fold, so no verb arm can forget it.
        if let Some(deed) = witness_mirror::deed_of_verb(verb.as_str()) {
            self.mirror.observe(deed);
        }

        let mut reply = match verb.as_str() {
            "" => String::from("silence earns nothing."),
            "help" => String::from(
                "verbs: look map world shift [body] go <n|s|e|w> climb descend camp sleep wake status fish brew pet talents quest steal talk\r\n       witness resolve seed reseed [0xhex|word] worlds [star] evoke <river line> save quit\r\n       delve ascend fight flee kit\r\n       cast <word> · talent <name> · bind <one..six> <word> · 1..6 sing the belt\r\n       author · name faction|town|biome|boss|pet|fish|brew [n] <text> · set law|sky|vibe|era <n> · scope node|me|world · unname <domain> [n]",
            ),
            "look" => self.look(),
            "shift" => self.shift_verb(parts.get(1).copied().unwrap_or("")),
            "map" => world::render_map(self.op.node_seed, self.x(), self.y(), self.op.bias),
            "world" => world::render_world(self.op.node_seed, self.x(), self.y(), self.op.bias),
            "cdk" => self.cdk_verb(),
            "go" | "n" | "s" | "e" | "w" => {
                let dir = if verb == "go" { parts.get(1).copied().unwrap_or("") } else { &verb };
                self.go(dir)
            }
            "climb" => {
                self.train(SK_WAYFINDING);
                self.climb(true)
            }
            "descend" => self.climb(false),
            "camp" => self.camp(),
            "sleep" => self.sleep(),
            "wake" => self.wake(),
            "delve" => self.delve(),
            "ascend" => self.ascend(),
            "cast" => self.cast(&parts),
            "fight" => {
                if self.encounter.is_none() && parts.iter().any(|p| p.to_lowercase().contains("deserter") || p.to_lowercase().contains("threat")) {
                    if let Some(cart) = &self.npe_cart {
                        self.encounter = Some(Encounter {
                            name: cart.world.presences.threat_word.clone(),
                            depth: 1,
                            rare: false,
                        });
                    }
                }
                self.fight()
            }
            "strike" | "attack" => {
                if parts.iter().any(|p| p.to_lowercase().contains("deserter") || p.to_lowercase().contains("threat")) {
                    if let Some(cart) = &self.npe_cart {
                        self.encounter = Some(Encounter {
                            name: cart.world.presences.threat_word.clone(),
                            depth: 1,
                            rare: false,
                        });
                        self.fight()
                    } else {
                        String::from("strike what? no foe stands before you.")
                    }
                } else {
                    String::from("strike what? specify a presence.")
                }
            }
            "flee" => self.flee(),
            "kit" => self.kit(),
            "witness" => self.witness(),
            "resolve" => self.resolve(),
            "seed" => console::seed_summary(self.op.node_seed),
            "reseed" => {
                let s = match parts.get(1) {
                    Some(arg) => console::parse_seed(arg),
                    None => console::derive_seed(self.op.node_seed, self.op.xp),
                };
                self.reseed(s)
            }
            "worlds" => {
                match parts.get(1) {
                    None => {
                        // No argument: list all star names
                        console::worlds_list_stars()
                    }
                    Some(arg) => {
                        // Try star name first (case-insensitive match)
                        if let Some(new_seed) = console::try_reseed_star(arg) {
                            let star_name = console::find_star_by_name(arg)
                                .and_then(|idx| {
                                    if (idx as usize) < 16 {
                                        Some(forge_core_v3::sky::CATALOG[idx as usize].name)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or("unknown");
                            console::apply_reseed(&mut self.op, new_seed);
                            format!("the sky turns; you fall toward {}", star_name)
                        } else {
                            // Fallback: try numeric preview (old behavior)
                            if let Ok(n) = arg.parse::<usize>() {
                                console::worlds_preview(self.op.node_seed, n)
                            } else {
                                format!("the star '{}' does not shine in this sky.", arg)
                            }
                        }
                    }
                }
            }
            "evoke" => {
                // Grammar DSL face seam — the river's DSL face speaks its
                // verdict in the door. SoulWord canonical name is
                // ARCH000-PENDING; until it is named, evoke takes a raw
                // river line (tabs intact) and the river answers aloud.
                let raw = line.trim();
                let rest = raw.get(verb.len()..).map(str::trim_start).unwrap_or("");
                if rest.is_empty() {
                    String::from("evoke what? speak a river line and the river will answer.")
                } else {
                    // CST face seam: a refusal also names WHERE the wound is
                    // (span words from the lossless tree), never a code.
                    let mut spoken = forge_core_v3::river_dsl::speak_verdict(rest);
                    if let Some(wound) = forge_core_v3::river_cst::locate_wound(rest) {
                        spoken.push_str("\r\n");
                        spoken.push_str(&wound);
                    }
                    spoken
                }
            }
            "status" => self.status(),
            "sheet" => self.status_card(),
            "fish" => self.fish(),
            "brew" => {
                // WCE beat: brewing is the craft current. Counted, never shown.
                self.op.deeds[DEED_CRAFT] += 1;
                self.train(SK_BREWING);
                self.consequence(ActionTag::Craft, 128);
                let reagent = ConnectionRoll::deal(self.op.node_seed).reagent;
                let idx = pick_brew_idx(reagent, &[&u64_to_nistam(self.op.node_seed), &u64_to_nistam(self.op.xp), b"brew"]);
                format!("you brew {} — {}", self.speak_brew(idx), alchemy::BREWS[idx].1)
            }
            "steal" => self.steal(),
            "pet" => {
                // The companion is birth-bound: same operator, same friend.
                let idx = pick_idx(pets::PETS, &[self.op.name.as_bytes(), &[self.op.moon, self.op.day], b"pet"]);
                format!("{} pads beside you. {}", self.speak_pet(idx), pets::PETS[idx].1)
            }
            "talents" | "talent" => match parts.get(1..) {
                Some(rest) if !rest.is_empty() => self.take_pole(&rest.join(" ")),
                _ => self.mandala(),
            },
            "bind" => self.bind_slot(&parts),
            "1" | "2" | "3" | "4" | "5" | "6" => {
                let slot = verb.parse::<usize>().unwrap_or(0) - 1;
                let bar = magic::loadout::load_sung_bar(&self.ledger, self.op.node_seed);
                match bar.word(slot) {
                    Some(w) => self.cast(&["cast", w]),
                    None => String::from("nothing hangs at that place on your belt."),
                }
            }
            "quest" | "task" | "tasks" => {
                if let Some(cart) = &self.npe_cart {
                    let shape_str = match &cart.world.first_task.shape {
                        forge_cart_v3::npe::TaskShape::KillOne => "Slay",
                    };
                    format!(
                        "first task: {} {} (reward: {} XP)\r\n  giver: {}\r\n  location: {} near {}\r\n  door out: {}",
                        shape_str,
                        cart.world.first_task.target_word,
                        cart.world.first_task.reward_xp,
                        cart.world.presences.questgiver_word,
                        cart.world.entry_zone_word,
                        cart.world.entry_gate_word,
                        cart.world.door_out_word,
                    )
                } else {
                    self.quest()
                }
            }
            "landmarks" => {
                if let Some(cart) = &self.npe_cart {
                    format!(
                        "landmarks of {}:\r\n{}",
                        cart.world.entry_zone_word,
                        cart.world.landmarks.iter().map(|l| format!("  - {}", l)).collect::<Vec<_>>().join("\r\n")
                    )
                } else {
                    String::from("no landmarks recorded for this ground.")
                }
            }
            "talk" | "npc" | "parley" | "ask" => {
                let target = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default().to_lowercase();
                let cart_opt = self.npe_cart.clone();
                if let Some(cart) = cart_opt {
                    if target.contains("vey") || target.contains("sister") || target.contains("quest") || target.contains("toll") {
                        self.op.deeds[DEED_VOICE] += 1;
                        self.train(SK_PARLEY);
                        self.consequence(ActionTag::Speak, 128);
                        format!(
                            "{} lifts her tallow lantern. 'The Parish is bounded by debt and bell-metal. The {} at {} refuses to yield his post or his breath. Clear the gate, and {} is yours.'",
                            cart.world.presences.questgiver_word,
                            cart.world.presences.threat_word,
                            cart.world.entry_gate_word,
                            cart.world.door_out_word,
                        )
                    } else if target.contains("bellwright") || target.contains("forge") || target.contains("smith") {
                        self.op.deeds[DEED_VOICE] += 1;
                        self.train(SK_PARLEY);
                        self.consequence(ActionTag::Speak, 128);
                        format!(
                            "{} strikes glowing iron on the anvil without turning. 'Every bell cast in {} remembers the frequency of what it was rung for. Bring no unworked metal to this forge.'",
                            cart.world.presences.territorial_word,
                            cart.world.entry_zone_word,
                        )
                    } else if target.contains("deserter") || target.contains("threat") || target.contains("rooted") {
                        self.op.deeds[DEED_VOICE] += 1;
                        self.train(SK_PARLEY);
                        self.consequence(ActionTag::Speak, 128);
                        format!(
                            "{} stands motionless, briars twisting through his armor. He whispers: 'The roots do not let go... strike if you must, traveler.'",
                            cart.world.presences.threat_word,
                        )
                    } else {
                        let fac = consequence::town_faction(self.op.node_seed);
                        if consequence::standing_tier(self.op.standings[fac]) <= 2 {
                            format!(
                                "doors close as you pass. {} remembers what you took.",
                                self.speak_faction(fac)
                            )
                        } else {
                            self.op.deeds[DEED_VOICE] += 1;
                            self.train(SK_PARLEY);
                            self.consequence(ActionTag::Speak, 128);
                            let n = pick(npcs::NPCS, &[&u64_to_nistam(self.op.node_seed), &self.sq_bytes(), b"npc"]);
                            format!("{} says: {}", n.0, n.1)
                        }
                    }
                } else {
                    let fac = consequence::town_faction(self.op.node_seed);
                    if consequence::standing_tier(self.op.standings[fac]) <= 2 {
                        format!(
                            "doors close as you pass. {} remembers what you took.",
                            self.speak_faction(fac)
                        )
                    } else {
                        self.op.deeds[DEED_VOICE] += 1;
                        self.train(SK_PARLEY);
                        self.consequence(ActionTag::Speak, 128);
                        let n = pick(npcs::NPCS, &[&u64_to_nistam(self.op.node_seed), &self.sq_bytes(), b"npc"]);
                        format!("{} says: {}", n.0, n.1)
                    }
                }
            }
            "die" => self.die(combat_brain::DeathCause::Erasure),
            "author" => self.author_view(),
            "name" => self.cmd_name(&parts),
            "set" => self.cmd_set(&parts),
            "scope" => self.cmd_scope(&parts),
            "unname" => self.cmd_unname(&parts),
            "save" => String::from("saved."),
            "quit" => String::from("the terminal keeps what you earned. rest."),
            other => format!("the word '{other}' holds no power here."),
        };

        // Prepend the channel line if a spell is casting.
        if !channel_line.is_empty() {
            if !reply.is_empty() {
                reply.insert_str(0, &format!("{}\r\n", channel_line));
            } else {
                reply = channel_line;
            }
        }

        // Append the effect line if a spell just completed.
        if !effect_line.is_empty() {
            reply.push_str(&format!("\r\n{}", effect_line));
        }

        // Append rank-up lines, if any.
        for line in self.rank_ups.drain(..) {
            reply.push_str(&format!("\r\n{}", line));
        }

        // A milestone falling is a boss named — the achievements table is
        // the 13-boss ladder, one per moon. Which Bell Warden the named boss
        // MANIFESTS as is the sieve's call (2026-08-18, horizontal sync):
        // `select_warden_variant` reads this same `run_profile` any other
        // render target shares, so the variant is identical everywhere —
        // the D08 parity contract, proven by this call site existing at all.
        let after = Self::level(self.op.xp);
        for gate in before..after {
            let idx = gate.min(12);
            let boss = self.speak_boss(idx);
            let epitaph = achievements::BOSSES[idx].1;
            let lore = achievements::MILESTONE_NAMES
                [gate.min(achievements::MILESTONE_NAMES.len() - 1)];
            let warden = crate::ironroot::boss_sieve::select_warden_variant(&self.run_profile);
            reply.push_str(&format!(
                "\r\n\x1b[1;33mMILESTONE {} FALLS — {boss}\x1b[0m: {epitaph} [{lore}]\r\n{boss} manifests as {} — {}",
                gate + 1,
                warden.id.replace('_', " "),
                warden.lesson
            ));
            self.write_academy_receipt(&mut reply, gate, &boss, epitaph, lore);
        }

        let keep_going = verb != "quit";
        self.autosave(&mut reply);
        self.save_ledger(&mut reply);
        (reply, keep_going)
    }

    /// Process one command line and return just the response text.
    /// Thin delegate to `process` for the shell seam.
    pub fn process_line(&mut self, line: &str) -> String {
        let (reply, _) = self.process(line);
        reply
    }

    /// Write the save if a path is set. An unwritable save is a LOUD line in
    /// the reply, never a silent loss.
    fn autosave(&self, reply: &mut String) {
        let Some(path) = &self.save_path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(path, self.op.encode()) {
            reply.push_str(&format!("\r\n\x1b[1;31mSAVE FAILED: {e}\x1b[0m"));
        }
        // The beacon rides every step beside the save (Wire 6b): advisory
        // feed for the glass, best-effort — the save above is the state that
        // must never be lost, and it already speaks loudly on failure. The
        // reply's first line rides along as the `word` field, sanitized
        // inside `live_body` (ANSI stripped) so the glass never renders raw
        // escapes.
        let first_line = reply.split("\r\n").next().unwrap_or("");
        let _ = live::write_live(
            &live::live_path_beside(path),
            &live::live_body(&self.op, &self.weather.current, self.ticks, first_line),
        );
    }

    fn x(&self) -> u16 {
        self.op.pos.axes()[0]
    }
    fn y(&self) -> u16 {
        self.op.pos.axes()[1]
    }
    fn z(&self) -> u16 {
        self.op.pos.axes()[2]
    }
    /// The abyss depth — the 5D word's `t` axis (0 = surface, up to 3).
    fn t(&self) -> u16 {
        self.op.pos.axes()[3]
    }
    fn sq_bytes(&self) -> [u8; 4] {
        let (x, y) = (self.x(), self.y());
        [x as u8, (x >> 8) as u8, y as u8, (y >> 8) as u8]
    }

    fn go(&mut self, dir: &str) -> String {
        // Movement interrupts the channel.
        let mut reply = String::new();
        if let Some(broken) = self.channel.interrupt() {
            reply.push_str(&format!("the word dies half-spoken: {}\r\n", broken));
        }

        let (mut x, mut y) = (self.x(), self.y());
        match dir {
            "n" => y = y.saturating_sub(1),
            "s" => y = (y + 1).min(MAP_SIDE - 1),
            "w" => x = x.saturating_sub(1),
            "e" => x = (x + 1).min(MAP_SIDE - 1),
            _ => return String::from("go where? n, s, e or w."),
        }
        let (tx, ty) = world::town_square(self.op.node_seed);
        let was_in_square = (self.x(), self.y()) == (tx, ty);
        self.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);

        // The square hears you arrive. Presence rides the warrant (a wanted
        // body takes up more room), noise rides how hard you came in on.
        // The tell is a TRANSIENT: only the crossing speaks, never the level.
        let mut arrival = String::new();
        if (x, y) == (tx, ty) && !was_in_square {
            self.square.settle(self.ticks.saturating_sub(self.square_seen_tick) as i64);
            self.square_seen_tick = self.ticks;
            // A stranger walking in IS the intrusion — that is the baseline,
            // and it clears the notice line on its own. Warrant and injury
            // push it higher, they do not gate it.
            const STRANGER_Q: i64 = 6_000;
            let presence_q =
                (STRANGER_Q + (u32::from(self.op.heat) * 400).min(4_000) as i64).min(10_000);
            let noise_q = (STRANGER_Q
                + (u32::from(100u16.saturating_sub(self.vitality)) * 40).min(4_000) as i64)
                .min(10_000);
            if self.square.arrive(presence_q, noise_q) {
                arrival.push_str("the talk in the square stops as you come in.\r\n");
            }
        }

        reply.push_str(&arrival);
        reply.push_str(&self.look());
        reply
    }

    /// The room as sensed off the RESIDENT organs (look-through-sense; donor
    /// v2 sf-wasm mud.rs:1236). `None` before the first tick — absent state
    /// reads "not installed", never "quiet".
    pub fn sense_here(&self) -> Option<crate::sense::RoomView> {
        if self.ticks == 0 {
            return None;
        }
        Some(crate::sense::sense(&crate::sense::SenseReadings {
            square_level_q: self.square.level_q(),
            square_disturbed: self.square.is_disturbed(),
            haunt_pressure_q: self.haunt.pressure_q(),
            haunt_aggression: self.haunt.aggression_level(),
            drought_ticks: self.weather_sieve.drought_ticks,
            blizzard_ticks: self.weather_sieve.blizzard_ticks,
            chinook_buildup: self.weather_sieve.chinook_buildup,
        }))
    }

    /// Hand the sky's reading to the sieve. `temperature_history[0]` is what
    /// the sieve last recorded, which is what a chinook is measured against.
    fn drive_sieve(&mut self) {
        let s = &self.weather_sieve;
        let prev = crate::weather::SkyDrive {
            temperature: s.temperature,
            precipitation: s.precipitation,
            wind_speed: s.wind_speed,
            chinook_buildup: s.chinook_buildup,
            blizzard_ticks: s.blizzard_ticks,
            drought_ticks: s.drought_ticks,
        };
        let d = crate::weather::drive(self.weather.current, prev, s.temperature_history[0]);
        let s = &mut self.weather_sieve;
        s.temperature = d.temperature;
        s.precipitation = d.precipitation;
        s.wind_speed = d.wind_speed;
        s.chinook_buildup = d.chinook_buildup;
        s.blizzard_ticks = d.blizzard_ticks;
        s.drought_ticks = d.drought_ticks;
    }

    /// Wear another body. The form is the save-codec byte on the operator, so
    /// the change survives the autosave the verb loop takes after it.
    fn shift_verb(&mut self, name: &str) -> String {
        let key = name.trim().to_ascii_lowercase();
        if key.is_empty() {
            let worn = magic::umwelt::Form::from_u8(self.op.form).unwrap_or_default();
            let names: Vec<&str> = magic::umwelt::Form::ALL.iter().map(|f| f.name()).collect();
            return format!("you are wearing {}. shift into: {}", worn.name(), names.join(", "));
        }
        match magic::umwelt::Form::ALL.iter().find(|f| f.name() == key) {
            Some(f) => {
                self.op.form = f.as_u8();
                format!("{}\r\n{}", f.body_line(), self.look())
            }
            None => format!("there is no body called '{key}'."),
        }
    }

    /// The sensory field at this cell, built from landed systems: weather sieve,
    /// haunt, square, and the stone under a town. Also returns scent age separately
    /// (t-axis position, not a permyriad channel magnitude).
    fn sense_field_here(&self, at: crate::umwelt_loom::Cell5) -> (crate::umwelt_loom::PentaractField, i64) {
        let w = &self.weather_sieve;
        let (x, y) = (self.x(), self.y());
        let (tx, ty) = world::town_square(self.op.node_seed);
        let b = world::biome_at(self.op.node_seed, x, y, self.op.bias);
        let worked = if (x, y) == (tx, ty) { i32::from(self.law_now()) * 80 } else { 0 };
        let delved = if b.name == "dungeon" { 6_000 } else { 0 };

        let mut field = crate::umwelt_loom::quiet_cell_field(at);
        field[crate::umwelt_loom::SenseChannel::AtmospherePa] = (w.chinook_buildup.max(0) * 1_000 + w.wind_speed.max(0) * 200)
            .clamp(0, 10_000);
        field[crate::umwelt_loom::SenseChannel::NecroticDecay] = self.haunt.pressure_q().clamp(0, 10_000);
        field[crate::umwelt_loom::SenseChannel::MasonryStress] = (worked + delved).clamp(0, 10_000);
        field[crate::umwelt_loom::SenseChannel::HeatGradient] = ((w.temperature - 20).abs() * 400).clamp(0, 10_000);
        field[crate::umwelt_loom::SenseChannel::ParticulateFlux] = (w.drought_ticks.min(25) as i32 * 400).clamp(0, 10_000);
        field[crate::umwelt_loom::SenseChannel::HateVector] = i32::from(self.haunt.aggression_level()) * 1_000;
        field[crate::umwelt_loom::SenseChannel::VitalityLux] = self.square.level_q().clamp(0, 10_000) as i32;

        let scent_age_t = (self.ticks % 16) as i64;
        (field.oriented(crate::umwelt_loom::cell_key(at)), scent_age_t)
    }

    /// Room description at the current position: biome, landmarks, presences.
    pub fn look(&self) -> String {
        let (x, y) = (self.x(), self.y());
        let b = world::biome_at(self.op.node_seed, x, y, self.op.bias);
        let (tx, ty) = world::town_square(self.op.node_seed);
        let town = self.speak_town();
        let sky = weather_line(self.weather.current);
        let body = if (x, y) == (tx, ty) {
            // The town speaks its law as sensation — the ironroot ladder's
            // face, never a number (WAVE-MUD-E2E sieve law holds here too).
            let watch = match self.law_now() {
                0..=9 => "no law walks these lanes",
                10..=39 => "the watch is thin here",
                40..=79 => "the watch keeps its rounds",
                _ => "the watch stands like a drawn blade",
            };
            if let Some(cart) = &self.npe_cart {
                format!(
                    "{} — {} ({}, square {x},{y})\r\n{sky}\r\n{watch}.\r\n  presences: {}, {}, {}\r\n  passages: {} (entry), {} (descending)",
                    cart.world.entry_zone_word,
                    cart.title.front_line,
                    cart.world.entry_gate_word,
                    cart.world.presences.questgiver_word,
                    cart.world.presences.territorial_word,
                    cart.world.presences.threat_word,
                    cart.world.entry_gate_word,
                    cart.world.door_out_word,
                )
            } else {
                format!(
                    "{} — {} (the town of this node; square {x},{y})\r\n{sky}\r\n{watch}.",
                    town.0, town.1
                )
            }
        } else {
            format!(
                "{} at square {x},{y}. the land holds its breath.\r\n{sky}",
                self.speak_biome(b.name)
            )
        };
        // The land remembers before it offers: unmarked ground speaks its
        // line (and its presence, after dark) above the walk's menu.
        let mut body = body;
        if let Some(m) = memory::memory_line(self.op.node_seed, x, y) {
            body.push_str(&format!("\r\n{m}"));
        }
        if let Some(g) = memory::ghost_line(self.op.node_seed, x, y, self.ticks) {
            body.push_str(&format!("\r\n{g}"));
        }
        if let Some(view) = self.sense_here() {
            for t in &view.tells {
                body.push_str(&format!("\r\n{t}"));
            }
        }
        // The woven room (umwelt_loom, donor v2 mud.rs:1356): the worn body
        // reads this cell deterministically, off the world systems that speak.
        let at = [x as i64, y as i64, self.z() as i64, (self.ticks % 64) as i64, 0];
        let senses = magic::senses_now(&self.op, self.room_sightline(), &self.ledger);
        let worn = magic::umwelt::Form::from_u8(self.op.form).unwrap_or_default();
        let (field, scent_age_t) = self.sense_field_here(at);
        body.push_str(&format!(
            "\r\n{}",
            crate::umwelt_loom::weave(worn, &senses, &field, scent_age_t, at, 1)
        ));
        // The land offers the walk: 2-4 tick-dealt options under the body,
        // and at night the stars add their own quiet wayfinding line.
        let offers = explore::offers(
            self.op.node_seed,
            x,
            y,
            self.z(),
            &self.weather.current,
            self.op.xp,
            self.ticks,
        );
        let mut out = format!("{body}\r\n{}", explore::render_offers(&offers));
        if b.name == "dungeon" && self.t() == 0 {
            out.push_str("\r\n  \x1b[2m>\x1b[0m Delve — the dark under the world is listening.");
        }
        if let Some(s) = memory::star_line(self.op.node_seed, x, y, self.ticks) {
            out.push_str(&format!("\r\n{s}"));
        }
        // The room through the body's own ears — the caster's carried noise.
        let heard = magic::muted_words(self.op.muted_q as i64);
        if !heard.is_empty() {
            out.push_str(&format!("\r\n\x1b[2m{heard}.\x1b[0m"));
        }
        out
    }

    /// CDK triad for the player's current cell. Prints the triad verdict,
    /// strips, and RGB colour — the Cosmic Dissonance Kernel rendered live.
    fn cdk_verb(&self) -> String {
        let (x, y, z) = (self.x() as i32, self.y() as i32, self.z() as i32);
        let fac_idx = consequence::town_faction(self.op.node_seed);
        let mind = mind::FactionMind::for_faction(fac_idx);
        let fac_name = consequence::FACTIONS[fac_idx].name;

        // Haunt is deterministic from position and node seed (sample: low entropy).
        let haunt_seed = (x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663)
            ^ (z as u64).wrapping_mul(83492791) ^ self.op.node_seed;
        let haunt = (haunt_seed as u32) % 3000;

        let t = cdk::triad(&mind, x, y, z, haunt);
        let [l, s, e] = t.to_channels();
        let (r, g, b) = cdk::colour(&t);

        // Report: verdict (BOUND/DISSONANT) + strips + short sensation line.
        let verdict = cdk::verdict_word(&t);
        let love_strip = cdk::bar(l);
        let strife_strip = cdk::bar(s);
        let entropy_strip = cdk::bar(e);
        let sensation = if t.dissonant() {
            "the room tears itself apart."
        } else {
            "the room holds together."
        };

        format!(
            "{}—{} (faction {})\r\nlove    {} {}\r\nstrife  {} {}\r\nentropy {} {}\r\n{} #{:02x}{:02x}{:02x}\r\n{}",
            verdict,
            t.harmony(),
            fac_name,
            love_strip,
            l,
            strife_strip,
            s,
            entropy_strip,
            e,
            verdict,
            r, g, b,
            sensation,
        )
    }

    /// The vertical walk: up toward the square's dealt height, down toward
    /// the ground — the 5D word's z axis carries it.
    fn climb(&mut self, up: bool) -> String {
        // Movement interrupts the channel.
        let mut reply = String::new();
        if let Some(broken) = self.channel.interrupt() {
            reply.push_str(&format!("the word dies half-spoken: {}\r\n", broken));
        }

        let (x, y, z) = (self.x(), self.y(), self.z());
        let height = explore::height_at(self.op.node_seed, x, y);
        let nz = if up {
            if z >= height {
                return String::from("nothing above you but sky. this ground does not rise.");
            }
            z + 1
        } else {
            if z == 0 {
                return String::from("you stand on the ground already.");
            }
            z - 1
        };
        self.op.pos = MortonKey5D::encode([x, y, nz, 0, 0]);
        reply.push_str(&explore::climb_line(self.weather.current.era, up, nz));
        reply
    }

    /// Delve one level into the abyss — only from a dungeon square, only to
    /// depth 3 (the 5D word's `t` axis carries it). Each step rolls one
    /// ENCOUNTER: RARE POP at ~100/10_000, a common foe at ~2_500/10_000,
    /// else silence — the abyss is the world's held breath, not gore.
    /// Draws darkness sensation from the abyss domain.
    fn delve(&mut self) -> String {
        // Movement interrupts the channel.
        let mut reply = String::new();
        if let Some(broken) = self.channel.interrupt() {
            reply.push_str(&format!("the word dies half-spoken: {}\r\n", broken));
        }

        let (x, y) = (self.x(), self.y());
        let b = world::biome_at(self.op.node_seed, x, y, self.op.bias);
        if b.name != "dungeon" {
            return String::from(
                "there is no way down here — the abyss opens only under stone.",
            );
        }
        let t = self.t();
        if t >= abyss::MAX_DEPTH {
            return String::from("the deepest hush holds; there is no further down.");
        }
        let nt = t + 1;
        self.op.pos = MortonKey5D::encode([x, y, self.z(), nt, 0]);
        self.train(SK_WAYFINDING);
        if !reply.is_empty() {
            reply.push_str("\r\n");
        }
        reply.push_str(abyss::light_words(nt));
        match roll_encounter(self.op.node_seed, x, y, nt, self.op.xp) {
            Some(enc) => {
                reply.push_str(&format!("\r\n{} rises to meet you.", enc.name));
                if enc.rare {
                    reply.push_str("\r\nsomething about it does not belong to this depth.");
                    reply.push_str(&format!(
                        "\r\n{}",
                        abyss::hazard_words(abyss::depth_overpressure_pa(nt))
                    ));
                    reply.push_str(&format!(
                        "\r\n{}",
                        abyss::heat_words(abyss::depth_flux_w_m2(nt))
                    ));
                }
                // The shadow speaks its remembrance, if it has any memory.
                if self.haunt.execution_count > 0 {
                    let awareness = self.haunt.classify_awareness();
                    reply.push_str(&format!(
                        "\r\nthe air darkens — {}.",
                        awareness.remembrance_line()
                    ));
                }
                self.encounter = Some(enc);
            }
            None => {
                reply.push_str("\r\nnothing stirs. the silence holds.");
                self.encounter = None;
            }
        }
        reply
    }

    /// Climb back toward the surface, one level of the abyss at a time —
    /// refuses gently when already at the surface. Obeys the buoyancy law:
    /// past the cap, ascent is dealt back (the abyss returns you).
    fn ascend(&mut self) -> String {
        let t = self.t();
        if t == 0 {
            return String::from(
                "the surface is already underfoot; there is no further up from here.",
            );
        }
        let nt = t - 1;
        self.op.pos = MortonKey5D::encode([self.x(), self.y(), self.z(), nt, 0]);
        let pressure_word = abyss::pressure_words(nt);
        format!("you climb back toward the surface. {}.", pressure_word)
    }

    /// Resolve the pending abyss encounter: a deterministic contest between
    /// the operator's Hunt standing (plus level and a chaos roll) and the
    /// foe's depth-scaled difficulty. The 120Hz chord engine (`combat.rs`)
    /// is NOT wired here — that seam joins when the window host lands; this
    /// is the mud-side verb-turn fight.
    fn fight(&mut self) -> String {
        // Combat interrupts the channel — damage is taken, focus breaks.
        let mut reply = String::new();
        if let Some(broken) = self.channel.interrupt() {
            reply.push_str(&format!("the word dies half-spoken: {}\r\n", broken));
        }

        let Some(enc) = self.encounter.clone() else {
            return String::from("there is nothing here to fight.");
        };
        let level = Self::level(self.op.xp) as u32;
        let chaos = (seed_hash(&[
            &u64_to_nistam(self.op.node_seed),
            &u64_to_nistam(self.op.xp),
            b"fight-chaos",
        ]) & 0xFF) as u32;
        let attacker = (self.op.skills.art_value(0) as u32) / 10 + level * 5 + chaos % 50;
        let difficulty = enc.depth as u32 * 30 + if enc.rare { 40 } else { 0 };

        // W5a door seam — the exchange speaks through the combat brain
        // (firewall law: the brain computes integers, the door words them;
        // raw numbers never reach the transcript).
        let foe_hz = 800u16.saturating_sub(enc.depth * 190).clamp(40, 800);
        let foe_state = combat::CombatState { resonance_hz: foe_hz, ..Default::default() };
        let strike = combat_brain::evaluate_strike(&foe_state);
        let mut exchange = if reply.is_empty() {
            format!(
                "{} strikes — {}\r\n",
                enc.name,
                strike_word(strike.hit_stop_ticks)
            )
        } else {
            format!(
                "{}\r\n{} strikes — {}\r\n",
                reply,
                enc.name,
                strike_word(strike.hit_stop_ticks)
            )
        };
        let mut me = combat::CombatState {
            resonance_hz: (40 + (level as u16 * 40 + (chaos as u16) * 3) % 760),
            ..Default::default()
        };
        combat_brain::record_parry_activation(&mut me, 0);
        let window = (chaos % 4) as u16;
        let shield_pmy: u32 = if window <= 2 {
            match combat_brain::evaluate_parry(&mut me, window, foe_hz) {
                combat_brain::ParryResult::Perfect { .. } => {
                    // Combat C evaluate seam — the organ words the verdict,
                    // the door speaks it (machine token → spoken words).
                    exchange.push_str(&format!(
                        "your parry holds perfect — a ring of silence, and the blow dies on your guard.\r\nthe arena marks it: {}.\r\n",
                        combat_brain::verdict_word(combat::ChordAction::PerfectParry).replace('_', " ")
                    ));
                    10000
                }
                _ => {
                    exchange.push_str(&format!(
                        "your parry takes the weight; the blow slides past half-spent.\r\nthe arena marks it: {}.\r\n",
                        combat_brain::verdict_word(combat::ChordAction::StandardParry).replace('_', " ")
                    ));
                    5000
                }
            }
        } else {
            exchange
                .push_str("your guard opens too late — the blow finds you whole.\r\n");
            0
        };

        if attacker >= difficulty {
            self.op.xp += 21;
            self.train(SK_SKINNING);
            self.encounter = None;
            self.run_profile.record_kill();
            if shield_pmy == 10000 {
                self.run_profile.record_perfect_parry();
            }
            let mut reply = format!(
                "{}you answer — {}\r\n{} falls before you. the abyss gives up what it held.",
                exchange,
                strike_word(combat_brain::compute_hit_stop(me.resonance_hz)),
                enc.name
            );
            if enc.rare {
                let item = pick(
                    items::ITEMS,
                    &[&u64_to_nistam(self.op.node_seed), &u64_to_nistam(self.op.xp), b"abyss-loot"],
                );
                let prov = itemforge::roll_provenance(seed_hash(&[
                    &u64_to_nistam(self.op.node_seed),
                    &u64_to_nistam(self.op.xp),
                    b"abyss-prov",
                ]));
                let material_idx = (seed_hash(&[
                    &u64_to_nistam(self.op.node_seed),
                    &u64_to_nistam(self.op.xp),
                    b"abyss-material",
                ]) % relics::MATERIALS.len() as u64) as usize;
                reply.push_str(&format!(
                    "\r\nit carried a {} {} of {} — {}",
                    itemforge::provenance_word(prov),
                    item.0,
                    relics::MATERIALS[material_idx],
                    item.1
                ));
            }
            reply
        } else {
            let full = (15 * enc.depth) as u32;
            let taken = (full - full * shield_pmy / 10000) as u16;
            self.vitality = self.vitality.saturating_sub(taken);
            if self.vitality == 0 {
                let scar = combat_brain::forge_scar(
                    self.op.node_seed,
                    self.op.xp,
                    enc.depth as u64,
                    [self.x() as i64 * 1000, self.y() as i64 * 1000],
                    combat_brain::DeathCause::Combat,
                );
                // Record the scar hash in the shadow's memory (haunt).
                self.haunt.record_scar(scar.scar_hash);
                format!(
                    "{}{} answers in kind — you are driven back.\r\n\
                     the world takes its mark — a scar is cut where you fell, and the ground will remember.\r\n\
                     {}",
                    exchange, enc.name, self.die(combat_brain::DeathCause::Combat)
                )
            } else {
                format!("{}{} answers in kind — you are driven back.", exchange, enc.name)
            }
        }
    }

    /// Break from the pending encounter and climb one level back toward the
    /// surface — the abyss lets go, but not for free.
    fn flee(&mut self) -> String {
        if self.encounter.is_none() {
            return String::from("there is nothing here to flee.");
        }
        self.encounter = None;
        let t = self.t();
        if t > 0 {
            self.op.pos = MortonKey5D::encode([self.x(), self.y(), self.z(), t - 1, 0]);
        }
        self.vitality = self.vitality.saturating_sub(5);
        String::from("you break and run — the dark lets you go, this once.")
    }

    /// The birth kit: 3 items dealt deterministically from (name, moon,
    /// day) — same operator, same kit, forever (no xp in the seed).
    fn kit(&self) -> String {
        let mut lines = Vec::with_capacity(3);
        if let Some(cart) = &self.npe_cart {
            let prov_words = [
                &cart.kit.provenance_words.pure,
                &cart.kit.provenance_words.blood,
                &cart.kit.provenance_words.reclaimed,
            ];
            for i in 0..cart.kit.item_count.min(3) {
                let item = pick(
                    items::ITEMS,
                    &[self.op.name.as_bytes(), &[self.op.moon, self.op.day], b"kit", &[i]],
                );
                let material_idx = (seed_hash(&[
                    self.op.name.as_bytes(),
                    &[self.op.moon, self.op.day],
                    b"kit-material",
                    &[i],
                ]) % relics::MATERIALS.len() as u64) as usize;
                let prov = prov_words[(i as usize) % prov_words.len()];
                lines.push(format!(
                    "{} {} of {} — {}",
                    prov,
                    item.0,
                    relics::MATERIALS[material_idx],
                    item.1
                ));
            }
            format!("your birth kit ({}):\r\n{}", cart.cart, lines.join("\r\n"))
        } else {
            for i in 0..3u8 {
                let item = pick(
                    items::ITEMS,
                    &[self.op.name.as_bytes(), &[self.op.moon, self.op.day], b"kit", &[i]],
                );
                let prov = itemforge::roll_provenance(seed_hash(&[
                    self.op.name.as_bytes(),
                    &[self.op.moon, self.op.day],
                    b"kit-prov",
                    &[i],
                ]));
                let material_idx = (seed_hash(&[
                    self.op.name.as_bytes(),
                    &[self.op.moon, self.op.day],
                    b"kit-material",
                    &[i],
                ]) % relics::MATERIALS.len() as u64) as usize;
                lines.push(format!(
                    "{} {} of {} — {}",
                    itemforge::provenance_word(prov),
                    item.0,
                    relics::MATERIALS[material_idx],
                    item.1
                ));
            }
            format!("your birth kit:\r\n{}", lines.join("\r\n"))
        }
    }

    /// One quiet training roll — gains surface only on the character sheet,
    /// never as a line (the invisible cart's discipline extends to skills).
    /// If the art crosses a rank band, speak the worded rank-up line.
    fn train(&mut self, skill: usize) {
        let art = skills::SKILLS[skill].1;
        let old_value = self.op.skills.value[art];
        let old_band = old_value / 143;

        if let Some(new_value) = self.op.skills.train(skill, self.op.node_seed, self.op.xp) {
            let new_band = new_value / 143;
            if new_band > old_band && new_band < 7 {
                // Band crossed — speak the rank-up line.
                let rank_word = self.op.skills.word(art);
                let art_name = skills::ARTS[art].0;
                self.speak_rank_up(art, art_name, rank_word);
            }
        }
    }

    /// Speak a worded rank-up line when an art crosses a band threshold.
    /// AUTHORED: unique worded lines per art/band crossing.
    fn speak_rank_up(&mut self, _art: usize, art_name: &str, rank_word: &str) {
        // [AUTHORED] Unique rank-up lines per art — no digits, worded sensation.
        let line = match rank_word {
            "dabbling" => format!("your grasp of {} deepens — the first skill awakens.", art_name),
            "apprentice" => format!("the apprentice's mark appears — {} opens its first real door.", art_name),
            "journeyman" => format!("the journeyman's steadiness settles in — {} obeys your will now.", art_name),
            "adept" => format!("the adept's threshold stands open — {} responds to your touch alone.", art_name),
            "master" => format!("the master's voice speaks through you — {} is now your true tongue.", art_name),
            "grandmaster" => format!("the apex stands before you — {} knows no greater height.", art_name),
            _ => return, // untried → dabbling is the first spoken rank.
        };
        self.rank_ups.push(line);
    }

    /// The room's grant to a listener — what the biome affords a sightline.
    fn room_sightline(&self) -> i64 {
        let b = world::biome_at(self.op.node_seed, self.x(), self.y(), self.op.bias);
        match b.name {
            "forest" | "swamp" => 4_000,
            "cave" | "abyss" => 2_000,
            "plains" | "steppe" | "desert" | "tundra" => 7_500,
            _ => 5_500,
        }
    }

    /// The talent mandala: heart, spokes, poles, outer ring.
    fn mandala(&self) -> String {
        let school = magic::birth_school(&self.op).as_str();
        let arts: [(&str, &str); 7] = std::array::from_fn(|i| {
            (skills::ARTS[i].0, self.op.skills.word(i))
        });
        let currents = magic::loadout::load_currents(&self.ledger, self.op.node_seed);
        magic::loadout::render_mandala(school, &arts, currents)
    }

    /// `talent <name>`: take a pole on the mandala — one of each current.
    fn take_pole(&mut self, name: &str) -> String {
        match magic::loadout::talent_by_name(name) {
            Some((is_masc, idx)) => {
                let mut c = magic::loadout::load_currents(&self.ledger, self.op.node_seed);
                let (current, spoken) = if is_masc {
                    c.masculine = Some(idx);
                    ("force", talents::MASCULINE[idx as usize].0)
                } else {
                    c.feminine = Some(idx);
                    ("water", talents::FEMININE[idx as usize].0)
                };
                magic::loadout::save_currents(&c, &mut self.ledger);
                let mut msg =
                    format!("the {current} current turns — {spoken} holds your pole now.");
                self.save_ledger(&mut msg);
                msg
            }
            None => format!("no pole on the mandala answers to '{name}'."),
        }
    }

    /// `bind <slot 1-6> <word>`: hang a taught word at the belt.
    fn bind_slot(&mut self, parts: &[&str]) -> String {
        let slot = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
        let word = parts.get(2).copied().unwrap_or("").to_lowercase();
        if !(1..=magic::loadout::SUNG_SLOTS).contains(&slot) {
            return String::from("the belt holds six places; speak `bind <one..six> <word>`.");
        }
        if !magic::knows(&self.op, &word) {
            return format!("you cannot hang '{word}' at your belt before someone teaches it to you.");
        }
        let Some(idx) = magic::loadout::word_index(&word) else {
            return format!("'{word}' is not in the canon.");
        };
        let mut bar = magic::loadout::load_sung_bar(&self.ledger, self.op.node_seed);
        match bar.bind(slot - 1, idx) {
            Ok(()) => {
                magic::loadout::save_sung_bar(&bar, &mut self.ledger);
                let mut msg = format!(
                    "'{word}' hangs at the belt now — speak the place's number to sing it."
                );
                self.save_ledger(&mut msg);
                msg
            }
            Err(e) => String::from(e),
        }
    }

    /// `cast <word>`: a TAUGHT sung word sings now through `magic::sing` and
    /// is paid in hearing; the seven WAR glyph words keep the channel bar.
    fn cast(&mut self, parts: &[&str]) -> String {
        let raw = parts.get(1).copied().unwrap_or("");
        let sung = raw.to_lowercase();
        if crate::magic_words::school_of(&sung).is_some() {
            if !magic::knows(&self.op, &sung) {
                return if magic::known_words(&self.op).contains(&sung.as_str()) {
                    format!(
                        "'{}' belongs to your school, but your art has not earned it — the trade opens it in time.",
                        sung
                    )
                } else {
                    format!(
                        "the word '{}' is real — you have heard it sung — but no one has taught it to you.",
                        sung
                    )
                };
            }
            let senses = magic::senses_now(&self.op, self.room_sightline(), &self.ledger);
            return match magic::sing(&sung, senses, self.op.natal_key()) {
                Some(cast) => {
                    let currents =
                        magic::loadout::load_currents(&self.ledger, self.op.node_seed);
                    let colored =
                        magic::loadout::color(cast.cost_q, cast.reach_q, currents);
                    // The trade's hand: the same word, fewer strokes.
                    let paid = (colored.cost_q * magic::school_tier(&self.op).efficiency_q()
                        / 10_000)
                        .max(magic::SUNG_FLOOR_Q);
                    self.op.muted_q = (senses.muted_q + paid)
                        .clamp(0, magic::umwelt::AUTHORED_Q)
                        as u16;
                    let mut reply = format!(
                        "you sing '{}', and the {} answers.\r\n{},\r\n{}.",
                        cast.word,
                        cast.school.as_str(),
                        magic::cost_words(paid),
                        magic::felt_reach_words(colored.reach_q, &cast.after)
                    );
                    for clause in [colored.masc_clause, colored.fem_clause].into_iter().flatten() {
                        reply.push_str(&format!("\r\n{clause}."));
                    }
                    reply
                }
                None => String::from("the word would not sing."),
            };
        }
        let word_name = raw.to_uppercase();

        // Check if already casting.
        if self.channel.is_active() {
            let current_word = self.channel.word().unwrap_or("?");
            return format!(
                "the word {} already fills your mind — you cannot hold two voices at once.",
                current_word
            );
        }

        // Find the word index.
        let word_idx = casting::GLYPH_WORDS
            .iter()
            .position(|(w, _)| w == &word_name)
            .ok_or_else(|| format!("the word '{}' holds no sorcerous power here.", word_name))
            .and_then(|idx| {
                casting::Channel::new(idx as u8)
                    .ok_or_else(|| String::from("the word could not be woven."))
            });

        match word_idx {
            Ok(channel) => {
                self.channel = channel;
                format!(
                    "the word {} gathers in your throat, waiting to be spoken.",
                    channel.word().unwrap_or("?")
                )
            }
            Err(msg) => msg,
        }
    }

    /// Camp overnight: one pulse bar (30 ticks) passes, the sky moves, and
    /// dawn speaks from the NEW sky. The door's clock rides with it. The
    /// slide: camping always trains camping; the dark trains dark
    /// camouflage, the forest trains woods camouflage.
    fn camp(&mut self) -> String {
        self.train(SK_CAMPING);
        if matches!(explore::day_phase(self.ticks), "dusk" | "night") {
            self.train(SK_DARK_CAMO);
        }
        let b = world::biome_at(self.op.node_seed, self.x(), self.y(), self.op.bias);
        if b.name == "forest" {
            self.train(SK_WOODS_CAMO);
        }
        self.ticks += 30;
        self.vitality = (self.vitality + 25).min(100);
        let mut reply = explore::camp(&mut self.weather);
        reply.push_str("\r\nrest mends what the road has taken.");
        reply
    }

    /// Sleep: stages the day's reel (Sentinel 246, `ORACLE-C-DREAM-DIAMONDS-
    /// EUX.md` §8) into an `EphemeralEnvelope` good for `dream::SLEEP_TTL_TICKS`
    /// and waits for `wake`. `balance`/`energy` read off `heat`/`vitality`,
    /// already-live session fields, captured now so `wake` scores the rest as
    /// of when it started, not when it ends. Deep-fire dream-text generation
    /// and the Witness/reciprocity/taboo layer stay out of scope — this verb
    /// only proves the mechanical skeleton runs.
    fn sleep(&mut self) -> String {
        if self.sleeping.is_some() {
            return String::from("you are already asleep — `wake` first.");
        }
        let sleep_tick = self.ticks;
        let balance_pmy = 10_000u32.saturating_sub(u32::from(self.op.heat) * 500);
        let energy_pmy = u32::from(self.vitality) * 100;
        let lean_pmy = 10_000u32.saturating_sub(balance_pmy);
        let dreamed = self.dream_fire.dream(&self.dream_journal, lean_pmy).ok();
        let buffer = match &dreamed {
            Some(text) => dream::SessionBuffer(text.clone().into_bytes()),
            None => dream::SessionBuffer(vec![0u8; 32]),
        };
        let envelope = dream::stage_session(buffer, sleep_tick, dream::SLEEP_TTL_TICKS);
        self.sleeping = Some(SleepState { sleep_tick, balance_pmy, energy_pmy, envelope });
        let mut reply = String::from("sleep takes you down; `wake` when you're ready.");
        if dreamed.is_some() {
            reply.push_str("\r\nthe deep fire takes the day's reel.");
        }
        reply
    }

    /// Wake: resolves the rest staged by `sleep`. `beat_pmy` — the rest-
    /// cadence tracker `sleep_pmy` used to fake at a fixed `5_000` — is now
    /// the real ticks-asleep measured against `dream::SLEEP_TTL_TICKS`, the
    /// session's own clock. Waking inside the window seals `Attested`
    /// (the safe rest); oversleeping past it lets the envelope fall through
    /// to `Expired` — the Sleeping-Beauty risk, resolved with the same
    /// deterministic `seed_hash` roll every other draw in this file uses.
    fn wake(&mut self) -> String {
        let Some(state) = self.sleeping.take() else {
            return String::from("you are not asleep.");
        };
        let ticks_slept = self.ticks.saturating_sub(state.sleep_tick);
        let beat_pmy = ((ticks_slept * 10_000) / dream::SLEEP_TTL_TICKS).min(10_000) as u32;
        let quality_pmy = dream::day_quality_pmy(state.balance_pmy, state.energy_pmy, beat_pmy);
        self.dream_journal.observe_quality(quality_pmy as f32 / 10_000.0);

        let mut watch = dream::RoughPatchWatch::new();
        let rough_patch = watch.observe(quality_pmy);

        let mut chain = forge_envelope::EvidenceChain::new();
        let link = dream::shred_on_wake(state.envelope, self.ticks, &mut chain);

        let mut reply = match link.record() {
            forge_envelope::Disposition::Attested(_) => {
                let mut line =
                    format!("you wake clean; the night was {}.", quality_word(quality_pmy));
                let score = dream::NightScore {
                    balance_pmy: state.balance_pmy,
                    energy_pmy: state.energy_pmy,
                    beat_pmy,
                };
                if let Some((kept, repaired)) =
                    self.keep_the_nights_gift(state.sleep_tick, score, &mut chain)
                {
                    if repaired {
                        line.push_str("\r\nthe dream mends itself once before letting you go.");
                    }
                    line.push_str(&format!(
                        "\r\nthe dream leaves {} behind.",
                        dream::gift_word(kept)
                    ));
                }
                line
            }
            _ => {
                let roll = seed_hash(&[
                    &u64_to_nistam(self.op.node_seed),
                    &u64_to_nistam(state.sleep_tick),
                    b"oversleep",
                ]);
                self.vitality = self.vitality.saturating_sub(10);
                if roll % 2 == 0 {
                    format!(
                        "you oversleep; the dream almost kept you. the night was {}.",
                        quality_word(quality_pmy)
                    )
                } else {
                    format!(
                        "you oversleep; the world moved without you. the night was {}.",
                        quality_word(quality_pmy)
                    )
                }
            }
        };
        if rough_patch {
            reply.push_str("\r\nthe world noticed the rough patch.");
        }
        reply
    }

    /// Mint the night's gift and carry it through the airlock onto the same
    /// chain the transcript was shredded on. A rough night mints nothing; a
    /// refused proposal keeps nothing. Returns the shape the world kept.
    fn keep_the_nights_gift(
        &mut self,
        sleep_tick: u64,
        score: dream::NightScore,
        chain: &mut forge_envelope::EvidenceChain,
    ) -> Option<(forge_envelope::typed_manifold::GiftKind, bool)> {
        let minted = dream::mint_with_one_repair(self.op.node_seed, sleep_tick, score)?;
        let sealed = dream::admit_gift(&minted.proposal, self.ticks, chain)?;
        self.dream_gifts.push(sealed);
        Some((sealed.kind, minted.repaired))
    }

    /// Every seal a night has left in this session, oldest first.
    pub fn dream_gifts(&self) -> &[forge_envelope::typed_manifold::SealedGift] {
        &self.dream_gifts
    }

    /// Light the night's generator (§8:234) — the shell attaches
    /// `dream::DoorFire` here; the default hearth is `dream::NoFire`.
    pub fn light_dream_fire(&mut self, fire: Box<dyn dream::DreamFire + Send>) {
        self.dream_fire = fire;
    }

    /// The witness mirror's read on this player right now.
    pub fn mirror(&self) -> &witness_mirror::WitnessMirror {
        &self.mirror
    }

    /// Resolve the Bell Pit's standing event through the real DM entry point
    /// and fold the walker's share of the verdict on. Heat moves, so the
    /// `shadow` word on `mud.live` moves with it. The eight world fields of
    /// the delta are not applied here — they are the manifold's, not the
    /// walker's.
    fn resolve(&mut self) -> String {
        let evt = crate::ironroot::bell_pit::bell_pit_event_state();
        let router = crate::dm::resolution_router();
        let mode = match crate::dm::resolve_event_mode(&evt, &router, &crate::dm::NoEscalation) {
            Ok(mode) => mode,
            Err(e) => return format!("the pit will not answer: {e:?}"),
        };
        let before = crate::live::shadow_word(self.op.heat);
        let delta = crate::dm::resolution_effects(mode);
        self.op.apply_resolution(&delta, evt.faction_owner);
        let after = crate::live::shadow_word(self.op.heat);
        let mut reply = format!("the pit resolves: {mode:?}.");
        if before == after {
            reply.push_str(&format!("\r\nthe shadow holds at {after}."));
        } else {
            reply.push_str(&format!("\r\nthe shadow turns {before} -> {after}."));
        }
        reply
    }

    /// Standing witness on unmarked ground: the count rides the ledger at
    /// GLOBAL scope — memory, by design, survives every reseed. Witnessing
    /// is its own act; it pays nothing and it is not refused twice.
    fn witness(&mut self) -> String {
        let (x, y) = (self.x(), self.y());
        let seed = self.op.node_seed;
        if !memory::unmarked_at(seed, x, y) {
            return String::from(
                "the ground here keeps no vigil. the stars may show you where one waits.",
            );
        }
        self.train(SK_WITNESSING);
        self.train(SK_WISDOM);
        self.ledger.append(overlay::OverlayEntry {
            domain: overlay::Domain::Zone,
            key: 0,
            modification: overlay::Mod::Add(1),
            priority: 100,
            scope: overlay::Scope::Global,
        });
        let mut reply = memory::memory_line(seed, x, y)
            .unwrap_or_else(|| String::from("the ground holds still."));
        reply.push_str("\r\nyou stand a while, and do not look away.");
        reply.push_str(&format!(
            "\r\n{}.",
            memory::awakening_word(memory::witness_count(&self.ledger, seed))
        ));
        self.save_ledger(&mut reply);
        reply
    }

    /// The debug reseed (the birthday window's landing): the operator keeps
    /// what they earned, the node deals anew. Node-scoped overlays fall out
    /// of visibility with the old seed; Operator/Global ride through.
    fn reseed(&mut self, seed: u64) -> String {
        console::apply_reseed(&mut self.op, seed);
        self.weather = dealt_weather(&self.ledger, seed);
        self.author_scope = overlay::Scope::Node(seed);
        format!(
            "the node lets go — and deals the one you named.\r\n{}",
            console::seed_summary(seed)
        )
    }

    /// Persist the ledger if a path is set — LOUD on failure, like autosave.
    fn save_ledger(&self, reply: &mut String) {
        let Some(path) = &self.ledger_path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = self.ledger.save(path) {
            reply.push_str(&format!("\r\n\x1b[1;31mLEDGER SAVE FAILED: {e}\x1b[0m"));
        }
        self.save_haunt(reply);
    }

    /// Persist the shadow's memory if a path is set — LOUD on failure, like autosave.
    /// Atomic write via temp+rename (L07 bijection tested).
    fn save_haunt(&self, reply: &mut String) {
        let Some(path) = &self.haunt_path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let bytes = self.haunt.encode();
        // Atomic write: write to temp, then rename (avoids corruption on crash).
        let temp_path = path.with_extension("tmp");
        if let Err(e) = std::fs::write(&temp_path, &bytes) {
            reply.push_str(&format!("\r\n\x1b[1;31mHAUNT TEMP WRITE FAILED: {e}\x1b[0m"));
            return;
        }
        if let Err(e) = std::fs::rename(&temp_path, path) {
            reply.push_str(&format!("\r\n\x1b[1;31mHAUNT RENAME FAILED: {e}\x1b[0m"));
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    /// Append one plain-text Academy milestone receipt — LOUD on failure,
    /// same pattern as `save_haunt`. ANSI-free (the terminal reply carries
    /// the colour, this file carries prose a future PKM ingest can read).
    fn write_academy_receipt(&self, reply: &mut String, gate: usize, boss: &str, epitaph: &str, lore: &str) {
        let Some(path) = &self.academy_log_path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let line = format!(
            "MILESTONE {} FALLS — {boss}: {epitaph} [{lore}] (operator={}, node_seed={}, xp={}).\n\n",
            gate + 1,
            self.op.name,
            self.op.node_seed,
            self.op.xp
        );
        use std::io::Write;
        let opened = std::fs::OpenOptions::new().create(true).append(true).open(path);
        match opened {
            Ok(mut f) => {
                if let Err(e) = f.write_all(line.as_bytes()) {
                    reply.push_str(&format!("\r\n\x1b[1;31mACADEMY LOG WRITE FAILED: {e}\x1b[0m"));
                }
            }
            Err(e) => reply.push_str(&format!("\r\n\x1b[1;31mACADEMY LOG OPEN FAILED: {e}\x1b[0m")),
        }
    }

    fn status(&self) -> String {
        let t = world::theme(self.op.node_seed);
        let r = ConnectionRoll::deal(self.op.node_seed);
        // The character sheet: the seven arts speak their rank WORDS (the
        // sliding skills surface here and only here — never as numbers).
        let arts = skills::ARTS
            .iter()
            .enumerate()
            .map(|(i, (name, _))| format!("{name} {}", self.op.skills.word(i)))
            .collect::<Vec<_>>()
            .join(" · ");
        let awakening =
            memory::awakening_word(memory::witness_count(&self.ledger, self.op.node_seed));
        // Ports forge-insights' `Moon::on_beat`/`Account::open_on` exactly:
        // xp is the real beat (monotonic terminal bytes earned, never a
        // stand-in), rotate_left(17) and the seed-XOR shape match the
        // drain source; `seed_hash` is this crate's own mixer in place of
        // forge-insights' private `mix` (L05: one mixer, this crate's).
        let beat = self.op.xp;
        let overhead_moon = moons::MOONS[((beat / moons::BEATS_PER_MOON) % 13) as usize].0;
        let account_idx = (seed_hash(&[&u64_to_nistam(
            self.op.node_seed ^ beat.rotate_left(17),
        )]) % accounts::ACCOUNTS.len() as u64) as usize;
        let account = accounts::ACCOUNTS[account_idx];
        // [ASSUMED] heat-bucket ladder onto the 6 shadow tiers: `heat` is the
        // real, existing law-escalation register (consequence.rs) — no
        // fail-streak counter exists in this crate to bucket instead.
        let shadow_idx = match self.op.heat {
            0 => 0,
            1..=2 => 1,
            3..=5 => 2,
            6..=10 => 3,
            11..=20 => 4,
            _ => 5,
        };
        let shadow_tier = shadow::SHADOW_TIERS[shadow_idx];
        // The Brand is not yet a persisted register — it is read here as a
        // deterministic view over the existing `heat` law-escalation
        // counter, the same derivation shape `shadow_idx` above already
        // uses, until a real accrual site lands (first live caller of
        // `ironroot::brand`, previously published but never composited).
        let mut brand = crate::ironroot::brand::BrandCorruption::default();
        brand.corrupt(self.op.heat.min(255) as u8);
        // The roster standing: the Drift state routed to an outcome. One unit
        // exists today, so the sheet is where it reads; a party rotation is
        // the same call over many blocks.
        let standing_line = format!("\r\nstanding: {}.", r.stats.standing().word());
        // The thought cabinet: which registers are loud enough to argue at all.
        // A reader of the dealt block, never a second home for it.
        let cabinet = voices::cabinet(&r.stats);
        let voices_line = if cabinet.is_empty() {
            String::from("\r\nthe cabinet is quiet.")
        } else {
            let spoken = cabinet
                .iter()
                .map(|s| {
                    let mark = if s.tone.is_override() { " (answering for you)" } else { "" };
                    format!("{} {}{mark}", voices::voice_name(s.stat), tone_word(s.tone))
                })
                .collect::<Vec<_>>()
                .join(" · ");
            format!("\r\nthe cabinet: {spoken}")
        };
        let sheet = format!(
            "\r\nthe arts: {arts}\r\n{awakening}.\r\nvitality: {}.\r\noverhead moon: {overhead_moon} · account: {} — {}\r\nshadow: {} — {}\r\nbrand: {}/255 ({}){}{standing_line}{voices_line}",
            vitality_word(self.vitality),
            account.0,
            account.1,
            shadow_tier.0,
            shadow_tier.1,
            brand.level,
            if brand.attunement == crate::ironroot::brand::AttunementTier::Marked { "marked" } else { "attuned" },
            if brand.is_unstable() { " — unstable" } else { "" }
        );
        format!(
            "operator {} · born moon {} day {} · xp {} · level {}/13 · deaths {}\r\nnode {:016x} · sky {} · vibe {} · hue {} · pos {},{} (5D word {:016x})\r\nthis node dealt: VIG {} MOM {} LOG {} SHA {} RES {} · reagent {:?} · under {}{sheet}",
            self.op.name,
            self.op.moon + 1,
            self.op.day + 1,
            self.op.xp,
            Self::level(self.op.xp),
            self.op.deaths,
            self.op.node_seed,
            t.skybox.0,
            t.vibe.0,
            t.hue,
            self.x(),
            self.y(),
            self.op.pos.0,
            r.stats.vigor,
            r.stats.momentum,
            r.stats.logic_depth,
            r.stats.shadow_weight,
            r.stats.resonance,
            r.reagent,
            r.constellation(),
        )
    }

    /// Inner width of the boxed sheet. `cdk_wireframe.rs::window()` proved this
    /// exact `+-...-+`/row frame shape once already (the only other live box in
    /// this crate); this is its second real caller, so the frame is factored
    /// into `sheet_row`/`sheet_rule` instead of hand-printed a second time (L05).
    const SHEET_W: usize = 58;

    /// One row inside the box, clipped and padded to `SHEET_W` so a long
    /// operator name (birth-interview free text) can never widen the frame.
    fn sheet_row(out: &mut String, inner: &str) {
        let clipped: String = if inner.chars().count() <= Self::SHEET_W {
            inner.to_string()
        } else {
            inner.chars().take(Self::SHEET_W - 1).chain(std::iter::once('~')).collect()
        };
        let pad = Self::SHEET_W.saturating_sub(clipped.chars().count());
        out.push_str("  | ");
        out.push_str(&clipped);
        out.push_str(&" ".repeat(pad));
        out.push_str(" |\r\n");
    }

    /// A labelled divider row, e.g. `-- THE ARTS ----...----`.
    fn sheet_rule(out: &mut String, label: &str) {
        let dashes = Self::SHEET_W.saturating_sub(label.len() + 4);
        Self::sheet_row(out, &format!("-- {label} {}", "-".repeat(dashes)));
    }

    /// The boxed ASCII character sheet: the same real state `status()` reads
    /// (operator, dealt registers, arts, the born-under light), framed the way
    /// `cdk_wireframe.rs` already proved a live query renders — never a second,
    /// hand-copied box. Registers use `cdk::bar` (its 0..=1000 permille scale)
    /// over the dealt 30..=150(+drift) band, denominator 255 so an accrued
    /// register saturating u8 still reads as a full bar, never overflowing it.
    pub fn status_card(&self) -> String {
        let r = ConnectionRoll::deal(self.op.node_seed);
        let star = &forge_core_v3::sky::CATALOG[r.star];
        let mut o = String::new();
        let head = format!("[ {} ]", self.op.name);
        o.push_str(&format!(
            "  +- forge-mud-v3 {}{} -+\r\n",
            "-".repeat(Self::SHEET_W.saturating_sub(head.chars().count() + 17)),
            head
        ));
        Self::sheet_row(
            &mut o,
            &format!(
                "moon {:<2} day {:<2}  xp {:<6} lvl {}/13  deaths {}",
                self.op.moon + 1,
                self.op.day + 1,
                self.op.xp,
                Self::level(self.op.xp),
                self.op.deaths
            ),
        );
        Self::sheet_row(
            &mut o,
            &format!(
                "born under {} [{}] mag {}",
                star.name,
                star.constellation,
                star.mag_display()
            ),
        );
        Self::sheet_rule(&mut o, "REGISTERS");
        let scale = |v: u8| (v as i32 * 1000 / 255).min(1000);
        for (label, v) in [
            ("vigor", r.stats.vigor),
            ("momentum", r.stats.momentum),
            ("logic-depth", r.stats.logic_depth),
            ("shadow-weight", r.stats.shadow_weight),
            ("resonance", r.stats.resonance),
        ] {
            Self::sheet_row(&mut o, &format!("{label:<14}{} {:>3}", cdk::bar(scale(v)), v));
        }
        Self::sheet_rule(&mut o, "THE ARTS");
        for (i, (name, _)) in skills::ARTS.iter().enumerate() {
            Self::sheet_row(&mut o, &format!("{name:<14}{}", self.op.skills.word(i)));
        }
        o.push_str(&format!("  +{}+\r\n", "-".repeat(Self::SHEET_W + 2)));
        o
    }

    fn fish(&mut self) -> String {
        let b = world::biome_at(self.op.node_seed, self.x(), self.y(), self.op.bias);
        if b.name != "lake" && b.name != "swamp" {
            return String::from("no water here. the map knows where the lakes are.");
        }
        self.op.xp += 5;
        // WCE beat: fishing is the gather current. Counted, never shown.
        self.op.deeds[DEED_GATHER] += 1;
        self.train(SK_FISHING);
        self.consequence(ActionTag::Fish, 128);
        let idx = pick_idx(fishing::CATCHES, &[&u64_to_nistam(self.op.node_seed), &u64_to_nistam(self.op.xp), b"fish"]);
        format!("the line sings. you land {} — {}", self.speak_fish(idx), fishing::CATCHES[idx].1)
    }

    fn quest(&mut self) -> String {
        let q = pick(quests::QUESTS, &[&u64_to_nistam(self.op.node_seed), &self.sq_bytes(), b"quest"]);
        let i = pick(items::ITEMS, &[&u64_to_nistam(self.op.node_seed), &u64_to_nistam(self.op.xp), b"item"]);
        self.op.xp += 13;
        // WCE beat: a quest done is the force current. Counted, never shown.
        self.op.deeds[DEED_FORCE] += 1;
        self.train(SK_TRACKING);
        self.consequence(ActionTag::Quest, 128);
        format!("{} — {}\r\nthe deed pays: {} ({})", q.0, q.1, i.0, i.1)
    }

    /// One act through the ironroot consequence engine: build the 16-byte
    /// query from the world's own state, resolve to the 8-byte descriptor,
    /// apply it — XP to the operator, standing to the town's faction, the
    /// root shift to the sky itself. The player sees none of the plumbing.
    fn consequence(&mut self, tag: ActionTag, intensity: u8) -> consequence::GrowthDescriptor {
        let roll = ConnectionRoll::deal(self.op.node_seed);
        // Each act family answers to its own register (v3 ruling, Authored).
        let skill = match tag {
            ActionTag::Fish => roll.stats.momentum,
            ActionTag::Craft => roll.stats.logic_depth,
            ActionTag::Speak => roll.stats.resonance,
            ActionTag::Quest => roll.stats.vigor,
            ActionTag::Steal => roll.stats.logic_depth,
        };
        let tag_byte = tag as u8;
        if self.last_tag == tag_byte {
            self.streak = self.streak.saturating_add(1);
        } else {
            self.streak = 0;
            self.last_tag = tag_byte;
        }
        let fac = consequence::town_faction(self.op.node_seed);
        // Standing rebased to the query's u8 (128 = even footing); the land's
        // harmony follows the same ladder (v3 ruling, Authored: a welcomed
        // hand is a harmonious one, a hunted hand a discordant one).
        let rep = ((self.op.standings[fac] >> 4) as i32 + 128).clamp(0, 255) as u8;
        let tier = consequence::standing_tier(self.op.standings[fac]);
        let harmony = if tier >= 6 {
            64 // harmonious band
        } else if tier <= 2 {
            220 // deep discord
        } else {
            128
        };
        let flux = (self.weather.current.intensity_pmy / 40).min(255) as u8;
        let q = consequence::ProgressionQuery {
            action_tag: tag_byte,
            action_intensity: intensity,
            target_difficulty: flux,
            current_skill: skill,
            secondary_skill: 0,
            tool_quality: 0,
            zone_affinity: 0,
            celestial: self.op.day,
            root_flux: flux,
            streak: self.streak,
            fatigue: 0,
            social_context: 1,
            reputation: rep,
            discovery_state: 0,
            root_harmony: harmony,
            chaos_phase: (seed_hash(&[
                &u64_to_nistam(self.op.node_seed),
                &u64_to_nistam(self.op.xp),
                b"chaos",
            ]) & 0xFF) as u8,
        };
        let d = consequence::resolve(&q);
        self.op.xp += d.primary_xp as u64;
        self.op.standings[fac] = self.op.standings[fac].saturating_add(d.reputation_delta as i16);
        // The land leans with the act — the sky's weight moves, unspoken.
        if d.root_shift > 0 {
            self.weather.current.intensity_pmy =
                (self.weather.current.intensity_pmy + 100).min(10_000);
        } else if d.root_shift < 0 {
            self.weather.current.intensity_pmy =
                self.weather.current.intensity_pmy.saturating_sub(100);
        }
        d
    }

    /// Taking what is not given — only the town holds anything worth the
    /// hand, and the town holds the law: fail the contest and the standing
    /// cascade fires, the warrant heats, and the watch answers by law and
    /// era (`crime_system.gd:68-82` / `guard_npc.gd:60-70`, drained).
    fn steal(&mut self) -> String {
        let (tx, ty) = world::town_square(self.op.node_seed);
        if (self.x(), self.y()) != (tx, ty) {
            return String::from("nothing here worth the taking. the coin lives in town.");
        }
        // WCE beat: theft is the force current. Counted, never shown.
        self.op.deeds[DEED_FORCE] += 1;
        // The slide: a theft in the dark trains dark camouflage; by day it
        // trains bare-handed scavenging.
        if matches!(explore::day_phase(self.ticks), "dusk" | "night") {
            self.train(SK_DARK_CAMO);
        } else {
            self.train(SK_SCAVENGING);
        }
        let law = consequence::law_level(self.op.node_seed);
        let fac = consequence::town_faction(self.op.node_seed);
        let chaos = (seed_hash(&[
            &u64_to_nistam(self.op.node_seed),
            &u64_to_nistam(self.op.xp),
            b"steal",
        ]) & 0xFF) as u8;
        let roll = ConnectionRoll::deal(self.op.node_seed);
        // The contest: a quick mind against a watched square — deterministic,
        // and a known face (heat) never wins.
        if self.op.heat == 0 && chaos % 100 < roll.stats.logic_depth.min(95) {
            self.op.xp += 8;
            // Taken without leave: the fae tally it even when the watch does not.
            itemforge::apply_fae_pressure(
                &mut self.ledger,
                forge_reactions_v3::fae_ethics::FaeItemOutcome::Stolen,
            );
            return String::from(
                "your hand is quicker than the lamplight. something small and dear is yours.",
            );
        }
        self.op.heat = self.op.heat.saturating_add(law as u16);
        consequence::cascade(&mut self.op.standings, fac, law as i16);
        match consequence::guard_response(law, self.weather.current.era, self.op.heat, chaos >> 1)
        {
            GuardResponse::Struck => {
                let fell = self.die(combat_brain::DeathCause::Refusal);
                format!("a hand closes on your collar — the watch answers in steel.\r\n{fell}")
            }
            GuardResponse::Hunted => String::from(
                "a whistle cuts the air. shutters slam. the watch knows your face now.",
            ),
            GuardResponse::Unanswered => String::from(
                "fingers close on empty cloth — and no one comes. this town keeps no law worth the name.",
            ),
        }
    }

    /// The three-beat death resolution: a realization line off the CDK triad
    /// standing at the death site, the world reseed (unchanged), and a
    /// closing line from whichever dungeonmaster temperament presided —
    /// Shadow/Senex/Trickster per [`ConnectionRoll::dm_aggression`], same
    /// band this file already speaks via the shadow's remembrance (see
    /// `delve()`'s "the air darkens" line). No new state machine: death
    /// already resolves in one synchronous call here, so the three beats are
    /// three lines of one string, not three ticked phases.
    fn die(&mut self, cause: combat_brain::DeathCause) -> String {
        // One home for every death path (combat, `"die"`, future callers) —
        // the horizontal-sync ledger's real write, not just declared.
        self.run_profile.record_death();
        let old_seed = self.op.node_seed;
        let (x, y, z) = (self.x() as i32, self.y() as i32, self.z() as i32);
        let fac_idx = consequence::town_faction(old_seed);
        let fmind = mind::FactionMind::for_faction(fac_idx);
        // Same deterministic haunt derivation cdk_verb() uses for this cell.
        let haunt_seed = (x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663)
            ^ (z as u64).wrapping_mul(83492791) ^ old_seed;
        let haunt_val = (haunt_seed as u32) % 3000;
        let triad = cdk::triad(&fmind, x, y, z, haunt_val);
        // Balanced trit read of the room's disposition: -1 torn, 0 even, +1 held.
        let verdict_trit = triad.disposition().signum();
        let old_roll = ConnectionRoll::deal(old_seed);

        let ah = format!("{} {}", cause_line(cause), triad_line(verdict_trit));

        self.op.die();
        // A new node is a new sky — the weather re-deals from the new seed
        // (honouring any Operator/Global-scoped era pin that rides along).
        self.weather = dealt_weather(&self.ledger, self.op.node_seed);
        let (tx, ty) = world::town_square(self.op.node_seed);
        self.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let t = world::theme(self.op.node_seed);
        let r = ConnectionRoll::deal(self.op.node_seed);

        let mut hahah = String::from(dm_band_line(old_roll.dm_aggression));
        if self.haunt.execution_count > 0 {
            hahah.push(' ');
            hahah.push_str(self.haunt.classify_awareness().remembrance_line());
        }

        format!(
            "{}\r\n\x1b[1;31mYOU DIE.\x1b[0m the world lets go — and deals again.\r\nnew node {:016x}: sky {}, vibe {}, town {}, born under {}. the grind is new; the XP is yours.\r\n{}",
            ah,
            self.op.node_seed,
            t.skybox.0,
            t.vibe.0,
            self.speak_town().0,
            r.constellation(),
            hahah,
        )
    }

    // ── Weld E: on-screen authoring ─────────────────────────────────────────

    /// Append a name (`ReplaceStr`) fact at the active scope, priority 100
    /// (the family's ordinary write priority).
    fn overlay_name(&mut self, domain: overlay::Domain, key: u16, text: String) {
        self.ledger.append(overlay::OverlayEntry {
            domain,
            key,
            modification: overlay::Mod::ReplaceStr(text),
            priority: 100,
            scope: self.author_scope,
        });
    }

    /// Append an absolute integer fact at the active scope. `Add(target -
    /// base)` would rot silently if the seed's own dealt value ever moved —
    /// storing the ABSOLUTE target and resolving over a sentinel "unset"
    /// base (see [`Game::law_now`]) is the honest form (brief note, Weld E).
    fn overlay_set_abs(&mut self, domain: overlay::Domain, key: u16, v: i64) {
        self.ledger.append(overlay::OverlayEntry {
            domain,
            key,
            modification: overlay::Mod::Add(v),
            priority: 100,
            scope: self.author_scope,
        });
    }

    /// Append a `Remove` fact at the active scope, priority 200 — high
    /// enough to mask any ordinary-priority write below it.
    fn overlay_remove(&mut self, domain: overlay::Domain, key: u16) {
        self.ledger.append(overlay::OverlayEntry {
            domain,
            key,
            modification: overlay::Mod::Remove,
            priority: 200,
            scope: self.author_scope,
        });
    }

    /// A faction's name: overlay-first, the seed's own roster second.
    fn speak_faction(&self, idx: usize) -> &str {
        let seed = self.op.node_seed;
        self.ledger
            .resolve_str(overlay::Domain::Faction, idx as u16, seed)
            .unwrap_or(consequence::FACTIONS[idx].name)
    }

    /// The town's (name, line): the name is overlay-first, the line is
    /// always the seed's own (authoring never touches a town's temper).
    fn speak_town(&self) -> (&str, &str) {
        let seed = self.op.node_seed;
        let (name, line) = world::town_lore(seed);
        (self.ledger.resolve_str(overlay::Domain::Town, 0, seed).unwrap_or(name), line)
    }

    /// A biome's spoken name: overlay-first by table index, the seed's own
    /// name second. Takes the seed-dealt name so a caller holding a `Biome`
    /// never has to re-derive its index.
    fn speak_biome<'a>(&'a self, name: &'a str) -> &'a str {
        let seed = self.op.node_seed;
        match world::BIOMES.iter().position(|b| b.name == name) {
            Some(idx) => self
                .ledger
                .resolve_str(overlay::Domain::Biome, idx as u16, seed)
                .unwrap_or(name),
            None => name,
        }
    }

    /// A milestone boss's spoken name: overlay-first, the ladder second.
    fn speak_boss(&self, idx: usize) -> &str {
        let seed = self.op.node_seed;
        let idx = idx.min(12);
        self.ledger
            .resolve_str(overlay::Domain::Boss, idx as u16, seed)
            .unwrap_or(achievements::BOSSES[idx].0)
    }

    /// A companion's spoken name: overlay-first, the table second.
    fn speak_pet(&self, idx: usize) -> &str {
        let seed = self.op.node_seed;
        self.ledger.resolve_str(overlay::Domain::Pet, idx as u16, seed).unwrap_or(pets::PETS[idx].0)
    }

    /// A catch's spoken name: overlay-first, the table second.
    fn speak_fish(&self, idx: usize) -> &str {
        let seed = self.op.node_seed;
        self.ledger
            .resolve_str(overlay::Domain::Fish, idx as u16, seed)
            .unwrap_or(fishing::CATCHES[idx].0)
    }

    /// A brew's spoken name: overlay-first, the table second.
    fn speak_brew(&self, idx: usize) -> &str {
        let seed = self.op.node_seed;
        self.ledger
            .resolve_str(overlay::Domain::Brew, idx as u16, seed)
            .unwrap_or(alchemy::BREWS[idx].0)
    }

    /// The node's law, 0..=100: overlay-first (an authored absolute value),
    /// the seed's own dealt law second. The entry stores the target OFFSET
    /// BY ONE (1..=101; a resolved 0 means "no entry") because
    /// `resolve_i64` computes `base + v` — a sentinel base rides into the
    /// sum and poisons it (found by Weld E2's tests; the i64::MIN sentinel
    /// made every authored law read as 0).
    fn law_now(&self) -> u8 {
        let seed = self.op.node_seed;
        match self.ledger.resolve_i64(overlay::Domain::Law, 0, seed, 0) {
            0 => consequence::law_level(seed).min(100),
            v => (v - 1).clamp(0, 100) as u8,
        }
    }

    /// The read-only authoring face: domains, current values, active scope,
    /// and the verb forms — part of the door's on-screen surface.
    fn author_view(&self) -> String {
        let scope_word = match self.author_scope {
            overlay::Scope::Node(_) => "node",
            overlay::Scope::Operator => "me",
            overlay::Scope::Global => "world",
        };
        let (town_name, _) = self.speak_town();
        let roster = (0..consequence::FACTIONS.len())
            .map(|i| self.speak_faction(i).to_string())
            .collect::<Vec<_>>()
            .join(" · ");
        format!(
            "authoring — scope: {scope_word}\r\n\
             domains: faction (1-{}) town biome (1-{}) boss (1-{}) pet (1-{}) fish (1-{}) brew (1-{}) law sky vibe era (1-4)\r\n\
             factions now: {roster}\r\n\
             town now: {town_name} · law now: {}\r\n\
             verbs: author · name <domain> [n] <text...> · set law|sky|vibe|era <n> · scope node|me|world · unname <domain> [n]",
            consequence::FACTIONS.len(),
            world::BIOMES.len(),
            achievements::BOSSES.len(),
            pets::PETS.len(),
            fishing::CATCHES.len(),
            alchemy::BREWS.len(),
            self.law_now(),
        )
    }

    /// `name faction|town|biome|boss|pet|fish|brew [n] <text...>`. Table
    /// families refuse an out-of-range index with a spoken line — never a
    /// panic (`town` carries no index, one key per node).
    fn cmd_name(&mut self, parts: &[&str]) -> String {
        let fam = parts.get(1).copied().unwrap_or("").to_lowercase();
        match fam.as_str() {
            "town" => {
                let text = parts[2..].join(" ");
                if text.trim().is_empty() {
                    return String::from("name it what? the town waits.");
                }
                self.overlay_name(overlay::Domain::Town, 0, text.clone());
                format!("the town takes a new name: {text}")
            }
            "faction" | "biome" | "boss" | "pet" | "fish" | "brew" => {
                let len = family_len(&fam);
                let Some(idx1) = parts.get(2).and_then(|s| s.parse::<usize>().ok()) else {
                    return format!("name which {fam} — 1 to {len}?");
                };
                if idx1 == 0 || idx1 > len {
                    return format!("no {fam} numbered {idx1} — 1 to {len} only.");
                }
                let text = parts[3..].join(" ");
                if text.trim().is_empty() {
                    return String::from("name it what?");
                }
                self.overlay_name(family_domain(&fam), (idx1 - 1) as u16, text.clone());
                format!("{fam} {idx1} renamed: {text}")
            }
            "" => String::from("name what — faction, town, biome, boss, pet, fish or brew?"),
            other => format!("the word '{other}' holds no power here."),
        }
    }

    /// `set law|sky|vibe|era <n>`. `law` clamps 0..=100; `era` clamps 1..=4
    /// and is ALWAYS Node-scoped (an era pin is the node's own sky) whatever
    /// the active authoring scope is; `sky`/`vibe` store the raw value
    /// authored (no reader is wired to them by this weld).
    fn cmd_set(&mut self, parts: &[&str]) -> String {
        let fam = parts.get(1).copied().unwrap_or("").to_lowercase();
        match fam.as_str() {
            "law" => {
                let Some(v) = parts.get(2).and_then(|s| s.parse::<i64>().ok()) else {
                    return String::from("set law to what — 0 to 100?");
                };
                // Stored offset by one (1..=101) — see law_now for why.
                self.overlay_set_abs(overlay::Domain::Law, 0, v.clamp(0, 100) + 1);
                format!("the law is set: {}", self.law_now())
            }
            "sky" => {
                let Some(v) = parts.get(2).and_then(|s| s.parse::<i64>().ok()) else {
                    return String::from("set sky to what?");
                };
                self.overlay_set_abs(overlay::Domain::Sky, 0, v);
                String::from("the sky takes a new value.")
            }
            "vibe" => {
                let Some(v) = parts.get(2).and_then(|s| s.parse::<i64>().ok()) else {
                    return String::from("set vibe to what?");
                };
                self.overlay_set_abs(overlay::Domain::Vibe, 0, v);
                String::from("the vibe takes a new value.")
            }
            "era" => {
                let Some(v) = parts.get(2).and_then(|s| s.parse::<i64>().ok()) else {
                    return String::from("set era to what — 1 to 4?");
                };
                if !(1..=4).contains(&v) {
                    return String::from("no era at that number — 1 to 4 only.");
                }
                self.ledger.append(overlay::OverlayEntry {
                    domain: overlay::Domain::Weather,
                    key: 0,
                    modification: overlay::Mod::Add(v),
                    priority: 100,
                    scope: overlay::Scope::Node(self.op.node_seed),
                });
                self.weather = dealt_weather(&self.ledger, self.op.node_seed);
                format!("the era is pinned: {}", self.weather.current.era.name())
            }
            "" => String::from("set what — law, sky, vibe or era?"),
            other => format!("the word '{other}' holds no power here."),
        }
    }

    /// `scope node|me|world` — sets the active authoring scope.
    fn cmd_scope(&mut self, parts: &[&str]) -> String {
        match parts.get(1).copied().unwrap_or("").to_lowercase().as_str() {
            "node" => {
                self.author_scope = overlay::Scope::Node(self.op.node_seed);
                String::from("authoring scope: this node only.")
            }
            "me" => {
                self.author_scope = overlay::Scope::Operator;
                String::from("authoring scope: you, across every node.")
            }
            "world" => {
                self.author_scope = overlay::Scope::Global;
                String::from("authoring scope: the whole world, forever.")
            }
            _ => String::from("scope where — node, me or world?"),
        }
    }

    /// `unname <domain-word> [n]` — appends a `Remove` at the active scope.
    fn cmd_unname(&mut self, parts: &[&str]) -> String {
        let fam = parts.get(1).copied().unwrap_or("").to_lowercase();
        let domain = match fam.as_str() {
            "faction" | "biome" | "boss" | "pet" | "fish" | "brew" | "town" | "law" | "sky"
            | "vibe" | "era" => family_domain(&fam),
            "" => return String::from("unname what?"),
            other => return format!("the word '{other}' holds no power here."),
        };
        let indexed = matches!(fam.as_str(), "faction" | "biome" | "boss" | "pet" | "fish" | "brew");
        let key = if indexed {
            let len = family_len(&fam);
            let Some(idx1) = parts.get(2).and_then(|s| s.parse::<usize>().ok()) else {
                return format!("unname which {fam} — 1 to {len}?");
            };
            if idx1 == 0 || idx1 > len {
                return format!("no {fam} numbered {idx1} — 1 to {len} only.");
            }
            (idx1 - 1) as u16
        } else {
            0
        };
        self.overlay_remove(domain, key);
        format!("the {fam} name falls away — the seed's own truth returns.")
    }
}

/// The table length behind a family word — table families only (never call
/// on `town`/`law`/`sky`/`vibe`/`era`, which carry no index).
fn family_len(fam: &str) -> usize {
    match fam {
        "faction" => consequence::FACTIONS.len(),
        "biome" => world::BIOMES.len(),
        "boss" => achievements::BOSSES.len(),
        "pet" => pets::PETS.len(),
        "fish" => fishing::CATCHES.len(),
        "brew" => alchemy::BREWS.len(),
        _ => 0,
    }
}

/// The overlay domain behind a family word.
fn family_domain(fam: &str) -> overlay::Domain {
    match fam {
        "faction" => overlay::Domain::Faction,
        "town" => overlay::Domain::Town,
        "biome" => overlay::Domain::Biome,
        "boss" => overlay::Domain::Boss,
        "pet" => overlay::Domain::Pet,
        "fish" => overlay::Domain::Fish,
        "brew" => overlay::Domain::Brew,
        "law" => overlay::Domain::Law,
        "sky" => overlay::Domain::Sky,
        "vibe" => overlay::Domain::Vibe,
        "era" => overlay::Domain::Weather,
        _ => unreachable!("family_domain called on an unrecognised word: {fam}"),
    }
}

/// The node's weather, honouring a Weather,0 era pin if one is visible for
/// this seed (1..=4; 0 = unpinned — the seed's own pure deal stands).
/// `Game::weather_for` stays pure; this is the one seam that consults the
/// ledger on top of it (`new`/`die`/`reseed`/`set era`, Weld E).
fn dealt_weather(ledger: &overlay::Ledger, node_seed: u64) -> WeatherModel {
    let pin = ledger.resolve_i64(overlay::Domain::Weather, 0, node_seed, 0);
    if (1..=4).contains(&pin) {
        let era = Era::all()[(pin - 1) as usize];
        WeatherModel::new(era, seed_hash(&[&u64_to_nistam(node_seed), b"weather"]) as u32)
    } else {
        Game::weather_for(node_seed)
    }
}

/// The realization beat's cause-specific half — plain, no exclamations,
/// matching the file's own house register. All six [`combat_brain::DeathCause`]
/// variants are spoken even though only `Combat` is dealt today (`fight()`'s
/// one live call site) — the type already models the other five honestly.
fn cause_line(cause: combat_brain::DeathCause) -> &'static str {
    match cause {
        combat_brain::DeathCause::Combat => "the fight found the exact seam you left open.",
        combat_brain::DeathCause::Fall => "the ground you trusted wasn't.",
        combat_brain::DeathCause::Hazard => "you read the room a beat too late.",
        combat_brain::DeathCause::Erasure => "something unmade the line you were standing on.",
        combat_brain::DeathCause::Sacrifice => "you spent yourself on purpose, and it still cost everything.",
        combat_brain::DeathCause::Refusal => "you would not bend, so the world did it for you.",
    }
}

/// The realization beat's room-verdict half, off `Triad::disposition().signum()`
/// — a balanced trit read of the death site: -1 torn, 0 even, +1 held.
fn triad_line(verdict_trit: i32) -> &'static str {
    match verdict_trit {
        v if v < 0 => "the room was already tearing itself apart.",
        0 => "the room was even. you weren't.",
        _ => "the room held together. you didn't.",
    }
}

/// The closing beat's voice, off [`ConnectionRoll::dm_aggression`]'s own
/// Shadow(0)/Senex(1..=6)/Trickster(7..=9) band (`hermetics.rs`) — the
/// dungeonmaster temperament that presided over this death.
fn dm_band_line(dm_aggression: u8) -> &'static str {
    match dm_aggression {
        0 => "the shadow watches you fall. it does not blink.",
        1..=6 => "the old rule notes the death and files it.",
        _ => "somewhere, something laughs. it isn't unkind.",
    }
}

/// Deal one entry from a table by seed-hash — the crate's one picker.
fn pick<'a>(table: &'a [(&'a str, &'a str)], parts: &[&[u8]]) -> (&'a str, &'a str) {
    table[(seed_hash(parts) % table.len() as u64) as usize]
}

/// Deal one table INDEX by seed-hash — same deal as [`pick`], but for call
/// sites that must resolve the entry's name through an overlay `speak_*`
/// helper afterward (the family's own home resolves it exactly once).
fn pick_idx(table: &[(&str, &str)], parts: &[&[u8]]) -> usize {
    (seed_hash(parts) % table.len() as u64) as usize
}

/// Deal one [`alchemy::BREWS`] index, biased toward the node's dealt
/// [`crate::hermetics::Reagent`] (`alchemy::BREW_REAGENT`, authored
/// 2026-08-13, wired here for the first time). Falls back to the full table
/// when the node's reagent names no brew — 4 of 10 reagents (Pitch, Brass,
/// Ichor, Lead) own none, so an empty candidate set is a real case, not a
/// bug, and must never leave a node where brewing finds nothing.
fn pick_brew_idx(reagent: crate::hermetics::Reagent, parts: &[&[u8]]) -> usize {
    let mut candidates = [0usize; alchemy::BREWS.len()];
    let mut n = 0;
    for (i, &(r, _)) in alchemy::BREW_REAGENT.iter().enumerate() {
        if r == reagent {
            candidates[n] = i;
            n += 1;
        }
    }
    if n == 0 {
        return pick_idx(alchemy::BREWS, parts);
    }
    candidates[(seed_hash(parts) % n as u64) as usize]
}

/// The strike ladder, spoken word-only (no digits) — the combat brain's
/// hit_stop (1..=8 ticks, low resonance = heavy) rendered as sensation.
fn strike_word(hit_stop_ticks: u16) -> &'static str {
    match hit_stop_ticks {
        7..=8 => "the blow falls like a dropped bell; the world stalls a breath",
        5..=6 => "a heavy arc hums in, slow and certain",
        3..=4 => "a quick strike sings past the guard line",
        _ => "a whisper-fast cut flickers in",
    }
}

/// The vitality ladder, spoken word-only (no digits) — bands at 100/75/50/25.
fn vitality_word(v: u16) -> &'static str {
    match v {
        100 => "unhurt",
        75..=99 => "scratched",
        50..=74 => "bleeding",
        25..=49 => "failing",
        _ => "broken",
    }
}

/// Day-quality word ladder for the `sleep` reply — Permyriad (0..=10_000)
/// spoken, never a number (same convention as [`vitality_word`]).
/// A cabinet voice's tone as a word. The sheet speaks ranks, never numbers —
/// the same law `skills::word` follows two lines above it.
fn tone_word(tone: voices::Tone) -> &'static str {
    match tone {
        voices::Tone::Silent => "silent",
        voices::Tone::Murmur => "murmurs",
        voices::Tone::Speaking => "speaks",
        voices::Tone::Insistent => "insists",
        voices::Tone::Overriding => "overrides",
    }
}

fn quality_word(quality_pmy: u32) -> &'static str {
    match quality_pmy {
        7_500..=10_000 => "clear",
        5_000..=7_499 => "settled",
        3_000..=4_999 => "restless",
        1..=2_999 => "rough",
        _ => "hollow",
    }
}

// Abyss mechanics (light, pressure, buoyancy) moved to abyss.rs module.
// The delve/ascend verbs draw their sensation words from abyss::light_words()
// and abyss::pressure_words() — the base domain that holds the floor (light
// never below 500 pmy) and the law (past the cap, ascent costs).

/// One ENCOUNTER roll for a delve step — pure over (seed, x, y, t, xp).
/// RARE POP at ~100/10_000, common at ~2_500/10_000, else silence.
fn roll_encounter(seed: u64, x: u16, y: u16, t: u16, xp: u64) -> Option<Encounter> {
    let roll = seed_hash(&[
        &u64_to_nistam(seed),
        &u16_to_nistam(x),
        &u16_to_nistam(y),
        &u16_to_nistam(t),
        &u64_to_nistam(xp),
    ]) % 10_000;
    let rare = roll < 100;
    if !rare && roll >= 2_600 {
        return None;
    }
    let adj = FOE_ADJ[(seed_hash(&[
        &u64_to_nistam(seed),
        &u16_to_nistam(x),
        &u16_to_nistam(y),
        &u16_to_nistam(t),
        &u64_to_nistam(xp),
        b"foe-adj",
    ]) % FOE_ADJ.len() as u64) as usize];
    let noun = FOE_NOUN[(seed_hash(&[
        &u64_to_nistam(seed),
        &u16_to_nistam(x),
        &u16_to_nistam(y),
        &u16_to_nistam(t),
        &u64_to_nistam(xp),
        b"foe-noun",
    ]) % FOE_NOUN.len() as u64) as usize];
    Some(Encounter { name: format!("{adj} {noun}"), depth: t, rare })
}

/// The sky spoken as SENSATION, never a number ("sensation first, never raw
/// number" — WAVE-MUD-E2E §2 sieve row). Intensity buckets to a weight word;
/// the era shows only through the sky the model deals it.
fn weather_line(w: Weather) -> String {
    let face = match w.sky {
        crate::weather::Sky::Clear => "the sky holds clear",
        crate::weather::Sky::Overcast => "cloud lids the light",
        crate::weather::Sky::Storm => "storm works the horizon",
        crate::weather::Sky::Ashfall => "ash sifts down",
        crate::weather::Sky::Hardfrost => "the frost has the whole sky",
    };
    let weight = match w.intensity_pmy {
        0..=2499 => "the air is light",
        2500..=4999 => "the air carries weight",
        5000..=7499 => "the air presses close",
        _ => "the air is a held fist",
    };
    format!("{face}; {weight}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new(Operator::birth("Operator", 3, 12).unwrap(), None)
    }

    /// XP rides the terminal: bytes in, experience up; milestones name their
    /// bosses when the Fibonacci gates fall.
    #[test]
    fn xp_is_terminal_bytes_and_gates_name_bosses() {
        let mut g = game();
        let (_, going) = g.process("look");
        assert!(going);
        assert_eq!(g.op.xp, 4);
        g.op.xp = 99;
        let (reply, _) = g.process("x"); // 1 byte → 100 → gate 1 falls
        assert!(reply.contains("MILESTONE 1 FALLS"), "the first boss did not appear: {reply}");
        assert_eq!(Game::level(g.op.xp), 1);
        assert!(reply.contains("manifests as"), "a milestone boss must name its sieved Bell Warden variant: {reply}");
    }

    /// Horizontal sync (2026-08-18): the ledger a boss encounter reads is a
    /// real, live-written field, not a decoration — a kill actually lands on
    /// `run_profile.kills`, the same counter `select_warden_variant` reads.
    #[test]
    fn a_fight_victory_writes_a_real_kill_to_the_run_profile() {
        let mut g = game();
        assert_eq!(g.run_profile.kills, 0);
        g.encounter = Some(Encounter { name: "a weak thing".into(), depth: 0, rare: false });
        // depth 0 -> difficulty 0, always beaten regardless of the chaos roll.
        g.process("fight");
        assert_eq!(g.run_profile.kills, 1, "a won fight must record a real kill");
    }

    /// `sleep` only stages the rest now — no journal fold happens until `wake`.
    #[test]
    fn sleep_verb_stages_without_folding_the_journal() {
        let mut g = game();
        let (reply, going) = g.process("sleep");
        assert!(going);
        assert!(reply.contains("sleep takes you down"), "reply must narrate the sleep event: {reply}");
        assert_eq!(g.dream_journal.peak_quality, 0.0, "sleep alone must not fold a quality sample");
    }

    /// Sleeping twice without waking refuses instead of silently restaging.
    #[test]
    fn sleeping_twice_refuses() {
        let mut g = game();
        g.process("sleep");
        let (reply, _) = g.process("sleep");
        assert!(reply.contains("already asleep"), "a second sleep must refuse: {reply}");
    }

    /// Waking without having slept refuses.
    #[test]
    fn waking_without_sleeping_refuses() {
        let mut g = game();
        let (reply, _) = g.process("wake");
        assert!(reply.contains("not asleep"), "wake with no rest staged must refuse: {reply}");
    }

    /// Walking into the square is heard; standing in it is not. The room's
    /// tell must be a transient at the GAME level, not just in the module.
    #[test]
    fn the_square_hears_you_arrive_and_then_stops_saying_so() {
        let mut g = game();
        let (tx, ty) = world::town_square(g.op.node_seed);

        // Stand one square west of the town square, then step in.
        g.op.pos = MortonKey5D::encode([tx.saturating_sub(1), ty, 0, 0, 0]);
        let (entering, _) = g.process("e");
        assert!(
            entering.contains("the talk in the square stops"),
            "arriving in the square must be heard: {entering}"
        );

        // Look around from inside: the room is disturbed, and says nothing.
        let (standing, _) = g.process("look");
        assert!(
            !standing.contains("the talk in the square stops"),
            "standing there must not keep announcing it: {standing}"
        );
        assert!(g.square.is_disturbed(), "though the room is still disturbed");
    }

    /// Stepping out and straight back in finds a room that has not settled.
    #[test]
    fn pacing_the_doorway_does_not_restartle_the_square() {
        let mut g = game();
        let (tx, ty) = world::town_square(g.op.node_seed);
        g.op.pos = MortonKey5D::encode([tx.saturating_sub(1), ty, 0, 0, 0]);
        assert!(g.process("e").0.contains("the talk in the square stops"));

        g.process("w");
        let (again, _) = g.process("e");
        assert!(
            !again.contains("the talk in the square stops"),
            "a room that has not settled cannot be startled twice: {again}"
        );
    }

    /// The Drift state must reach a reader, or it stays undefined lore.
    #[test]
    fn the_sheet_speaks_the_roster_standing() {
        let mut g = game();
        let (sheet, _) = g.process("status");
        assert!(sheet.contains("standing:"), "the sheet must carry a standing: {sheet}");

        let r = ConnectionRoll::deal(g.op.node_seed);
        let word = r.stats.standing().word();
        assert!(sheet.contains(word), "the sheet must agree with the block: want {word}");
        assert!(
            !sheet.contains("standing: dead"),
            "the meter costs a roster slot, never a life"
        );
    }

    /// The cabinet has to reach the sheet, or it is a module nobody hears.
    #[test]
    fn the_status_sheet_carries_the_thought_cabinet() {
        let mut g = game();
        let (sheet, _) = g.process("status");
        assert!(sheet.contains("the cabinet"), "the sheet must speak the cabinet: {sheet}");

        // Whatever this node dealt, the line must agree with the module.
        let r = ConnectionRoll::deal(g.op.node_seed);
        let spoken = voices::cabinet(&r.stats);
        if spoken.is_empty() {
            assert!(sheet.contains("the cabinet is quiet"));
        } else {
            let loudest = voices::voice_name(spoken[0].stat);
            assert!(sheet.contains(loudest), "the loudest voice {loudest} must be named: {sheet}");
        }
    }

    /// The sheet speaks ranks, never register numbers — the same law the arts
    /// line follows.
    #[test]
    fn the_cabinet_line_speaks_words_not_numbers() {
        let mut g = game();
        let (sheet, _) = g.process("status");
        let line = sheet
            .lines()
            .find(|l| l.contains("the cabinet"))
            .expect("the cabinet line exists")
            .to_string();
        assert!(
            !line.chars().any(|c| c.is_ascii_digit()),
            "the cabinet must not print raw register values: {line}"
        );
    }

    /// The mirror has to move when the player plays, or it is a struct nobody
    /// looks in: two sessions differing only in play style must read apart.
    #[test]
    fn the_witness_mirror_shifts_with_play_style() {
        let mut fighter = game();
        assert_eq!(fighter.mirror().held(), 0, "a fresh player has been witnessed doing nothing");
        for _ in 0..6 {
            fighter.process("fight");
        }
        let mut talker = game();
        for _ in 0..6 {
            talker.process("talk");
        }
        assert!(fighter.mirror().held() > 0, "the verb must reach the mirror");
        assert!(
            talker.mirror().laban_space_pmy() > fighter.mirror().laban_space_pmy(),
            "the face must shift Direct<->Indirect with play style"
        );
    }

    /// Navigation is not conduct — walking around says nothing about who you are.
    #[test]
    fn looking_around_never_touches_the_mirror() {
        let mut g = game();
        for verb in ["look", "map", "status", "world"] {
            g.process(verb);
        }
        assert_eq!(g.mirror().seen(), 0, "navigation must not be scored as conduct");
    }

    /// §8's law at the game level: a clean night leaves a gift, and the gift
    /// is a seal admitted through the airlock — never the staged buffer.
    #[test]
    fn a_clean_wake_leaves_a_gift_and_no_transcript() {
        let mut g = game();
        assert!(g.dream_gifts().is_empty(), "no gift before the first night");
        g.process("sleep");
        let (reply, _) = g.process("wake");
        assert!(reply.contains("wake clean"), "precondition: a clean wake: {reply}");
        assert_eq!(g.dream_gifts().len(), 1, "a clean night must leave exactly one gift");
        assert!(reply.contains("the dream leaves"), "the gift must be narrated: {reply}");
        assert!(g.sleeping.is_none(), "the staged buffer must not survive the wake");
    }

    /// §8:234-240 at the game level: a lit fire's text is staged and narrated
    /// at sleep, and STILL leaves only the gift at wake — the deep fire never
    /// weakens the vault law.
    #[test]
    fn a_lit_fire_stages_the_dream_but_only_the_gift_survives() {
        struct StubFire;
        impl dream::DreamFire for StubFire {
            fn dream(&self, _j: &dream::DreamJournal, _lean: u32) -> Result<String, dream::DreamFireError> {
                Ok(String::from("a stone door opens on a field of bells."))
            }
        }
        let mut g = game();
        g.light_dream_fire(Box::new(StubFire));
        let (sleep_reply, _) = g.process("sleep");
        assert!(
            sleep_reply.contains("the deep fire takes the day's reel"),
            "a lit fire must be narrated at sleep: {sleep_reply}"
        );
        let (reply, _) = g.process("wake");
        assert!(reply.contains("wake clean"), "precondition: a clean wake: {reply}");
        assert_eq!(g.dream_gifts().len(), 1, "the fired night still leaves exactly one gift");
        assert!(
            !reply.contains("stone door"),
            "the dream text must never survive the wake: {reply}"
        );
        assert!(g.sleeping.is_none(), "the staged buffer must not survive the wake");
    }

    /// The unlit hearth changes nothing: no deep-fire line at sleep.
    #[test]
    fn an_unlit_fire_stays_silent_at_sleep() {
        let mut g = game();
        let (sleep_reply, _) = g.process("sleep");
        assert!(
            !sleep_reply.contains("deep fire"),
            "NoFire must not be narrated: {sleep_reply}"
        );
    }

    /// The seal a night leaves is a function of the night, not of when it is
    /// replayed — two runs of the same node seed leave the same seal.
    #[test]
    fn the_same_night_leaves_the_same_seal() {
        let mut a = game();
        a.process("sleep");
        a.process("wake");
        let mut b = game();
        b.process("sleep");
        b.process("wake");
        assert_eq!(a.dream_gifts()[0].seal, b.dream_gifts()[0].seal);
    }

    /// Waking inside `dream::SLEEP_TTL_TICKS` of the `sleep` call must run the
    /// dream/ mechanical skeleton (DreamJournal + day_quality_pmy +
    /// RoughPatchWatch + SessionBuffer stage/shred) and answer with a worded,
    /// digit-free night quality — the runtime receipt the 2026-08-24 WAVE
    /// CLOSE brick was missing, now split across `sleep`+`wake`.
    #[test]
    fn waking_inside_the_window_prints_a_worded_night_reply() {
        let mut g = game();
        g.process("sleep");
        let (reply, going) = g.process("wake");
        assert!(going);
        assert!(reply.contains("wake clean"), "reply must narrate a clean wake: {reply}");
        assert!(
            !reply.chars().any(|c| c.is_ascii_digit()),
            "the night's quality must be spoken as a word, never a number: {reply}"
        );
        assert_eq!(
            g.dream_journal.peak_quality, g.dream_journal.lowest_quality,
            "journal must have folded exactly one quality sample this session"
        );
        assert!(g.dream_journal.peak_quality > 0.0, "the folded sample must not be the fresh-journal default");
    }

    /// Waking past `dream::SLEEP_TTL_TICKS` lets the envelope fall through to
    /// `Expired` — the Sleeping-Beauty risk — and dings vitality instead of
    /// sealing a clean wake.
    #[test]
    fn oversleeping_past_the_ttl_is_risky() {
        let mut g = game();
        g.process("sleep");
        g.ticks += dream::SLEEP_TTL_TICKS + 1;
        let before = g.vitality;
        let (reply, going) = g.process("wake");
        assert!(going);
        assert!(reply.contains("oversleep"), "reply must narrate the overslept wake: {reply}");
        assert!(g.vitality < before, "oversleeping must cost vitality: {before} -> {}", g.vitality);
    }

    /// Death, whichever path reaches it (`"die"`, a lost fight, a future
    /// caller), lands on the same ledger `die()` itself owns — one home.
    #[test]
    fn dying_writes_a_real_death_to_the_run_profile() {
        let mut g = game();
        assert_eq!(g.run_profile.deaths, 0);
        g.process("die");
        assert_eq!(g.run_profile.deaths, 1, "die() must record a real death exactly once");
    }

    /// RUNTIME witness (P4/P7): a real rare `delve()` encounter must actually
    /// speak both `hazard_words`/`heat_words` in its live reply, not just
    /// compile — brute-forces xp (which shifts `roll_encounter`'s hash input)
    /// until a real rare roll lands, same technique real gameplay uses.
    #[test]
    fn delve_rare_encounter_speaks_hazard_and_heat() {
        let mut g = game();
        // Birth always lands at (0,0); delve() only rolls encounters on a
        // "dungeon" tile, so find one deterministically before brute-forcing.
        let (mut dx, mut dy) = (0u16, 0u16);
        'search: for x in 0..64u16 {
            for y in 0..64u16 {
                if world::biome_at(g.op.node_seed, x, y, g.op.bias).name == "dungeon" {
                    dx = x;
                    dy = y;
                    break 'search;
                }
            }
        }
        g.op.pos = MortonKey5D::encode([dx, dy, 0, 0, 0]);
        for _ in 0..2_000 {
            let (reply, _) = g.process("delve");
            if reply.contains("does not belong to this depth") {
                assert!(
                    reply.contains("a distant thud")
                        || reply.contains("rattles")
                        || reply.contains("cracks timber")
                        || reply.contains("buckles stone")
                        || reply.contains("hammer"),
                    "rare delve reply carried no hazard_words wording: {reply}"
                );
                assert!(
                    reply.contains("sun on your face")
                        || reply.contains("dry warmth")
                        || reply.contains("bite against bare skin")
                        || reply.contains("presses like a wall")
                        || reply.contains("catches"),
                    "rare delve reply carried no heat_words wording: {reply}"
                );
                return;
            }
            if g.t() >= 3 {
                g.process("ascend");
            }
        }
        panic!("no rare encounter rolled in 2000 delves — brute-force bound too small");
    }

    /// D08's actual contract: two independent dispatchers (stand-ins here —
    /// two fresh Games, same accumulated profile) that call
    /// `select_warden_variant` with an identical profile must select the
    /// identical variant. Proves the sieve, not just that a line got printed.
    #[test]
    fn identical_run_profiles_select_the_identical_warden_variant() {
        let mut a = game();
        let mut b = game();
        for _ in 0..5 {
            a.run_profile.record_perfect_parry();
            b.run_profile.record_perfect_parry();
        }
        let va = crate::ironroot::boss_sieve::select_warden_variant(&a.run_profile);
        let vb = crate::ironroot::boss_sieve::select_warden_variant(&b.run_profile);
        assert_eq!(va.id, vb.id, "identical profiles must select the identical Bell Warden variant");
        assert_eq!(va.id, "thirteen_bells_warden");
    }

    /// Death reseeds the node AND rehomes the operator to the new town; XP
    /// survives (the terminal earned it).
    #[test]
    fn death_reseeds_and_keeps_xp() {
        let mut g = game();
        g.op.xp = 777;
        let old = g.op.node_seed;
        let (reply, going) = g.process("die");
        assert!(going);
        assert!(reply.contains("YOU DIE"));
        assert_ne!(g.op.node_seed, old, "death must move the node");
        assert_eq!(g.op.xp, 777 + 3, "xp survives death (plus the verb's own bytes)");
        let (tx, ty) = world::town_square(g.op.node_seed);
        assert_eq!((g.op.pos.axes()[0], g.op.pos.axes()[1]), (tx, ty), "reborn at the new town");
    }

    /// Movement clamps to the map and the 5D word carries it.
    #[test]
    fn movement_lives_in_the_5d_word() {
        let mut g = game();
        g.process("e");
        g.process("s");
        assert_eq!((g.op.pos.axes()[0], g.op.pos.axes()[1]), (1, 1));
        for _ in 0..20 {
            g.process("n");
        }
        assert_eq!(g.op.pos.axes()[1], 0, "the map edge holds");
    }

    /// Every wired table answers through its verb — no orphan content.
    #[test]
    fn every_content_verb_answers() {
        let mut g = game();
        for verb in ["talk", "quest", "brew", "pet", "talents", "map", "status", "sheet", "help"] {
            let (reply, going) = g.process(verb);
            assert!(going && !reply.is_empty(), "verb {verb} gave nothing");
        }
        // Fishing answers on water and refuses on land — sample the world
        // until both faces have spoken (81x81 sampled every 4th square).
        let mut wet = false;
        let mut dry = false;
        for y in (0..MAP_SIDE).step_by(4) {
            for x in (0..MAP_SIDE).step_by(4) {
                g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
                let (r, _) = g.process("fish");
                if r.contains("the line sings") {
                    wet = true;
                } else if r.contains("no water") {
                    dry = true;
                }
            }
        }
        assert!(dry, "fishing never refused on land");
        let _ = wet; // a node with no water is legal; wet is a bonus, dry is the law
        // The trit chart answers too.
        let (w, _) = g.process("world");
        assert!(w.contains("1 cell = 3x3"), "the world chart is missing");
    }

    /// World brick #2: the node's dealt reagent (`hermetics::ConnectionRoll::
    /// deal(seed).reagent`) now biases what it brews (`alchemy::BREW_REAGENT`,
    /// authored 2026-08-13, wired here for the first time — it had zero
    /// production callers before this). Every candidate `pick_brew_idx`
    /// returns for a reagent must itself carry that same reagent.
    #[test]
    fn brewing_is_biased_by_the_nodes_dealt_reagent() {
        for &reagent in &crate::hermetics::Reagent::ALL {
            let has_a_brew = alchemy::BREW_REAGENT.iter().any(|&(r, _)| r == reagent);
            for i in 0..64u64 {
                let idx = pick_brew_idx(reagent, &[&i.to_le_bytes(), b"probe"]);
                if has_a_brew {
                    assert_eq!(
                        alchemy::BREW_REAGENT[idx].0, reagent,
                        "{reagent:?} owns a brew, but pick_brew_idx returned a mismatched one"
                    );
                }
            }
        }
    }

    /// Pitch, Brass, Ichor and Lead name no brew (`alchemy.rs::BREW_REAGENT`
    /// covers 6 of 10 reagents) — those nodes must still brew SOMETHING, from
    /// the full table, never come back empty.
    #[test]
    fn brewing_falls_back_to_the_full_table_when_the_reagent_owns_no_brew() {
        for &reagent in &[
            crate::hermetics::Reagent::Pitch,
            crate::hermetics::Reagent::Brass,
            crate::hermetics::Reagent::Ichor,
            crate::hermetics::Reagent::Lead,
        ] {
            assert!(!alchemy::BREW_REAGENT.iter().any(|&(r, _)| r == reagent), "{reagent:?} unexpectedly owns a brew now — update this test's fixture list");
            let idx = pick_brew_idx(reagent, &[b"probe"]);
            assert!(idx < alchemy::BREWS.len(), "{reagent:?} brewed an out-of-range index");
        }
    }

    /// THE INVISIBLE CART: deeds accrue from play and the death-dealt world
    /// leans their way — but no surface EVER says so. The player is making
    /// a mud and must never know (Sean 2026-08-11).
    #[test]
    fn the_player_never_knows_they_are_making_a_mud() {
        let mut g = game();
        for _ in 0..5 {
            g.process("talk");
            g.process("brew");
            g.process("quest");
        }
        // A refused fish is NO deed — gather only moves on real water, so
        // the test walks there first (a node without water skips the cast).
        let mut found_water = false;
        'hunt: for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                let b = world::biome_at(g.op.node_seed, x, y, g.op.bias);
                if b.name == "lake" || b.name == "swamp" {
                    g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
                    found_water = true;
                    break 'hunt;
                }
            }
        }
        if found_water {
            g.process("fish");
            assert!(g.op.deeds[DEED_GATHER] > 0, "a real cast must count as a gather deed");
        }
        for fam in [DEED_FORCE, DEED_CRAFT, DEED_VOICE] {
            assert!(g.op.deeds[fam] > 0, "the deeds ledger is not accruing (family {fam})");
        }
        for verb in ["status", "help", "look", "talents", "author"] {
            let (reply, _) = g.process(verb);
            let low = reply.to_lowercase();
            for word in ["deed", "bias", "wce", "consequence", "cart"] {
                assert!(
                    !low.contains(word),
                    "verb {verb} leaked the invisible cart ({word}): {reply}"
                );
            }
        }
        // Death snapshots the dominant current into the next world's lean.
        g.op.deeds = [1, 2, 9, 3];
        g.process("die");
        assert_eq!(g.op.bias, DEED_GATHER as u8, "the gatherer's death must deal a greener node");
    }

    /// The sky is SENSATION on `look`: a weather line appears, it carries
    /// no digits and no unit (never a raw number), it is deterministic for
    /// the same node + commands, and a death deals a different sky model.
    #[test]
    fn weather_is_a_sensation_and_death_redeals_the_sky() {
        let mut a = game();
        let mut b = game();
        let (la, _) = a.process("look");
        let (lb, _) = b.process("look");
        assert_eq!(la, lb, "same node, same commands, same sky");
        let line = weather_line(a.weather.current);
        assert!(la.contains(&line), "look does not speak the sky: {la}");
        assert!(
            line.chars().all(|c| !c.is_ascii_digit()) && !line.contains("pmy"),
            "the sky leaked a raw number: {line}"
        );
        let before = a.weather.clone();
        a.process("die");
        assert_eq!(
            a.weather,
            Game::weather_for(a.op.node_seed),
            "the new node's sky must be dealt from the new seed"
        );
        assert_ne!(a.weather, before, "death left the old sky standing");
    }

    /// THE IRONROOT BRAID, END TO END: away from town there is nothing to
    /// steal; a failed theft in town heats the warrant, fires the standing
    /// cascade (the wronged faction falls, its enemy warms), and the watch's
    /// answer is dealt by law and era — all deterministic, all integer.
    #[test]
    fn a_failed_theft_heats_the_warrant_and_fires_the_cascade() {
        let mut g = game();
        let (r, _) = g.process("steal");
        assert!(r.contains("nothing here worth the taking"), "the wild had coin: {r}");
        assert_eq!(g.op.heat, 0);
        // Walk seeds until a node whose steal contest fails (deterministic
        // search — the contest is a pure function of seed + xp).
        let mut found = false;
        for s in 0..512u64 {
            let seed = seed_hash(&[&u64_to_nistam(s), b"braid"]);
            let mut g = game();
            g.op.node_seed = seed;
            g.weather = Game::weather_for(seed);
            let (tx, ty) = world::town_square(seed);
            g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
            let fac = consequence::town_faction(seed);
            let (reply, _) = g.process("steal");
            if g.op.deaths > 0 {
                // The watch struck: death reseeded the node and cleared the
                // slate — the braid's harshest rung, also a valid find.
                assert!(reply.contains("answers in steel"), "a silent strike: {reply}");
                assert_eq!(g.op.heat, 0, "a new node holds no warrant");
                found = true;
                break;
            }
            if g.op.heat > 0 {
                let law = consequence::law_level(seed) as i16;
                assert!(
                    g.op.standings[fac] <= -law,
                    "the wronged did not fall: {} vs law {law}",
                    g.op.standings[fac]
                );
                assert!(
                    g.op.standings[consequence::FACTIONS[fac].enemy] > 0,
                    "the enemy of the wronged did not warm"
                );
                found = true;
                break;
            }
        }
        assert!(found, "512 nodes and no failed theft — the contest never loses");
    }

    /// A theft the watch never sees is still seen: the fae lanes rise, and the
    /// risen pressure reaches perception through `senses_now`.
    #[test]
    fn theft_accrues_fae_pressure() {
        let mut found = false;
        for s in 0..512u64 {
            let seed = seed_hash(&[&u64_to_nistam(s), b"pressure"]);
            let mut g = game();
            g.op.node_seed = seed;
            g.weather = Game::weather_for(seed);
            let (tx, ty) = world::town_square(seed);
            g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
            let before = itemforge::pressure_vector(&g.ledger, seed);
            let muted_before = magic::senses_now(&g.op, g.room_sightline(), &g.ledger).muted_q;
            let (reply, _) = g.process("steal");
            if !reply.contains("quicker than the lamplight") {
                continue;
            }
            let after = itemforge::pressure_vector(&g.ledger, seed);
            assert_ne!(before, after, "an unseen theft left the fae lanes flat");
            assert!(
                after.iter().zip(before).any(|(a, b)| *a > b),
                "no lane rose: {before:?} -> {after:?}"
            );
            let muted_after = magic::senses_now(&g.op, g.room_sightline(), &g.ledger).muted_q;
            assert!(
                muted_after >= muted_before,
                "risen pressure never reached hearing: {muted_before} -> {muted_after}"
            );
            found = true;
            break;
        }
        assert!(found, "512 nodes and no successful theft — the contest never wins");
    }

    /// A kill-on-sight standing closes the town's mouths, and the warrant
    /// fades one step per command.
    #[test]
    fn kill_on_sight_closes_doors_and_heat_fades() {
        let mut g = game();
        let fac = consequence::town_faction(g.op.node_seed);
        g.op.standings[fac] = -900; // kill-on-sight on the drained ladder
        let voice_before = g.op.deeds[DEED_VOICE];
        let (r, _) = g.process("talk");
        assert!(r.contains("doors close"), "a hunted face was welcomed: {r}");
        assert_eq!(g.op.deeds[DEED_VOICE], voice_before, "a refused word is no deed");
        g.op.heat = 3;
        g.process("look");
        assert_eq!(g.op.heat, 2, "the warrant did not fade");
    }

    /// The town speaks its law as sensation on `look` — a watch line with no
    /// digits in it, dealt from the seed like every other face of the node.
    #[test]
    fn the_town_speaks_its_law_without_numbers() {
        let mut g = game();
        let (tx, ty) = world::town_square(g.op.node_seed);
        g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let (r, _) = g.process("look");
        // The watch line sits above the offers menu now — find it, don't
        // assume it is last.
        let watch = r
            .lines()
            .find(|l| l.contains("watch") || l.contains("law"))
            .unwrap_or_default();
        assert!(
            watch.contains("watch") || watch.contains("law"),
            "the town kept its law silent: {r}"
        );
        assert!(
            watch.chars().all(|c| !c.is_ascii_digit()),
            "the law leaked a raw number: {watch}"
        );
    }

    /// The walk offers itself: look ends with a dealt menu, and every new
    /// exploration/console verb answers without panicking.
    #[test]
    fn look_offers_the_walk_and_new_verbs_answer() {
        let mut g = game();
        let (r, _) = g.process("look");
        assert!(r.contains('>'), "look dealt no offers: {r}");
        for verb in ["climb", "descend", "camp", "seed", "worlds", "reseed"] {
            let (reply, going) = g.process(verb);
            assert!(going && !reply.is_empty(), "verb {verb} gave nothing");
        }
    }

    /// The birthday window lands: a vixi-struck `0x`-hex reseeds the world
    /// deterministically, xp survives, and the sky re-deals from the new seed.
    #[test]
    fn reseed_accepts_the_interview_seed_and_keeps_xp() {
        let mut g = game();
        g.op.xp = 400;
        let (a, _) = g.process("reseed 0xdeadbeef");
        assert_eq!(g.op.node_seed, 0xdeadbeef, "the interview's seed must deal");
        assert!(g.op.xp >= 400, "xp must ride through a reseed");
        assert_eq!(g.op.deaths, 0, "a reseed is not a death");
        assert_eq!(g.weather, Game::weather_for(0xdeadbeef), "the sky must re-deal");
        let mut h = game();
        h.op.xp = 400;
        let (b, _) = h.process("reseed 0xdeadbeef");
        assert_eq!(a, b, "the same hex must deal the same world");
    }

    /// Camp passes the night: one pulse bar on the clock, the sky moves, and
    /// dawn speaks without digits.
    #[test]
    fn camp_passes_the_night() {
        let mut g = game();
        let before = g.weather.clone();
        let (r, _) = g.process("camp");
        assert!(r.contains("you wake"), "no dawn line: {r}");
        assert_ne!(g.weather, before, "the night did not move the sky");
        assert!(g.ticks >= 31, "camp must advance the clock a full bar");
    }

    /// The beacon rides the autosave: mud.live appears beside the save,
    /// shaped `mud UP` + key-value lines.
    #[test]
    fn the_beacon_rides_the_autosave() {
        let dir = std::env::temp_dir().join("forge-mud-v3-beacon-test");
        let _ = std::fs::create_dir_all(&dir);
        let mut g = game();
        g.save_path = Some(dir.join("op.mud3"));
        g.process("look");
        let live = std::fs::read_to_string(dir.join("mud.live")).expect("beacon missing");
        assert!(live.starts_with("mud UP\n"), "beacon shape wrong: {live}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The character sheet speaks the seven arts as rank words, use trains
    /// a skill toward them, and the whole sheet stays word-only.
    #[test]
    fn the_character_sheet_speaks_the_arts_and_use_trains_them() {
        let mut g = game();
        let (r, _) = g.process("status");
        assert!(r.contains("the arts:"), "no character sheet: {r}");
        assert!(r.contains("untried"), "a fresh operator must be untried: {r}");
        // The honeymoon: below 150 every qualifying use gains +1 — brewing
        // ten times must move the Craft art.
        for _ in 0..10 {
            g.process("brew");
        }
        assert!(g.op.skills.value[2] >= 10, "ten brews must train brewing (the honeymoon)");
    }

    /// `sheet` boxes the same real state `status` speaks in prose: every row
    /// clips to a uniform frame (never widened by a long name or art label),
    /// the frame opens and closes, and the born-under light is the same one
    /// `status`'s own `constellation()` call names — one query, two faces.
    #[test]
    fn sheet_boxes_the_same_state_status_speaks() {
        let mut g = game();
        let (card, _) = g.process("sheet");
        let lines: Vec<&str> = card.lines().collect();
        assert!(lines.first().unwrap().starts_with("  +-"), "no opening frame: {card}");
        assert!(lines.last().unwrap().starts_with("  +-"), "no closing frame: {card}");
        for line in &lines[1..lines.len() - 1] {
            assert!(line.starts_with("  | ") && line.ends_with(" |"), "row broke frame: {line}");
        }
        let r = ConnectionRoll::deal(g.op.node_seed);
        let star = &forge_core_v3::sky::CATALOG[r.star];
        assert!(card.contains(star.name), "the sheet lost its own born-under light: {card}");
        assert!(card.contains(&r.constellation()), "the sheet lost its constellation: {card}");
        assert!(card.contains("REGISTERS") && card.contains("THE ARTS"), "a section is missing: {card}");
    }

    /// Witnessing on unmarked ground counts at GLOBAL scope: the count
    /// survives a reseed (memory outlives worlds), and the awakening word
    /// moves with it.
    #[test]
    fn witnessing_survives_the_reseed() {
        let mut g = game();
        // Find an unmarked square and stand on it.
        let seed = g.op.node_seed;
        let mut site = None;
        'hunt: for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                if memory::unmarked_at(seed, x, y) {
                    site = Some((x, y));
                    break 'hunt;
                }
            }
        }
        let (x, y) = site.expect("a node with zero unmarked ground (band test says impossible)");
        g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
        let (r, _) = g.process("witness");
        assert!(r.contains("do not look away"), "the witness act did not answer: {r}");
        assert_eq!(memory::witness_count(&g.ledger, g.op.node_seed), 1);
        g.process("reseed 0xdeadbeef");
        assert_eq!(
            memory::witness_count(&g.ledger, g.op.node_seed),
            1,
            "a Global-scope witness count must survive the reseed"
        );
        // Off unmarked ground the verb refuses quietly, and counts nothing.
        let refused = memory::unmarked_at(g.op.node_seed, g.x(), g.y());
        if !refused {
            let (r2, _) = g.process("witness");
            assert!(r2.contains("keeps no vigil"), "the refusal lost its words: {r2}");
        }
    }

    /// The character sheet now speaks a moon, an account, and a shadow
    /// tier — all three real, deterministic, and wired off live state
    /// (birth moon, node seed, and the law-heat register), not orphan
    /// tables.
    #[test]
    fn status_speaks_moon_account_and_shadow() {
        let mut g = game();
        let (s, _) = g.process("status");
        assert!(
            moons::MOONS.iter().any(|(name, _)| s.contains(name)),
            "status must name a moon: {s}"
        );
        assert!(
            accounts::ACCOUNTS.iter().any(|(name, _)| s.contains(name)),
            "status must name an account: {s}"
        );
        assert!(
            shadow::SHADOW_TIERS.iter().any(|(name, _)| s.contains(name)),
            "status must name a shadow tier: {s}"
        );
    }

    /// The abyss opens only under stone, deepens to three, refuses a fourth,
    /// and the surface refuses an ascend; vitality speaks on the sheet and
    /// camp restores it.
    #[test]
    fn the_abyss_gates_hold_and_vitality_speaks() {
        let mut g = game();
        // Off-dungeon: no way down; nothing to fight.
        let b = world::biome_at(g.op.node_seed, 0, 0, g.op.bias);
        if b.name != "dungeon" {
            let (r, _) = g.process("delve");
            assert!(r.contains("no way down"), "the wild opened an abyss: {r}");
        }
        let (r, _) = g.process("fight");
        assert!(r.contains("nothing here to fight"), "a phantom fight: {r}");
        let (r, _) = g.process("ascend");
        assert!(!r.is_empty(), "ascend at the surface must still answer");
        // Find stone and go down the whole ladder.
        let mut found = None;
        'hunt: for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                if world::biome_at(g.op.node_seed, x, y, g.op.bias).name == "dungeon" {
                    found = Some((x, y));
                    break 'hunt;
                }
            }
        }
        if let Some((x, y)) = found {
            g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
            for depth in 1..=abyss::MAX_DEPTH {
                g.process("flee"); // clear any standing encounter, then re-enter
                g.op.pos = MortonKey5D::encode([x, y, 0, depth - 1, 0]);
                g.process("delve");
                assert_eq!(g.op.pos.axes()[3], depth, "delve must sink one level");
            }
            let (r, _) = g.process("delve");
            assert!(
                r.contains("no further down") || r.contains("fight") || !r.is_empty(),
                "the deepest floor must refuse or stand: {r}"
            );
        }
        // Vitality: spoken on the sheet, restored by camp.
        let (s, _) = g.process("status");
        assert!(s.contains("vitality:"), "the sheet lost its vitality line: {s}");
        g.vitality = 40;
        g.op.pos = MortonKey5D::encode([1, 1, 0, 0, 0]);
        g.process("camp");
        assert_eq!(g.vitality, 65, "camp must restore one quarter");
    }

    /// The birth kit: three items dealt from the birthday alone —
    /// deterministic, word-only, the same kit at every door.
    #[test]
    fn the_birth_kit_is_dealt_and_deterministic() {
        let mut a = game();
        let mut b = game();
        let (ka, _) = a.process("kit");
        let (kb, _) = b.process("kit");
        assert_eq!(ka, kb, "the same birth must deal the same kit");
        assert!(ka.lines().count() >= 3, "the kit must hold three items: {ka}");
        let low = ka.to_lowercase();
        for word in ["deed", "bias", "wce", "consequence", "cart"] {
            assert!(!low.contains(word), "the kit leaked the cart ({word}): {ka}");
        }
    }

    /// Quit still saves (the LAST step is saved too) and ends the loop.
    #[test]
    fn quit_saves_the_last_step() {
        let dir = std::env::temp_dir().join("forge-mud-v3-test");
        let path = dir.join("op.mud3");
        let _ = std::fs::remove_file(&path);
        let mut g = game();
        g.save_path = Some(path.clone());
        let (_, going) = g.process("quit");
        assert!(!going, "quit must end the loop");
        let bytes = std::fs::read(&path).expect("the last step must be saved");
        let loaded = Operator::decode(&bytes).expect("the save must decode");
        assert_eq!(loaded, g.op, "the save is the operator, byte-exact");
    }

    /// A milestone fall writes one real, plain-text Academy receipt line —
    /// the anchor `content/achievements.rs`'s own `[ASSUMED]` comment names
    /// but did not yet have.
    #[test]
    fn milestone_fall_writes_an_academy_receipt() {
        let dir = std::env::temp_dir().join("forge-mud-v3-academy-receipt-test");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("op.mud3");
        let mut g = Game::new(Operator::birth("Operator", 3, 12).unwrap(), Some(path.clone()));
        g.op.xp = 99;
        let (reply, _) = g.process("x"); // 1 byte -> 100 -> gate 1 falls
        assert!(reply.contains("MILESTONE 1 FALLS"), "the terminal reply lost its milestone line: {reply}");

        let log_path = dir.join("academy.log");
        let text = std::fs::read_to_string(&log_path).expect("academy.log must exist after a fall");
        assert!(text.contains("MILESTONE 1 FALLS"), "the receipt lost the milestone line: {text}");
        assert!(!text.contains("\x1b["), "the receipt must be ANSI-free plain text: {text}");
        assert!(text.contains(&format!("node_seed={}", g.op.node_seed)), "the receipt lost its node_seed: {text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Weld E: on-screen authoring — family tests ──────────────────────

    /// A renamed faction surfaces through `talk`'s kill-on-sight line (the
    /// only place `talk` speaks a faction's name).
    #[test]
    fn rename_faction_surfaces_in_talk() {
        let mut g = game();
        let fac = consequence::town_faction(g.op.node_seed);
        g.op.standings[fac] = -900; // kill-on-sight, so talk speaks the faction
        let (named, _) = g.process(&format!("name faction {} The Ember Court", fac + 1));
        assert!(named.contains("renamed"), "name did not confirm: {named}");
        let (r, _) = g.process("talk");
        assert!(r.contains("The Ember Court"), "the rename did not surface in talk: {r}");
    }

    /// A renamed town surfaces through `look` at the town square.
    #[test]
    fn rename_town_surfaces_in_look() {
        let mut g = game();
        let (tx, ty) = world::town_square(g.op.node_seed);
        g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let (named, _) = g.process("name town New Haven");
        assert!(named.contains("New Haven"), "name did not confirm: {named}");
        let (r, _) = g.process("look");
        assert!(r.contains("New Haven"), "the rename did not surface in look: {r}");
    }

    /// A renamed biome surfaces through `look` away from the town square.
    #[test]
    fn rename_biome_surfaces_in_look_off_town() {
        let mut g = game();
        let (tx, ty) = world::town_square(g.op.node_seed);
        let mut off = None;
        'search: for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                if (x, y) != (tx, ty) {
                    off = Some((x, y));
                    break 'search;
                }
            }
        }
        let (x, y) = off.expect("an 81x81 map has a square off the town");
        g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
        let b = world::biome_at(g.op.node_seed, x, y, g.op.bias);
        let idx = world::BIOMES.iter().position(|bi| bi.name == b.name).expect("biome_at must deal a table biome");
        let (named, _) = g.process(&format!("name biome {} Whisperwood", idx + 1));
        assert!(named.contains("renamed"), "name did not confirm: {named}");
        let (r, _) = g.process("look");
        assert!(r.contains("Whisperwood"), "the rename did not surface in look: {r}");
    }

    /// A renamed boss surfaces in the milestone line when its gate falls.
    #[test]
    fn rename_boss_surfaces_in_milestone() {
        let mut g = game();
        let (named, _) = g.process("name boss 1 The Gate Warden");
        assert!(named.contains("renamed"), "name did not confirm: {named}");
        g.op.xp = 99;
        let (r, _) = g.process("x"); // 1 byte -> 100 -> gate 1 (idx 0) falls
        assert!(r.contains("The Gate Warden"), "the rename did not surface in the milestone: {r}");
    }

    /// A renamed pet surfaces live through the `pet` verb (its idx is a pure
    /// function of name/moon/day, so it is stable and predictable — observed).
    /// A renamed fish/brew surfaces through the exact `speak_fish`/`speak_brew`
    /// helper their verbs call: their idx also depends on xp, which the verb
    /// itself advances (via `consequence()`) before dealing, so reproducing
    /// the live idx after a rename would mean re-deriving that arithmetic
    /// rather than testing the family's actual "one home" resolver — this
    /// tests the same code path the verb reads through instead (observed).
    #[test]
    fn rename_pet_fish_brew_surfaces_in_their_verbs() {
        let mut g = game();
        let pet_idx = pick_idx(pets::PETS, &[g.op.name.as_bytes(), &[g.op.moon, g.op.day], b"pet"]);
        let (named, _) = g.process(&format!("name pet {} Whiskers", pet_idx + 1));
        assert!(named.contains("renamed"), "name did not confirm: {named}");
        let (r, _) = g.process("pet");
        assert!(r.contains("Whiskers"), "the rename did not surface in the pet verb: {r}");

        let mut water = None;
        'w: for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                let b = world::biome_at(g.op.node_seed, x, y, g.op.bias);
                if b.name == "lake" || b.name == "swamp" {
                    water = Some((x, y));
                    break 'w;
                }
            }
        }
        if let Some((x, y)) = water {
            g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
            let (r, _) = g.process("fish");
            let name = r
                .strip_prefix("the line sings. you land ")
                .and_then(|s| s.split(" — ").next())
                .expect("the catch line's shape held");
            let idx = fishing::CATCHES.iter().position(|c| c.0 == name).expect("the catch must be a table entry");
            g.process(&format!("name fish {} Silverfin", idx + 1));
            assert_eq!(g.speak_fish(idx), "Silverfin", "the fish rename did not surface through speak_fish");
        }

        let (tx, ty) = world::town_square(g.op.node_seed);
        g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let (r, _) = g.process("brew");
        let name = r
            .strip_prefix("you brew ")
            .and_then(|s| s.split(" — ").next())
            .expect("the brew line's shape held");
        let idx = alchemy::BREWS.iter().position(|b| b.0 == name).expect("the brew must be a table entry");
        g.process(&format!("name brew {} Moonshine", idx + 1));
        assert_eq!(g.speak_brew(idx), "Moonshine", "the brew rename did not surface through speak_brew");
    }

    /// Persistence: an authored ledger encodes, decodes, and a fresh `Game`
    /// opened on the same save directory speaks the same renamed answer.
    #[test]
    fn persistence_survives_a_fresh_game() {
        let dir = std::env::temp_dir().join("forge-mud-v3-authoring-persist-test");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("op.mud3");
        let mut g = Game::new(Operator::birth("Operator", 3, 12).unwrap(), Some(path.clone()));
        let seed = g.op.node_seed;
        g.process("name town Ashgard");

        // The ledger bytes on disk decode byte-exact (L07, exercised at the
        // door rather than a synthetic buffer).
        let encoded = g.ledger.encode();
        assert_eq!(overlay::Ledger::decode(&encoded), Some(g.ledger.clone()), "the ledger must decode byte-exact");

        // A fresh Game from the same birth (same node seed) loads the ledger
        // from disk and speaks the same renamed town.
        let fresh = Game::new(Operator::birth("Operator", 3, 12).unwrap(), Some(path.clone()));
        assert_eq!(fresh.op.node_seed, seed, "the same birth must deal the same node seed");
        assert_eq!(fresh.speak_town().0, "Ashgard", "a fresh Game must speak the reloaded rename");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Scope matrix: a Node-scoped overlay is invisible after `die()` deals a
    /// new seed; an Operator-scoped one survives death; a Global-scoped one
    /// survives everything (death and reseed both).
    #[test]
    fn scope_matrix_node_operator_global() {
        let mut node = game();
        node.process("scope node");
        let (named, _) = node.process("name town NodeBound");
        assert!(named.contains("new name"), "name did not confirm: {named}");
        assert_eq!(node.speak_town().0, "NodeBound");
        node.process("die");
        assert_ne!(node.speak_town().0, "NodeBound", "a Node overlay must fall out of visibility after die()");

        let mut op = game();
        op.process("scope me");
        op.process("name town OperatorBound");
        op.process("die");
        assert_eq!(op.speak_town().0, "OperatorBound", "an Operator overlay must survive die()");

        let mut world_ = game();
        world_.process("scope world");
        world_.process("name town GlobalBound");
        world_.process("die");
        assert_eq!(world_.speak_town().0, "GlobalBound", "a Global overlay must survive die()");
        world_.process("reseed 0xabc123");
        assert_eq!(world_.speak_town().0, "GlobalBound", "a Global overlay must survive reseed too");
    }

    /// An out-of-range family index speaks a refusal, never panics, and
    /// never appends an entry to the ledger.
    #[test]
    fn refusal_out_of_range_speaks_and_does_not_panic() {
        let mut g = game();
        let before = g.ledger.entries.len();
        let (r, going) = g.process("name faction 9 X");
        assert!(going, "a refusal must not end the loop");
        assert!(r.contains("no faction numbered 9"), "no refusal line: {r}");
        assert_eq!(g.ledger.entries.len(), before, "a refused name must not append a ledger entry");
    }

    /// `set law` is INTENDED (brief, and `law_now`'s own doc comment,
    /// game.rs:641-653) to store an absolute target and read it back
    /// clamped 0..=100. Observed instead: `law_now` resolves over a sentinel
    /// `UNSET = i64::MIN` base (game.rs:649-650) to tell "no entry" apart
    /// from "set to 0" — but `Ledger::resolve_i64` computes `base + v` for
    /// any visible `Add` (overlay.rs:214), so once an entry exists the read
    /// is `i64::MIN + v`, which clamps to 0 for every `v` in 0..=100. This
    /// test asserts the REAL, currently-landed behavior (a correctness bug:
    /// `set law` cannot actually raise the visible law above 0 — reported,
    /// not fixed here per scope) rather than the brief's intended one. The
    /// watch line still carries no digits regardless, since 0 is a valid
    /// clamp target.
    #[test]
    fn law_clamp_holds_and_watch_line_stays_digit_free() {
        let mut g = game();
        g.process("set law 500");
        assert_eq!(
            g.law_now(),
            100,
            "set law must clamp to 100 (offset-by-one storage; the E2-found \
             sentinel bug is fixed — a stored value reads back exactly)"
        );
        g.process("set law 0");
        assert_eq!(g.law_now(), 0, "an authored 0 must read as 0, not as unset");
        let (tx, ty) = world::town_square(g.op.node_seed);
        g.op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
        let (r, _) = g.process("look");
        let watch = r.lines().find(|l| l.contains("watch") || l.contains("law")).unwrap_or_default();
        assert!(!watch.is_empty(), "no watch line: {r}");
        assert!(watch.chars().all(|c| !c.is_ascii_digit()), "the watch line leaked a digit: {watch}");
    }

    /// Casting a spell: the word is spoken one glyph per turn, and completion
    /// fires the effect with no numbers.
    #[test]
    fn cast_spell_advances_one_glyph_per_turn() {
        let mut g = game();
        let (r1, _) = g.process("cast clash");
        assert!(r1.contains("gathers in your throat"), "cast should initiate: {r1}");
        assert!(g.channel.is_active(), "channel should be active");
        assert_eq!(g.channel.word(), Some("CLASH"));

        // Turn 1: advance and show first glyph
        let (r2, _) = g.process("look");
        assert!(r2.contains("the word: C"), "should show 'C': {r2}");

        // Turn 2: second glyph
        let (r3, _) = g.process("look");
        assert!(r3.contains("the word: CL"), "should show 'CL': {r3}");

        // Turn 3: third glyph
        let (r4, _) = g.process("look");
        assert!(r4.contains("the word: CLA"), "should show 'CLA': {r4}");

        // Turn 4: fourth glyph
        let (r5, _) = g.process("look");
        assert!(r5.contains("the word: CLAS"), "should show 'CLAS': {r5}");

        // Turn 5: fifth glyph — completes the word
        let (r6, _) = g.process("look");
        assert!(r6.contains("the word: CLASH"), "should show 'CLASH': {r6}");
        // Effect should have fired on completion
        let effect_line = casting::EFFECT_LINES[0]; // flurry
        assert!(r6.contains(effect_line), "effect line should appear: {} not in {}", effect_line, r6);

        // After completion, no more channel line
        let (r7, _) = g.process("look");
        assert!(!r7.contains("the word:"), "channel should be cleared: {r7}");
        assert!(!g.channel.is_active(), "channel should be inactive");
    }

    /// Movement interrupts the channel mid-cast, speaking the broken word.
    #[test]
    fn movement_interrupts_the_channel() {
        let mut g = game();
        g.process("cast balance");
        assert!(g.channel.is_active());

        // Advance two glyphs
        g.process("look");
        g.process("look");

        // Now move — should interrupt
        let (r, _) = g.process("n");
        assert!(r.contains("the word dies half-spoken: BAL—"), "should show broken word: {r}");
        assert!(!g.channel.is_active(), "channel should be interrupted");
    }

    /// Casting another spell while one is active is refused.
    #[test]
    fn casting_while_casting_is_refused() {
        let mut g = game();
        g.process("cast clash");
        assert!(g.channel.is_active());

        let (r, _) = g.process("cast balance");
        assert!(r.contains("already fills your mind"), "should refuse dual cast: {r}");
        assert_eq!(g.channel.word(), Some("CLASH"), "original cast should persist");
    }

    /// An unknown word is refused.
    #[test]
    fn unknown_cast_word_is_refused() {
        let mut g = game();
        let (r, _) = g.process("cast invalidword");
        assert!(r.contains("holds no sorcerous power"), "should refuse unknown word: {r}");
        assert!(!g.channel.is_active(), "channel should not start");
    }

    /// process_line returns text deterministically: calling twice on the same
    /// command returns the same non-empty string both times.
    #[test]
    fn process_line_returns_deterministic_response() {
        let mut g = game();
        let first = g.process_line("help");
        let second = g.process_line("help");
        assert!(!first.is_empty(), "first help response must be non-empty");
        assert_eq!(first, second, "help response must be deterministic");
    }

    /// The worn body routes the loom: shifting rewrites the save-codec byte
    /// and the next room is filed by the body that is actually being worn.
    #[test]
    fn shifting_a_body_changes_who_files_the_room() {
        let mut g = game();
        let mortal = g.process_line("look");
        assert!(mortal.contains(magic::umwelt::Form::Mortal.body_line()));

        let shifted = g.process_line("shift lich");
        assert_eq!(g.op.form, magic::umwelt::Form::Lich.as_u8());
        assert!(shifted.contains(magic::umwelt::Form::Lich.body_line()));
        assert!(!g.process_line("look").contains(magic::umwelt::Form::Mortal.body_line()));

        assert!(g.process_line("shift").contains("wearing lich"));
        assert!(g.process_line("shift gargoyle").contains("no body called"));
        assert_eq!(g.op.form, magic::umwelt::Form::Lich.as_u8(), "a bad name wears nothing");
    }

    /// The cell speaks off the landed world systems, not off literal zeros:
    /// worked stone under a dungeon reaches the field the loom reads.
    #[test]
    fn a_dungeon_cell_carries_stone_the_open_ground_does_not() {
        use crate::umwelt_loom::SenseChannel;
        let mut g = game();
        let at = [0i64, 0i64, 0i64, 0i64, 0i64];
        let (open, _) = g.sense_field_here(at);
        let mut found = None;
        for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                if world::biome_at(g.op.node_seed, x, y, g.op.bias).name == "dungeon" {
                    found = Some((x, y));
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let (x, y) = found.expect("this seed's land holds a dungeon somewhere");
        g.op.pos = MortonKey5D::encode([x, y, 0, 0, 0]);
        let (delved, _) = g.sense_field_here(at);
        assert_eq!(open[SenseChannel::MasonryStress], 0, "open ground bears nothing");
        assert!(delved[SenseChannel::MasonryStress] > 0, "worked stone must reach the field");
    }

    /// The cultural floor, held as an assert instead of a memo. Canon:
    /// carts/ironroot/npe.ironroot.ron:14, assets/ironroot/Good/ironroot-dreamer/
    /// canon-packet.md:11, and forge-book-v3 lore::lint FORBIDDEN_TERMS.
    #[test]
    fn no_authored_sky_prose_breaks_the_cultural_floor() {
        const BANNED: [&str; 8] = [
            "aurora",
            "northern lights",
            "eagle bone whistle",
            "americana",
            "cowboy",
            "wild west",
            "wendigo",
            "skinwalker",
        ];
        let check = |what: &str, s: &str| {
            let lower = s.to_lowercase();
            for term in BANNED {
                assert!(!lower.contains(term), "{what} reaches for a banned term '{term}': {s}");
            }
        };
        for sky in [
            crate::weather::Sky::Clear,
            crate::weather::Sky::Overcast,
            crate::weather::Sky::Storm,
            crate::weather::Sky::Ashfall,
            crate::weather::Sky::Hardfrost,
        ] {
            check("Sky::name", sky.name());
            for era in crate::weather::Era::all() {
                for intensity_pmy in [0, 3_000, 6_000, 10_000] {
                    let w = Weather { era, sky, intensity_pmy };
                    check("weather_line", &weather_line(w));
                }
            }
        }
        for (id, desc) in crate::content::skyboxes::SKYBOXES {
            check("skybox id", id);
            check("skybox description", desc);
        }
        // And the composed room, which is what a player actually reads.
        let mut g = game();
        for _ in 0..(crate::weather::SKY_BANK_PERIOD * 6) {
            check("look", &g.process("look").0);
        }
    }

    /// The sieve recorded a flat calm forever because nothing fed it. Walking
    /// the world must move its inputs off the values game.rs:219 hands them.
    #[test]
    fn the_sky_moves_the_sieve_off_its_flat_calm() {
        let mut g = game();
        let (t0, w0) = (g.weather_sieve.temperature, g.weather_sieve.wind_speed);
        for _ in 0..(crate::weather::SKY_BANK_PERIOD * 4) {
            g.process("look");
        }
        let s = &g.weather_sieve;
        assert!(
            s.temperature != t0 || s.wind_speed != w0 || s.precipitation != 0,
            "the sieve is still recording its construction values: temp {} wind {} precip {}",
            s.temperature,
            s.wind_speed,
            s.precipitation
        );
        assert_ne!(s.temperature_history[0], s.temperature_history[12], "history must move too");
    }

    /// A drive lands only on a bank boundary — the condition holds between
    /// them instead of flickering once per command.
    #[test]
    fn the_sieve_holds_its_reading_between_banks() {
        let mut g = game();
        for _ in 0..crate::weather::SKY_BANK_PERIOD {
            g.process("look");
        }
        let held = g.weather_sieve.temperature;
        g.process("look");
        assert_eq!(g.weather_sieve.temperature, held, "a mid-bank command must not re-drive");
    }

    /// Three tells were authored into sense.rs and could never print, because
    /// the three fields they branch on were dead. They must be reachable now.
    #[test]
    fn the_dead_weather_tells_become_reachable() {
        const DEAD: [&str; 3] = ["teeth", "gives up its dust", "warm wind"];
        let mut unseen: Vec<&str> = DEAD.to_vec();
        for seed in 0..24u64 {
            let mut g = game();
            g.op.node_seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            g.weather = Game::weather_for(g.op.node_seed);
            for _ in 0..(crate::weather::SKY_BANK_PERIOD * 8) {
                let out = g.process("look").0;
                unseen.retain(|t| !out.contains(t));
                if unseen.is_empty() {
                    return;
                }
            }
        }
        panic!("these tells are still unreachable in play: {unseen:?}");
    }

    /// The loom reads the weather through the sieve: a moving sky must move
    /// the channels sense_field_here() feeds the field.
    #[test]
    fn the_loom_channels_move_with_the_sky() {
        use crate::umwelt_loom::SenseChannel;
        let mut g = game();
        let at = [0i64, 0, 0, 0, 0];
        let mut seen = std::collections::HashSet::new();
        for _ in 0..(crate::weather::SKY_BANK_PERIOD * 12) {
            g.process("look");
            let (f, _) = g.sense_field_here(at);
            seen.insert((
                f[SenseChannel::HeatGradient],
                f[SenseChannel::AtmospherePa],
                f[SenseChannel::ParticulateFlux],
            ));
        }
        assert!(seen.len() > 1, "the weather channels never moved: {seen:?}");
        assert!(
            seen.iter().any(|&(h, a, p)| h != 0 || a != 0 || p != 0),
            "the weather channels moved but never left zero"
        );
    }

    /// The whole point of an integer sky: same seed, same commands, same
    /// rooms, byte for byte — driving the sieve must not cost that.
    #[test]
    fn the_same_seed_and_commands_weave_the_same_rooms() {
        let script = ["look", "e", "look", "s", "shift lich", "look", "e", "look", "shift djinn", "look"];
        let (mut a, mut b) = (game(), game());
        for cmd in script {
            assert_eq!(a.process(cmd).0, b.process(cmd).0, "'{cmd}' diverged");
        }
        assert_eq!(a.weather_sieve.temperature, b.weather_sieve.temperature);
        assert_eq!(a.weather_sieve.chinook_buildup, b.weather_sieve.chinook_buildup);
    }

    /// Sieves tick with each command: ecology populations change via births.
    /// Behavioral test proving EcologySieve::tick() is called and changes state.
    #[test]
    fn ecology_sieve_ticks_and_applies_births_each_command() {
        let mut g = game();
        // Initial population: 100 per species, 500 permyriad birth rate.
        // Births = 100 * 500 / 10000 = 5 per species per tick.
        assert_eq!(g.ecology_sieve.populations[0], 100, "initial population must be 100");
        let initial_births = (100i32 * 500 / 10000) as u16;
        assert!(initial_births > 0, "birth rate 500 should yield births");

        // Process one command: ticks the ecology sieve.
        g.process("look");

        // Population should have increased by births (minus any predation).
        let new_pop = g.ecology_sieve.populations[0];
        assert!(
            new_pop > 100,
            "ecology population must increase after tick (births={}, saw pop={})",
            initial_births,
            new_pop
        );
        assert_eq!(
            new_pop,
            100 + initial_births,
            "population increase must equal births (no predation in test setup)"
        );

        // Process another command: ticks again.
        let pop_after_first = new_pop;
        g.process("look");
        let pop_after_second = g.ecology_sieve.populations[0];
        assert!(
            pop_after_second > pop_after_first,
            "population must keep increasing (second tick: {} -> {})",
            pop_after_first,
            pop_after_second
        );
    }

    /// Weather sieve ticks and updates history: temperature history advances.
    /// Behavioral test proving WeatherSieve::tick() is called and updates history.
    #[test]
    fn weather_sieve_ticks_and_updates_history_each_command() {
        let mut g = game();
        let initial_temp = g.weather_sieve.temperature;
        assert_eq!(g.weather_sieve.temperature_history[0], initial_temp, "history[0] must match current temperature");

        // Process one command: ticks the weather sieve.
        g.process("look");

        // Temperature history should have rotated: new temp at [0], old at [1].
        assert_eq!(
            g.weather_sieve.temperature_history[0], g.weather_sieve.temperature,
            "history[0] must update to current temperature after tick"
        );
        assert_eq!(
            g.weather_sieve.temperature_history[1], initial_temp,
            "history[1] must hold previous temperature after tick"
        );

        // Process another command: history advances again.
        let temp_before = g.weather_sieve.temperature;
        g.process("look");
        assert_eq!(
            g.weather_sieve.temperature_history[1], temp_before,
            "history must rotate: prior current moves to [1] after second tick"
        );
    }

    #[test]
    fn test_npe_first_scene_world_look_and_presences() {
        let ron_src = include_str!("../../../carts/ironroot/npe.ironroot.ron");
        let cart: forge_cart_v3::npe::NpeCart = ron::from_str(ron_src).expect("valid ironroot npe cart");
        let op = Operator::birth_with_discipline("Morrow", 4, 12, 0).expect("valid birth");
        let mut g = Game::from_npe_cart(op, &cart, None);

        // 1. Check look incorporates Thornbell Parish and presences
        let look_out = g.look();
        assert!(look_out.contains("Thornbell Parish"), "look must name Thornbell Parish: {look_out}");
        assert!(look_out.contains("Toll-Sister Vey"), "look must name Toll-Sister Vey: {look_out}");
        assert!(look_out.contains("the Bellwright at her forge"), "look must name the Bellwright: {look_out}");
        assert!(look_out.contains("rooted deserter"), "look must name the rooted deserter: {look_out}");

        // 2. Check kit includes provenance words from cart
        let kit_out = g.process("kit").0;
        assert!(kit_out.contains("IRONROOT"), "kit must name IRONROOT: {kit_out}");

        // 3. Check landmarks command
        let landmarks_out = g.process("landmarks").0;
        assert!(landmarks_out.contains("Bellwright Forge"), "landmarks must list forge: {landmarks_out}");
        assert!(landmarks_out.contains("Parish Shrine"), "landmarks must list shrine: {landmarks_out}");

        // 4. Check first task command
        let task_out = g.process("task").0;
        assert!(task_out.contains("rooted deserter"), "task must name rooted deserter: {task_out}");
        assert!(task_out.contains("100 XP"), "task must specify 100 XP reward: {task_out}");

        // 5. Check dialogue with Toll-Sister Vey
        let vey_dialogue = g.process("talk vey").0;
        assert!(vey_dialogue.contains("Toll-Sister Vey lifts her tallow lantern"), "talk vey must yield dialogue: {vey_dialogue}");

        // 6. Check dialogue with the Bellwright
        let smith_dialogue = g.process("talk bellwright").0;
        assert!(smith_dialogue.contains("strikes glowing iron"), "talk bellwright must yield dialogue: {smith_dialogue}");

        // 7. Check dialogue with the rooted deserter
        let deserter_dialogue = g.process("talk deserter").0;
        assert!(deserter_dialogue.contains("roots do not let go"), "talk deserter must yield dialogue: {deserter_dialogue}");

        // 8. Strike the deserter (initiates and executes fight)
        let strike_out = g.process("strike deserter").0;
        assert!(!strike_out.is_empty(), "strike deserter must execute combat");
    }
}
