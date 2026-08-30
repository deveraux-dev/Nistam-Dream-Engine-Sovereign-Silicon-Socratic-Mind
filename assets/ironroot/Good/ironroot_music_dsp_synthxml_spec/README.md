# IRONROOT Music DSP + SynthXML Specs

This package defines the technical music layer for IRONROOT.

## Files

1. `SPEC_01_MUSIC_DSP_SYNTHXML.md`
   - Main architecture spec.
   - MusicXML → SynthXML → MIDI 2.0 → Faust → Synthesia.

2. `SPEC_02_SYNTHXML_SCHEMA_DRAFT.md`
   - Practical XML structure and complete example.

3. `SPEC_03_RUST_HARMONIC_RUNTIME_API.md`
   - Rust-facing type and function draft.

## Core Rule

```text
A song is not a track.
A song is a recoverable piece of world memory.
```
