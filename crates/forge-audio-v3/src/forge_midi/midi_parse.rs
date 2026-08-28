//! Standard MIDI File (SMF) parser.
//!
//! Parses format-0 and format-1 SMF bytes into a time-sorted
//! `Vec<(u32, MidiEvent)>` where the tick is the file's PPQN unit.
//! Conversion to MetronomeClock ticks is `midi_seq`'s job (slice 2).
//!
//! Zero external dependencies. No unsafe.

/// A channel-message or meta event from an SMF track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiEvent {
    /// MIDI channel 0-15. For `MidiEventKind::Tempo` (meta), this is 0.
    pub channel: u8,
    pub kind: MidiEventKind,
}

/// The payload of a `MidiEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiEventKind {
    NoteOn  { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    Control { cc: u8, value: u8 },
    Program { program: u8 },
    /// Pitch bend: -8192 (full down) .. 8191 (full up).
    PitchBend { value: i16 },
    /// SMF meta 0x51: microseconds per quarter note (e.g. 500000 = 120 BPM).
    Tempo { us_per_beat: u32 },
}

/// Errors from `parse_midi`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Buffer shorter than a valid header.
    TooShort,
    /// Wrong chunk magic, unsupported format, or SMPTE timecode division.
    BadMagic,
    /// A variable-length quantity spans more than 4 bytes.
    VlqOverflow,
    /// A chunk claims more data than the buffer contains.
    UnexpectedEof,
}

/// A parsed Standard MIDI File: the PPQN `division` plus all events.
///
/// `division` (ticks per quarter-note) is required to convert ticks → seconds,
/// which the recorded path gets for free from the sample count. Kept here so the
/// ForgeAudio ingest seam can size a composure from a `.mid` file's length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Smf {
    /// Ticks per quarter-note (PPQN). Always the metrical form — SMPTE is rejected.
    pub division: u16,
    /// All events, sorted by absolute tick.
    pub events: Vec<(u32, MidiEvent)>,
}

impl Smf {
    /// Wall-clock length in seconds.
    ///
    /// Uses the first `Tempo` meta (or 120 BPM if none). Mid-file tempo changes
    /// are approximated by the opening tempo — exact per-segment integration is a
    /// follow-up for when the composure needs precise interior timing.
    pub fn duration_secs(&self) -> f64 {
        let last_tick = self.events.last().map(|(t, _)| *t).unwrap_or(0);
        let ppq = self.division.max(1) as f64;
        let us_per_beat = self
            .events
            .iter()
            .find_map(|(_, e)| match e.kind {
                MidiEventKind::Tempo { us_per_beat } => Some(us_per_beat),
                _ => None,
            })
            .unwrap_or(500_000) as f64;
        (last_tick as f64 / ppq) * (us_per_beat / 1_000_000.0)
    }
}

/// Parse a Standard MIDI File (format 0 or 1) from raw bytes, keeping the header
/// `division` (see [`Smf`]).
///
/// Returns all note, control, program, pitch-bend, and tempo events sorted by
/// absolute MIDI tick (PPQN unit from the file header). Format-2
/// (pattern-sequence) and SMPTE timecode are rejected with `BadMagic`.
pub fn parse_smf(data: &[u8]) -> Result<Smf, ParseError> {
    let mut pos = 0usize;

    // ── MThd header ─────────────────────────────────────────────────────────
    read_tag(data, &mut pos, b"MThd")?;
    let hdr_len  = read_u32(data, &mut pos)?;
    if hdr_len < 6 { return Err(ParseError::TooShort); }
    let format   = read_u16(data, &mut pos)?;
    let ntracks  = read_u16(data, &mut pos)?;
    let division = read_u16(data, &mut pos)?;
    // Skip any non-standard extra header bytes.
    pos += (hdr_len - 6) as usize;

    if format > 1 { return Err(ParseError::BadMagic); }        // format 2 unsupported
    if division & 0x8000 != 0 { return Err(ParseError::BadMagic); }  // SMPTE unsupported

    // ── tracks ───────────────────────────────────────────────────────────────
    let mut all: Vec<(u32, MidiEvent)> = Vec::new();

    for _ in 0..ntracks {
        read_tag(data, &mut pos, b"MTrk")?;
        let track_len = read_u32(data, &mut pos)? as usize;
        if pos + track_len > data.len() { return Err(ParseError::UnexpectedEof); }
        let track_slice = &data[pos..pos + track_len];
        pos += track_len;

        parse_track(track_slice, &mut all)?;
    }

    // Format 1 interleaves tracks; stable sort preserves within-tick ordering.
    all.sort_by_key(|(tick, _)| *tick);

    Ok(Smf { division, events: all })
}

/// Parse an SMF, returning only the time-sorted events (PPQN ticks).
///
/// Thin wrapper over [`parse_smf`] for callers that don't need the division
/// (e.g. `midi_seq`, which is handed the PPQN separately).
pub fn parse_midi(data: &[u8]) -> Result<Vec<(u32, MidiEvent)>, ParseError> {
    parse_smf(data).map(|smf| smf.events)
}

// ── internal ─────────────────────────────────────────────────────────────────

fn parse_track(data: &[u8], out: &mut Vec<(u32, MidiEvent)>) -> Result<(), ParseError> {
    let mut pos = 0usize;
    let mut abs_tick: u32 = 0;
    let mut running_status: u8 = 0;

    while pos < data.len() {
        let delta = read_vlq(data, &mut pos)?;
        abs_tick = abs_tick.saturating_add(delta);

        if pos >= data.len() { return Err(ParseError::UnexpectedEof); }
        let b = data[pos];

        // ── meta ─────────────────────────────────────────────────────────────
        if b == 0xFF {
            pos += 1;
            if pos >= data.len() { return Err(ParseError::UnexpectedEof); }
            let meta_type = data[pos]; pos += 1;
            let meta_len  = read_vlq(data, &mut pos)? as usize;
            if pos + meta_len > data.len() { return Err(ParseError::UnexpectedEof); }
            let meta_data = &data[pos..pos + meta_len];
            pos += meta_len;
            running_status = 0;

            if meta_type == 0x51 && meta_len == 3 {
                let us = (meta_data[0] as u32) << 16
                       | (meta_data[1] as u32) << 8
                       |  meta_data[2] as u32;
                out.push((abs_tick, MidiEvent {
                    channel: 0,
                    kind: MidiEventKind::Tempo { us_per_beat: us },
                }));
            }
            // 0x2F = end-of-track: loop exits naturally when pos == data.len().
            continue;
        }

        // ── sysex ─────────────────────────────────────────────────────────────
        if b == 0xF0 || b == 0xF7 {
            pos += 1;
            let sysex_len = read_vlq(data, &mut pos)? as usize;
            if pos + sysex_len > data.len() { return Err(ParseError::UnexpectedEof); }
            pos += sysex_len;
            running_status = 0;
            continue;
        }

        // ── channel message (with running status) ─────────────────────────────
        let status = if b & 0x80 != 0 {
            running_status = b;
            pos += 1;
            b
        } else {
            // Data byte: reuse previous status.
            running_status
        };

        if status == 0 { return Err(ParseError::UnexpectedEof); }

        let msg_type = (status >> 4) & 0x0F;
        let channel  = status & 0x0F;

        match msg_type {
            0x8 => {
                let note = read_byte(data, &mut pos)?;
                let vel  = read_byte(data, &mut pos)?;
                out.push((abs_tick, MidiEvent {
                    channel,
                    kind: MidiEventKind::NoteOff { note, velocity: vel },
                }));
            }
            0x9 => {
                let note = read_byte(data, &mut pos)?;
                let vel  = read_byte(data, &mut pos)?;
                // NoteOn with velocity=0 is a NoteOff by MIDI convention.
                let kind = if vel == 0 {
                    MidiEventKind::NoteOff { note, velocity: 0 }
                } else {
                    MidiEventKind::NoteOn { note, velocity: vel }
                };
                out.push((abs_tick, MidiEvent { channel, kind }));
            }
            0xA => {
                let _ = read_byte(data, &mut pos)?;
                let _ = read_byte(data, &mut pos)?;
            }
            0xB => {
                let cc    = read_byte(data, &mut pos)?;
                let value = read_byte(data, &mut pos)?;
                out.push((abs_tick, MidiEvent {
                    channel,
                    kind: MidiEventKind::Control { cc, value },
                }));
            }
            0xC => {
                let program = read_byte(data, &mut pos)?;
                out.push((abs_tick, MidiEvent {
                    channel,
                    kind: MidiEventKind::Program { program },
                }));
            }
            0xD => {
                let _ = read_byte(data, &mut pos)?;
            }
            0xE => {
                let lsb = read_byte(data, &mut pos)? as u16;
                let msb = read_byte(data, &mut pos)? as u16;
                let raw = ((msb << 7) | lsb) as i32 - 8192;
                out.push((abs_tick, MidiEvent {
                    channel,
                    kind: MidiEventKind::PitchBend { value: raw as i16 },
                }));
            }
            _ => return Err(ParseError::UnexpectedEof),
        }
    }

    Ok(())
}

// ── byte-level helpers ────────────────────────────────────────────────────────

fn read_tag(data: &[u8], pos: &mut usize, tag: &[u8; 4]) -> Result<(), ParseError> {
    if *pos + 4 > data.len() { return Err(ParseError::TooShort); }
    if data[*pos..*pos + 4] != *tag { return Err(ParseError::BadMagic); }
    *pos += 4;
    Ok(())
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, ParseError> {
    if *pos + 2 > data.len() { return Err(ParseError::UnexpectedEof); }
    let v = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, ParseError> {
    if *pos + 4 > data.len() { return Err(ParseError::UnexpectedEof); }
    let v = u32::from_be_bytes([data[*pos], data[*pos+1], data[*pos+2], data[*pos+3]]);
    *pos += 4;
    Ok(v)
}

fn read_byte(data: &[u8], pos: &mut usize) -> Result<u8, ParseError> {
    if *pos >= data.len() { return Err(ParseError::UnexpectedEof); }
    let b = data[*pos];
    *pos += 1;
    Ok(b)
}

/// Read a variable-length quantity (max 4 bytes -> 28-bit value).
fn read_vlq(data: &[u8], pos: &mut usize) -> Result<u32, ParseError> {
    let mut val: u32 = 0;
    for _ in 0..4 {
        if *pos >= data.len() { return Err(ParseError::UnexpectedEof); }
        let b = data[*pos];
        *pos += 1;
        val = (val << 7) | (b & 0x7F) as u32;
        if b & 0x80 == 0 { return Ok(val); }
    }
    Err(ParseError::VlqOverflow)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Format 0, 96 PPQN, 1 track:
    //   tick 0  -- NoteOn  ch0 note=60 vel=100
    //   tick 96 -- NoteOff ch0 note=60 vel=0
    //   tick 96 -- ProgramChange ch0 prog=0
    // Track bytes (15 = 0x0F):
    //   00 90 3C 64   delta=0 NoteOn
    //   60 80 3C 00   delta=96 NoteOff
    //   00 C0 00      delta=0 ProgramChange
    //   00 FF 2F 00   delta=0 EndOfTrack
    const SMF_BASIC: &[u8] = &[
        0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, // MThd len=6
        0x00, 0x00,                                       // format=0
        0x00, 0x01,                                       // ntracks=1
        0x00, 0x60,                                       // division=96
        0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x0F, // MTrk len=15
        0x00, 0x90, 0x3C, 0x64,   // delta=0   NoteOn  ch0 60 100
        0x60, 0x80, 0x3C, 0x00,   // delta=96  NoteOff ch0 60 0
        0x00, 0xC0, 0x00,         // delta=0   Program ch0 0
        0x00, 0xFF, 0x2F, 0x00,   // delta=0   EndOfTrack
    ];

    #[test]
    fn parse_basic_format0() {
        let evs = parse_midi(SMF_BASIC).unwrap();
        assert_eq!(evs.len(), 3);
        assert_eq!(evs[0], (0, MidiEvent { channel: 0, kind: MidiEventKind::NoteOn { note: 60, velocity: 100 } }));
        assert_eq!(evs[1], (96, MidiEvent { channel: 0, kind: MidiEventKind::NoteOff { note: 60, velocity: 0 } }));
        assert_eq!(evs[2], (96, MidiEvent { channel: 0, kind: MidiEventKind::Program { program: 0 } }));
    }

    #[test]
    fn noteon_velocity_zero_becomes_noteoff() {
        // Track: delta=0 NoteOn ch0 note=60 vel=0 -> should parse as NoteOff
        // Track bytes (8 bytes):  00 90 3C 00   00 FF 2F 00
        let smf: &[u8] = &[
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x90, 0x3C, 0x00,   // NoteOn vel=0 -> NoteOff
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let evs = parse_midi(smf).unwrap();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].1.kind, MidiEventKind::NoteOff { note: 60, velocity: 0 });
    }

    // Running status: NoteOn status set at first event reused for the second.
    // Track bytes (11 bytes):
    //   00 90 3C 64   NoteOn ch0 60 100 (sets running_status=0x90)
    //   00 3E 50      running NoteOn ch0 62 80
    //   00 FF 2F 00   EndOfTrack
    #[test]
    fn running_status_reused() {
        let smf: &[u8] = &[
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x0B,
            0x00, 0x90, 0x3C, 0x64,   // NoteOn 60 100
            0x00, 0x3E, 0x50,         // running NoteOn 62 80
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let evs = parse_midi(smf).unwrap();
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].1.kind, MidiEventKind::NoteOn { note: 60, velocity: 100 });
        assert_eq!(evs[1].1.kind, MidiEventKind::NoteOn { note: 62, velocity: 80 });
        assert_eq!(evs[0].0, 0);
        assert_eq!(evs[1].0, 0);
    }

    // Tempo meta (0xFF 0x51 0x03 us[3]).
    // 500000 us/beat = 120 BPM.
    #[test]
    fn tempo_meta_parsed() {
        // Track bytes (12): 00 FF 51 03 07 A1 20   00 FF 2F 00
        let smf: &[u8] = &[
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x0B,
            0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20,  // 500000 us
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let evs = parse_midi(smf).unwrap();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].1.kind, MidiEventKind::Tempo { us_per_beat: 500_000 });
    }

    // Control change.
    #[test]
    fn control_change_parsed() {
        // Track: 00 B9 07 64   (delta=0, CC ch9 cc=7 val=100)   then EOT
        let smf: &[u8] = &[
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x08,
            0x00, 0xB9, 0x07, 0x64,
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let evs = parse_midi(smf).unwrap();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].1, MidiEvent { channel: 9, kind: MidiEventKind::Control { cc: 7, value: 100 } });
    }

    // Pitch bend: lsb=0x00 msb=0x40 -> raw=(0x40<<7|0)=8192 -> value=0 (center).
    #[test]
    fn pitch_bend_center_is_zero() {
        let smf: &[u8] = &[
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x08,
            0x00, 0xE0, 0x00, 0x40,   // pitch bend center
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let evs = parse_midi(smf).unwrap();
        assert_eq!(evs[0].1.kind, MidiEventKind::PitchBend { value: 0 });
    }

    // VLQ encoding edge cases.
    #[test]
    fn vlq_single_byte() {
        let data = &[0x00u8];
        let mut pos = 0;
        assert_eq!(read_vlq(data, &mut pos).unwrap(), 0);
        let data = &[0x7Fu8];
        let mut pos = 0;
        assert_eq!(read_vlq(data, &mut pos).unwrap(), 127);
    }

    #[test]
    fn vlq_two_bytes() {
        // 0x81 0x00 = 128
        let data = &[0x81u8, 0x00];
        let mut pos = 0;
        assert_eq!(read_vlq(data, &mut pos).unwrap(), 128);
        // 0xFF 0x7F = 16383
        let data = &[0xFFu8, 0x7F];
        let mut pos = 0;
        assert_eq!(read_vlq(data, &mut pos).unwrap(), 16383);
    }

    #[test]
    fn vlq_overflow_rejected() {
        // 5 bytes with continuation bits set (>4 bytes)
        let data = &[0x81u8, 0x80, 0x80, 0x80, 0x00];
        let mut pos = 0;
        assert_eq!(read_vlq(data, &mut pos).unwrap_err(), ParseError::VlqOverflow);
    }

    #[test]
    fn bad_magic_rejected() {
        let bad = &[0x00u8; 20];
        assert_eq!(parse_midi(bad).unwrap_err(), ParseError::BadMagic);
    }

    #[test]
    fn too_short_rejected() {
        // Completely empty -> TooShort (read_tag can't even read the magic).
        assert_eq!(parse_midi(&[]).unwrap_err(), ParseError::TooShort);
        // Magic present but header truncated -> UnexpectedEof (past magic, read_u32 fails).
        assert_eq!(parse_midi(&[0x4D, 0x54, 0x68, 0x64]).unwrap_err(), ParseError::UnexpectedEof);
    }

    // Format-1 two-track merge: both tracks get merged and sorted by tick.
    // Track 1: tick 0 NoteOn ch0 60 100, tick 48 NoteOff ch0 60 0
    // Track 2: tick 0 NoteOn ch1 64 80,  tick 48 NoteOff ch1 64 0
    // Merged sorted: 4 events, ticks [0,0,48,48]
    #[test]
    fn format1_two_tracks_merged() {
        // Track bytes (each 12 = 0x0C):
        //   00 90 3C 64   delta=0 NoteOn
        //   30 80 3C 00   delta=48 NoteOff
        //   00 FF 2F 00   EOT
        let track = |ch_note_on: u8, ch_note_off: u8, note: u8| -> Vec<u8> {
            vec![
                0x4D, 0x54, 0x72, 0x6B, 0x00, 0x00, 0x00, 0x0C,
                0x00, ch_note_on, note, 0x64,
                0x30, ch_note_off, note, 0x00,
                0x00, 0xFF, 0x2F, 0x00,
            ]
        };
        let mut smf = vec![
            0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x01,             // format=1
            0x00, 0x02,             // ntracks=2
            0x00, 0x60,
        ];
        smf.extend(track(0x90, 0x80, 60)); // ch0 note 60
        smf.extend(track(0x91, 0x81, 64)); // ch1 note 64

        let evs = parse_midi(&smf).unwrap();
        assert_eq!(evs.len(), 4);
        // All tick-0 events come before tick-48 events.
        assert_eq!(evs[0].0, 0);
        assert_eq!(evs[1].0, 0);
        assert_eq!(evs[2].0, 48);
        assert_eq!(evs[3].0, 48);
    }
}
