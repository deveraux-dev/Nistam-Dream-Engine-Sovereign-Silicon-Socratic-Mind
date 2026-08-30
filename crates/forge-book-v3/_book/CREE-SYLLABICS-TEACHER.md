# CREE-SYLLABICS-TEACHER — Four-Crate Fold Map

> Status: WIRED (all sources exist). Next: integration tick + HITL verification.
> Date: 2026-07-03
> Owner: Sean Morin

## Thesis

A multimodal Cree syllabics teacher that unifies SEE + HEAR + PROGRESS:

1. **SEE** — `forge-overlay` renders the glyph via `vixi_render` Overlay z-layer
2. **HEAR** — `forge-midi::midi_out` plays the syllabic's phoneme as live MIDI
3. **MAP** — `forge-calligraphy::syllabic_to_event` bridges glyph geometry → MIDI event
4. **PROGRESS** — `forge-insights` tracks which syllabics the learner has earned

The syllabary's structure IS the curriculum: orientation = vowel (pitch),
shape = consonant (timbre), superscript = final (percussion). When you SEE ᐸ,
you HEAR A3 (stop onset drops an octave, vowel A = A4 - 12 = A3), you see
amber (warm stop colour from synesthesia palette), and forge-insights marks
"PA learned" once you've correctly identified it 3 times.

---

## Crate Topology (no new edges)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        13forge-studio.exe                            │
│                      (the teacher HOST)                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐    ┌──────────────────┐    ┌───────────────┐  │
│  │ forge-overlay   │    │ forge-midi       │    │ forge-insights│  │
│  │ (vixi_render)   │    │ (midi_out)       │    │ (progression) │  │
│  │                 │    │                  │    │               │  │
│  │ Overlay z-layer │    │ WinMM NoteOn/Off │    │ ProgressEvent │  │
│  │ renders glyph + │    │ zero-alloc send  │    │ Milestone     │  │
│  │ colour field    │    │ feature:winmm-out│    │ CreativeCoach │  │
│  └────────┬────────┘    └────────┬─────────┘    └───────┬───────┘  │
│           │                      │                      │          │
│           │    ┌─────────────────┴──────────────┐       │          │
│           │    │ forge-calligraphy              │       │          │
│           │    │ (syllabic_to_event + cree_syllabics) │ │          │
│           │    │                                │       │          │
│           │    │ SyllabicEntry → SyllabicMidiEvent    │ │          │
│           │    │ text_to_events() batch         │       │          │
│           │    │ UCAS_MAIN/UCAS_EXTENDED tables │       │          │
│           │    └────────────────────────────────┘       │          │
│           │                                             │          │
│  ┌────────┴─────────────────────────────────────────────┴────────┐ │
│  │ tree-sitter-vixel::synesthesia                                │ │
│  │ NodeVoice { material_id, note, voice }                        │ │
│  │ The COLOUR source: material_id → VixelMaterial → TokenSheet   │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Dependency edges (all pre-existing or internal):
- `forge-overlay` → `forge-canvas`, `forge-vix` (layout engine)
- `forge-midi` → `forge-hal`, `forge-harmonics` (+ windows-sys optional)
- `forge-insights` → none (standalone, serde only)
- `forge-calligraphy` → none (serde/sha2 only, Firewall Law)
- `tree-sitter-vixel` → none (zero-alloc CST parser)
- **No new cross-crate edges required.** The HOST (forge-studio) owns the glue.

---

## Component Status

| Component | Crate | File | Status |
|-----------|-------|------|--------|
| UCAS table (content) | forge-calligraphy | `cree_syllabics.rs` | ✅ LIVE |
| Phoneme→MIDI adapter | forge-calligraphy | `syllabic_to_event.rs` | ✅ WIRED |
| WinMM MIDI out sink | forge-midi | `midi_out.rs` | ✅ WIRED (feature: winmm-out) |
| Synesthesia colour | tree-sitter-vixel | `synesthesia.rs` | ✅ LIVE |
| Overlay renderer | forge-overlay | `vixi_render.rs` | ✅ LIVE |
| Progression tracker | forge-insights | `lib.rs` | ✅ LIVE |
| **Teacher host tick** | forge-studio | TBD (new fn) | ⬜ NEXT |
| **Lesson sequencer** | forge-studio | TBD (new mod) | ⬜ NEXT |

---

## Data Flow (one teacher tick)

```
                    User input: "show me ᐸ"
                           │
                           ▼
┌─ forge-calligraphy ──────────────────────────────────┐
│  by_char('ᐸ') → SyllabicEntry(0x1438, 'ᐸ', "PA")   │
│  syllabic_to_event(&entry) → SyllabicMidiEvent {     │
│      channel: 0, note: 57, velocity: 90,             │
│      duration_ms: 300, is_final: false               │
│  }                                                    │
└──────────────────────────────────────────────────────┘
                           │
              ┌────────────┼────────────────┐
              ▼            ▼                ▼
┌─ forge-midi ──┐  ┌─ forge-overlay ───┐  ┌─ forge-insights ────┐
│ MidiOut::open()│  │ vixi_render:     │  │ ProgressEvent {     │
│ .note_on(      │  │   Overlay z-layer│  │   tick: 4201,       │
│   0, 57, 90)   │  │   render glyph ᐸ │  │   actor: User,      │
│                │  │   + colour from   │  │   domain: 12,       │
│ (300ms later)  │  │   synesthesia     │  │   action: AssetView │
│ .note_off(0,57)│  │   material_id     │  │   outcome: 10000    │
└────────────────┘  └──────────────────┘  │ }                   │
                                           │ check_milestones()  │
                                           └─────────────────────┘
```

---

## Synesthesia Colour Mapping (the SEE channel)

Each syllabic's onset class maps to a synesthesia material_id, which resolves
to a colour through the authored TokenSheet:

| Onset Class | material_id | Semantic | Expected Colour |
|-------------|------------|----------|-----------------|
| Vowel-only | MAT_VOID (0) | base truth | warm paper |
| Stop (p,t,k) | MAT_STRUCTURAL (1) | grid anchor | steel blue |
| Affricate (c) | MAT_CONTROL (2) | control | amber |
| Fricative (s) | MAT_ACOUSTIC (5) | resonance | violet |
| Nasal (m,n) | MAT_IMMUTABLE (3) | foundation | deep green |
| Approx (w,y) | MAT_KINETIC (4) | motion | orange glow |
| Final | MAT_ACTION (7) | percussive | bright flash |

This means the COLOUR of each glyph in the overlay carries learning signal:
students start to associate "steel blue glyphs = stops = percussive low notes."

---

## Pitch Architecture (the HEAR channel)

### Pentatonic Foundation (C-major, no clashes)

```
Vowel:  E=C4(60)  I=E4(64)  O=G4(67)  A=A4(69)
        └─ front ─┘  └─ high ─┘  └─ back ─┘  └─ open ─┘

Onset:  Vowel    Stop     Affr    Fric    Nasal   Approx
        +0       -12      -10     +12     -12     +0
        (oct 4)  (oct 3)  (oct 3) (oct 5) (oct 3) (oct 4)
```

This gives a ~3 octave range (C3–A5) across the full syllabary, with:
- Low/warm notes for foundation glyphs (stops, nasals)
- Mid notes for neutral glyphs (vowels, approximants)
- High/bright notes for airy glyphs (fricatives)

### Finals → Drum Channel (GM percussion)

| Final | Name | GM Drum | Sound |
|-------|------|---------|-------|
| ᐟ (t) | FINAL ACUTE | 37 | Side stick |
| ᐠ (k) | FINAL GRAVE | 36 | Bass drum |
| ᐢ (s) | FINAL TOP HALF RING | 42 | Closed hi-hat |
| ᐣ (n) | FINAL RIGHT HALF RING | 38 | Snare ghost |
| ᐤ (w) | FINAL RING | 54 | Tambourine |
| ᐨ (c) | FINAL HORIZONTAL | 76 | Wood block |
| ᑊ (p) | FINAL BOTTOM HALF | 35 | Bass drum soft |
| ᒼ (m) | FINAL MIDDLE DOT | 80 | Muted triangle |

---

## Lesson Progression (the PROGRESS channel)

### Tier 1: Pure Vowels (7 glyphs)
```
ᐁ(E) ᐃ(I) ᐅ(O) ᐊ(A) ᐄ(II) ᐆ(OO) ᐋ(AA)
```
- Milestone: "Vowel Ear" — identify all 4 vowel pitches 3× each
- Unlock: tier 2 consonant series

### Tier 2: Stop Series (12 glyphs)
```
ᐸ(PA) ᐯ(PE) ᐱ(PI) ᐳ(PO) — listen: low register, same 4 pitches
ᑕ(TA) ᑌ(TE) ᑎ(TI) ᑐ(TO) — listen: same register, different attack
ᑲ(KA) ᑫ(KE) ᑭ(KI) ᑯ(KO) — listen: same register, harder attack
```
- Milestone: "Stop Voice" — identify onset AND vowel for 8/12
- Unlock: tier 3 full syllabary

### Tier 3: Complete Syllabary
```
+ Affricate (c), Fricative (s), Nasal (m,n), Approximant (y)
+ Long vowels + labialised (w-dot) variants
+ Finals (drum percussion channel)
```
- Milestone: "Full Circle" — text_to_events("ᓀᐦᐃᔭᐍᐏᐣ") plays correctly
- Unlock: free composition mode

### Tier 4: Composition
- Write words → hear them play as music
- forge-insights tracks "words composed" as AssetCreate actions
- CreativeCoach whispers encourage: "You just played 'nêhiyawêwin' — the
  word for the language itself. Listen to how the nasals (low) frame the
  fricatives (high)."

---

## Integration Points (forge-studio host)

The teacher HOST lives in `forge-studio` and orchestrates the four crates:

```rust
// Pseudocode — the teacher tick in forge-studio
fn teacher_tick(
    lesson: &LessonState,
    overlay: &mut HotSwapOverlay,
    midi: &MidiOut,
    ledger: &mut ProgressionLedger,
) {
    let entry = lesson.current_glyph();

    // SEE: render glyph in overlay
    let spec = build_glyph_overlay_spec(entry);
    overlay.compose(&spec);

    // HEAR: play the syllabic's voice
    if let Some(ev) = syllabic_to_event(entry) {
        midi.note_on(ev.channel, ev.note, ev.velocity).ok();
        // schedule note_off after ev.duration_ms
    }

    // PROGRESS: record the exposure
    ledger.record(ProgressEvent {
        tick: lesson.tick,
        actor: ActorKind::User,
        domain: 12, // calligraphy domain
        action: ActionKind::AssetCreate,
        outcome: lesson.score(),
    });
}
```

---

## What's Already Done vs What's Next

### ✅ Done (this session)
1. `forge-midi/src/midi_out.rs` — WinMM MIDI output sink (feature: winmm-out)
2. `forge-calligraphy/src/syllabic_to_event.rs` — phoneme→MIDI adapter
3. This plan document

### ✅ Previously Live
4. `forge-calligraphy/src/cree_syllabics.rs` — complete UCAS table
5. `forge-overlay/src/vixi_render.rs` — Overlay z-layer renderer
6. `tree-sitter-vixel/src/synesthesia.rs` — NodeVoice colour+note table
7. `forge-insights/src/lib.rs` — progression ledger + milestones
8. `forge-midi/src/keyboard_drum.rs` — GM drum mapping

### ⬜ Next Steps (fold, not build)
9. **Teacher host module** in forge-studio — the integration tick
10. **Lesson sequencer** — VixiScript-authored lesson definitions
11. **Glyph overlay widget** — WidgetSpec for the big-glyph display
12. **HITL verification** — Sean plays it, hears it, approves mappings

---

## Research Sources

- **Script structure**: Wikipedia "Cree syllabics" — orientation = vowel,
  base shape = consonant, unique among abugidas (featurally encoded)
- **Phonology**: r12a.github.io Plains Cree orthography summary —
  4 vowel classes (i, o, a, ê), 10 onset consonants, non-tonal
- **Unicode**: U+1400–U+167F (main), U+18B0–U+18FF (extended) = 726 chars
- **MIDI**: Windows Multimedia (WinMM) `midiOutShortMsg` — channel voice
  messages as single u32 word, zero-latency to GS Wavetable synth
- **Pedagogy**: James Evans taught Cree syllabics using birchbark + soot;
  "virtually all Cree became literate within a few years" — the script's
  regular structure (rotation = vowel) makes it naturally learnable.
  Our teacher adds the auditory channel to exploit this regularity.
