# IRONROOT Music Logic Primer

## Purpose

This primer maps the Music DSP + SynthXML system directly into the IRONROOT game.

The music layer is not an accessory. It is a readable world-system.

```text
Sheet music
→ MusicXML
→ SynthXML
→ MIDI 2.0 expressive events
→ Faust DSP
→ Harmonic Runtime
→ Game state
→ Player perception
```

Core rule:

```text
A song is not a track.
A song is a recoverable piece of world memory.
```

---

# 1. What Music Means In IRONROOT

In IRONROOT, music is a **state language**.

It carries:

- place memory,
- death memory,
- route memory,
- hidden account pressure,
- First Lock clues,
- faction pressure,
- Name-Shear warnings,
- Yod / Quincunx tension,
- public ledger echoes,
- social room-state.

Traditional game audio asks:

```text
What should the player hear here?
```

IRONROOT asks:

```text
What is the world trying to remember here?
```

---

# 2. The Three Audio Cultures

IRONROOT has three separated but interoperable audio cultures.

## A. Folk Memory

Old world / village / ritual layer.

Uses:

```text
bells
lutes
choirs
nyckelharpa
hurdy-gurdy
frame drum
wood creak
wind
funeral song
tavern voice
```

Function:

```text
preserve ancestry, village identity, public ritual, hidden grief
```

---

## B. DJ / Threadkeeper Culture

In-world underground social layer.

Uses:

```text
kick
sub
vinyl hiss
tape drift
dub delay
room rumble
pressure loops
crowd murmur
filter sweeps
```

Function:

```text
move the room, stabilize crowds, open routes, manipulate harmonic pressure
```

Rule:

```text
DJ culture makes the room move.
```

---

## C. Secret Radio / Ledger Broadcast

External / parallel witness layer.

Uses:

```text
broadcast fragments
dead air
static
weather voices
field recordings
bell inserts
ledger phrases
degraded archives
```

Function:

```text
preserve impossible memory and leak ledger echoes into the real world
```

Rule:

```text
Radio makes the world remember.
```

---

# 3. The Music Pipeline

## Authoring Pipeline

```text
Composer / found score / field recording
↓
Sheet music
↓
MusicXML
↓
Harmonic Compiler
↓
SynthXML
↓
MIDI 2.0 event graph
↓
Faust DSP routing
↓
Runtime mixer / Synthesia projection
```

---

## Runtime Pipeline

```text
Zone state
+ Era state
+ Hidden account drift
+ First Lock state
+ Death Scars
+ Puzzle Scars
+ Name-Shear pressure
+ Yod / Quincunx pressure
+ Public Ledger hash
+ Player proximity
↓
Harmonic Runtime
↓
Thread state
↓
MIDI 2.0 / Faust / Mixer / Synthesia
↓
Player hears world memory
```

---

# 4. SynthXML Role

SynthXML is the game’s semantic music format.

MusicXML says:

```text
these notes exist
```

SynthXML says:

```text
these notes mean something in IRONROOT
```

SynthXML contains:

- source hash,
- tempo,
- mode,
- account pressure,
- quincunx definitions,
- Yod / Finger-of-God pressure,
- threads,
- notes,
- silence events,
- primitives,
- Faust processors,
- First Lock hints,
- ledger markers.

---

# 5. Hidden Accounts Through Music

The hidden account system should be heard before it is understood.

| Hidden Account | Musical Behavior |
|---|---|
| Red Debt | unresolved dominant, interruptive cadence |
| Stone Root | low drones, fifths, immovable pulse |
| Double Witness | mirrored canon, dual voices |
| Grave-Water | drowned reverb, submerged cadence |
| Crownless Roar | dominant melody overpowers mix |
| Clean Index | pure intervals, corrected rhythm |
| Equal Knife | symmetrical tension/release |
| Venom Wedding | beautiful dissonance, sweet poison |
| Far Wound | drifting tonic, unresolved travel |
| Last Toll | recursive bells, toll patterns |
| Hollow Star | missing root, broken tonic |
| Mercy Drowned | soft plagal cadence, forgiving descent |
| Outside Wheel | meaningful silence |

Player-facing rule:

```text
Never say zodiac.
Let the player hear pressure before naming it.
```

---

# 6. First Locks Through Music

First Locks can use music in four ways.

## A. Clue Carrier

A song hides a route, count, phrase, missing note, or toll pattern.

Example:

```text
The Last Toll First Lock is not solved by ringing the bell.
It is solved by noticing the bell phrase always leaves one toll absent.
```

---

## B. Ritual Input

Players reproduce or complete a harmonic pattern.

This can use:

- Synthesia projection,
- bell recall,
- route tones,
- DJ Threadkeeper transitions,
- MusicXML-derived notation.

---

## C. World-State Change

After a First Lock solve:

```text
the world_first relic is minted
the Echo version enters the loot pool
the public ledger updates
the radio may leak a phrase
the zone’s harmonic pressure changes
```

---

## D. Echo Farming

Later players do not receive the 1/1 relic.

They receive:

```text
weakened Echo relic
degraded score fragment
Puzzle Scar
broadcast residue
```

---

# 7. Name-Shear Audio

Name-Shear should be unforgettable but accessible.

It should not be just “loud horror audio.”

Name-Shear audio logic:

```text
remove a voice
break a note lane
silence a bell partial
drop the tonic
corrupt a ledger phrase
leave dead air
```

For audio accessibility, expose:

- reduced intensity,
- replacement tone,
- visual-only mode,
- haptic-only mode,
- subtitles / direction cues.

---

# 8. Synthesia As Game UI

Synthesia is not a piano trainer.

It is harmonic literacy UI.

Projection modes:

| Mode | Use |
|---|---|
| FolkNotation | songs, hymns, village music |
| BellArc | tolls, bells, First Locks |
| ThreadLanes | harmonic runtime threads |
| DjPulseGrid | Threadkeeper / DJ spaces |
| BroadcastWaterfall | radio / ledger broadcast |
| FirstLockRitual | puzzle solve |
| NameShearBreak | erasure / broken notation |
| VowlessSilence | hidden lanes / refusal |

---

# 9. MIDI 2.0 Role

MIDI 2.0 is used because the game needs expressive per-note state.

Each note/toll can carry:

- pitch drift,
- pressure,
- timbre,
- witness weight,
- route memory,
- hidden account bias,
- ledger weight,
- Hollow pressure.

This lets music behave like game state, not just playback.

---

# 10. Faust DSP Role

Faust expresses the state.

Faust does not decide lore.

```text
Game state decides meaning.
Faust makes meaning audible.
```

Required processors:

```text
bell_model
bell_memory_filter
hollow_tonic_remover
gravewater_lowpass
last_toll_recursion
quincunx_phase_splitter
yod_converger
finger_of_god_compressor
witness_echo
name_shear_silencer
shortwave_drift
thread_filter
route_delay
crowd_resonator
```

---

# 11. Audio Primitives

Audio primitives are the atomic sound vocabulary.

Examples:

| Primitive | Meaning |
|---|---|
| Bell Toll | accounting / death / Last Toll |
| Rope Creak | structural instability |
| Wood Knock | witness acknowledgment |
| Choir Residue | public memory |
| Ash Hiss | erasure proximity |
| Hollow Gap | missing information |
| Kick/Sub | room pressure / DJ pulse |
| Static | broadcast uncertainty |
| Dead Air | missing record / Vowless |

Primitives are not just samples.

They are semantic sound-glyphs.

---

# 12. Semantic Mixer

Do not expose studio terms to designers or players.

Use:

```text
Warmth
Distance
Fog
Memory
Decay
Witness
Tension
Bell Weight
Ash
Hollow
Root
Crowd
Static
```

Under the hood these map to:

```text
gain
EQ
filtering
delay
reverb
compression
saturation
spatialization
```

---

# 13. Game Integration Points

## Zone Runtime

Each zone has:

- account pressure,
- era,
- mood,
- harmonic profile,
- MusicXML/SynthXML source,
- audio primitive pool.

---

## Combat

Combat uses:

- superior dexter audio override,
- charge-head focus motif,
- quincunx tension split,
- Yod convergence,
- First Lock boss phase sound.

---

## Crafting

Crafted artifacts can have:

- provenance motif,
- maker signature,
- charge resonance,
- relic harmonic identity.

---

## Crime / Chronothief

Crime can alter:

- route rhythms,
- hidden pulse patterns,
- broadcast fragments,
- forbidden DJ white labels.

---

## Public Ledger

Ledger events become:

- radio echoes,
- bell inserts,
- public chant motifs,
- score fragments.

---

## Death Scars

Death Scars can generate:

- replayable primitive chains,
- TCG audio identity,
- ghost melody fragments,
- location-specific hums.

---

# 14. Determinism

All music logic must be deterministic until the DSP boundary.

Hash:

```text
MusicXML source
SynthXML file
world seed
zone hash
ledger hash
thread IDs
event IDs
MIDI 2.0 event graph
Faust parameter packets
mixer state
```

This supports:

- replay,
- lockstep,
- world-first proofing,
- viral death cards,
- consistent puzzle solves.

---

# 15. Design Guardrails

Do:

```text
make music readable over time
hide meaning in repetition
use silence as content
make bells emotional before mechanical
let players learn by hearing
preserve separation between DJ and Radio
```

Do not:

```text
make everything a rhythm game
turn radio into exposition
make DJ culture a class gimmick
overuse horror stingers
expose zodiac labels
force music theory on players
```

---

# Final Primer Law

```text
IRONROOT music is a memory system first, an audio system second.
```
