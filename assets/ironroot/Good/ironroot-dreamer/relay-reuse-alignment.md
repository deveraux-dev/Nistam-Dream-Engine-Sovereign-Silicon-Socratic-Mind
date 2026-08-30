# Relay Reuse Alignment — Five Legs onto the New Narrative
### Recon 2026-08-16 (5 haiku riders, receipt-gated) · aligns the old ephemeral/relay systems to the Thirteenth Moon corpus · everything below is proposed canon for Sean's keep-or-cut

## The Unifying Find (the synthesis nobody planned)

The NOSTR kind bands already ARE the cosmology's three-layer truth law:

| NOSTR band | Engine meaning | Canon meaning |
|---|---|---|
| Regular kinds (1000–9999) | stored, replayable events — the sieves | **the Ledger** — Recorded, witnessed, permanent |
| Replaceable kinds (30000–39999) | world state | **the world as it stands** — what the Simulation Layer holds |
| Ephemeral kinds (20000–29999) | relays transmit, never store — the 13th sieve | **the Live Layer** — speech and presence that leave no record; the Free Graves' native band; and the Erasure's hunting ground: *what passes unwitnessed here is how the quiet wins by default* |

Receipt: `F:\v3\.forge\HANDOFF-2026-08-15-sieve-world-nostr.md:10–18` (kind mapping, "seen in transit, held by no one"); terraforma recon HOOK-04. This one table aligns all five legs.

## Leg 1 — AKWEB Ephemeral Relay Chat (astrakey.net) · FOUND WHOLE

**What it was** (receipts: `E:\13forge-super\_merged\AKWEB\docs\plans\2026-02-24-signal-void-lobby-design.md`, `…\2026-02-24-ephemeral-vault-design.md`, `…\ASTRAKEY_TECH_REPORT_v1.md`, `…\AKWEB\hauntology\README`, `F:\v3\TODO\dirge-of-ironroot\OLDPYGOTHROUGH\coop-ephemeral\js\gate.js`):
- Ephemeral kinds **29333** presence beacons (5s heartbeat FSM: ACTIVE/STALE/LOST) · **29334** question-cast · **29335** answer-offer handshake · **29336** shoutbox with TTL decay.
- **Star Map lobby**: signals orbit; orbit radius = signal age; arrivals chime; opacity = remaining TTL — *decay as medium*.
- **Ephemeral Vault**: in-memory only, residue-zero, purged on close. **ECDH P-384** ephemeral keys burned on exit; handshake verified by a **Shared Verse** — a three-word sigil ("Amber. Silence. Pulse."). PoW spam gate; WebRTC E2E tunnel; Theta-Void Hum (55/59Hz binaural) as audio identity.

**What it becomes in canon:**
- **Wind-speech** — the Free Graves' channel (their sound is wind, or no sound). Tavern-talk the Index Monks can NEVER audit, because the relays themselves refuse to hold it. The Vowless can speak here without converting grief into record.
- **The Name-Shear reversed** — chosen decay instead of forced erasure; presence without ledger instead of administrative void. This is the corpus's missing gentle answer to the Shear, and it needs one canon line in a future weld: *what fades by choice was never taken.*
- **The Shared Verse IS a cast-word**: a three-glyph handshake sentence, interruption-vulnerable like all casting — the Alap before content, trust before data.
- The Star Map lobby re-skins as reading the sky: peers as faint lights among the sixteen (Live-Layer lights — never among the named sixteen, always beneath them).

**Gaps:** `hauntology` planned as an AKWEB sibling repo — design phase, unbuilt as of 2026-08-15.

## Leg 2 — Ephemeral Co-op · FOUND (design + partial substrate)

**What it was** (receipts: `F:\NewRepo\MUD_SYSTEMS_PRIMER.md:125–129, 318–323`, `F:\NewRepo\_vault\_plans\pins\ghost-moon\CONSTELLATION-2026-07-09.md`):
- UDP multicast presence (239.13.13.7:13137): visiting peers render as **passive glowing ghosts** — visual-only, 5s keepalive then pruned, never a combat/ledger write. Ghost-moon playhead doctrine: reads the tape, never writes the grid.

**What it becomes in canon (the five hooks, kept):**
1. **Guest keepers are Live-Layer apparitions** — witnessed echoes only; their acts write to *their own* Book (One Writer per Truth). Co-op griefing is structurally impossible: "I did not author that; I witnessed a ghost commit it."
2. **The duet is THE co-op verb** — interlocking incompleteness made multiplayer: shared cast-sentences, one glyph per keeper per turn, recorded asymmetrically (caster/witness) in each Book.
3. **The Witness Balcony is the spectator stand** — guests' presence feeds the arena's witness-weight; what they repeatedly watch, the Shadow learns.
4. **Tombstone etiquette between keepers** — partners' stones acknowledge each other as witness echoes. Ledger poetry, no mechanics.
5. **"Ephemeral" defined once, canonically**: not "evaporates from all records" — *inscribes only into its own Book.*

**Gaps:** netcode (keepalive/prune impl) and ghost-render shader live in quarry lanes, unverified in v3; the two-keeper duet verb is unimplemented; 5 of 8 roots unchecked for further co-op docs.

## Leg 3 — The Terraforma Daemon Server · FOUND (plan + live substrate)

**What it is** (receipts: recovered transcript `cc-transcript-1786870335685.txt:6517` names the "Terraform Server"; `crates/forge-daemon-door` TCP 13013 whitelist; `timeline_recorder.rs` BLAKE3-chained tape; `forge-audio-v3/src/sovereign_comms/mod.rs` NostrEvent/NostrClient stub, loopback-only per ARCH-008; ADR-0025 "daemon owns the loop; CLI is ephemeral cattle"):
- Addressing: **(tick_id, moon, code_hash)** — the moon is the batch boundary, thirteen natural lanes. Deterministic replay via SealedTuple tick ordering.

**What it becomes in canon (hooks kept):**
1. **Spatial-cell conservation is the daemon's first LAW**: `SET LAW "never allow double-spending of spatial cells"` — one (pubkey, moon, tick) tuple owns one mutation. The tutorial's example law is literally the server's real invariant.
2. **Terraforming = planting the grave-orchard back to light**: every Rootcalling/Stonecalling state-delta signs to the relay; the BLAKE3 tape IS the orchard's recovery ledger.
3. **Tombstones are terrain objects on the relay** — permanent kind-band events; every map readback includes them.
4. **The Erasure is unwitnessed ephemeral decay** — the daemon logs the 13th sieve's passage without publishing; quiet = the Erasure winning by default; the 120Hz heartbeat is the counter-pressure. *This is now mechanical canon, not metaphor.*
5. **Era-moods as server-vibe**: sealed moments carry a 6-bit essence codeword; thirteen consecutive decay-heavy moons drift the relay's tone — the world is remembered as it was felt.

**Gaps:** NostrClient is a stub (no real sign/publish); Schnorr/secp256k1 not in Cargo.lock; "terraforma" name unverified in E:\ tapes (transcript-only receipt).

## Leg 4 — The Java Signal Dating App · NOT FOUND (ask Sean)

Checked: F:\v3 (found `ironroot-signal-v3` — signal-*routing* DSP, not Signal-*protocol*; not it), F:\NewRepo, plus a bounded depth-4 filename sweep of E:\13forge-super\_merged, F:\NewRepo, F:\v3\TODO, E:\newgap for `pom.xml`/`build.gradle`/`*.apk`: **zero Java project markers**. Legal claim per the absence-gate: *not in the checked game trees; deep Java scans of E:\.airgap and F:\_quarry incomplete; most likely it lives outside these roots entirely* (personal projects dir, old drive, or a repo host). **ARCH000: name the path and this leg completes.** The alignment is pre-authored and waiting: Signal's pairwise sealed sessions = the world's truly private speech; the matching/handshake flow re-skins as the Shared Verse Alap; each party's Book records only their side.

## Leg 5 — Séance / 13-Door / Hauntology · FOUND LIVE IN V3 (the crown)

**What it is** (receipts: `F:\v3\TODO\quarry-sort\_stale\GHOST-CONSTELLATION-v3-2026-08-10.md:75–104`, `F:\v3\MYTHOS.md:27–31`, `DIALOGUE-FLOW-REQUIREMENTS.md:23–25, 146–189`, `crates/forge-mud-v3/src/haunt.rs`, `dm.rs:625–699`, `F:\NewRepo\…\26-the-seance-of-the-second-kind.md`):
- **The Two-Kind Séance**: APPARITION (project ghost state to pixels, witnessed back exact) vs **HAUNT** (drop presence-as-drive into the field; it settles into standing resonance). Four gestures: SHOW · WITNESS · HAUNT · EXHUME.
- **The Three Hauntings** by spectral norm: fades / won't rest / **poltergeist** (refused at construction).
- **The 13-structure**: five trits in a byte leaves exactly 13 sentinel states (243–255) — the **control envelope** holding absence, tombstone, overflow. *The absent moon is present by being named absent.* Port **13013 is the door into that space.**
- **Spores**: 8-byte choice-echoes propagating by a second-kind ripple (salience across epochs); dormant **HAUNT charts** unlock at salience thresholds; `haunt.rs` ShadowMemory persists scars across runs, integer-only, with awareness tiers ending in *ConfusedByVowless*.

**What it becomes for the NOSTR MUD relays (hooks kept, welded to naming):**
1. **The Thirteen Doors ARE the sentinels** — not a fourteenth thirteen but the same one: bells, zone/events, endings, the Moon, the refusal, and the doors all live in the control envelope, the layer that holds what must exist without being spent. (This RESOLVES the corpus's thirteen-reconciliation question rather than adding to it.)
2. **The spore tape is the external Tombstone**: dead keepers' choice-echoes ship on the relay; new keepers inherit standing resonances — the dead's presence is their choices still rippling.
3. **BIND RELAY is the séance**: onboarding already surfaces prior keepers' marks (T16); the full form binds {SoulWord, scars, spore-tape} — the dead keeper's Shadow knows what they learned.
4. **Ghost words**: a prior keeper's remembrance-line re-solved from the field's own memory — an echo, never an NPC.
5. **The poltergeist is the griefer ghost done right**: an unresolved, unstable spore manifests as hazard or rare encounter gated on the dead keeper's scar-hash — hauntology made visible, MercyGate-bounded (it may flavor and obstruct, never permanently punish).
6. **The Hollow Astronomers know the doors** — their "forbidden corpse-routes" are EXHUME reads of the field's long-range correlations.

**Gaps:** HAUNT-chart threshold crossing has no pixel-lit witness yet (photon law); Schaeffer sound-object doctrine absent (Sean authors or drops); the fae/W-D-court overlay unbound in v3.

## The Astrakey Constellation (second wave, 3 riders, 2026-08-16)

### Leg 6 — The Identity/Matching Stack (astrakey_zkp/vc/core/lore + oracle_engine) · FOUND WHOLE — and it likely IS Leg 4's lineage

**What it is** (receipts: `F:\13forge-super\_merged\AKWEB\apps\web\src\lib\astrakey_{zkp,vc,lore}.ts`, `F:\AKWEB\apps\web\src\lib\astrakey_core.ts` + `oracle_engine.ts`; all cross-root copies byte-identical at export level — no drift):
- **astrakey_core**: birth date/time/place → ephemeris math → sun/moon/ascendant "DNA"; element archetypes (The Sovereign, The Sage, The Oracle…); shadow labels for missing elements; **resonance scoring between two DNAs** (sun ±30 / moon ±40 / asc ±30, max 100); **resilience bands** QUIESCENT → STOIC_TENSION → PRODUCTIVE_FRICTION → **FORGED_IN_VOID**; **3-word sigils** from `hash(pk1‖pk2‖session)` — "Amber. Gold. Crown."
- **astrakey_zkp**: commit-and-blind — prove resonance ≥ threshold WITHOUT revealing the raw values; optional formal reveal after trust.
- **astrakey_vc**: NOSTR **kind 20003** signed archetype claims, self-sovereign, with audit receipts.
- This is compatibility-matching machinery — a dating stack. **Leg 4's "Java Signal dating app" is almost certainly an earlier incarnation of this same lineage** [INFERRED — Sean confirms]; either way the TS stack is the living donor.

**Cross-leg confirmation:** the sigil mapping here and the relay chat's **Shared Verse** (Leg 1) are the *same mechanism* — hash → three words — designed once, found twice independently. The handshake system is one system.

**What it becomes in canon — keep the mechanism, swap the derivation (the no-zodiac revision already made this exact move narratively):**
1. **Method-DNA**: "DNA" re-derives from the ledger, not the sky — Oath Discipline distribution, mercy/debt ratios, spore salience. Identity from deed, never birth. Resonance = compatibility of *methods*.
2. **The Blind Witness**: ZKP proves you hold Name Fragments without writing the name — the technical enforcement of the Two Names rule, the Glass Witness ending's machinery, the Index Athenaeum's provenance gate.
3. **The Glass Handshake**: two keepers exchange commits; the resilience band becomes visible (Forged in Void), the underlying truths stay private. One Writer per Truth, cryptographically.
4. **Sigils = Shared Verses = 3-glyph cast-words**: one handshake grammar across chat, matching, and casting. A forged sigil drops a Tombstone.
5. **Archetype titles fold into the 15 root-titles** — earned from ledger proof alone (the VC claim IS the proof object); shadow labels ("The Wanderer") become recoverable debts, with lore still unauthored (gap).

### Leg 7 — astrakey-sigma (the arcade game) · FOUND; wrong clothes, right bones

Receipts: `…\astrakey-sigma\{App,index}.tsx`, `services/{gameLogic,audio}.ts`, `types/constants/metadata`, `ARCANUM_CONTEXT.txt.md`, PixelForge note. A privilege-escalation arcade loop (move/dash/pulse, hit-stop, boss cycles every 5th tier) in CRT neon — **aesthetic violates the locked art posture; mechanics port cleanly**: hit-stop + particle feedback → Knife-2D feel; privilege tiers → gate progression; procedural WebAudio whose pulse already sweeps 150→40Hz (the descending grief contour, accidentally canon). ARCANUM's line for the corpus margin: *"Masquerade: True Name buried, Persona is ghost."* Verdict (rider + conductor concur): **deep re-skin into the Knife lens or shelve as spinoff** — no third option; its grid has no geography, its entropy has no ledger.

### Leg 8 — v3-Live + Plans + Tooling · the working spine

Receipts: `F:\v3\crates\forge-arena-v3\src\astrakey_sieve\derivation.rs` (LIVE: HMAC-SHA256 per-system seed derivation from master primes — the deterministic derivation spine, SoulWord-adjacent); `Downloads\v9\astrakey-item-bridge.rs` (GLB validation + "Ledger-not-Bible" provenance stamping); astrakey-core-port design ("State is Law", i64 fixed-point, 8-Bit Fracture 255-caps); `F:\docs\00_INBOX\astrakey-plans-ref\` — **headless-server-buildout** (SpectatorSession + LedgerPayload flush every 60 ticks: the terraforma daemon's direct ancestor) and **combat-skeleton-entropy-narni** (frame-counted ActionState machine: Idle/Attack-phases/Dash/Parry/Stagger; entropy 1000 cap, parry bonus 80; NARNI Psych-Break already documented as "reskinned as **Edict Surge** in Ironroot").
**Tooling gift for the nine-month plan**: `palette_auditor.py` (fail-closed per-pixel palette compliance) and `codex_sanitizer.py` (LLM-output → schema validation, 8-bit bounds) are working donors for consistency gates #1 (quantize-lock) and #4 (caption round-trip) — adapt to the 32-token master palette instead of hand-rolling new tools.
New root catalogued: **F:\docs = live working documentation tree** (not tape, not quarry). `F:\AKWEB` = live AKWEB write surface. Dormant: the Godot `astrakey_forge` LRU addon (superseded by the sovereign engine).

## What This File Asks of Sean

1. Confirm the lineage read: astrakey.net's matching stack ⊇ the "Java Signal dating app" (if a distinct Java build still matters, name its path; otherwise Leg 4 closes as superseded-by-Leg-6).
2. Bless the naming welds: **wind-speech** (ephemeral chat) · **the Shared Verse as 3-glyph cast-word AND sigil** (one handshake grammar) · **the Thirteen Doors = the sentinel envelope** · **method-DNA** (resonance from deeds, never birth) · the Shear-counter line (*what fades by choice was never taken*).
3. Ordering call for when engineering resumes: terraforma substrate is furthest along; the séance system is live and needs only the relay stitch; the identity stack needs only the derivation swap; palette_auditor/codex_sanitizer adapt in an afternoon.
4. astrakey-sigma — **Sean's standing word (2026-08-16): "have to think about it… would love to incorp it after a deep alignment to this system."** Status: PARKED as a deep-alignment candidate, leaning incorporate. Not a re-skin-now task, not shelved. When it comes up, the alignment work is: ledger-physics under the loop (every action inscribes), faction call-and-response into the audio engine, era-mood geography under the grid, ascension tiers over the privilege stat, palette → the 32-token master set. The mechanics core (hit-stop, dash/pulse, 150→40Hz pulse) is already canon-shaped.
