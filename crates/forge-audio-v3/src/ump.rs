//! UMP transport + MoM routing tag. Ported from
//! `F:\NewRepo\crates\forge-core\src\ump.rs` (v2 Crate Zero). ONE adaptation:
//! `crate::scene::MaterialId` (a bare `pub type MaterialId = u16;` in v2's
//! `scene.rs`, a module never ported to v3) inlined directly here rather than
//! porting a whole `scene` module for one type alias.

/// Acoustic-material id — `forge-core::scene`'s canonical type in v2 (a bare
/// `u16` alias there too; inlined here, not reinvented).
pub type MaterialId = u16;

/// The UMP transport-word width in bytes — the MIDI 2.0 128-bit packet.
pub const UMP_WORD_BYTES: usize = 16;

/// Pure MIDI 2.0 transport word — 128 bits.
#[derive(Debug, Clone, Copy)]
pub struct UmpWord(pub [u8; 16]);

/// MoM routing annotation. `colour_id` + `material_id` + `essence_id`
/// = VixelAtom triad (without phase). Every musician IS a vixel.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RoutingTag {
    pub weight: f32,
    pub channel: u8,
    pub colour_id: u32,
    pub material_id: MaterialId,
    pub essence_id: u8,
}

impl RoutingTag {
    pub fn new(weight: f32, channel: u8, colour_id: u32, material_id: MaterialId, essence_id: u8) -> Self {
        Self { weight, channel, colour_id, material_id, essence_id: essence_id & 0x3F }
    }
}

/// A routed UMP event — transport word + its MoM harness tag.
#[derive(Debug, Clone, Copy)]
pub struct RoutedUmp {
    pub word: UmpWord,
    pub tag: RoutingTag,
}

pub const FAMILY_CST_VOICE: u8 = 0x35;
pub const FAMILY_PHYSICS: u8 = 0xCA;
pub const FAMILY_ROADIE: u8 = 0x56;
pub const FAMILY_PTY_ROUTE: u8 = 0xA9;

const SPREAD: [u8; 8] = [0x0F, 0xF0, 0x33, 0xCC, 0x55, 0xAA, 0x69, 0x96];

impl UmpWord {
    pub fn from_node_voice(material_id: u8, note: u8, voice: u8) -> Self {
        let mat_code = SPREAD[(material_id.min(10) as usize * 7) / 10];
        let note_code = SPREAD[(note.saturating_sub(36) as usize / 8).min(7)];
        let voice_code = SPREAD[(voice & 0x07) as usize];
        let pitch_class = note % 12;
        let mut w = [FAMILY_CST_VOICE; 16];
        w[8..16].copy_from_slice(&[
            mat_code, note_code, voice_code, pitch_class,
            mat_code, note_code, voice_code, pitch_class,
        ]);
        Self(w)
    }

    pub fn from_physics_event(kind: u8, material_hash: u64, impulse_q: i32, resonance_hz: i32) -> Self {
        let mat = (material_hash ^ (material_hash >> 16) ^ (material_hash >> 32) ^ (material_hash >> 48)) as u16;
        let [mat_hi, mat_lo] = mat.to_be_bytes();
        let kind_code = SPREAD[(kind & 0x07) as usize];
        let imp_code = SPREAD[impulse_bucket(impulse_q) as usize];
        let res_code = SPREAD[resonance_bucket(resonance_hz) as usize];
        let mut w = [FAMILY_PHYSICS; 16];
        w[8..16].copy_from_slice(&[
            kind_code, mat_hi, mat_lo, imp_code, res_code,
            kind_code ^ mat_lo, imp_code ^ res_code, kind_code ^ mat_hi,
        ]);
        Self(w)
    }

    pub fn from_roadie_event(severity: u8, diagnosis: u8) -> Self {
        let sev_code = SPREAD[(severity & 0x07) as usize];
        let diag_code = SPREAD[(diagnosis & 0x07) as usize];
        let mix = sev_code ^ diag_code;
        let mut w = [FAMILY_ROADIE; 16];
        w[8..16].copy_from_slice(&[sev_code, diag_code, mix, sev_code, diag_code, mix, sev_code, diag_code]);
        Self(w)
    }

    pub fn from_pty_route(sid: u8, margin: u32) -> Self {
        let sid_code = SPREAD[(sid & 0x07) as usize];
        let margin_code = SPREAD[margin_bucket(margin) as usize];
        let mix = sid_code ^ margin_code;
        let sid_raw = sid & 0x07;
        let mut w = [FAMILY_PTY_ROUTE; 16];
        w[8..16].copy_from_slice(&[sid_code, margin_code, mix, sid_raw, sid_code, margin_code, mix, sid_raw]);
        Self(w)
    }
}

fn impulse_bucket(impulse_q: i32) -> u8 {
    let q = impulse_q.unsigned_abs().min(10_000);
    (q * 7 / 10_000) as u8
}

fn resonance_bucket(resonance_hz: i32) -> u8 {
    match resonance_hz.max(0) as u32 {
        0..=60 => 0,
        61..=150 => 1,
        151..=400 => 2,
        401..=1_000 => 3,
        1_001..=2_500 => 4,
        2_501..=6_000 => 5,
        6_001..=12_000 => 6,
        _ => 7,
    }
}

fn margin_bucket(margin: u32) -> u8 {
    match margin {
        0..=9 => 0,
        10..=24 => 1,
        25..=49 => 2,
        50..=99 => 3,
        100..=249 => 4,
        250..=599 => 5,
        600..=1_499 => 6,
        _ => 7,
    }
}

pub mod packet {
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct Ump {
        pub words: [u32; 4],
    }

    impl Ump {
        #[inline]
        pub const fn new(words: [u32; 4]) -> Self {
            Self { words }
        }
        #[inline]
        pub const fn mt(self) -> u8 {
            ((self.words[0] >> 28) & 0x0f) as u8
        }
        #[inline]
        pub const fn group(self) -> Group {
            Group(((self.words[0] >> 24) & 0x0f) as u8)
        }
        #[inline]
        pub const fn status(self) -> u8 {
            ((self.words[0] >> 20) & 0x0f) as u8
        }
    }

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Group(pub u8);

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Channel(pub u8);

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Stamped<T> {
        pub universal_tick_us: i64,
        pub payload: T,
    }
}

pub mod event {
    pub type Group = u8;
    pub type Channel = u8;
    pub type Note = u8;

    pub mod cc {
        pub const CAMERA: u8 = 70;
        pub const LIGHT: u8 = 71;
        pub const BLOOM: u8 = 72;
        pub const ANIM: u8 = 73;
        pub const CUTOFF: u8 = 74;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum UmpEvent {
        NoteOn { group: Group, channel: Channel, note: Note, velocity: u16, attr_type: u8, attr: u16 },
        NoteOff { group: Group, channel: Channel, note: Note, velocity: u16, attr_type: u8, attr: u16 },
        ControlChange { group: Group, channel: Channel, index: u8, value: u32 },
        PerNoteController { group: Group, channel: Channel, note: Note, index: u8, value: u32 },
    }

    const MT_CHANNEL_VOICE_2: u32 = 0x4;
    const ST_NOTE_OFF: u32 = 0x8;
    const ST_NOTE_ON: u32 = 0x9;
    const ST_CC: u32 = 0xB;
    const ST_PNC: u32 = 0x0;

    impl UmpEvent {
        pub fn to_ump(&self) -> [u32; 2] {
            match *self {
                UmpEvent::NoteOn { group, channel, note, velocity, attr_type, attr } =>
                    note_words(ST_NOTE_ON, group, channel, note, velocity, attr_type, attr),
                UmpEvent::NoteOff { group, channel, note, velocity, attr_type, attr } =>
                    note_words(ST_NOTE_OFF, group, channel, note, velocity, attr_type, attr),
                UmpEvent::ControlChange { group, channel, index, value } => {
                    let w0 = (MT_CHANNEL_VOICE_2 << 28) | ((group as u32 & 0xF) << 24) | (ST_CC << 20)
                        | ((channel as u32 & 0xF) << 16) | ((index as u32 & 0x7F) << 8);
                    [w0, value]
                }
                UmpEvent::PerNoteController { group, channel, note, index, value } => {
                    let w0 = (MT_CHANNEL_VOICE_2 << 28) | ((group as u32 & 0xF) << 24) | (ST_PNC << 20)
                        | ((channel as u32 & 0xF) << 16) | ((note as u32 & 0x7F) << 8) | (index as u32 & 0xFF);
                    [w0, value]
                }
            }
        }
    }

    #[inline]
    fn note_words(status: u32, group: Group, channel: Channel, note: Note, velocity: u16, attr_type: u8, attr: u16) -> [u32; 2] {
        let w0 = (MT_CHANNEL_VOICE_2 << 28) | ((group as u32 & 0xF) << 24) | (status << 20)
            | ((channel as u32 & 0xF) << 16) | ((note as u32 & 0x7F) << 8) | (attr_type as u32 & 0xFF);
        let w1 = ((velocity as u32) << 16) | (attr as u32);
        [w0, w1]
    }

    #[inline]
    pub fn permille_to_cc(v: i32) -> u32 {
        let v = v.clamp(0, 1000) as u64;
        ((v * (u32::MAX as u64)) / 1000) as u32
    }

    #[inline]
    pub fn automation_cc(group: Group, channel: Channel, lane: u8, permille: i32) -> UmpEvent {
        UmpEvent::ControlChange { group, channel, index: lane, value: permille_to_cc(permille) }
    }

    #[inline]
    pub fn note_on(group: Group, channel: Channel, note: Note, velocity: u16) -> UmpEvent {
        UmpEvent::NoteOn { group, channel, note, velocity, attr_type: 0, attr: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ump_word_is_query_width() {
        assert_eq!(std::mem::size_of::<UmpWord>(), UMP_WORD_BYTES);
        assert_eq!(UMP_WORD_BYTES, 16);
    }

    #[test]
    fn essence_id_masked_to_six_bits() {
        let tag = RoutingTag::new(0.5, 0, 0, 0, 0xFF);
        assert_eq!(tag.essence_id, 0x3F, "essence must clamp to 6-bit");
    }

    fn hamming(a: &UmpWord, b: &UmpWord) -> u32 {
        a.0.iter().zip(b.0.iter()).map(|(x, y)| (x ^ y).count_ones()).sum()
    }

    #[test]
    fn families_are_hamming_separated() {
        let nv = UmpWord::from_node_voice(2, 60, 1);
        let ph = UmpWord::from_physics_event(2, 0xDEAD_BEEF_CAFE_F00D, 5_000, 440);
        let rd = UmpWord::from_roadie_event(2, 1);
        assert!(hamming(&nv, &ph) >= 32);
        assert!(hamming(&nv, &rd) >= 32);
        assert!(hamming(&ph, &rd) >= 32);
    }

    #[test]
    fn quantizers_span_their_ranges() {
        assert_eq!(impulse_bucket(0), 0);
        assert_eq!(impulse_bucket(10_000), 7);
        assert_eq!(resonance_bucket(0), 0);
        assert_eq!(resonance_bucket(20_000), 7);
    }
}
