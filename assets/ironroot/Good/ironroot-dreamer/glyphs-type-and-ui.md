# Visual Schema: Glyphs, Type & Diegetic UI
## The Knife Sees Only the World

Every UI state exists as an inscribed object, mark, or sound in Veyrholt. No chrome floats above the world.

## THE MASTER MARKS: Mason's Glyph System

Sixteen base marks, each carved into an 8×8 pixel cell on a trit lattice (three stroke states per position: absent, present, doubled). Composed horizontally into statements. Appear on: stone course (Cathedral vertical authority), ledger margins, skin-brands, oath-weapons, Spirit echoes.

| Mark | Name | Meaning | Stroke Pattern | Home |
|---|---|---|---|---|
| ⊥ | **Threshold** | Entry, binding, line-cross | Single horizontal + vertical drop | Foundation stone |
| ⊳ | **Debt** | Owed, delayed, accrued | Rightward tick + tail | Ledger margin |
| ⊲ | **Paid** | Settled, moved past, closed | Leftward tick + rest | Ledger margin |
| ⊗ | **Vow** | Sworn, bound, unbreakable | Crossed center + four points | Oath-weapon |
| ═ | **Toll** | Passage tax, rhythm count, repetition | Double horizontal + pulse gap | Toll-gate stone |
| ⬚ | **Void** | Erased, administrative removal, gap | Hollow square, clean edges | Cleanse sites |
| ◐ | **Mercy** | Refusal of violence, restraint, witness | Half-filled circle, open side | Shrine clay |
| ▰ | **Weight** | Stone, gravity, anchor, institution | Filled rectangle + doubled left | Anvil mark |
| ≈ | **Flow** | Water, path, Spirit route, testimony | Wavy line, three undulations | Canal stone |
| ⟡ | **Fracture** | Damage, scarring, learned pain, memory | Jagged line, asymmetric breaks | War-zone ledger |
| ⊙ | **Root** | Growing, life-link, anchor-deep, graft | Circle + cross + four radial lines | Buried passage |
| ◆ | **Glass** | Reflection, clear seeing, dangerous sight | Diamond, high-contrast facets | Lung stone |
| ▴ | **Ash** | Burning, purge, irreversible change | Upward triangle + drift tail | Cinder Parish |
| ≋ | **Breath** | Speech, silence, testimony stolen, spell-cut | Horizontal lines, shortening rhythm | Index shelf |
| ∞ | **Pattern** | Recursion, self-reference, habit-loop | Interlocked curves, no start | Shadow scar |
| ⟜ | **Nail** | Hammered debt, violence-made-permanent | Downward spike + ring mark | Anvil Ward |

Composition: read left-to-right as a statement. A debt-mark (⊳) followed by a toll-mark (═) reads "owed payment." Doubled strokes on the lattice mean *stress* or *urgency*. Ascension happens when five marks align in a personal ledger—the player's silhouette splits into vow-sutures matching the marks, never animalized.

## SCAR-LANGUAGE: The Material Pattern Tongue

Replaces star-signs entirely. Five legible nail-constellation patterns:

- **The Dead Bell** (four nails in descending vertical line, closest-to-farthest): death, silence, archive; reads on grave-markers and death-toll walls.
- **The Hammer Ring** (nine nails in circle, one centered): work, oath-forging, Anvil blessing; reads on weapon hafts and forge-hoard markers.
- **The Toll Cross** (five nails: four cardinal + center): balanced debt, payment-moment, threshold; reads on gates and transition stones.
- **The Root Spiral** (seven nails spiraling inward): descent, recovery, passage inward; reads on dungeon entrances and Spirit-route maps.
- **The Witness Scatter** (irregular three nails, no pattern, closest ≤ 3px): mercy granted, person-held-alive, non-debt; reads on safe-rest caches and memory-saved ledgers.

Vow-sutures (armor splitting into glowing lines during ascension) follow the same nail-count and arc logic — never antlers, never wings.

Fracture lines (burn maps on stone and flesh) are burned-ash traces of cast-word pronunciation — the scar carries the name of what was spoken. Ledger ink under skin (Brand scars) reads as institutional allegiance: Toll-Saint chain, Index Monk page-corner, Ledger Church bell-rope. The Shadow's boiling interior holds only one honest geometry: scars inherited from events the player has lived.

## TYPOGRAPHY LAW (Sean, 2026-08-16 — supersedes all prior font stacks)

**Two faces only.**

1. **EB Garamond** — the humane voice. All dialogue, internal narration, the Book's pages, titles and act headings (Garamond at display sizes, letter-spaced, replaces any carved-title face), print and media. Italic for the Voice; small caps for headings.
2. **A humanist sans** — the institutional/instrumental voice. Ledger stamps, HUD labels, terminal surface, menus, captions, assessment notices. *Exact face: Sean's pick pending — "Humanist Sub" needs its full name before asset lock.* [BLOCKED on Sean naming the face]

Retired by this ruling: the 2DAK font-test stack (IBM Plex Mono ledger / Cinzel title), and this lane's earlier proposal (Tektur / IBM Plex Mono / Iosevka). CommitMono remains only as the *currently shipping* face of the live mud terminal binary (manifest receipt), which raises the one open seam:

> **SEAM — terminal monospace**: a REPL wants a monospaced grid; EB Garamond and most humanist sans are proportional. Either (a) a monospaced cut in the humanist family is blessed for the terminal grid, or (b) the terminal keeps a mono exception under the humanist sans's voice. Sean's call. Until then, frame-by-frame docs that say "terminal face" mean: the humanist sans, or its blessed mono cut.

## DIEGETIC HUD LAW: No Floats, No Meters, Only the World

**Terminal Lens (MUD REPL)**
- Health: bell count (audible chimes; at half health the interval shortens; at critical, the bell cracks — descending pitch, sound glitches).
- Resources (cast-words, vows, momentum): echoes of prior cast-words visible in the ledger margin as fading text; a new cast-word overwrites the oldest. Vow-count shown as notches carved in stone (in terminal shorthand `[====]`, never a bar).
- Direction: in-world landmarks (bell towers, destroyed gates, visible fires) named in room-text. Spirit routes (known only by dying there) marked in the ledger as `<SCAR: Toll Drain, third niche>`.
- Time: bell-count accumulation (13 bells = one full hour in-fiction). The Metronome breath (10-second swell, fog motion) happens without notation.
- Threat: footsteps audible in text; NPC dialogue shifts to formal ledger-speak (bureaucratic, faster rhythm); wounds appear as physical text ("your left hand trembles"); Shadow presence noted as `[STAIN]`; Erasure proximity as silence lengthening in the room-text.

**Knife 2D Lens**
- Health: silhouette color shifts (void-black → grave-iron dusk → ash-white as damage mounts). At critical, the outline flickers and the heartbeat becomes visible inside the silhouette as a red pulse.
- Position & spacing: measured in 1-pixel = 1-voxel units. No numbers. Parry distance is visible (enemy reach shows as a visual arc when they commit; the player learns by frame-reading).
- Cast-words: as the word forms, the player's silhouette glows with trace-marks (scar-lines building the glyph in real time). Interruption cuts the glyph mid-stroke — the damage appears in the scar-map.
- Shadow scar-map: the player's own silhouette **is** the display. Inherited scars render as fixed geometry inside the outline. No separate panel.
- Momentum/state: pose communicates. Crouched = grounded, high-armor. Raised guard = stamina held. After-image trails for a half-second after dodge or parry = recovery speed made visible, never stated.

## TOMBSTONE & SHADOW RENDER RULES

**Tombstone silhouette family** (readable at 16px, un-deletable)
The Tombstone is a fixed silhouette — a death-shape carved into memory and rendered as a permanent environmental obstacle. Once placed, it is visually final: opaque, non-animating, hard shadow. The silhouette's pose encodes the death (standing = ambush, kneeling = mercy received, twisted = violent end, still = exhaustion). Future runs pass through these monuments. A crowded graveyard is genuinely crowded — players physically navigate around prior deaths.

**Shadow blot rules** (boiling absence, one honest geometry)
The Shadow renders as a roiling, smoke-like absence in the player's silhouette interior. Its only stable geometry is the inherited scar-pattern — scar pixels never move, never repeat a frame's churn. Boiling pixels animate at 8Hz (unpredictable churn, no cycle) and never overwrite scar-lines. A chain-burn scar from the Tollroad is the only geometry that **holds still** while the rest writhes. Rendering: scar pixels = fixed palette (grave-iron); boiling pixels = dithered ash + void-black at frame-by-frame randomness, never a smooth gradient.

## BANNER STEGANOGRAPHY: Hidden Glyphs in Weave

Faction glyphs hide inside banner art via negative space and weave-pattern edges. A Toll-Saint banner's vertical stripe rhythm (═) encodes `[TOLL]` when the player stands directly before it and the light angle aligns — no prompt teaches this; the player learns by reading. An Index Monk banner's frayed edge reads as `[RECORD]` when the page-curl angle matches a specific standing position. Reading is an **active stance** — stand still, crouch, turn to match the intended angle. No UI confirms it; the ledger updates silently when the read succeeds. A Ledger Church banner's marks appear at the intersections of stone-course masonry, readable only from outside the sanctuary.

## TERMINAL ↔ 2D SHARED LANGUAGE: One World, Two Cameras

Both lenses use the **same palette** (void-black, crimson-witness, bell-bronze, grave-iron, ash, bone), **same glyphs** (Master Marks appear in terminal text and on 2D silhouettes/environment alike), **same sound vocabulary** (bell tones, Name-Shear transient, toll-pulse, cast-word phonemes).

When the player transitions between lenses (dying in 2D → the Spirit side read as terminal → returning to 2D at the Orchard), the **same event ledger** carries forward. A Tombstone placed in 2D is navigable as a stone cairn in terminal view. A vow inscribed in the terminal ledger appears as a scar on the player's armor in 2D.

Phosphor text is treated as diegetic inscription — the ledger is written in light, not paper. This honesty (no fourth-wall split between "display" and "world") is the unifying law: the MUD screen and the Knife camera are two ways of seeing **the same Veyrholt**.
