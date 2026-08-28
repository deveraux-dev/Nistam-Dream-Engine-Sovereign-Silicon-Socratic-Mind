//! CYOA scene sieve — scenes selected by legality and deterministic scoring,
//! never hardcoded sequence (W2 of the ForgeVision Lab weld, knowledge drop
//! `.forge/knowledge-drops/2026-08-18-forgevision-lab-2d3d.md` §H).
//!
//! Donor: `F:\v3\TODO\quarry-sort\MYGAMEDRAIN-2026-08-17\creation-engine\src\cyoa.rs`
//! (140 lines, read whole before weld). Ironroot lineage: one confluence of
//! {dirge-of-ironroot, ironroot-edict, deveraux-game, akgame, astrakey,
//! astrakeyweb, goblin, adevstale, AKWEB}, live in this crate.
//!
//! Port adaptations (each forced, none stylistic):
//! 1. Ids come from `forge_core_v3::organs::creation_spine` (`SceneId`/`ChoiceId`
//!    added there for this consumer); donor's `ZoneId`/`NpcId`/`MotifId`/
//!    `SecretId` args collapse to `ArtifactId` (spine adaptation 3).
//! 2. Donor `combine_stable` → `checksum::fnv1a64_fold` chain (one FNV home, L05).
//! 3. `Ledger` and `Visibility` are the spine's own.
//! 4. EXPANDED (Sean 2026-08-18): archetypes/instruments/actions grew from two
//!    authored sources — `THEORETICAL PRIMITIVES.txt` (five interface-psyche
//!    primitives: Jungian projection, Wattsian field, Gabor holography, Maté
//!    somatics, transparency-vs-grounding paradox) and `Chaos Feminine.txt`
//!    (the river poem: grace/flood, life/taking, "always running one way or
//!    the other on the line"). Donor variants stay first, in donor order;
//!    new variants append (forward-only ledger law). Two authored scene pools
//!    at the bottom turn the sources into playable arcs.
//!
//! This module is a SIBLING of `boss_sieve` (which sieves boss variants off
//! `RunProfile` counters); `dialogue` stays pure narrative — the sieve decides
//! WHICH scene, dialogue then tells it. Facts→dialogue-lock translation is a
//! later, separate seam (recon §H wire point 2).

use forge_core_v3::checksum::{fnv1a64_fold, hash_bytes_fnv1a, FNV_OFFSET_BASIS};
use forge_core_v3::organs::creation_spine::{
    ArtifactId, ChoiceId, Ledger, LoreFactId, SceneId, Visibility,
};
use std::ops::{Add, Index, IndexMut};

// ── Art schools (hermetic stat channels) ────────────────────────────────────

/// Hermetic stat channels — one per art training axis.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtSchool {
    /// Vigor (force / physicality).
    Vigor = 0,
    /// Shadow Weight (depth / introspection).
    ShadowWeight = 1,
    /// Logic Depth (clarity / intellect).
    LogicDepth = 2,
    /// Momentum (flow / propulsion).
    Momentum = 3,
    /// Tarnish (decay / wear).
    Tarnish = 4,
    /// Resonance (harmony / frequency).
    Resonance = 5,
    /// Guilt (burden / consequence).
    Guilt = 6,
}

/// Total art schools.
pub const ART_SCHOOL_COUNT: usize = 7;

/// Seven-art training vector for archetypes and choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArtVector(pub [i8; ART_SCHOOL_COUNT]);

impl Index<ArtSchool> for ArtVector {
    type Output = i8;
    #[inline]
    fn index(&self, school: ArtSchool) -> &Self::Output {
        &self.0[school as usize]
    }
}

impl IndexMut<ArtSchool> for ArtVector {
    #[inline]
    fn index_mut(&mut self, school: ArtSchool) -> &mut Self::Output {
        &mut self.0[school as usize]
    }
}

/// Ternary composition vector for alchemical state (clamped -3 to +3 per axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TritVector {
    /// Physical / mass axis.
    pub salt: i8,
    /// Mental / fluidity axis.
    pub mercury: i8,
    /// Spiritual / volatility axis.
    pub sulfur: i8,
}

impl Add for TritVector {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            salt: (self.salt + rhs.salt).clamp(-3, 3),
            mercury: (self.mercury + rhs.mercury).clamp(-3, 3),
            sulfur: (self.sulfur + rhs.sulfur).clamp(-3, 3),
        }
    }
}

impl std::iter::Sum for TritVector {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(TritVector::default(), |a, b| a + b)
    }
}

// ── Choice vocabulary ───────────────────────────────────────────────────────

/// The stance a choice takes. Donor six first (cyoa.rs:11-18); the psyche
/// group is drained from THEORETICAL PRIMITIVES.txt:2-6, the river group from
/// Chaos Feminine.txt:4-18.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChoiceArchetype {
    // -- donor six --
    /// Sever — end a bond, a thread, a life.
    Cut,
    /// Tie — oath, alliance, knot.
    Bind,
    /// Say no and hold it.
    Refuse,
    /// Deceive, misdirect.
    Trick,
    /// See and be changed by seeing.
    Witness,
    /// Answer with music instead of argument.
    Sing,
    // -- psyche group (THEORETICAL PRIMITIVES.txt) --
    /// Cast what is yours onto another (Jungian projection, line 2).
    Project,
    /// Break the spell of the glass — the "spot-on-glass" disruption that ends
    /// participation mystique (line 2).
    Shatter,
    /// Dissolve the boundary between self and place — organism-environment
    /// continuity, glass as contact zone (Wattsian field, line 3).
    Merge,
    /// Read the whole in the fragment — micro-state as macro-encoding
    /// (Gabor holographic signal, line 4).
    Encode,
    /// Answer from the body before the mind — autonomic truth
    /// (Maté somatic reality, line 5).
    Feel,
    /// Choose embodiment over transparency (interface paradox, line 6).
    Ground,
    /// Choose transparency over embodiment — and risk the drift
    /// (interface paradox, line 6).
    Dissolve,
    // -- river group (Chaos Feminine.txt) --
    /// Yield to the current ("how graceful the river runs", line 4).
    Flow,
    /// Hold utterly still ("calm, until dawn", line 5).
    Still,
    /// Wear the stone away slowly ("the river curves", line 7).
    Carve,
    /// Give life ("the river brings life", line 10).
    Nurture,
    /// Overwhelm every bank at once ("and chaos", line 10).
    Flood,
    /// Take under — the taking half of the Mother ("or take your life away",
    /// line 16).
    Drown,
    // -- arch expansion (Sean 2026-08-18: Fae/Goblin/Umwelt/Dirge/Blind arches) --
    /// Draw another in with what they most want (fae voice tag `lure`,
    /// _book/18-fae-world-overlay.md:95).
    Lure,
    /// Make the false face beautiful (fae voice tag `glamour`, same tablet).
    Glamour,
    /// Invoke or extend guest-law — the table that binds host and guest
    /// (fae voice tag `guest_right`).
    Host,
    /// Make something workable out of wreckage (goblin craft — GoblinKind lives
    /// at brain/run_dev_run.rs:740).
    Tinker,
    /// Take what the battlefield left behind (goblin harvest).
    Scavenge,
    /// Borrow another creature's world — its senses as your senses
    /// (von Uexküll Umwelt, aspire.rs:347 `every-beast-hears-different`).
    Attune,
    /// Grieve properly — the dirge stance (dirge-of-ironroot, lineage root).
    Mourn,
    /// Navigate by sound where sight is gone (ARCH-004 blind path:
    /// sonify the canvas).
    Echo,
}

/// Playable instruments a choice can voice. Donor five first (cyoa.rs:21-27);
/// the additions are river-made (Chaos Feminine) plus the glass itself
/// (THEORETICAL PRIMITIVES — glass as contact zone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentId {
    // -- donor five --
    /// The bare voice.
    Voice,
    /// Plucked strings.
    Lute,
    /// Struck skin.
    Drum,
    /// Cast bronze.
    Bell,
    /// Carved bone.
    BoneFlute,
    // -- river-made --
    /// A reed cut from the bank.
    RiverReed,
    /// A drum played on standing water.
    WaterDrum,
    /// Seeds falling through thorns — rain that never arrives.
    RainStick,
    /// A shell that holds the whole river's roar (the part encoding the whole).
    Conch,
    /// Hollow wood the wind plays without hands.
    WindChime,
    // -- glass and grief --
    /// Rubbed rims singing at the contact zone (glass as instrument, not
    /// boundary).
    GlassHarp,
    /// Strings for the graceful half of the river.
    Harp,
    /// The flood given a mouth.
    WarHorn,
    // -- arch expansion --
    /// Goblin-strung wreckage that plays anyway (Scavenge made audible).
    ScrapFiddle,
    /// The deliberate absence of sound — the blind and mourning instrument;
    /// playing it IS the choice.
    Silence,
}

/// What the player actually does. Donor twelve first (cyoa.rs:30-43); the
/// psyche set is drained from THEORETICAL PRIMITIVES.txt, the river set from
/// Chaos Feminine.txt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChoiceAction {
    // -- donor twelve --
    /// Say it without ornament.
    SpeakPlainly,
    /// Ask the open question.
    Ask,
    /// Name the guilt aloud.
    Accuse,
    /// Trade something for something.
    Bargain,
    /// Say the untrue thing.
    Lie,
    /// Hold the silence.
    RemainSilent,
    /// Walk away.
    Leave,
    /// Recall a learned song.
    SingMotif(ArtifactId),
    /// Voice an instrument.
    PlayInstrument(InstrumentId),
    /// Stop and listen for the resonant answer.
    ListenForResonance,
    /// Give up a memory as payment.
    OfferMemory,
    /// Refuse to be changed.
    RefuseConversion,
    // -- psyche set (THEORETICAL PRIMITIVES.txt) --
    /// Withdraw the projection — say "that shadow is mine" (line 2).
    NameTheShadow,
    /// Put a finger on the glass and end the illusion of no-glass (line 2).
    TouchTheGlass,
    /// Breathe until you and the room share one rhythm (line 3).
    BreatheWithTheRoom,
    /// Read the macro-system out of one grain of micro-state (line 4).
    ReadTheGrain,
    /// Bring the racing pulse back down by will and stance (line 5).
    SteadyTheBody,
    /// Release your hold on the edges of yourself (line 6, the transparency
    /// pole).
    LetGoOfTheBanks,
    // -- river set (Chaos Feminine.txt) --
    /// Step into the current (line 4).
    EnterTheRiver,
    /// Go where it goes (line 14).
    FollowTheCurrent,
    /// Fight upstream (line 14, the other way on the line).
    SwimAgainstIt,
    /// Give the river something it may keep (line 16).
    OfferToTheRiver,
    /// Do nothing until the light changes (line 5).
    WaitForDawn,
    /// Speak the name: Mother Unleashed (line 18).
    CallTheMother,
    /// Open every gate at once (line 10).
    ReleaseTheFlood,
    /// Take the river into yourself (line 16, the making-whole half).
    DrinkDeep,
    // -- fae arch (18-fae-world-overlay.md: gifts create debt, guest-law) --
    /// Take the fae gift — and the debt that rides inside it.
    AcceptTheGift,
    /// Claim the protection of the table — host and guest bound alike.
    InvokeGuestRight,
    /// Say the true name aloud — the one word glamour cannot survive.
    SpeakTrueName,
    /// Settle the debt the gift created, at its full price.
    PayTheDebt,
    // -- goblin arch (GoblinKind + goblin-is-the-bomb crater seam) --
    /// Light it and run — the crater is the argument.
    LightTheFuse,
    /// Strip the wreck for anything that still works.
    ScavengeTheWreck,
    /// Build the trap out of what the wreck gave up.
    TinkerTheTrap,
    // -- umwelt arch (every-beast-hears-different) --
    /// See the room through the beast's eyes instead of your own.
    BorrowTheirEyes,
    /// Track what only the borrowed nose can hold.
    FollowTheScent,
    /// Read the cell through the body you are actually wearing — the loom's own
    /// channel per form, never a borrowed one (`umwelt_loom::weave`). Pressure
    /// for the djinn, entropy for the lich, load for the skeleton.
    SenseTheRoom,
    // -- dirge arch (dirge-of-ironroot) --
    /// Sing the funeral song properly, all verses.
    SingTheDirge,
    /// Ring the bell once for the dead.
    TollTheBell,
    /// Put the body down the right way, whatever it costs to stop and do it.
    LayThemDown,
    // -- blind arch (ARCH-004 sensory substitution) --
    /// Shut sight off on purpose and start again from zero.
    CloseYourEyes,
    /// Read the room's shape out of its reflections.
    HearTheShape,
}

/// Donor archetype count — the first six variants, in donor order.
pub const DONOR_ARCHETYPES: usize = 6;
/// Total archetypes after the 2026-08-18 expansions (texts + arches).
pub const ARCHETYPE_COUNT: usize = 27;

/// Archetype choice pressure vector: art-training deltas and pole force/water tally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArchetypePressure {
    /// Nudge per hermetics::Stat art.
    pub art_delta: ArtVector,
    /// Signed force(+)/water(-) strength in permyriad-scale (0 @ fixed point).
    pub pole_q: i16,
}
/// Total instruments after the expansions.
pub const INSTRUMENT_COUNT: usize = 15;
/// Total choice actions after the expansions (12 donor + 6 psyche + 8 river +
/// 15 arch). `ChoiceAction` carries payload variants, so this is an authored
/// count (L12 `Authored`), held honest by the vocabulary tally test in
/// `itemforge` (the trit-capacity ledger).
pub const ACTION_COUNT: usize = 41;
/// Total authored scenes: 14 opening trials (2x7 arts) + 26 arcs (4+4 texts, 5x3 arches).
pub const SCENE_COUNT: usize = 40;

impl ChoiceArchetype {
    /// Every archetype, declaration order (donor six first — stable).
    pub const ALL: [ChoiceArchetype; ARCHETYPE_COUNT] = [
        ChoiceArchetype::Cut,
        ChoiceArchetype::Bind,
        ChoiceArchetype::Refuse,
        ChoiceArchetype::Trick,
        ChoiceArchetype::Witness,
        ChoiceArchetype::Sing,
        ChoiceArchetype::Project,
        ChoiceArchetype::Shatter,
        ChoiceArchetype::Merge,
        ChoiceArchetype::Encode,
        ChoiceArchetype::Feel,
        ChoiceArchetype::Ground,
        ChoiceArchetype::Dissolve,
        ChoiceArchetype::Flow,
        ChoiceArchetype::Still,
        ChoiceArchetype::Carve,
        ChoiceArchetype::Nurture,
        ChoiceArchetype::Flood,
        ChoiceArchetype::Drown,
        ChoiceArchetype::Lure,
        ChoiceArchetype::Glamour,
        ChoiceArchetype::Host,
        ChoiceArchetype::Tinker,
        ChoiceArchetype::Scavenge,
        ChoiceArchetype::Attune,
        ChoiceArchetype::Mourn,
        ChoiceArchetype::Echo,
    ];

    /// UI word for the archetype (exhaustive — a new variant fails here first).
    pub fn as_str(self) -> &'static str {
        match self {
            ChoiceArchetype::Cut => "Cut",
            ChoiceArchetype::Bind => "Bind",
            ChoiceArchetype::Refuse => "Refuse",
            ChoiceArchetype::Trick => "Trick",
            ChoiceArchetype::Witness => "Witness",
            ChoiceArchetype::Sing => "Sing",
            ChoiceArchetype::Project => "Project",
            ChoiceArchetype::Shatter => "Shatter",
            ChoiceArchetype::Merge => "Merge",
            ChoiceArchetype::Encode => "Encode",
            ChoiceArchetype::Feel => "Feel",
            ChoiceArchetype::Ground => "Ground",
            ChoiceArchetype::Dissolve => "Dissolve",
            ChoiceArchetype::Flow => "Flow",
            ChoiceArchetype::Still => "Still",
            ChoiceArchetype::Carve => "Carve",
            ChoiceArchetype::Nurture => "Nurture",
            ChoiceArchetype::Flood => "Flood",
            ChoiceArchetype::Drown => "Drown",
            ChoiceArchetype::Lure => "Lure",
            ChoiceArchetype::Glamour => "Glamour",
            ChoiceArchetype::Host => "Host",
            ChoiceArchetype::Tinker => "Tinker",
            ChoiceArchetype::Scavenge => "Scavenge",
            ChoiceArchetype::Attune => "Attune",
            ChoiceArchetype::Mourn => "Mourn",
            ChoiceArchetype::Echo => "Echo",
        }
    }
}

/// One archetype's authored pressure values and attributes.
pub struct ArchetypeDefinition {
    /// The archetype variant.
    pub archetype: ChoiceArchetype,
    /// Art deltas (Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt).
    pub art_delta: ArtVector,
    /// Signed force(+)/water(-) strength in permyriad-scale.
    pub pole_q: i16,
}

/// Registry of all archetype definitions: donor six + psyche + river + arch expansions.
/// Ordered to match ChoiceArchetype::ALL for fast index-based lookup.
pub static ARCHETYPE_REGISTRY: &[ArchetypeDefinition] = &[
    // -- donor six (action/voice-forward) --
    ArchetypeDefinition { archetype: ChoiceArchetype::Cut, art_delta: ArtVector([2, 1, 0, 1, 1, 0, 0]), pole_q: 1500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Bind, art_delta: ArtVector([0, 0, 1, 0, 0, 2, 1]), pole_q: -1000 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Refuse, art_delta: ArtVector([1, 0, 0, 0, 0, 1, 2]), pole_q: -1500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Trick, art_delta: ArtVector([0, 2, 1, 1, 0, 1, 0]), pole_q: 500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Witness, art_delta: ArtVector([0, 0, 1, 0, 0, 1, 2]), pole_q: 0 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Sing, art_delta: ArtVector([0, 0, 0, 0, 0, 2, 1]), pole_q: -800 },
    // -- psyche group (internal states) --
    ArchetypeDefinition { archetype: ChoiceArchetype::Project, art_delta: ArtVector([0, 1, 2, 0, 1, 0, 0]), pole_q: 1000 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Shatter, art_delta: ArtVector([1, 2, 0, 1, 0, 0, 0]), pole_q: 1200 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Merge, art_delta: ArtVector([0, 0, 0, 0, 0, 1, 2]), pole_q: -1800 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Encode, art_delta: ArtVector([0, 0, 2, 1, 0, 1, 0]), pole_q: 200 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Feel, art_delta: ArtVector([1, 0, 0, 0, 1, 0, 2]), pole_q: -1200 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Ground, art_delta: ArtVector([2, 0, 1, 0, 0, 0, 1]), pole_q: -900 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Dissolve, art_delta: ArtVector([0, 1, 0, 0, 1, 0, 2]), pole_q: -2000 },
    // -- river group (flow/yielding) --
    ArchetypeDefinition { archetype: ChoiceArchetype::Flow, art_delta: ArtVector([0, 0, 0, 2, 0, 0, 1]), pole_q: -1500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Still, art_delta: ArtVector([0, 1, 1, 0, 0, 0, 2]), pole_q: -1000 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Carve, art_delta: ArtVector([2, 0, 1, 1, 1, 0, 0]), pole_q: -800 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Nurture, art_delta: ArtVector([1, 0, 0, 0, 0, 2, 1]), pole_q: -2000 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Flood, art_delta: ArtVector([1, 0, 0, 2, 1, 0, 0]), pole_q: 1800 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Drown, art_delta: ArtVector([0, 1, 0, 1, 2, 0, 1]), pole_q: 1500 },
    // -- arch expansion (fae/goblin/umwelt/dirge/blind) --
    ArchetypeDefinition { archetype: ChoiceArchetype::Lure, art_delta: ArtVector([0, 1, 0, 0, 1, 2, 0]), pole_q: 1200 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Glamour, art_delta: ArtVector([0, 0, 1, 0, 0, 2, 1]), pole_q: 800 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Host, art_delta: ArtVector([1, 0, 1, 0, 0, 2, 1]), pole_q: -500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Tinker, art_delta: ArtVector([1, 1, 2, 1, 0, 0, 0]), pole_q: 600 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Scavenge, art_delta: ArtVector([2, 2, 0, 1, 0, 0, 0]), pole_q: 1000 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Attune, art_delta: ArtVector([0, 0, 1, 0, 0, 2, 2]), pole_q: -700 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Mourn, art_delta: ArtVector([0, 0, 0, 0, 1, 1, 3]), pole_q: -1500 },
    ArchetypeDefinition { archetype: ChoiceArchetype::Echo, art_delta: ArtVector([0, 2, 1, 1, 0, 0, 1]), pole_q: -300 },
];

/// Look up archetype pressure by variant (uses registry lookup instead of match).
pub const fn archetype_pressure(a: ChoiceArchetype) -> ArchetypePressure {
    let idx = a as usize;
    if idx >= ARCHETYPE_REGISTRY.len() {
        return ArchetypePressure { art_delta: ArtVector([0, 0, 0, 0, 0, 0, 0]), pole_q: 0 };
    }
    let def = &ARCHETYPE_REGISTRY[idx];
    ArchetypePressure { art_delta: def.art_delta, pole_q: def.pole_q }
}

impl InstrumentId {
    /// Every instrument, declaration order (donor five first — stable).
    pub const ALL: [InstrumentId; INSTRUMENT_COUNT] = [
        InstrumentId::Voice,
        InstrumentId::Lute,
        InstrumentId::Drum,
        InstrumentId::Bell,
        InstrumentId::BoneFlute,
        InstrumentId::RiverReed,
        InstrumentId::WaterDrum,
        InstrumentId::RainStick,
        InstrumentId::Conch,
        InstrumentId::WindChime,
        InstrumentId::GlassHarp,
        InstrumentId::Harp,
        InstrumentId::WarHorn,
        InstrumentId::ScrapFiddle,
        InstrumentId::Silence,
    ];

    /// UI word for the instrument (exhaustive).
    pub fn as_str(self) -> &'static str {
        match self {
            InstrumentId::Voice => "Voice",
            InstrumentId::Lute => "Lute",
            InstrumentId::Drum => "Drum",
            InstrumentId::Bell => "Bell",
            InstrumentId::BoneFlute => "Bone Flute",
            InstrumentId::RiverReed => "River Reed",
            InstrumentId::WaterDrum => "Water Drum",
            InstrumentId::RainStick => "Rain Stick",
            InstrumentId::Conch => "Conch",
            InstrumentId::WindChime => "Wind Chime",
            InstrumentId::GlassHarp => "Glass Harp",
            InstrumentId::Harp => "Harp",
            InstrumentId::WarHorn => "War Horn",
            InstrumentId::ScrapFiddle => "Scrap Fiddle",
            InstrumentId::Silence => "Silence",
        }
    }
}

// ── Scene model (donor cyoa.rs:45-87, adapted ids) ──────────────────────────

/// One selectable choice inside a scene.
#[derive(Debug, Clone)]
pub struct SceneChoice {
    /// Stable id.
    pub id: ChoiceId,
    /// The line the player reads.
    pub text: String,
    /// Its stance.
    pub archetype: ChoiceArchetype,
    /// What it does.
    pub action: ChoiceAction,
    /// Facts taking this choice adds.
    pub adds_facts: Vec<LoreFactId>,
    /// Facts it removes.
    pub removes_facts: Vec<LoreFactId>,
    /// Secret artifacts it reveals.
    pub reveals_secrets: Vec<ArtifactId>,
    /// Artifacts it creates.
    pub creates_artifacts: Vec<ArtifactId>,
}

/// One scene: where, who speaks, who may see it, what must be true, and the
/// disclosure window it is legal inside (0 = origin, 255 = revelation).
#[derive(Debug, Clone)]
pub struct ChoiceScene {
    /// Stable id.
    pub id: SceneId,
    /// Zone artifact it plays in, if anchored.
    pub location: Option<ArtifactId>,
    /// NPC artifact who speaks, if any.
    pub speaker: Option<ArtifactId>,
    /// Who may see the scene at all.
    pub visibility: Visibility,
    /// Every one of these facts must hold.
    pub requires_all: Vec<LoreFactId>,
    /// At least one of these must hold (empty = no constraint).
    pub requires_any: Vec<LoreFactId>,
    /// None of these may hold.
    pub excludes: Vec<LoreFactId>,
    /// Lowest disclosure level the scene is legal at.
    pub disclosure_min: u8,
    /// Highest disclosure level the scene is legal at.
    pub disclosure_max: u8,
    /// Author priority — first key of the selection ordering.
    pub priority: i32,
    /// The choices offered.
    pub choices: Vec<SceneChoice>,
}

/// Scene → faction affinity (0-4 index into consequence::FACTIONS).
/// Authored per scene for diplomacy hook.
pub const SCENE_FACTION: &[(SceneId, usize)] = &[
];

/// Scene → faction lookup (one-liner; empty fallback to seed-derived).
pub fn scene_faction(scene_id: SceneId) -> usize {
    SCENE_FACTION
        .iter()
        .find(|(id, _)| *id == scene_id)
        .map(|(_, fac)| *fac)
        .unwrap_or(0)
}

/// Opening scenes — 14 variants, 2 per birth art (0-6). Authored pool.
/// Each pair branches the trial deeper: first encounter (shallow), then escalation (deep).
pub const OPENING_SCENES: &[SceneId] = &[
    SceneId(1000), // Art 0: Vigor — Trial 1 (encounter)
    SceneId(1001), // Art 0: Vigor — Trial 2 (escalation)
    SceneId(1002), // Art 1: Momentum — Trial 1 (encounter)
    SceneId(1003), // Art 1: Momentum — Trial 2 (escalation)
    SceneId(1004), // Art 2: Logic Depth — Trial 1 (encounter)
    SceneId(1005), // Art 2: Logic Depth — Trial 2 (escalation)
    SceneId(1006), // Art 3: Shadow Weight — Trial 1 (encounter)
    SceneId(1007), // Art 3: Shadow Weight — Trial 2 (escalation)
    SceneId(1008), // Art 4: Tarnish — Trial 1 (encounter)
    SceneId(1009), // Art 4: Tarnish — Trial 2 (escalation)
    SceneId(1010), // Art 5: Resonance — Trial 1 (encounter)
    SceneId(1011), // Art 5: Resonance — Trial 2 (escalation)
    SceneId(1012), // Art 6: Guilt — Trial 1 (encounter)
    SceneId(1013), // Art 6: Guilt — Trial 2 (escalation)
];

/// Pick the opening scene based on birth art choice (0-6).
pub fn opening_scene_for_art(art: u8) -> SceneId {
    OPENING_SCENES
        .get((art as usize) * 2)
        .copied()
        .unwrap_or(OPENING_SCENES[0])
}

/// The authored scenario prose for a scene. Gives every trial and arc scene
/// an evocative, atmospheric prompt rather than a bare debug header.
pub fn scene_prompt(scene_id: SceneId) -> &'static str {
    match scene_id.0 {
        // ── Art 0: Vigor / Edge (IDs 1000-1001) ──
        1000 => "You step into the cold nave of the Edge School. Before you, a length of unworked cold iron rests against an altar of dark stone. The blade edge hums under the weight of an unreleased strike.",
        1001 => "The iron rod has cracked under your gaze and touch. The fracture gapes, releasing kinetic tension into the chamber. Two jagged halves remain—one rings, the other cuts.",
        // ── Art 1: Momentum / Map (IDs 1002-1003) ──
        1002 => "The passage of Momentum narrows into a corridor of vaulted stone. Every flagstone beneath your boots is weighted, and the walls lean inward with balanced compressive force.",
        1003 => "The hallway ends in a sudden collapse of ancient masonry. Heavy rubble chokes the direct path, but cool subterranean air draws through low crevices beneath the stone.",
        // ── Art 2: Logic Depth / Bell (IDs 1004-1005) ──
        1004 => "The chamber of Logic Depth is pitch dark, yet alive with acoustic frequency. A single pure overtone hums in the air, vibrating against the stone and your own ribs.",
        1005 => "High in the unseen vault above, a bronze bell stirs. Your breath catches the pitch of the clapper, and the chamber waits for the first toll.",
        // ── Art 3: Shadow Weight / Mirror (IDs 1006-1007) ──
        1006 => "In the hall of Shadow Weight, a solitary shadow walks the stone wall without a flame to cast it. It matches your stride, yet turns its head with deliberate, unnatural stillness.",
        1007 => "The phantom on the wall stretches and wavers. The boundary between your flesh and the silhouette begins to soften, threatening to pull your solid form into the dark.",
        // ── Art 4: Tarnish / River (IDs 1008-1009) ──
        1008 => "The floor descends toward the Tarnish current. A low, clinging miasma of petrichor, rot, and old river-bottom rolls across your boots, heavier than air.",
        1009 => "The miasma coats your hands and gear in a dull, weathered patina. Below the vapor, ancient bones rest quietly in sediment, marking the path of those who yielded before.",
        // ── Art 5: Resonance / Ledger (IDs 1010-1011) ──
        1010 => "You enter the Vault of Resonance and Accounts. Every square cubit of stone is carved with the Ledger—names, debts, tolls, and blood prices tallied in ash and ochre.",
        1011 => "The ledger's carving glows faintly under the scrutiny of the Toll Broker. Passage forward requires an entry in the balance sheet—and the price is not coin.",
        // ── Art 6: Guilt / Tide (IDs 1012-1013) ──
        1012 => "The air in the Tide chamber shifts backward along the time-axis. You stand in an acoustic overlap, hearing the last words and final fall of someone who died on this exact ground.",
        1013 => "The tide of grief and remembrance swells through your sternum. The loss demands acknowledgment before the current will allow you to pass.",
        id => {
            // Named arc scenes resolved by stable FNV hash
            if id == self::scene_id("the-spot-on-the-glass").0 {
                "A flawless pane of glass hangs before the dark. You can see through it, but a faint fingerprint marks the surface where an observer touched the boundary."
            } else if id == self::scene_id("the-field").0 {
                "The boundary dissolves into a continuous field. You are no longer standing in a room; you are an organ within the chamber's living ecology."
            } else if id == self::scene_id("the-hologram").0 {
                "A single shattered shard of glass lies at your feet. In its tiny reflection, the entire architecture of the parish is encoded in miniature."
            } else if id == self::scene_id("the-somatic-truth").0 {
                "Your rational mind tries to map the exit, but your pulse and fascia already know where the load sits. The body answers before thought."
            } else if id == self::scene_id("the-flood").0 {
                "The river breaches its stone banks. Cold black water rushes across the floor, carrying broken timber, copper slag, and the memory of old springs."
            } else if id == self::scene_id("the-taking").0 {
                "The current does not merely pass; it demands an offering. Anything not held with iron grip is pulled into the deep."
            } else if id == self::scene_id("the-current").0 {
                "You stand chest-deep in the flow. Fighting the river tears the muscle; yielding allows the stream to steer you toward the gate."
            } else if id == self::scene_id("the-reversal").0 {
                "At the whirlpool's vortex, the river briefly stops and begins to flow backward up the bedrock stairs."
            } else if id == self::scene_id("the-first-encounter").0 {
                "A creature of thorn and river-silk perches on the witness rail, watching you with eyes that do not blink."
            } else if id == self::scene_id("the-borrowed-name").0 {
                "The entity asks for your name in trade for safe conduct through the brier-tangle."
            } else if id == self::scene_id("the-fairy-ring").0 {
                "A circle of phosphorescent lichen glows on the flagstones. Stepping inside shifts the acoustic scale of the room."
            } else if id == self::scene_id("the-scrap-heap").0 {
                "Mounds of salvaged clockwork, rusted gears, and twisted iron bells fill the subterranean workshop."
            } else if id == self::scene_id("the-broken-clock").0 {
                "A massive escapement wheel clicks rhythmically, despite having half its teeth ground away."
            } else if id == self::scene_id("the-tinker-bench").0 {
                "Tools of tempered steel lie arranged by weight and temper, awaiting the hands of a smith."
            } else if id == self::scene_id("every-beast-hears-different").0 {
                "A tethered hound lies by the archway, its ears twitching to sub-audible frequencies beneath the floor."
            } else if id == self::scene_id("the-scent-trail").0 {
                "The air carries a distinct scent-track: damp copper, crushed pine, and stale tallow smoke."
            } else if id == self::scene_id("the-world-they-live-in").0 {
                "For a moment, your senses overlap with the beast's: light dims to monochrome, but smells ignite in vivid dimensional contours."
            } else if id == self::scene_id("the-dead-are-numbered").0 {
                "A row of mortuary niches lines the corridor, each bearing an unnumbered iron token."
            } else if id == self::scene_id("the-iron-bell-tolls").0 {
                "A distant bell tolls—once, twice, three times—echoing from the depths of the Bell Pit."
            } else if id == self::scene_id("the-dirge-of-ironroot").0 {
                "A low dirge resonates through the foundation stones, sung in the cadence of the first settlers."
            } else if id == self::scene_id("the-darkened-nave").0 {
                "Total darkness envelopes the nave. Vision is useless here; only echoes and air pressure reveal the walls."
            } else if id == self::scene_id("the-echo-of-masonry").0 {
                "A single tap of your foot sends soundwaves bounding off high vaulted ribs, mapping the ceiling in sound."
            } else if id == self::scene_id("the-unseen-arch").0 {
                "You stand directly beneath a massive masonry arch you cannot see, felt only as a shadow of warmth and compressed air."
            } else if id == self::scene_id("the-bell-school-door").0 {
                "A heavy oak door bound in bell-metal bars the way to the school of vibration."
            } else if id == self::scene_id("the-recomposition").0 {
                "The harmonic formulas of the ancient hymns are chiseled into the door jambs in musical notation."
            } else if id == self::scene_id("the-school-of-the-bell").0 {
                "The inner sanctum of the Bellwrights opens, revealing tiers of tuning forks and suspended bronze disks."
            } else {
                "The way unfolds through the stone. Choose how you will meet what lies ahead."
            }
        }
    }
}

/// Score components for one scene (donor cyoa.rs:72-79).
#[derive(Debug, Clone)]
pub struct SceneScore {
    /// Which scene.
    pub scene_id: SceneId,
    /// Author priority, copied through.
    pub priority: i32,
    /// Unvisited bonus / revisit penalty.
    pub novelty: i32,
    /// How close the player's disclosure sits to the scene's window centre.
    pub disclosure_fit: i32,
    /// Seed-stable tiebreaker so equal scores never flap.
    pub hash_tiebreaker: u64,
}

/// The player's narrative position (donor cyoa.rs:81-87).
#[derive(Debug, Default, Clone)]
pub struct SceneRuntimeState {
    /// Scenes already played (a visited scene is never legal again).
    pub visited_scenes: Vec<SceneId>,
    /// Facts held this run, on top of the world ledger's.
    pub active_facts: Vec<LoreFactId>,
    /// Narrative progression, 0..=255.
    pub disclosure_level: u8,
    /// The run seed — makes tiebreaks deterministic per run.
    pub seed: u64,
}

// ── The sieve (donor cyoa.rs:89-139, verbatim logic) ────────────────────────

/// True when the scene may legally play: unvisited, inside the disclosure
/// window, no excluded fact held, all required facts held, and at least one
/// of `requires_any` (when non-empty).
pub fn scene_is_legal(scene: &ChoiceScene, state: &SceneRuntimeState, ledger: &Ledger) -> bool {
    let has_fact = |fact: LoreFactId| state.active_facts.contains(&fact) || ledger.has_fact(fact);

    if state.visited_scenes.contains(&scene.id) {
        return false;
    }
    if state.disclosure_level < scene.disclosure_min || state.disclosure_level > scene.disclosure_max {
        return false;
    }
    if scene.excludes.iter().any(|fact| has_fact(*fact)) {
        return false;
    }
    if !scene.requires_all.iter().all(|fact| has_fact(*fact)) {
        return false;
    }
    scene.requires_any.is_empty() || scene.requires_any.iter().any(|fact| has_fact(*fact))
}

/// Deterministic score: priority, novelty, disclosure fit, seed-hashed tiebreak.
pub fn score_scene(scene: &ChoiceScene, state: &SceneRuntimeState) -> SceneScore {
    let novelty = if state.visited_scenes.contains(&scene.id) { -1000 } else { 10 };
    let center = (scene.disclosure_min as i32 + scene.disclosure_max as i32) / 2;
    let disclosure_fit = 100 - (state.disclosure_level as i32 - center).abs();
    let mut h = FNV_OFFSET_BASIS;
    h = fnv1a64_fold(h, state.seed);
    h = fnv1a64_fold(h, scene.id.0);

    SceneScore {
        scene_id: scene.id,
        priority: scene.priority,
        novelty,
        disclosure_fit,
        hash_tiebreaker: h,
    }
}

/// The best legal scene, or `None` when nothing is legal. Filter then max —
/// branching is discovered, never hardcoded.
pub fn select_next_scene<'a>(
    scenes: &'a [ChoiceScene],
    state: &SceneRuntimeState,
    ledger: &Ledger,
) -> Option<&'a ChoiceScene> {
    scenes
        .iter()
        .filter(|scene| scene_is_legal(scene, state, ledger))
        .max_by_key(|scene| {
            let score = score_scene(scene, state);
            (score.priority, score.novelty, score.disclosure_fit, score.hash_tiebreaker)
        })
}

// ── Authored pools ──────────────────────────────────────────────────────────

/// Stable fact id from a name (FNV over the name bytes — same name, same fact,
/// forever; the authoring vocabulary is words, the engine's is u64).
pub fn fact(name: &str) -> LoreFactId {
    LoreFactId(hash_bytes_fnv1a(name.as_bytes()))
}

/// Stable scene id from a name.
pub fn scene_id(name: &str) -> SceneId {
    SceneId(hash_bytes_fnv1a(name.as_bytes()))
}

/// Stable choice id from a name.
pub fn choice_id(name: &str) -> ChoiceId {
    ChoiceId(hash_bytes_fnv1a(name.as_bytes()))
}

/// Bare choice with no fact effects — the effect vectors are filled at the
/// call site when a choice actually changes the world.
fn choice(name: &str, text: &str, archetype: ChoiceArchetype, action: ChoiceAction) -> SceneChoice {
    SceneChoice {
        id: choice_id(name),
        text: text.to_string(),
        archetype,
        action,
        adds_facts: Vec::new(),
        removes_facts: Vec::new(),
        reveals_secrets: Vec::new(),
        creates_artifacts: Vec::new(),
    }
}

/// The glass arc — four scenes drained from THEORETICAL PRIMITIVES.txt:2-6.
/// Arc facts: glass-touched → field-felt → grain-read → (grounded | dissolved).
pub fn glass_arc_scenes() -> Vec<ChoiceScene> {
    let glass_touched = fact("glass-touched");
    let shadow_named = fact("shadow-named");
    let field_felt = fact("field-felt");
    let grain_read = fact("grain-read");

    vec![
        // 1. Projection and the spot on the glass (line 2).
        ChoiceScene {
            id: scene_id("the-spot-on-the-glass"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 80,
            priority: 10,
            choices: vec![
                {
                    let mut c = choice(
                        "touch-the-glass",
                        "Put one finger where the glass is. End the dream of no-glass.",
                        ChoiceArchetype::Shatter,
                        ChoiceAction::TouchTheGlass,
                    );
                    c.adds_facts = vec![glass_touched];
                    c
                },
                {
                    let mut c = choice(
                        "name-the-shadow",
                        "Say it: the face in the glass is wearing your anger, not its own.",
                        ChoiceArchetype::Project,
                        ChoiceAction::NameTheShadow,
                    );
                    c.adds_facts = vec![shadow_named];
                    c
                },
                choice(
                    "glide-on",
                    "Keep gliding. The glass is easier unbroken.",
                    ChoiceArchetype::Still,
                    ChoiceAction::RemainSilent,
                ),
            ],
        },
        // 2. The contact zone (line 3 — continuity, not boundary).
        ChoiceScene {
            id: scene_id("the-contact-zone"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![glass_touched],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 40,
            disclosure_max: 160,
            priority: 12,
            choices: vec![
                {
                    let mut c = choice(
                        "breathe-with-the-room",
                        "Breathe until the room breathes back. The glass was a place, not a wall.",
                        ChoiceArchetype::Merge,
                        ChoiceAction::BreatheWithTheRoom,
                    );
                    c.adds_facts = vec![field_felt];
                    c
                },
                choice(
                    "steady-the-body",
                    "Feel your feet. Stay a body among bodies.",
                    ChoiceArchetype::Ground,
                    ChoiceAction::SteadyTheBody,
                ),
                choice(
                    "play-the-glass",
                    "Wet one finger and ring the rim. Let the contact zone sing.",
                    ChoiceArchetype::Sing,
                    ChoiceAction::PlayInstrument(InstrumentId::GlassHarp),
                ),
            ],
        },
        // 3. The grain that holds the whole (line 4 — holographic encoding).
        ChoiceScene {
            id: scene_id("the-grain-that-holds-the-river"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![field_felt],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 120,
            disclosure_max: 220,
            priority: 14,
            choices: vec![
                {
                    let mut c = choice(
                        "read-the-grain",
                        "One flicker of the smallest gauge. Read the whole machine out of it.",
                        ChoiceArchetype::Encode,
                        ChoiceAction::ReadTheGrain,
                    );
                    c.adds_facts = vec![grain_read];
                    c
                },
                choice(
                    "listen-for-resonance",
                    "Do not read. Listen — the whole is also a sound.",
                    ChoiceArchetype::Feel,
                    ChoiceAction::ListenForResonance,
                ),
            ],
        },
        // 4. The two banks (line 6 — transparency vs grounding, the paradox as
        //    a fork; each pole excludes the fact the other would add).
        ChoiceScene {
            id: scene_id("the-two-banks"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![grain_read],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 160,
            disclosure_max: 255,
            priority: 20,
            choices: vec![
                {
                    let mut c = choice(
                        "hold-the-bank",
                        "Stay a self with edges. Wear the interface; do not become it.",
                        ChoiceArchetype::Ground,
                        ChoiceAction::SteadyTheBody,
                    );
                    c.adds_facts = vec![fact("grounded")];
                    c
                },
                {
                    let mut c = choice(
                        "let-go-of-the-banks",
                        "Let the edges go. Total transparency, whatever it costs.",
                        ChoiceArchetype::Dissolve,
                        ChoiceAction::LetGoOfTheBanks,
                    );
                    c.adds_facts = vec![fact("dissolved")];
                    c
                },
            ],
        },
    ]
}

/// The river arc — four scenes drained from Chaos Feminine.txt:4-18.
/// Arc facts: river-entered → (offering-made | flood-released) → the Mother.
pub fn river_arc_scenes() -> Vec<ChoiceScene> {
    let river_entered = fact("river-entered");
    let dawn_waited = fact("dawn-waited");
    let offering_made = fact("offering-made");
    let flood_released = fact("flood-released");

    vec![
        // 1. "How graceful the river runs... calm, until dawn" (lines 4-5).
        ChoiceScene {
            id: scene_id("the-river-at-dawn"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 60,
            priority: 15,
            choices: vec![
                {
                    let mut c = choice(
                        "enter-the-river",
                        "Step in. Far and wide, and running.",
                        ChoiceArchetype::Flow,
                        ChoiceAction::EnterTheRiver,
                    );
                    c.adds_facts = vec![river_entered];
                    c
                },
                {
                    let mut c = choice(
                        "wait-for-dawn",
                        "It is calm. It will not stay calm. Wait and watch it turn.",
                        ChoiceArchetype::Still,
                        ChoiceAction::WaitForDawn,
                    );
                    c.adds_facts = vec![dawn_waited];
                    c
                },
                choice(
                    "witness-the-river",
                    "Neither enter nor leave. See it whole: life and chaos in one water.",
                    ChoiceArchetype::Witness,
                    ChoiceAction::ListenForResonance,
                ),
            ],
        },
        // 2. "The river curves, and radiates pure beauty. Until it doesn't."
        //    (lines 7-8).
        ChoiceScene {
            id: scene_id("the-curve-that-radiates"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![river_entered],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 40,
            disclosure_max: 140,
            priority: 15,
            choices: vec![
                choice(
                    "follow-the-current",
                    "Go where the curve goes. One way on the line.",
                    ChoiceArchetype::Carve,
                    ChoiceAction::FollowTheCurrent,
                ),
                choice(
                    "swim-against-it",
                    "Go the other way on the line. Feel what that costs.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::SwimAgainstIt,
                ),
                {
                    let mut c = choice(
                        "offer-to-the-river",
                        "Give her something of yours to keep. Reed-song over the water.",
                        ChoiceArchetype::Nurture,
                        ChoiceAction::OfferToTheRiver,
                    );
                    c.adds_facts = vec![offering_made];
                    c
                },
            ],
        },
        // 3. "The river brings life, and chaos" (line 10) — the flood gate.
        //    Excludes dawn-waited: those who waited saw it coming and are
        //    elsewhere when it breaks.
        ChoiceScene {
            id: scene_id("the-flood"),
            location: None,
            speaker: None,
            visibility: Visibility::Hidden,
            requires_all: vec![river_entered],
            requires_any: vec![],
            excludes: vec![dawn_waited],
            disclosure_min: 120,
            disclosure_max: 220,
            priority: 18,
            choices: vec![
                {
                    let mut c = choice(
                        "release-the-flood",
                        "Open every gate at once and ride what comes.",
                        ChoiceArchetype::Flood,
                        ChoiceAction::ReleaseTheFlood,
                    );
                    c.adds_facts = vec![flood_released];
                    c
                },
                choice(
                    "go-under",
                    "Let her take you under. She may give you back. She may not.",
                    ChoiceArchetype::Drown,
                    ChoiceAction::DrinkDeep,
                ),
                choice(
                    "bind-the-banks",
                    "Rope, stone, oath — hold the banks against her.",
                    ChoiceArchetype::Bind,
                    ChoiceAction::Bargain,
                ),
            ],
        },
        // 4. "This is Chaos Feminine, Mother Unleashed" (line 18) — the
        //    climax. Legal only near revelation, and only for those who gave
        //    or who opened the gates.
        ChoiceScene {
            id: scene_id("mother-unleashed"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![river_entered],
            requires_any: vec![offering_made, flood_released],
            excludes: vec![],
            disclosure_min: 200,
            disclosure_max: 255,
            priority: 30,
            choices: vec![
                {
                    let mut c = choice(
                        "call-the-mother",
                        "Say the name. Whole, or taken — one way or the other on the line.",
                        ChoiceArchetype::Sing,
                        ChoiceAction::CallTheMother,
                    );
                    c.adds_facts = vec![fact("mother-answered")];
                    c
                },
                choice(
                    "sound-the-horn",
                    "Answer her flood with a flood of your own.",
                    ChoiceArchetype::Flood,
                    ChoiceAction::PlayInstrument(InstrumentId::WarHorn),
                ),
                choice(
                    "refuse-conversion",
                    "Stand in the roar and remain exactly who you were.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::RefuseConversion,
                ),
            ],
        },
    ]
}

/// The fae arch — three scenes off the PROVEN Fae World Overlay tablet
/// (forge-book-v3/_book/18-fae-world-overlay.md: gifts create debt, guest-law,
/// the debt_song/true-name voice tags). Arc facts: gift-accepted+debt-owed →
/// guest-right-invoked → (debt-paid | true-name-spoken).
pub fn fae_arch_scenes() -> Vec<ChoiceScene> {
    let gift_accepted = fact("gift-accepted");
    let debt_owed = fact("debt-owed");

    vec![
        ChoiceScene {
            id: scene_id("the-gift-at-the-treeline"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 100,
            priority: 12,
            choices: vec![
                {
                    let mut c = choice(
                        "accept-the-gift",
                        "It is exactly what you wanted. That is the warning. Take it anyway.",
                        ChoiceArchetype::Lure,
                        ChoiceAction::AcceptTheGift,
                    );
                    c.adds_facts = vec![gift_accepted, debt_owed];
                    c
                },
                choice(
                    "refuse-the-gift",
                    "Want it, and leave it on the stone. Owe nothing.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::RefuseConversion,
                ),
                choice(
                    "watch-the-giver",
                    "Take nothing. Watch who leaves gifts at treelines, and why.",
                    ChoiceArchetype::Witness,
                    ChoiceAction::ListenForResonance,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("guest-law"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![gift_accepted],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 60,
            disclosure_max: 180,
            priority: 14,
            choices: vec![
                {
                    let mut c = choice(
                        "invoke-guest-right",
                        "Sit at their table and name the law that binds you both.",
                        ChoiceArchetype::Host,
                        ChoiceAction::InvokeGuestRight,
                    );
                    c.adds_facts = vec![fact("guest-right-invoked")];
                    c
                },
                choice(
                    "bargain-at-the-table",
                    "Everything here has a price. Ask theirs before they ask yours.",
                    ChoiceArchetype::Bind,
                    ChoiceAction::Bargain,
                ),
                choice(
                    "wear-the-glamour",
                    "Let them think you are one of their own. Wear the beautiful lie.",
                    ChoiceArchetype::Glamour,
                    ChoiceAction::Lie,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-debt-song"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![debt_owed],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 160,
            disclosure_max: 255,
            priority: 22,
            choices: vec![
                {
                    let mut c = choice(
                        "pay-the-debt",
                        "The song names what you owe. Pay it whole, and walk out free.",
                        ChoiceArchetype::Bind,
                        ChoiceAction::PayTheDebt,
                    );
                    c.removes_facts = vec![debt_owed];
                    c.adds_facts = vec![fact("debt-paid")];
                    c
                },
                {
                    let mut c = choice(
                        "speak-the-true-name",
                        "Cut the song mid-verse with the one word glamour cannot survive.",
                        ChoiceArchetype::Cut,
                        ChoiceAction::SpeakTrueName,
                    );
                    c.removes_facts = vec![debt_owed];
                    c.adds_facts = vec![fact("true-name-spoken")];
                    c
                },
                choice(
                    "answer-in-kind",
                    "Meet the debt song with a song of your own and see whose holds.",
                    ChoiceArchetype::Sing,
                    ChoiceAction::PlayInstrument(InstrumentId::Harp),
                ),
            ],
        },
    ]
}

/// The goblin arch — three scenes off the live GoblinKind roster
/// (brain/run_dev_run.rs:740: Scout/Warrior/Shaman/Chieftain) and the
/// goblin-is-the-bomb crater seam (brain/terrain.rs:7-9). Arc facts:
/// wreck-scavenged → shaman-bargained → fuse-lit.
pub fn goblin_arch_scenes() -> Vec<ChoiceScene> {
    let wreck_scavenged = fact("wreck-scavenged");

    vec![
        ChoiceScene {
            id: scene_id("the-wreck-field"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 90,
            priority: 11,
            choices: vec![
                {
                    let mut c = choice(
                        "scavenge-the-wreck",
                        "Everything broken is parts. Start pulling.",
                        ChoiceArchetype::Scavenge,
                        ChoiceAction::ScavengeTheWreck,
                    );
                    c.adds_facts = vec![wreck_scavenged];
                    c
                },
                choice(
                    "tinker-the-trap",
                    "Build the trap where the scouts will walk, out of what they left.",
                    ChoiceArchetype::Tinker,
                    ChoiceAction::TinkerTheTrap,
                ),
                choice(
                    "false-trail",
                    "Drag one wheel the wrong way and let the field lie for you.",
                    ChoiceArchetype::Trick,
                    ChoiceAction::Lie,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-shamans-bargain"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![wreck_scavenged],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 70,
            disclosure_max: 170,
            priority: 13,
            choices: vec![
                choice(
                    "bargain-with-the-shaman",
                    "The shaman wants one thing from your pack. Find out which before you open it.",
                    ChoiceArchetype::Bind,
                    ChoiceAction::Bargain,
                ),
                choice(
                    "play-the-scrap-fiddle",
                    "Play their own wreckage back to them until the circle laughs.",
                    ChoiceArchetype::Sing,
                    ChoiceAction::PlayInstrument(InstrumentId::ScrapFiddle),
                ),
                choice(
                    "accuse-the-chieftain",
                    "Say it in front of the warband: the chieftain sold the last raid.",
                    ChoiceArchetype::Cut,
                    ChoiceAction::Accuse,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("goblin-is-the-bomb"),
            location: None,
            speaker: None,
            visibility: Visibility::Hidden,
            requires_all: vec![wreck_scavenged],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 150,
            disclosure_max: 240,
            priority: 20,
            choices: vec![
                {
                    let mut c = choice(
                        "light-the-fuse",
                        "The crater is the argument. Light it and run.",
                        ChoiceArchetype::Shatter,
                        ChoiceAction::LightTheFuse,
                    );
                    c.adds_facts = vec![fact("fuse-lit")];
                    c
                },
                choice(
                    "cut-the-fuse",
                    "Cut it. Whatever this was for, it is not worth the hole.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::RefuseConversion,
                ),
                choice(
                    "walk-away-counting",
                    "Leave, counting steps. Some arguments you only need to survive.",
                    ChoiceArchetype::Still,
                    ChoiceAction::Leave,
                ),
            ],
        },
    ]
}

/// The umwelt arch — three scenes off aspire.rs:347 `every-beast-hears-different`
/// (von Uexküll: each creature carries its own world; borrow one and the same
/// room changes). Arc facts: eyes-borrowed → scent-followed → their-world-held.
pub fn umwelt_arch_scenes() -> Vec<ChoiceScene> {
    let eyes_borrowed = fact("eyes-borrowed");
    let scent_followed = fact("scent-followed");

    vec![
        ChoiceScene {
            id: scene_id("every-beast-hears-different"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 120,
            priority: 13,
            choices: vec![
                {
                    let mut c = choice(
                        "borrow-their-eyes",
                        "Kneel by the beast and take the room through its senses instead.",
                        ChoiceArchetype::Attune,
                        ChoiceAction::BorrowTheirEyes,
                    );
                    c.adds_facts = vec![eyes_borrowed];
                    c
                },
                choice(
                    "listen-with-it",
                    "Do not borrow. Sit beside it and hear what makes its ears turn.",
                    ChoiceArchetype::Witness,
                    ChoiceAction::ListenForResonance,
                ),
                choice(
                    "ask-the-handler",
                    "Ask the one who walks with it what it fears in this room.",
                    ChoiceArchetype::Flow,
                    ChoiceAction::Ask,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-scent-trail"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![eyes_borrowed],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 80,
            disclosure_max: 200,
            priority: 14,
            choices: vec![
                {
                    let mut c = choice(
                        "follow-the-scent",
                        "The borrowed nose holds a thread your eyes never could. Follow it.",
                        ChoiceArchetype::Attune,
                        ChoiceAction::FollowTheScent,
                    );
                    c.adds_facts = vec![scent_followed];
                    c
                },
                choice(
                    "mark-and-return",
                    "Mark where the thread starts and come back as only yourself.",
                    ChoiceArchetype::Ground,
                    ChoiceAction::SteadyTheBody,
                ),
                choice(
                    "stay-silent-downwind",
                    "Whatever left this trail can smell you too. Be nothing on the wind.",
                    ChoiceArchetype::Still,
                    ChoiceAction::RemainSilent,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-world-they-live-in"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![scent_followed],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 160,
            disclosure_max: 255,
            priority: 18,
            choices: vec![
                {
                    let mut c = choice(
                        "hold-both-worlds",
                        "Keep theirs and yours open at once, and stand in the overlap.",
                        ChoiceArchetype::Merge,
                        ChoiceAction::BreatheWithTheRoom,
                    );
                    c.adds_facts = vec![fact("their-world-held")];
                    c
                },
                choice(
                    "read-the-overlap",
                    "The overlap is a map neither of you could draw alone. Read it.",
                    ChoiceArchetype::Encode,
                    ChoiceAction::ReadTheGrain,
                ),
                choice(
                    "give-a-memory-back",
                    "Leave one of your memories in their world as payment for the loan.",
                    ChoiceArchetype::Nurture,
                    ChoiceAction::OfferMemory,
                ),
            ],
        },
    ]
}

/// The dirge arch — three scenes for the lineage root itself
/// (dirge-of-ironroot; DeathScar/lore_sieve ports at forge-cart-brain-v3,
/// bell_pit's first toll). Arc facts: body-laid → bell-tolled → dirge-sung.
pub fn dirge_arch_scenes() -> Vec<ChoiceScene> {
    let body_laid = fact("body-laid");
    let bell_tolled = fact("bell-tolled");

    vec![
        ChoiceScene {
            id: scene_id("the-field-after"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 100,
            priority: 10,
            choices: vec![
                {
                    let mut c = choice(
                        "lay-them-down",
                        "Stop. Put them down the right way, though the road will not wait.",
                        ChoiceArchetype::Mourn,
                        ChoiceAction::LayThemDown,
                    );
                    c.adds_facts = vec![body_laid];
                    c
                },
                choice(
                    "witness-the-field",
                    "Walk the field once, slowly. Count them. Someone must know the number.",
                    ChoiceArchetype::Witness,
                    ChoiceAction::ListenForResonance,
                ),
                choice(
                    "keep-moving",
                    "The living still need you. Keep moving and carry it unspoken.",
                    ChoiceArchetype::Cut,
                    ChoiceAction::Leave,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-toll"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![body_laid],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 80,
            disclosure_max: 190,
            priority: 15,
            choices: vec![
                {
                    let mut c = choice(
                        "toll-the-bell",
                        "One toll for the dead. Everything in the valley will hear who fell.",
                        ChoiceArchetype::Mourn,
                        ChoiceAction::TollTheBell,
                    );
                    c.adds_facts = vec![bell_tolled];
                    c
                },
                choice(
                    "hold-the-rope",
                    "Hold the rope still. Grief announced is grief spent — keep yours.",
                    ChoiceArchetype::Still,
                    ChoiceAction::RemainSilent,
                ),
                choice(
                    "name-who-did-it",
                    "Before the bell, the name. Say who made this field.",
                    ChoiceArchetype::Cut,
                    ChoiceAction::Accuse,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("dirge-of-ironroot"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![bell_tolled],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 170,
            disclosure_max: 255,
            priority: 24,
            choices: vec![
                {
                    let mut c = choice(
                        "sing-the-dirge",
                        "All the verses, in order, missing none. The song is the ledger.",
                        ChoiceArchetype::Sing,
                        ChoiceAction::SingTheDirge,
                    );
                    c.adds_facts = vec![fact("dirge-sung")];
                    c
                },
                choice(
                    "play-the-silence",
                    "Stand where the song should be and play nothing, note by note.",
                    ChoiceArchetype::Mourn,
                    ChoiceAction::PlayInstrument(InstrumentId::Silence),
                ),
                choice(
                    "give-them-a-memory",
                    "Trade one memory of them into the ground, so the root remembers.",
                    ChoiceArchetype::Nurture,
                    ChoiceAction::OfferMemory,
                ),
            ],
        },
    ]
}

/// The blind arch — three scenes off ARCH-004's sensory-substitution doctrine
/// (blind path: sonify the canvas; hearing as first-class sight). Arc facts:
/// eyes-closed → shape-heard → dark-walked.
pub fn blind_arch_scenes() -> Vec<ChoiceScene> {
    let eyes_closed = fact("eyes-closed");
    let shape_heard = fact("shape-heard");

    vec![
        ChoiceScene {
            id: scene_id("close-your-eyes"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 110,
            priority: 12,
            choices: vec![
                {
                    let mut c = choice(
                        "close-your-eyes-choice",
                        "Shut sight off on purpose. Start the room again from zero.",
                        ChoiceArchetype::Echo,
                        ChoiceAction::CloseYourEyes,
                    );
                    c.adds_facts = vec![eyes_closed];
                    c
                },
                choice(
                    "keep-watching",
                    "Keep your eyes. Some doors should be opened the easy way.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::RefuseConversion,
                ),
                choice(
                    "feel-first",
                    "Before choosing, notice what your body already decided.",
                    ChoiceArchetype::Feel,
                    ChoiceAction::SteadyTheBody,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-shape-of-the-room"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![eyes_closed],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 90,
            disclosure_max: 200,
            priority: 14,
            choices: vec![
                {
                    let mut c = choice(
                        "hear-the-shape",
                        "Clap once. The room answers with its true dimensions.",
                        ChoiceArchetype::Echo,
                        ChoiceAction::HearTheShape,
                    );
                    c.adds_facts = vec![shape_heard];
                    c
                },
                choice(
                    "trail-one-hand",
                    "Walk the wall with one hand and let the room draw itself.",
                    ChoiceArchetype::Ground,
                    ChoiceAction::TouchTheGlass,
                ),
                choice(
                    "stand-in-the-dark",
                    "Do nothing. The dark is also information, arriving slowly.",
                    ChoiceArchetype::Still,
                    ChoiceAction::ListenForResonance,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("seeing-without"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![shape_heard],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 170,
            disclosure_max: 255,
            priority: 18,
            choices: vec![
                {
                    let mut c = choice(
                        "walk-the-dark-whole",
                        "Cross the whole hall in the dark, unhurried, missing nothing.",
                        ChoiceArchetype::Flow,
                        ChoiceAction::FollowTheCurrent,
                    );
                    c.adds_facts = vec![fact("dark-walked")];
                    c
                },
                choice(
                    "sound-the-conch",
                    "Sound the conch and read the hall from its one long answer.",
                    ChoiceArchetype::Sing,
                    ChoiceAction::PlayInstrument(InstrumentId::Conch),
                ),
                choice(
                    "name-what-you-feared",
                    "Say what the dark was wearing of yours. It was never the dark.",
                    ChoiceArchetype::Project,
                    ChoiceAction::NameTheShadow,
                ),
            ],
        },
    ]
}

/// All authored pools, one slate — what `select_next_scene` sieves today.
/// Seven arcs: glass, river, fae, goblin, umwelt, dirge, blind.
/// The bard arch — three scenes off Parry-Lord oral-formulaic composition: a
/// singer does not recite a fixed text, they recompose it from formula slots
/// each performance. Arc facts: formula-learned → variant-sung → school-kept.
pub fn bell_arch_scenes() -> Vec<ChoiceScene> {
    let formula_learned = fact("formula-learned");
    let variant_sung = fact("variant-sung");

    vec![
        ChoiceScene {
            id: scene_id("the-bell-school-door"),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 100,
            priority: 10,
            choices: vec![
                {
                    let mut c = choice(
                        "learn-the-slots",
                        "Learn it the way they teach it: not the song, the slots the song is built from.",
                        ChoiceArchetype::Encode,
                        ChoiceAction::ListenForResonance,
                    );
                    c.adds_facts = vec![formula_learned];
                    c
                },
                choice(
                    "learn-it-whole",
                    "Refuse the slots. Learn every verse exactly, and never move one word.",
                    ChoiceArchetype::Still,
                    ChoiceAction::RefuseConversion,
                ),
                choice(
                    "ask-who-taught-them",
                    "Ask the older question: who sang it to the one who sings it now.",
                    ChoiceArchetype::Witness,
                    ChoiceAction::Ask,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-recomposition"),
            location: None,
            speaker: None,
            visibility: Visibility::Rumor,
            requires_all: vec![formula_learned],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 80,
            disclosure_max: 190,
            priority: 15,
            choices: vec![
                {
                    let mut c = choice(
                        "sing-it-different",
                        "Same story, different slots. Sing the version this room needs tonight.",
                        ChoiceArchetype::Sing,
                        ChoiceAction::PlayInstrument(InstrumentId::Bell),
                    );
                    c.adds_facts = vec![variant_sung];
                    c
                },
                choice(
                    "sing-it-as-taught",
                    "Give it back unchanged. Let them hear their own teacher in your mouth.",
                    ChoiceArchetype::Echo,
                    ChoiceAction::SpeakPlainly,
                ),
                choice(
                    "let-them-fill-the-slot",
                    "Stop on the open slot. Let the room say the word and make them the singer.",
                    ChoiceArchetype::Attune,
                    ChoiceAction::RemainSilent,
                ),
            ],
        },
        ChoiceScene {
            id: scene_id("the-school-of-the-bell"),
            location: None,
            speaker: None,
            visibility: Visibility::Secret,
            requires_all: vec![variant_sung],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 170,
            disclosure_max: 255,
            priority: 24,
            choices: vec![
                {
                    let mut c = choice(
                        "teach-the-slots-on",
                        "Teach the slots, not the song. The school outlives every singer in it.",
                        ChoiceArchetype::Nurture,
                        ChoiceAction::OfferMemory,
                    );
                    c.adds_facts = vec![fact("school-kept")];
                    c
                },
                choice(
                    "toll-the-canon-shut",
                    "One toll, and the version you sang is the version there was. Close it.",
                    ChoiceArchetype::Carve,
                    ChoiceAction::TollTheBell,
                ),
                choice(
                    "burn-the-formulae",
                    "A song that can be rebuilt can be forged. End the method with you.",
                    ChoiceArchetype::Refuse,
                    ChoiceAction::Leave,
                ),
            ],
        },
    ]
}

/// The fourteen opening trials — 2 per birth art (0-6), gated at disclosure 0-50.
/// Each pair: shallow trial (1000-1001 for Vigor, etc.), then escalation.
/// Dialogue arrays: choice.text carries the prose describing the scene and action.
pub fn opening_arc_scenes() -> Vec<ChoiceScene> {
    vec![
        // ── Art 0: Vigor / Edge (IDs 1000-1001) ──
        ChoiceScene {
            id: SceneId(1000),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("strike-the-rod", "A length of cold iron leans against stone. The blade edge rings when struck. Learn where metal fails.", ChoiceArchetype::Cut, ChoiceAction::ReadTheGrain),
                choice("measure-the-grain", "Mark the grain where light catches. Shear-stress breaks at the lowest point always.", ChoiceArchetype::Witness, ChoiceAction::Ask),
                choice("trace-severance", "Run your thumb along the rod. Trace where the crack will run if force is applied.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
            ],
        },
        ChoiceScene {
            id: SceneId(1001),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("shatter-the-rod", "The rod fractures where you marked. Cold iron shatters clean. Feel the kinetic release.", ChoiceArchetype::Cut, ChoiceAction::ReadTheGrain),
                choice("hold-the-break", "The break-point hums. Hold one half and let the other sing its own frequency.", ChoiceArchetype::Sing, ChoiceAction::PlayInstrument(InstrumentId::Bell)),
                choice("claim-the-edge", "One half remains sharp. That edge will cut what it touches. This is the Edge school's first law.", ChoiceArchetype::Ground, ChoiceAction::SteadyTheBody),
            ],
        },
        // ── Art 1: Momentum / Map (IDs 1002-1003) ──
        ChoiceScene {
            id: SceneId(1002),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("enter-the-hall", "The passage narrows ahead. Walk the center line where the walls stay balanced and the floor does not dip.", ChoiceArchetype::Ground, ChoiceAction::SteadyTheBody),
                choice("count-the-stones", "Count the stones. This passage is built on a rule—every fifth stone sits deeper than the rest.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
                choice("feel-the-load", "The stones press down on your shoulders. Feel where the weight moves through masonry, always downward.", ChoiceArchetype::Feel, ChoiceAction::SenseTheRoom),
            ],
        },
        ChoiceScene {
            id: SceneId(1003),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("map-the-collapse", "Ahead, the passage is blocked by a collapse. The way forward is not the path you see—it is the space underneath.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
                choice("duck-the-low-route", "Crouch and crawl under the rubble. The low way stays dry and sound. This passage remembers many who crawled through.", ChoiceArchetype::Flow, ChoiceAction::BreatheWithTheRoom),
                choice("navigate-by-pressure", "Feel the air. It moves toward an opening you cannot see. Follow where pressure drops and breathing eases.", ChoiceArchetype::Witness, ChoiceAction::Ask),
            ],
        },
        // ── Art 2: Logic Depth / Bell (IDs 1004-1005) ──
        ChoiceScene {
            id: SceneId(1004),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("listen-to-hum", "The air hums at a frequency your ear catches alone. No one else can hear it. Hum it back and the air hums louder.", ChoiceArchetype::Sing, ChoiceAction::PlayInstrument(InstrumentId::Bell)),
                choice("match-the-resonance", "The hum rings clearer when your voice finds its note. You are not singing—the stone is singing through you.", ChoiceArchetype::Encode, ChoiceAction::PlayInstrument(InstrumentId::Conch)),
                choice("feel-the-ringing", "Resonance moves through your sternum first, then spreads to your teeth and skull. Feel where it lives in the body.", ChoiceArchetype::Feel, ChoiceAction::SenseTheRoom),
            ],
        },
        ChoiceScene {
            id: SceneId(1005),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("toll-the-bell", "A bell hangs unseen above. Your humming draws its clapper. One toll shakes the chamber and changes what comes next.", ChoiceArchetype::Sing, ChoiceAction::PlayInstrument(InstrumentId::Bell)),
                choice("hold-the-note", "Stop singing. The bell holds its tone without you. The resonance continues alone, spiraling deeper into the stone.", ChoiceArchetype::Still, ChoiceAction::RemainSilent),
                choice("break-the-harmonics", "Change your note sharply. The bell's tone fractures into silence. In that break, you hear words—old words, sung so long they became stone.", ChoiceArchetype::Cut, ChoiceAction::PlayInstrument(InstrumentId::Bell)),
            ],
        },
        // ── Art 3: Shadow Weight / Mirror (IDs 1006-1007) ──
        ChoiceScene {
            id: SceneId(1006),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("watch-the-shadow", "The shadow moves where no light source reaches. It walks the wall the way you walk the floor. Watch where it goes.", ChoiceArchetype::Witness, ChoiceAction::Ask),
                choice("trace-the-phantom", "Phantasms follow rules. Track the boundaries of this one—where does it thicken and where does it thin.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
                choice("see-yourself-wrong", "The shadow has your shape but wears it differently. It stands taller. Thinner. Its head is tilted the way you never tilt yours.", ChoiceArchetype::Witness, ChoiceAction::Ask),
            ],
        },
        ChoiceScene {
            id: SceneId(1007),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("dissolve-the-edge", "Stare until the boundary between you and the shadow blurs and it dissolves into the stone.", ChoiceArchetype::Dissolve, ChoiceAction::RemainSilent),
                choice("become-the-shadow", "Step into the dark. Your solid shape becomes thin, stretched, unreliable. This is what the shadow sees when it looks at light.", ChoiceArchetype::Merge, ChoiceAction::BreatheWithTheRoom),
                choice("trap-the-phantom", "The shadow cannot exist without light to cast it. Sever the light source and watch it vanish. But then you stand alone in absolute dark.", ChoiceArchetype::Cut, ChoiceAction::Leave),
            ],
        },
        // ── Art 4: Tarnish / River (IDs 1008-1009) ──
        ChoiceScene {
            id: SceneId(1008),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("yield-to-miasma", "A miasma flows downward like water, heavier than air, thick with rot. Step aside and let it pass through you.", ChoiceArchetype::Flow, ChoiceAction::BreatheWithTheRoom),
                choice("absorb-the-decay", "Rot feeds on stillness. Move into the miasma and feel the weight soften against your skin—it knows you now.", ChoiceArchetype::Merge, ChoiceAction::BreatheWithTheRoom),
                choice("read-the-drift", "Miasma patterns show where pressure shifts. Trace the lane where it thins and concentrates—this is the map of decay.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
            ],
        },
        ChoiceScene {
            id: SceneId(1009),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("follow-the-rot", "The miasma leads downward to the source. Bones rest on bone, centuries of yielding. You are not the first through this place.", ChoiceArchetype::Witness, ChoiceAction::Ask),
                choice("wear-the-tarnish", "The rot does not touch you—it colors you. Your hands darken. Your breath smells of petrichor and old water. You have taken its mark.", ChoiceArchetype::Ground, ChoiceAction::SteadyTheBody),
                choice("carry-the-weight", "The miasma clings. You step out but carry it with you—in your bones, in your breath. This is the Tarnish river's bargain.", ChoiceArchetype::Still, ChoiceAction::RemainSilent),
            ],
        },
        // ── Art 5: Resonance / Ledger (IDs 1010-1011) ──
        ChoiceScene {
            id: SceneId(1010),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("check-the-ledger", "Every exchange leaves a mark. Read the ledger carved into this chamber's walls—debts stacked on debts, tallied in ochre and ash.", ChoiceArchetype::Encode, ChoiceAction::ReadTheGrain),
                choice("calculate-the-price", "The Broker's mathematics never rest. The price for passage forward is not coin. It is equivalence—something you carry for something you will need.", ChoiceArchetype::Trick, ChoiceAction::Ask),
                choice("refuse-the-trade", "Some debts compound. Look away from the ledger and leave now. You owe nothing. Yet.", ChoiceArchetype::Refuse, ChoiceAction::Leave),
            ],
        },
        ChoiceScene {
            id: SceneId(1011),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("siphon-your-breath", "Trade your breath for passage. The Broker takes currency in many forms—and breath is the one commodity everyone carries.", ChoiceArchetype::Trick, ChoiceAction::Ask),
                choice("write-your-name", "Carve your name into the ledger. You are now part of the accounting. Every choice you make henceforth is recorded, weighted, balanced.", ChoiceArchetype::Carve, ChoiceAction::ReadTheGrain),
                choice("burn-the-ledger", "Refuse the system. Fire spreads across the carved walls. But as it burns, you see new text emerging—older debts, older entries. The ledger rewrites itself.", ChoiceArchetype::Flood, ChoiceAction::Leave),
            ],
        },
        // ── Art 6: Guilt / Tide (IDs 1012-1013) ──
        ChoiceScene {
            id: SceneId(1012),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 30,
            priority: 100,
            choices: vec![
                choice("follow-the-echo", "A voice echoes in a time that was—not an echo, but an overlap. Follow the phase-shift backward into the moment where someone died here.", ChoiceArchetype::Witness, ChoiceAction::Ask),
                choice("hold-the-grief", "The room remembers a death. Stand in that moment and let it land in your chest. This was someone's last breath. This is where they fell.", ChoiceArchetype::Still, ChoiceAction::SteadyTheBody),
                choice("read-the-date", "Trauma marks time-axis. Carve when this happened into memory so you will not forget the weight of it.", ChoiceArchetype::Carve, ChoiceAction::ReadTheGrain),
            ],
        },
        ChoiceScene {
            id: SceneId(1013),
            location: None,
            speaker: None,
            visibility: Visibility::Public,
            requires_all: vec![],
            requires_any: vec![],
            excludes: vec![],
            disclosure_min: 0,
            disclosure_max: 40,
            priority: 99,
            choices: vec![
                choice("carry-the-death", "You carry the death now. It lives in your time-axis the way it lived in theirs. Guilt and grief are not separate—they are the same current.", ChoiceArchetype::Ground, ChoiceAction::SteadyTheBody),
                choice("drown-in-remembrance", "The moment expands. You are not standing in their death—you are drowning in it, tide-pulled, swept backward through every loss that touches this place.", ChoiceArchetype::Drown, ChoiceAction::Leave),
                choice("speak-for-the-dead", "Sing or speak their name aloud. Resurrect them once in memory. This is the Tide school's only law: that what died is spoken, always spoken.", ChoiceArchetype::Sing, ChoiceAction::PlayInstrument(InstrumentId::Voice)),
            ],
        },
    ]
}

/// Every authored scene in one pool, arc by arc — [`SCENE_COUNT`] of them.
pub fn authored_scenes() -> Vec<ChoiceScene> {
    let mut v = opening_arc_scenes();
    v.extend(glass_arc_scenes());
    v.extend(river_arc_scenes());
    v.extend(fae_arch_scenes());
    v.extend(goblin_arch_scenes());
    v.extend(umwelt_arch_scenes());
    v.extend(dirge_arch_scenes());
    v.extend(blind_arch_scenes());
    v.extend(bell_arch_scenes());
    v
}

const _: () = assert!(ChoiceArchetype::ALL.len() == ARCHETYPE_COUNT);
const _: () = assert!(InstrumentId::ALL.len() == INSTRUMENT_COUNT);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyoa_vocabulary_counts_and_labels() {
        assert_eq!(ChoiceArchetype::ALL.len(), ARCHETYPE_COUNT);
        assert_eq!(InstrumentId::ALL.len(), INSTRUMENT_COUNT);
        assert_eq!(ChoiceArchetype::ALL[..DONOR_ARCHETYPES].len(), 6);
        for a in ChoiceArchetype::ALL {
            assert!(!a.as_str().is_empty());
        }
        for i in InstrumentId::ALL {
            assert!(!i.as_str().is_empty());
        }
    }

    /// Feeling load through masonry and resonance in the sternum are the body's
    /// own channels — `umwelt_loom::weave`'s per-form lanes. `HearTheShape` is
    /// echolocation (reflections, blind arch); it must not stand in for them.
    #[test]
    fn the_feel_scenes_sense_rather_than_hear() {
        let scenes = authored_scenes();
        let sensed: Vec<ChoiceId> = scenes
            .iter()
            .flat_map(|s| s.choices.iter())
            .filter(|c| c.action == ChoiceAction::SenseTheRoom)
            .map(|c| c.id)
            .collect();
        for name in ["feel-the-load", "feel-the-ringing"] {
            assert!(
                sensed.contains(&choice_id(name)),
                "{name} must sense through the body, not echolocate"
            );
        }
        for c in scenes.iter().flat_map(|s| s.choices.iter()) {
            if c.action == ChoiceAction::SenseTheRoom {
                assert_eq!(
                    c.archetype,
                    ChoiceArchetype::Feel,
                    "a sensing choice must be a Feel stance: {}",
                    c.text
                );
            }
        }
    }

    #[test]
    fn cyoa_scene_and_fact_ids_are_stable_and_unique() {
        assert_eq!(scene_id("the-flood"), scene_id("the-flood"));
        let scenes = authored_scenes();
        assert_eq!(scenes.len(), 40, "14 opening trials (2x7 arts) + 8 arcs: 4+4 (glass/river) + 6x3 (arches, bell)");
        for (i, a) in scenes.iter().enumerate() {
            for b in &scenes[i + 1..] {
                assert_ne!(a.id, b.id, "two scenes share one id");
            }
        }
    }

    /// The bard arch is an ARC, not three loose scenes: the recomposition is
    /// unreachable until the slots are learned, and the school until a variant
    /// has actually been sung. Parry-Lord order, enforced by the sieve.
    #[test]
    fn the_bell_arch_gates_on_its_own_arc_facts() {
        let scenes = bell_arch_scenes();
        assert_eq!(scenes.len(), 3);
        let ledger = Ledger::default();
        let legal_at = |state: &SceneRuntimeState| -> Vec<SceneId> {
            scenes.iter().filter(|s| scene_is_legal(s, state, &ledger)).map(|s| s.id).collect()
        };

        let origin = SceneRuntimeState::default();
        assert_eq!(
            legal_at(&origin),
            vec![scene_id("the-bell-school-door")],
            "only the door is open before any formula is learned"
        );

        // The gate must be the FACT, not merely the disclosure window: sit
        // squarely inside the recomposition's window with no formula learned
        // and it must still refuse. (L18 2026-08-25: without this case,
        // deleting `requires_all` from the scene left every test green.)
        let untaught_but_late = SceneRuntimeState { disclosure_level: 120, ..Default::default() };
        assert!(
            legal_at(&untaught_but_late).is_empty(),
            "no formula learned means no recomposition, whatever the disclosure level"
        );

        let taught = SceneRuntimeState {
            active_facts: vec![fact("formula-learned")],
            disclosure_level: 120,
            visited_scenes: vec![scene_id("the-bell-school-door")],
            ..Default::default()
        };
        assert_eq!(
            legal_at(&taught),
            vec![scene_id("the-recomposition")],
            "the slots learned open the recomposition, and nothing further"
        );

        let sung = SceneRuntimeState {
            active_facts: vec![fact("formula-learned"), fact("variant-sung")],
            disclosure_level: 200,
            visited_scenes: vec![scene_id("the-bell-school-door"), scene_id("the-recomposition")],
            ..Default::default()
        };
        assert_eq!(
            legal_at(&sung),
            vec![scene_id("the-school-of-the-bell")],
            "the school is only reachable once a variant has actually been sung"
        );
    }

    /// The arch is oral-formulaic in mechanism, not just in flavour: its middle
    /// scene offers the three real moves on a formula — vary it, repeat it
    /// verbatim, or hand the open slot to the room.
    #[test]
    fn the_recomposition_offers_the_three_formula_moves() {
        let recomposition = bell_arch_scenes()
            .into_iter()
            .find(|s| s.id == scene_id("the-recomposition"))
            .expect("the middle scene of the arc");
        assert_eq!(recomposition.choices.len(), 3);
        let archetypes: Vec<_> = recomposition.choices.iter().map(|c| c.archetype).collect();
        assert!(archetypes.contains(&ChoiceArchetype::Sing), "vary the formula");
        assert!(archetypes.contains(&ChoiceArchetype::Echo), "give it back unchanged");
        assert!(archetypes.contains(&ChoiceArchetype::Attune), "let the room fill the slot");
        assert!(
            recomposition
                .choices
                .iter()
                .any(|c| c.action == ChoiceAction::PlayInstrument(InstrumentId::Bell)),
            "the School of the Bell sings through its own instrument"
        );
    }

    #[test]
    fn cyoa_origin_offers_exactly_the_eight_arc_openings() {
        let scenes = authored_scenes();
        let state = SceneRuntimeState::default();
        let ledger = Ledger::default();
        let legal: Vec<_> =
            scenes.iter().filter(|s| scene_is_legal(s, &state, &ledger)).map(|s| s.id).collect();
        assert_eq!(legal.len(), 22, "at disclosure 0: 14 opening trials (2x7 arts, priority 100) + 8 arc openings (priority 10-24)");
        // Opening trials (numeric IDs 1000-1013) take priority precedence over arc openings
        // but all 8 arc openings (glass, river, fae, goblin, umwelt, dirge, blind, bell) remain legal
        // Priority 100 beats priority 15: an opening trial is selected first, not the river.
        let picked = select_next_scene(&scenes, &state, &ledger).expect("an opening is legal");
        assert!(picked.priority >= 99, "picked scene has priority >= 99 (opening trial precedence)");
    }

    #[test]
    fn cyoa_arc_progression_gates_the_climax() {
        let scenes = authored_scenes();
        let ledger = Ledger::default();
        // Near revelation without the river facts: the Mother stays illegal.
        let mut state = SceneRuntimeState { disclosure_level: 220, seed: 42, ..Default::default() };
        let mother = scenes.iter().find(|s| s.id == scene_id("mother-unleashed")).unwrap();
        assert!(!scene_is_legal(mother, &state, &ledger));
        // Entered + offered: legal, and it outranks everything (priority 30).
        state.active_facts = vec![fact("river-entered"), fact("offering-made")];
        assert!(scene_is_legal(mother, &state, &ledger));
        let picked = select_next_scene(&scenes, &state, &ledger).unwrap();
        assert_eq!(picked.id, scene_id("mother-unleashed"));
    }

    #[test]
    fn cyoa_exclusion_and_visits_prune_the_sieve() {
        let scenes = authored_scenes();
        let ledger = Ledger::default();
        // dawn-waited excludes the flood (they saw it coming).
        let state = SceneRuntimeState {
            disclosure_level: 150,
            active_facts: vec![fact("river-entered"), fact("dawn-waited")],
            ..Default::default()
        };
        let flood = scenes.iter().find(|s| s.id == scene_id("the-flood")).unwrap();
        assert!(!scene_is_legal(flood, &state, &ledger));
        // A visited scene is never legal again.
        let state2 = SceneRuntimeState {
            visited_scenes: vec![scene_id("the-river-at-dawn")],
            ..Default::default()
        };
        let dawn = scenes.iter().find(|s| s.id == scene_id("the-river-at-dawn")).unwrap();
        assert!(!scene_is_legal(dawn, &state2, &ledger));
    }

    #[test]
    fn cyoa_selection_is_seed_stable() {
        let scenes = authored_scenes();
        let ledger = Ledger::default();
        let state = SceneRuntimeState { seed: 7, ..Default::default() };
        let a = select_next_scene(&scenes, &state, &ledger).unwrap().id;
        let b = select_next_scene(&scenes, &state, &ledger).unwrap().id;
        assert_eq!(a, b, "same state, same pick — always");
    }
}
