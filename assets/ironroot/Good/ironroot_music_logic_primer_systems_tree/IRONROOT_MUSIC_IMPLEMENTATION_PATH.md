# IRONROOT Music Logic — Implementation Path

## Immediate Goal

Turn the Music DSP + SynthXML system into usable game logic without overbuilding the final audio tools yet.

---

# Phase 1 — Canonical Types

Create:

```text
forge-harmonics/src/
  lib.rs
  musicxml_extract.rs
  synthxml.rs
  account_mapping.rs
  harmonic_threads.rs
  midi2_events.rs
  faust_params.rs
  synthesia_projection.rs
  audio_primitives.rs
  semantic_mixer.rs
  proof.rs
```

---

# Phase 2 — MusicXML Import

Minimum viable importer:

```text
read MusicXML
extract title
extract tempo
extract notes/rests
extract voice/part count
extract interval histogram
extract repetition score
extract tonic stability estimate
emit MusicXmlExtract
```

Do not start with full musicology.

Start with deterministic extraction.

---

# Phase 3 — SynthXML Compiler

Compiler output:

```text
SynthScore
SynthThread[]
SynthEvent[]
AccountPressureVector
FaustProcessorBinding[]
ScoreHash
```

Minimal target:

```text
one MusicXML file
→ one SynthXML file
→ one playable thread
→ one Faust parameter route
→ one Synthesia projection lane
```

---

# Phase 4 — Account Mapping

Implement simple scoring first:

```text
drone fifths → Stone Root
repetition → Last Toll
missing tonic → Hollow Star
dissonance → Venom Wedding
mirrored interval → Double Witness
soft plagal → Mercy Drowned
unresolved cadence → Red Debt
silence → Outside Wheel
```

The first version can be crude.

It only needs to be deterministic.

---

# Phase 5 — MIDI 2.0 Event Graph

Convert SynthEvents into IronrootMidi2Events.

Each event carries:

```text
note
pitch_32
velocity_32
pressure_32
timbre_32
account
witness_q
hollow_q
route_memory_q
ledger_weight_q
```

---

# Phase 6 — Faust Bridge

First processors to implement:

```text
bell_memory_filter
hollow_tonic_remover
gravewater_lowpass
last_toll_recursion
name_shear_silencer
```

Delay DJ processors until the folk/ledger layer is stable.

---

# Phase 7 — Synthesia Projection

Start with three modes:

```text
BellArc
FolkNotation
NameShearBreak
```

Then add:

```text
DjPulseGrid
BroadcastWaterfall
FirstLockRitual
```

---

# Phase 8 — Game Hooks

Wire into:

```text
Zone Runtime
First Lock Registry
Public Ledger
Death Scar Replay
Combat Boss Phase
Secret Radio Scheduler
DJ Threadkeeper Rooms
```

---

# Phase 9 — Determinism Tests

Required tests:

```text
same MusicXML → same SynthXML hash
same SynthXML + world seed → same MIDI event hash
same world state → same Faust parameter hash
same First Lock state → same harmonic pressure
Name-Shear accessibility does not change game-state hash
Vowless silence suppresses projection without corrupting proof hash
```

---

# Phase 10 — Asset Generation

Only after the above:

```text
generate bell primitives
generate folk fragments
generate radio bumpers
generate Synthesia lane visuals
generate DJ primitive kits
generate First Lock audio motifs
```

---

# Clean First Milestone

Build this first:

```text
One Bellman-like MusicXML fragment
↓
SynthXML compile
↓
Last Toll account pressure
↓
BellArc Synthesia projection
↓
Faust bell_memory_filter
↓
First Lock hint marker
```

That proves the entire stack.
