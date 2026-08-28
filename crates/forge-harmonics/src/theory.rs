//! Music as DATA: the scale registry, the knob catalog, and the tuning.
//! Ported 2026-08-27 from F:\NewRepo\crates\technothesia\src\theory.rs:151-346
//! (data layer only; the wgpu `run_theory` surface and all float stayed behind).

use crate::cents_floor::Cents;
use crate::euclid::EuclidBresenham;
use crate::scale_mask::ScaleMask;
use crate::scale_voice::{note_to_mhz, VoicePreset};

/// The twelve pitch-class names, index = pitch class.
pub const NOTE_NAMES: [&str; 12] =
    ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// One scale as data: semitone offsets from the root, NOT absolute MIDI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScaleDef {
    /// Display name; `scale_by_name` matches on it.
    pub name: &'static str,
    /// One-line plain-language description for a picker.
    pub blurb: &'static str,
    /// Ascending semitone offsets from the root, all under 12.
    pub degrees: &'static [u8],
    /// True when the third is minor; drives the Camelot letter.
    pub is_minor: bool,
}

/// The registry `scale_voice::CMD_SCALE_ROTA` indexes. Order is load-bearing:
/// 2=Dorian, 8=Minor Pentatonic, 9=Major Pentatonic, 10=Blues.
pub const SCALES: &[ScaleDef] = &[
    ScaleDef { name: "Major",            blurb: "Bright and happy — the default 'do-re-mi'.",                   degrees: &[0, 2, 4, 5, 7, 9, 11],                 is_minor: false },
    ScaleDef { name: "Natural Minor",    blurb: "Dark and sad — the classic 'serious' sound.",                  degrees: &[0, 2, 3, 5, 7, 8, 10],                 is_minor: true  },
    ScaleDef { name: "Dorian",           blurb: "Minor but hopeful — funky, folky, never quite gloomy.",        degrees: &[0, 2, 3, 5, 7, 9, 10],                 is_minor: true  },
    ScaleDef { name: "Phrygian",         blurb: "Spanish / metal flavour — tense and exotic from note two.",    degrees: &[0, 1, 3, 5, 7, 8, 10],                 is_minor: true  },
    ScaleDef { name: "Lydian",           blurb: "Dreamy and floating — major with a magic 'lift'.",             degrees: &[0, 2, 4, 6, 7, 9, 11],                 is_minor: false },
    ScaleDef { name: "Mixolydian",       blurb: "Bluesy major — rock and groove without the sadness.",          degrees: &[0, 2, 4, 5, 7, 9, 10],                 is_minor: false },
    ScaleDef { name: "Locrian",          blurb: "Unstable and uneasy — rarely a home, great for dread.",        degrees: &[0, 1, 3, 5, 6, 8, 10],                 is_minor: true  },
    ScaleDef { name: "Harmonic Minor",   blurb: "Minor with a dramatic, 'Arabian' leading tone.",               degrees: &[0, 2, 3, 5, 7, 8, 11],                 is_minor: true  },
    ScaleDef { name: "Minor Pentatonic", blurb: "Five notes, can't play a wrong one — the jam-session scale.",  degrees: &[0, 3, 5, 7, 10],                       is_minor: true  },
    ScaleDef { name: "Major Pentatonic", blurb: "Five bright notes — folk songs and whistling tunes.",          degrees: &[0, 2, 4, 7, 9],                        is_minor: false },
    ScaleDef { name: "Blues",            blurb: "Pentatonic plus the gritty 'blue note' — soul and grit.",      degrees: &[0, 3, 5, 6, 7, 10],                    is_minor: true  },
    ScaleDef { name: "Chromatic",        blurb: "All twelve notes — total freedom, total responsibility.",      degrees: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], is_minor: false },
];

/// Registry index of Major Pentatonic — the set `PENTATONIC_C` spells.
pub const MAJOR_PENTATONIC: usize = 9;
/// Registry index of Harmonic Minor — carries the Art of Fugue subject's C#.
pub const HARMONIC_MINOR: usize = 7;
/// Registry index of Chromatic — all twelve, the scale lock released.
pub const CHROMATIC: usize = 11;

/// Look a scale up by its registry name.
pub fn scale_by_name(name: &str) -> Option<&'static ScaleDef> {
    SCALES.iter().find(|s| s.name == name)
}

/// Fold degrees rooted at `root_pc` into a 12-bit pitch-class mask.
pub fn mask_from_degrees(root_pc: u8, degrees: &[u8]) -> ScaleMask {
    let mut bits = 0u16;
    let mut i = 0;
    while i < degrees.len() {
        bits |= 1 << ((root_pc + degrees[i]) % 12);
        i += 1;
    }
    ScaleMask(bits)
}

/// Reference pitch as a value, not a frozen const (Sean 2026-08-27:
/// 432 for the sky, 440 for scores).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// Frequency of A4 in millihertz.
    pub ref_a_mhz: u32,
}

/// A440 — what an imported score plays at, so Bach sounds like Bach.
pub const CONCERT: Tuning = Tuning { ref_a_mhz: 440_000 };
/// A432 — what the star field rings at.
pub const ALCHEMICAL: Tuning = Tuning { ref_a_mhz: 432_000 };

impl Tuning {
    /// `note_to_mhz` is the exact A440 table; any other anchor is one integer
    /// ratio off it, so CONCERT stays bit-exact with the landed fast path.
    pub fn note_mhz(self, midi: u8) -> u64 {
        let base = note_to_mhz(midi) as u64;
        if self.ref_a_mhz == 440_000 {
            base
        } else {
            base * self.ref_a_mhz as u64 / 440_000
        }
    }
}

/// The emotional target a generated bed aims at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    /// Settled, low drive.
    Calm = 0,
    /// Lifted, major-leaning.
    Bright = 1,
    /// Unsettled, pushing.
    Tense = 2,
    /// Heavy, minor-leaning.
    Sad = 3,
}

/// How a knob is presented and stepped.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KnobKind {
    /// Continuous within min..max.
    Slider,
    /// Discrete increments of `step`.
    Stepper,
    /// One of an enumerated `choices` list.
    Choice,
}

/// One enumerated option on a `KnobKind::Choice` knob.
#[derive(Clone, Copy)]
pub struct KnobChoice {
    /// The value stored when this option is selected.
    pub value: i32,
    /// Display label.
    pub label: &'static str,
}

/// One authored control on the theory surface.
#[derive(Clone, Copy)]
pub struct Knob {
    /// Which knob this is; indexes `CATALOG`.
    pub id: KnobId,
    /// Display name.
    pub label: &'static str,
    /// One-line plain-language description.
    pub blurb: &'static str,
    /// Unit suffix, empty for unitless.
    pub unit: &'static str,
    /// Presentation and stepping mode.
    pub kind: KnobKind,
    /// Inclusive lower bound.
    pub min: i32,
    /// Inclusive upper bound.
    pub max: i32,
    /// Increment for stepper and slider.
    pub step: i32,
    /// Value a fresh `TheoryState` starts at.
    pub default: i32,
    /// Enumerated options, empty unless `kind` is `Choice`.
    pub choices: &'static [KnobChoice],
}

/// Stable index into `CATALOG`; the discriminant IS the slot.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum KnobId {
    /// Home pitch class, 0..11.
    Root = 0,
    /// Index into `SCALES`.
    Scale = 1,
    /// Octave offset from 4.
    Octave = 2,
    /// Beats per minute.
    Tempo = 3,
    /// Swing percentage.
    Swing = 4,
    /// Cents bend between keys.
    Microtune = 5,
    /// Euclidean pulses per bar.
    EuclidPulses = 6,
    /// Euclidean steps per bar.
    EuclidSteps = 7,
    /// Emotional target.
    Mood = 8,
    /// Drive percentage.
    Energy = 9,
    /// Stacked instrument beds.
    Layers = 10,
    /// Instrument body preset.
    Voice = 11,
}

impl KnobId {
    /// Number of knobs, and the length of `CATALOG`.
    pub const COUNT: usize = 12;
    /// Every knob in slot order.
    pub const ALL: [KnobId; Self::COUNT] = [
        KnobId::Root, KnobId::Scale, KnobId::Octave, KnobId::Tempo,
        KnobId::Swing, KnobId::Microtune, KnobId::EuclidPulses, KnobId::EuclidSteps,
        KnobId::Mood, KnobId::Energy, KnobId::Layers, KnobId::Voice,
    ];
    /// This knob's authored definition.
    pub fn knob(self) -> &'static Knob {
        &CATALOG[self as usize]
    }
}

const NOTE_CHOICES: [KnobChoice; 12] = [
    KnobChoice { value: 0,  label: "C"  }, KnobChoice { value: 1,  label: "C#" },
    KnobChoice { value: 2,  label: "D"  }, KnobChoice { value: 3,  label: "D#" },
    KnobChoice { value: 4,  label: "E"  }, KnobChoice { value: 5,  label: "F"  },
    KnobChoice { value: 6,  label: "F#" }, KnobChoice { value: 7,  label: "G"  },
    KnobChoice { value: 8,  label: "G#" }, KnobChoice { value: 9,  label: "A"  },
    KnobChoice { value: 10, label: "A#" }, KnobChoice { value: 11, label: "B"  },
];
const SCALE_CHOICES: [KnobChoice; 12] = [
    KnobChoice { value: 0,  label: "Major"            }, KnobChoice { value: 1,  label: "Natural Minor"    },
    KnobChoice { value: 2,  label: "Dorian"           }, KnobChoice { value: 3,  label: "Phrygian"         },
    KnobChoice { value: 4,  label: "Lydian"           }, KnobChoice { value: 5,  label: "Mixolydian"       },
    KnobChoice { value: 6,  label: "Locrian"          }, KnobChoice { value: 7,  label: "Harmonic Minor"   },
    KnobChoice { value: 8,  label: "Minor Pentatonic" }, KnobChoice { value: 9,  label: "Major Pentatonic" },
    KnobChoice { value: 10, label: "Blues"            }, KnobChoice { value: 11, label: "Chromatic"        },
];
const MOOD_CHOICES: [KnobChoice; 4] = [
    KnobChoice { value: 0, label: "Calm" }, KnobChoice { value: 1, label: "Bright" },
    KnobChoice { value: 2, label: "Tense" }, KnobChoice { value: 3, label: "Sad" },
];
const VOICE_CHOICES: [KnobChoice; 3] = [
    KnobChoice { value: 0, label: "Glass (bell)" },
    KnobChoice { value: 1, label: "Reed (breath)" },
    KnobChoice { value: 2, label: "Hearth (warm)" },
];
const NONE: &[KnobChoice] = &[];

/// Every knob, authored once, indexed by `KnobId`.
pub const CATALOG: [Knob; KnobId::COUNT] = [
    Knob { id: KnobId::Root,         label: "Root Note", unit: "",      blurb: "The 'home' pitch everything leans toward.",           kind: KnobKind::Choice,  min: 0,    max: 11,  step: 1, default: 0,   choices: &NOTE_CHOICES  },
    Knob { id: KnobId::Scale,        label: "Scale",     unit: "",      blurb: "The set of notes allowed.",                           kind: KnobKind::Choice,  min: 0,    max: 11,  step: 1, default: 0,   choices: &SCALE_CHOICES },
    Knob { id: KnobId::Octave,       label: "Octave",    unit: "oct",   blurb: "Same notes, higher or lower.",                        kind: KnobKind::Stepper, min: -2,   max: 2,   step: 1, default: 0,   choices: NONE           },
    Knob { id: KnobId::Tempo,        label: "Tempo",     unit: "BPM",   blurb: "Beats per minute.",                                   kind: KnobKind::Slider,  min: 40,   max: 220, step: 5, default: 120, choices: NONE           },
    Knob { id: KnobId::Swing,        label: "Swing",     unit: "%",     blurb: "0 = straight and robotic; raise it to get loose.",    kind: KnobKind::Slider,  min: 0,    max: 100, step: 5, default: 0,   choices: NONE           },
    Knob { id: KnobId::Microtune,    label: "Microtune", unit: "cents", blurb: "Bend pitch BETWEEN piano keys. 100 cents = one key.", kind: KnobKind::Slider,  min: -100, max: 100, step: 5, default: 0,   choices: NONE           },
    Knob { id: KnobId::EuclidPulses, label: "Pulses",    unit: "hits",  blurb: "How many hits land in the bar, spread evenly.",       kind: KnobKind::Stepper, min: 0,    max: 16,  step: 1, default: 4,   choices: NONE           },
    Knob { id: KnobId::EuclidSteps,  label: "Steps",     unit: "slots", blurb: "Slots the pulses spread across.",                     kind: KnobKind::Stepper, min: 1,    max: 32,  step: 1, default: 16,  choices: NONE           },
    Knob { id: KnobId::Mood,         label: "Mood",      unit: "",      blurb: "The emotional target.",                               kind: KnobKind::Choice,  min: 0,    max: 3,   step: 1, default: 0,   choices: &MOOD_CHOICES  },
    Knob { id: KnobId::Energy,       label: "Energy",    unit: "%",     blurb: "How hard the track drives.",                          kind: KnobKind::Slider,  min: 0,    max: 100, step: 5, default: 50,  choices: NONE           },
    Knob { id: KnobId::Layers,       label: "Layers",    unit: "",      blurb: "How many instrument beds stack at once.",             kind: KnobKind::Stepper, min: 1,    max: 4,   step: 1, default: 2,   choices: NONE           },
    Knob { id: KnobId::Voice,        label: "Voice",     unit: "",      blurb: "The instrument body.",                                kind: KnobKind::Choice,  min: 0,    max: 2,   step: 1, default: 2,   choices: &VOICE_CHOICES },
];

/// The knob values plus the tuning they resolve against.
#[derive(Clone, Copy)]
pub struct TheoryState {
    vals: [i32; KnobId::COUNT],
    tuning: Tuning,
}

impl Default for TheoryState {
    fn default() -> Self {
        let mut vals = [0i32; KnobId::COUNT];
        let mut i = 0;
        while i < KnobId::COUNT {
            vals[i] = CATALOG[i].default;
            i += 1;
        }
        Self { vals, tuning: CONCERT }
    }
}

impl TheoryState {
    /// Defaults from `CATALOG`, tuned to `CONCERT`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Defaults from `CATALOG`, tuned to `tuning`.
    pub fn with_tuning(tuning: Tuning) -> Self {
        Self { tuning, ..Self::default() }
    }

    /// Current value of one knob.
    #[inline]
    pub fn get(&self, id: KnobId) -> i32 {
        self.vals[id as usize]
    }

    /// Set one knob, clamped to its own authored bounds.
    #[inline]
    pub fn set(&mut self, id: KnobId, v: i32) {
        let k = id.knob();
        self.vals[id as usize] = v.clamp(k.min, k.max);
    }

    /// Restore every knob to its default, keeping the tuning.
    pub fn reset(&mut self) {
        let tuning = self.tuning;
        *self = Self { tuning, ..Self::default() };
    }

    /// The reference pitch this state resolves against.
    #[inline]
    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    /// Change the reference pitch.
    #[inline]
    pub fn set_tuning(&mut self, tuning: Tuning) {
        self.tuning = tuning;
    }

    /// Home pitch class, 0..11.
    pub fn root_pc(&self) -> u8 {
        self.get(KnobId::Root) as u8 % 12
    }

    /// The selected registry entry.
    pub fn scale(&self) -> ScaleDef {
        SCALES[(self.get(KnobId::Scale) as usize).min(SCALES.len() - 1)]
    }

    /// The selected scale as a pitch-class mask at the current root.
    pub fn scale_mask(&self) -> ScaleMask {
        mask_from_degrees(self.root_pc(), self.scale().degrees)
    }

    /// The root as an absolute MIDI note at the current octave.
    pub fn root_midi(&self) -> u8 {
        let o = 4 + self.get(KnobId::Octave);
        (12 * (o + 1) + self.root_pc() as i32).clamp(0, 127) as u8
    }

    /// The scale's notes as absolute MIDI, and how many are in range.
    pub fn scale_notes(&self) -> ([u8; 12], usize) {
        let mut out = [0u8; 12];
        let degrees = self.scale().degrees;
        let base = self.root_midi() as i32;
        let mut n = 0;
        let mut i = 0;
        while i < degrees.len() && n < 12 {
            let note = base + degrees[i] as i32;
            if (0..=127).contains(&note) {
                out[n] = note as u8;
                n += 1;
            }
            i += 1;
        }
        (out, n)
    }

    /// One MIDI note in millihertz, through this tuning and any microtune.
    pub fn note_freq_mhz(&self, midi: u8) -> u64 {
        let base = self.tuning.note_mhz(midi);
        let cents = self.get(KnobId::Microtune);
        if cents == 0 {
            base
        } else {
            Cents(cents).to_millihertz(base)
        }
    }

    /// The Euclidean pulse pattern, and how many steps are live.
    pub fn euclid_pattern(&self) -> ([bool; 32], usize) {
        let k = self.get(KnobId::EuclidPulses).max(0) as u32;
        let n = (self.get(KnobId::EuclidSteps).max(1) as u32).min(32);
        let mut e = EuclidBresenham::new(k, n);
        let mut out = [false; 32];
        let mut i = 0;
        while i < n as usize {
            out[i] = e.next_step();
            i += 1;
        }
        (out, n as usize)
    }

    /// Milliseconds per beat at the current tempo.
    #[inline]
    pub fn ms_per_beat(&self) -> u32 {
        60_000 / self.get(KnobId::Tempo).max(1) as u32
    }

    /// The selected emotional target.
    pub fn mood(&self) -> Mood {
        match self.get(KnobId::Mood) {
            0 => Mood::Calm,
            1 => Mood::Bright,
            2 => Mood::Tense,
            _ => Mood::Sad,
        }
    }

    /// The selected instrument body.
    pub fn voice(&self) -> VoicePreset {
        VoicePreset::from_u8(self.get(KnobId::Voice) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dnb::MINOR_SCALE;
    use crate::scale_voice::PENTATONIC_C;

    #[test]
    fn scales_registry_is_data() {
        assert_eq!(SCALES.len(), 12, "the registry is the v2 twelve");
        for s in SCALES {
            assert!(!s.degrees.is_empty(), "{} has no degrees", s.name);
            assert_eq!(s.degrees[0], 0, "{} must start on the root", s.name);
            for w in s.degrees.windows(2) {
                assert!(w[1] > w[0], "{} degrees must ascend", s.name);
            }
            assert!(*s.degrees.last().unwrap() < 12, "{} must stay inside one octave", s.name);
        }
        assert_eq!(SCALES[2].name, "Dorian");
        assert_eq!(SCALES[8].name, "Minor Pentatonic");
        assert_eq!(SCALES[MAJOR_PENTATONIC].name, "Major Pentatonic");
        assert_eq!(SCALES[10].name, "Blues");
        assert_eq!(SCALES[CHROMATIC].degrees.len(), 12);
    }

    /// PENTATONIC_C is not deleted and not duplicated: it is the SAME set as
    /// registry slot 9, expressed as absolute C4 notes instead of degrees.
    #[test]
    fn pentatonic_c_is_the_registrys_major_pentatonic() {
        let mut from_const: Vec<u8> = PENTATONIC_C.iter().map(|n| n % 12).collect();
        from_const.sort_unstable();
        from_const.dedup();
        let mut from_registry: Vec<u8> = SCALES[MAJOR_PENTATONIC].degrees.to_vec();
        from_registry.sort_unstable();
        assert_eq!(from_const, from_registry, "PENTATONIC_C drifted from the registry");
    }

    /// MINOR_SCALE is already degree-form, so it must match slot 1 exactly.
    #[test]
    fn minor_scale_is_the_registrys_natural_minor() {
        assert_eq!(MINOR_SCALE.as_slice(), SCALES[1].degrees);
    }

    /// D harmonic minor spells the seven degrees the Camelot letter reads off.
    #[test]
    fn harmonic_minor_spells_its_seven_degrees() {
        let mask = mask_from_degrees(2, SCALES[HARMONIC_MINOR].degrees);
        assert_eq!(mask.0.count_ones(), 7, "seven degrees, seven bits");
        for pc in [2u8, 4, 5, 7, 9, 10, 1] {
            assert!(mask.0 & (1 << pc) != 0, "D harmonic minor is missing pitch class {pc}");
        }
    }

    /// The claim on `HARMONIC_MINOR`, checked against the score instead of a
    /// typed list: every pitch Bach writes in bars 1-5 of Contrapunctus I is a
    /// degree of D harmonic minor, and the C# that forces the choice is there.
    #[cfg(feature = "musicxml")]
    #[test]
    fn harmonic_minor_carries_the_art_of_fugue_subject() {
        let xml = include_str!("../fixtures/contrapunctus_i_exposition.musicxml");
        let score = crate::musicxml_extract::musicxml_to_score(xml.as_bytes()).expect("Bach parses");
        let mask = mask_from_degrees(2, SCALES[HARMONIC_MINOR].degrees);

        let mut sounded = 0u16;
        for pitch in score.events.iter().filter_map(|e| e.pitch) {
            let pc = pitch % 12;
            assert!(
                mask.0 & (1 << pc) != 0,
                "MIDI {pitch} (pc {pc}) is outside D harmonic minor",
            );
            sounded |= 1 << pc;
        }
        assert_eq!(sounded, 0b0000_0010_1011_0110, "D E F G A C# — B-flat never sounds");
        assert!(sounded & (1 << 1) != 0, "C# — the subject's accidental — must sound");
    }

    #[test]
    fn concert_is_bit_exact_with_the_landed_table() {
        for midi in [0u8, 21, 60, 69, 81, 127] {
            assert_eq!(CONCERT.note_mhz(midi), note_to_mhz(midi) as u64);
        }
        assert_eq!(CONCERT.note_mhz(69), 440_000);
    }

    #[test]
    fn alchemical_is_432_and_stays_an_octave_lattice() {
        assert_eq!(ALCHEMICAL.note_mhz(69), 432_000);
        assert_eq!(ALCHEMICAL.note_mhz(81), 864_000);
        assert_eq!(ALCHEMICAL.note_mhz(57), 216_000);
    }

    #[test]
    fn microtune_bends_between_the_keys() {
        let mut st = TheoryState::new();
        let plain = st.note_freq_mhz(69);
        st.set(KnobId::Microtune, 50);
        let bent = st.note_freq_mhz(69);
        assert!(bent > plain, "a positive microtune must raise the pitch");
        assert!(bent < st.tuning().note_mhz(70), "half a semitone must stay under the next key");
    }

    #[test]
    fn knob_catalog_is_indexed_by_its_own_id() {
        for (i, id) in KnobId::ALL.iter().enumerate() {
            assert_eq!(CATALOG[i].id as usize, *id as usize);
            assert!(CATALOG[i].default >= CATALOG[i].min);
            assert!(CATALOG[i].default <= CATALOG[i].max);
        }
    }

    #[test]
    fn set_clamps_to_the_knobs_own_bounds() {
        let mut st = TheoryState::new();
        st.set(KnobId::Tempo, 9_999);
        assert_eq!(st.get(KnobId::Tempo), 220);
        st.set(KnobId::Tempo, -5);
        assert_eq!(st.get(KnobId::Tempo), 40);
    }
}
