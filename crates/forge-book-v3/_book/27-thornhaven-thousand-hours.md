# Thornhaven — The Thousand-Hour City

**Date:** 2026-08-18 · **Status:** [AUTHORED] plan over [PROVEN] corpus · **Companion:** `F:\v3\.forge\design\ROOTLESS-CCG-2026-08-18.md` (the game this city anchors)

This chapter is a HANDOFF. It exists so that no future session re-discovers Thornhaven
piecemeal. Everything inventoried here was path-verified on 2026-08-18 (Test-Path, all
true). Tags: [PROVEN] cites a live F:\v3 path; donor/tape receipts are named in prose
and stay where they are (E:\ and quarry roots are read-only tape, never write targets).

---

## 1. What Thornhaven IS

The Ironroot Thorn city — the tutorial lattice and the content anchor of ROOTLESS.
One city, constant in structure, shifting in essence across four eras:

- **Ancient** (survival — raw nature, primal craft)
- **Golden** (hope — classic MMO warmth: guild, market, guard)
- **Decay** (desperation — moral grey, collapsing infrastructure)
- **Void** (data — reality-shift, cybernetic collapse)

Receipt: the v2 questline doc (`F:\NewRepo\_vault\docs\011-thornhaven-questline.md.md`,
1.3KB — "mechanics are not lectured; they are performed"). The Era enum
{Ancient, Golden, Decay, Void} already exists as code in the MYGAMEDRAIN quarry
(`lore/spatial/terrain.rs`, lore_terrain module) and the era-variant city art already
exists rendered: golden/ancient/void variants of the market and inn
([PROVEN] `F:\v3\TODO\quarry-sort\RAMUSPRIME\assets\quantized-good\` — thornhaven_full,
market_void, market_ancient, cellar; plus [PROVEN] `F:\v3\assets\ironroot\Good\location\thornhaven_full.png`).
The art pipeline that made them is the photo-quantize GPU proof lane (v2 receipt
`F:\NewRepo\.forge\proofs\photo_quantize_gpu\thornhaven_overhead-gpu.png`).

**The buried truth (spoiler-tier, The Architect):** beneath the Forgotten Hall is a
nameless lich. He built the stone circle (Ancient), founded the city (Golden), his
death caused the Decay, his consciousness IS the Void. The four eras are not time
travel — they are layers of one mind: remembering, dreaming, decaying, digitizing.
The game happens inside his memory. Donor: notebooklm export 185-the_architect.md.md
(quarantine tape, 7.6K chars). This is the spine that makes 1000 hours ONE story
instead of four disconnected reskins — every era transition is a change of mental
state, not of calendar.

**The tone contract (2DAK revenge prompt, Desktop, 7.4KB):** earned fury driven by
personal loss; institutions complicit; no quest markers, no floating numbers;
environmental storytelling only ("those who know will know"). Constraints ruled in
that doc and binding here: no aurora imagery, no Eagle Bone Whistle, not-Americana.
Its Act-1 structure (Forest → Bandit Camp → scripted death → Spirit Run → Return
Ascended → Boss) is ALREADY the landed Act-1 weapon corpus's home
([PROVEN] `F:\v3\crates\forge-mud-v3\src\weapon_wireframes.rs` — thorngate_forest
zones are the named zones of weapons_act1.json).

## 2. What already EXISTS (the pre-paid content)

| Piece | Where | State |
|---|---|---|
| Questline frame (4-era on-ramp, level-20 Zodiac convergence) | v2 _vault docs 011 | donor prose |
| Thornbell Parish hub loop — Bellwright Forge, Market Row, Witness Rail, Toll Gate, Parish Shrine (+ more), each with trit annotation (−1 inherited debt / 0 the work / +1 the yield), era-moods, props, faction sound signatures | [PROVEN] `F:\v3\assets\ironroot\Good\ironroot-dreamer\thornhaven-terrace.md` (21KB, in-tree) | AUTHORED, rich |
| Town data (rooms/NPCs) | v2 forge-book compost thornhaven-town.data.js (19.7KB) | donor data |
| Zone geometry | thorngate_forest_01-03.json (repos-mirror + E:\v3 tape) + thornhaven_builder.gd (37KB, v3 TODO quarry — Godot, transpile-class donor like WeaponWireframes was) | donor |
| Top-down kit | thornhaven_topdown_v4_kit.svg (24KB, v3 TODO quarry) | donor |
| City soundscape generator | gen_thornhaven.rs (v2 forge-audio example, 11KB) | donor code |
| Faction roster the Parish sounds cite (Anvil Covenant hammer-ring, Toll-Saints chain-rattle, Index Monks page-flip, Widow Courts, Free Guilds) | [PROVEN] `F:\v3\crates\forge-cart-brain-v3\src\faction_mind.rs` — 8 factions, 10 actions, live | LANDED |
| Bell Pit (the arena Thornhaven's Toll Gate exits toward) | [PROVEN] `F:\v3\crates\forge-mud-v3\src\ironroot\bell_pit.rs` | LANDED |
| Era-capable terrain + 13 zone archetypes | [PROVEN] `F:\v3\crates\forge-mud-v3\src\brain\terrain_sieve.rs` | LANDED |
| The scene/choice sieve the questline runs on | [PROVEN] `F:\v3\crates\forge-mud-v3\src\ironroot\cyoa.rs` — 23 scenes, disclosure-gated | LANDED |
| Duel core for the card layer | [PROVEN] `F:\v3\crates\forge-arena-v3\src\duel.rs` — 7-7-7 | LANDED |

## 3. The thousand hours, framed honestly [AUTHORED — budget shape, numbers Estimate]

1000 hours is not 1000 hours of hand-authored script. It is:

- **~4× era passes over ONE city.** The city is authored once (Parish rooms, keep,
  stone circle, tunnels, Forgotten Hall); each era re-skins mood, prices, NPCs
  present, and which doors exist. The art proves this works — same market, three eras,
  three feelings. Authoring cost ≈ 1 city + 3 overlays, play cost ≈ 4 cities.
- **The grind-anything mint.** Every repeatable loop (fishing the 25 catches, brewing
  the 12, talent lines, taming the 23 companions, faction clocks) emits card-provenance
  events (ROOTLESS law: a replayable receipt can be a card). Grind depth is vocabulary
  × era × faction-state — combinatorial, not hand-written.
- **The CYOA lattice as quest glue.** Scenes are legality-sieved (disclosure windows +
  facts), so quest ORDER is discovered, not scripted — the same authored scenes replay
  differently per run. 23 scenes exist; the questline doc implies ~20 tutorial quests;
  the four-era convergence at level 20 reveals the Zodiac meter (Brand wheel — landed).
- **Faction clocks as content multiplier.** Witness Rail shows which NPCs survived
  prior runs; Toll Gate ends most runs; Market Row moves prices under faction pressure.
  The eight faction minds are live code — the social sim runs itself once wired.
- **Player-hosted endgame.** BellPit 3v3v3 and Rootless servers carry the hours past
  authored content (design doc, Ring 3).

Budget sketch (Estimate, ratify at ARCH000): 200h authored spine (4-era chronicle +
Architect thread + 12 zodiac bosses) · 300h systemic loops (mint/factions/crafts) ·
500h social+arena (duels, seasons, servers).

## 4. The work-queue handoff (in port order, smallest first)

1. **W-THORN1**: port Era + lore_terrain overlay from MYGAMEDRAIN quarry into mud
   (sibling of the landed terrain_sieve — the era re-skin switch).
2. **W-THORN2**: transpile thornhaven_builder.gd + thorngate_forest JSONs → MaterialGrid
   zone loads (the WeaponWireframes.gd precedent — Godot transpiles clean).
3. **W-THORN3**: Thornbell Parish rooms as cyoa scene pools (terrace md is already
   trit-annotated — the rooms are half-compiled scenes).
4. **W-THORN4**: port trig_table.rs (13 HiddenAccounts + Yod/superior_dexter — dedupe
   its twin in celestial.rs) — the level-20 Zodiac convergence needs it.
5. **W-THORN5**: gen_thornhaven.rs soundscape → forge-audio-v3 (faction timbres are
   specified in the terrace doc; the W-AUDIO1 spatial codebook port is its sibling).
6. **W-THORN6**: The Architect thread as sealed lore (visibility Forbidden until the
   convergence — the artifact codex's Visibility enum is built for exactly this).

**Vertical form (Sean 2026-08-18, concept image "tiered city cross-section"):** the city
reads as STRATA — keep and towers above, colonnaded galleries mid, undercrofts and the
Forgotten Hall beneath. The eras map to depth as much as to time: descending Thornhaven
IS descending the Architect's mind. Level design law: every era transition should be
walkable as a vertical transition somewhere in the city.

## 4a2. The scale law + the raid answer (Sean 2026-08-18, late)

**"Huge but small — like Skyrim."** Ruled as the scale law. Skyrim's trick was never
extent; Whiterun is ~20 buildings and feels like a capital. The levers, applied here:
DENSITY per cell (every Parish room already carries props/sounds/trits — the terrace
doc's granularity is the target everywhere) · LANDMARK sightlines (the keep visible
from every street; the fountain audible) · WINDING paths + gated verticality (alleys,
the cellar, the strata — you re-cross the same 80×80m and it keeps unfolding) ·
STATE multiplication (4 era dressings × faction clocks × Void layer = one small city
with more distinct place-states than a 10× map). Never answer "too small" with more
metres; answer with more MEANING per metre.

**The zone-events corpus is a generator, not a list.** 13moons zone_events.json
(200 events, 5 zones × 40; copied tape→live at `assets/ironroot/events/zone_events.json`
[PROVEN]) has the right schema — zone / entity / trigger / health_range / narrative /
sound_cue / gameplay_effect / severity(whisper→presence→encounter→crisis). 200 authored
events are too small for 1000 hours AND exactly big enough as PRIORS: the seeded
generator (the WordTable pattern, determinism=identity) composes events from
vocabulary × trigger × era × severity — authored small, played huge. Same law as the
packs: sell entropy, replay content.

**The Void IS the group-and-raid zone.** The live tree already holds the raid tier:
[PROVEN] `F:\v3\crates\forge-reactions-v3\src\world_boss_data.rs` — **13 faction world
bosses** (the 12+1 again), each with FOUR solution paths where Combat is listed LAST
(The Receipt Eater falls to Provenance; The Anchor That Sang falls to Music; the
Coin Whale to Fishing) and a true/echo/broken relic ladder. Boss 0, Iron Bailiff
Dornsbane, zone-biases THORNHAVEN itself; boss 12, The Crowned Spawn That Was Not
Spawned, lives in "unspawned zones" — a Void-native boss with no echo and no broken
relic. The raid trigger is also landed: `Tithe::should_manifest_branded()` fires at
debt 255 — the world's anger literally summons the encounter. Arena donor:
`F:\docs\00_INBOX\irontoask\world\parallax_boss_arena.tscn` (verified, transpile-class).
So the Void layer = corpse-runs + PvP + the 13-boss raid roster, and a raid can be
won by fishing, song, or refusal — EverQuest's stakes with this tree's verbs.

## 4a3. HOW BIG — the zone ceiling and the 5D mandala (Sean 2026-08-18, closing ruling:
"the zone was too small. Denser. How big CAN we go, how big SHOULD we go — and the
mandala pattern that forced players out in a loop, in 5D")

**CAN (hard word ceilings, from landed types):** MortonKey5D carries 12 bits/axis =
**4096 cells/axis = 2.05 km per side** at the 500 mm cell — the absolute per-zone-word
ceiling (16.7M cells/plane; at 8-byte EcologyPCM8 texels that is 134 MB/plane — needs
the ZoneStreamManager LRU pattern, donor verified on tape). Morton8's compact form
caps at 1024 cells = 512 m.

**SHOULD (trit-native, Estimate — ratify):** a petal zone = **3^7 = 2187 cells/axis
≈ 1.09 km per side**; the city stays the 160×160 DENSITY ISLAND inside its petal
(the Skyrim law: the city is dense, the zone is big). World = **12 petal zones + the
center = 13** (the motif, again): ~14 km² of zone landmass — Skyrim-class extent with
EQ-zone structure, and every petal is one 60-bit Morton word with room to spare.
GenreBounds Open3D row updates 729 → 2187 extent when ratified (one const + test).

**THE 5D MANDALA LAW [AUTHORED — and the arithmetic makes it beautiful]:** the wheel
forces the loop by number theory, not by walls:
- The world is the zodiac wheel: 12 petal zones around the center city (petal i =
  Brand i; its trigon element sets the biome).
- **Exits rotate by +5 — the QUINCUNX step.** 5 is coprime with 12, so stepping +5
  from the center visits ALL TWELVE petals exactly once before any repeat — the
  mandala's forced outward circulation is a property of 5 mod 12, unbreakable and
  unwikiable. (The quincunx is the Yod's own angle — the wheel walks its
  finger-of-god path.)
- **The pump is landed:** the novelty sieve (visited = −1000, cyoa scoring) applied
  at zone scale — re-entering a completed petal scores below every fresh one; the
  center always ejects you along the +5.
- **In 5D the loop is a HELIX:** completing a full lap (12 petals) advances T one
  era-tick and S one disclosure stratum — same petals, higher floor. The mandala
  never repeats because the fifth axis climbs: you return to petal 1 in a world one
  stratum deeper. Descending the Architect's mind (§4b vertical law) and lapping the
  wheel are the same motion seen from two axes.
- Center = Thornhaven; dead-center composition lawful here alone (shot bible).

## 4b. The geometry spec (recovered 2026-08-18)

The dirge design-bible holds the full buildable plan (tape:
`F:\13forge-super\_merged\reposold\dirge-of-ironroot\design-bible\05-thornhaven-zone-geometry.md`
— read whole, digested here so the tape need not be reopened):

- **Plan:** 80×80 m cobblestone ground; 4 m stone walls; north gate (10 m archway) with
  two guard towers; central courtyard (fountain + 4 torch posts); market district EAST
  (4 stalls, crates, awnings); Hearthfire Inn WEST (12×8×5 m, hearth, cellar stairs);
  residential SOUTH (clustered small buildings, 2 m alleys, dead ends "for
  Chronothieves").
- **Light law:** golden-hour directional (1.0,0.9,0.7 @ 0.7, 30°), 8-12 warm torches
  (1.0,0.7,0.3 @ 1.2, range 8-10 m), warm fog 0.01.
- **Material words:** stone .6/.55/.45 r.9 · cobble .45/.4/.35 r.95 · wood .5/.35/.2
  r.85 · metal .3/.3/.3 m.7 r.4 — four materials build the whole city.
- **v3 translation:** 80 m at the landed MaterialGrid's 500 mm/cell = a 160×160-cell
  zone (current GRID_W×H is 128×64 — W-THORN2 either tiles 2 grids or ratifies a city
  grid size). Style lock: `F:\v3\TODO\ironroot-edict\game\assets\style_lock\thornhaven_isometric.png` [PROVEN].
- The reference-art set the bible names (courtyard/inn/gate/tiered-cutaway/isometric/
  overhead/full-map) is the SAME seven images now spread across RAMUSPRIME
  quantized-good + assets\ironroot\Good + the style_lock dir — the art debt is paid.

## 4b2. THE FLOW SHEETS — canon maps, live in-tree (Sean 2026-08-18: "it needs movement, flow")

The definitive MVP maps already exist, [PROVEN] under `F:\v3\web\13forge.com\assets\maps\`:
- **Thornbell Parish** (ironroot-mvp-2d-map-thornbell-parish.q.png + town-hub blueprint
  variant): hub top-down with the CORE TOWN LOOP drawn as flow — 1 return from
  arena/dungeon → 2 identify materials → 3 craft/trade → 4 ONE social/economic choice →
  5 advance faction clocks → 6 prepare next run. Factions on-sheet: Ledger Church,
  Toll-Saints, Free Guilds, Index Monks. NAMED NPC ROSTER: Master Edda Bellwright
  (smith), Toll-Sister Vey (gatewarden), Scribe Iven, Mara of the Free Guilds,
  Index Clerk Oth, the Silent Beggar (rumor & omens).
- **The Bell Pit** (arena blueprint + strategy map): ringed arena, Bell Core center,
  Witness Balcony, Root Cracks hazards, Boss Seal; FOUR ARENA MODES (Knife 2D, Root 3D,
  Ledger Tactical, Spirit Death — the Void death-mode is on the sheet); 5-wave sequence
  ending in the Bell Warden; ARENA LOOP: enter → wave → bell emits pressure →
  fight/parry/REFUSE → event writes to LEDGER → between-wave choice → SIEVE modifies
  next wave → boss manifests. The drawn loop runs on the landed ledger + sieve.
- **The Under-Orchard** (dungeon blueprint + 2D map): five branches off a hub —
  Root Cellar (combat), Toll Drain (diplomacy/theft), Bell Vein (crafting/extraction),
  Spirit Fold (DEATH ROUTE), Grave-Mine — plus the Locked Deep Door (beyond current
  access: the descent toward the Forgotten Hall). Material families Bell Bronze /
  Bell Diamond / Grave Iron; provenance sources on-sheet (Blood/Stolen/Grave/Pure/
  Reclaimed = the landed itemforge Provenance enum, drawn).
- **The Ironroot Cathedral-Fortress** (2 vertical-profile sheets): the strata as
  engineering drawings — 224 m of spires to rock-anchor vaults, flying-buttress load
  paths, masonry master-marks, and AD QUADRATUM proportion studies in the margin
  (the composition law, already on the blueprints).
- Companion art in `web/13forge.com/assets/`: the 12 zodiac knight concepts (the
  Brand roster's faces), the Selvarya Moon Oracle sheet (card-frame prototype for the
  thinker/oracle cards), the promo ("The tick counts the time. The moon gates the
  word. The hash holds the weight. No server in the sky.").

**THE FLOW LAW (ruled by the sheets themselves):** every space ships with its loop
strip — movement is authored as a LOOP, not a floorplan. A district without a drawn
player-flow is not done. Town 6 steps · arena 8 · dungeon 6; the loops chain
(town → pit → orchard → town) into the run cycle the terrace doc described in prose.

## 4c. The formula and the production law (Sean 2026-08-18)

**"Stardew meets Pathfinder meets Stargate meets MTG meets LoL — Sovereign."**
Decoded against landed systems: Stardew = the Parish daily loop + era warmth ·
Pathfinder = the 8-register/Brand/subclass depth · Stargate = era/vertical transitions
as portals through one place (the Architect's strata; Link primitive) · MTG = the
ROOTLESS card layer (7-7-7 duels, $1.25 seed packs) · LoL = BellPit 3v3v3 seasons on
Rootless servers · Sovereign = zero foreign engine, every layer this tree's own law.

**Production law: ONE city, FOUR dressings.** Author Thornhaven's geometry once (the
80×80 bible plan); each era is a dressing pass — palette/material swap (four material
words make this cheap), light law swap (golden hour IS the Golden era; Void relights
the same stones), prop/NPC roster swap, door topology deltas. Whether a dressing bakes
to a mesh or stays a texel overlay is a W-THORN2 implementation choice — the LAW is
that geometry is authored exactly once. Mesh remains an export codec (session doctrine):
derivable things are queried, authored things are keyframed, triangles only on the way
out the door.

## 4c2. THE FIRST TWENTY HOURS — the hook (Sean 2026-08-18: "no PvP till 20-25h;
those first 20 are the ones that hook ya")

The gate was already authored: the questline doc's **level-20 Convergence** ("the ledger
reveals the Zodiac resonance initiated by all player choices") IS the PvP unlock. You
don't reach PvP by grinding TO it; you reach it by becoming legible to the ledger.
The hook ladder, every rung on landed or donor material [AUTHORED plan]:

- **H0-1 · The cold open.** Birth screen: one star, one polarity, one moon, a name —
  NO class picker (Birth is cut from the ledger chain, never rerolled). Monochrome
  world. The Forest: walk, quiet tension, something wrong you can't name. First
  weapon: The Hearthstone Oath — "it remembers what you forgot."
- **H1-3 · The scripted death.** Bandit Camp overwhelms you. You DIE — scripted.
  Solo Spirit Run through the mirrored forest (death is traversal, not failure).
  Return Ascended, wreck what killed you, boss payoff. THE hook beat.
- **H3-6 · The Parish opens.** The town loop runs: return → identify → craft/trade →
  ONE social choice → faction clocks → prepare. First brew, first catch, first
  companion. Witness Rail shows a name that wasn't there before: the world remembers.
- **H6-10 · The Pit and the Orchard.** Bell Pit waves 1-5 (the drawn arena loop:
  bell pressure, parry/REFUSE, ledger writes, sieve modifies the next wave), first
  Bell Warden. Under-Orchard branches open — the player chooses an identity by
  PLAYSTYLE (combat / diplomacy / crafting / death-route), not a menu.
- **H10-15 · The colour tease.** First Chronicle era-transition: ONE fully saturated
  Golden scene, then back to grey. The colour is a promise now. School pick
  (subclass lands), talents, disclosure-gated arch scenes begin firing.
- **H15-20 · The audacity beat.** First world-boss ATTEMPT: Iron Bailiff Dornsbane
  biases Thornhaven, and his solution paths include Diplomacy / Provenance / REFUSAL —
  a level-10 character can face a world boss and win WITHOUT fighting. The EQ awe
  moment, by verb instead of level.
- **H20-25 · THE CONVERGENCE.** The ledger reads every choice made since birth and
  reveals the Zodiac resonance — the Brand confirms, colour FLOODS the character
  (first permanent colour in the monochrome world), and the Void opens: the shared
  death-layer, raids, PvP. The gate is the payoff: PvP arrives as a revelation
  earned, never a menu unlocked.

Hook mechanics carrying the 20 hours: visible colour restoration as the world's
progress bar · death-as-traversal · provenance on every item · one social choice per
loop (scarcity makes it weigh) · the Witness Rail replaying your own history.

## 4c2b. NUMBERS — player time & assets per 5-level band [all ESTIMATE, ARCH000 ratifies]

Pacing anchor: 20-25h → level-20 Convergence (ruled) ⇒ **~5-6 play-hours per 5-level
band**, 4 bands to the gate. Presentation ruling (Sean): **2D top-down for pets and
cards** (3D stays the deferred export lane). Art pipeline = the PROVEN photo-quantize
GPU lane (the Thornhaven era set came from it) + sprite_blob + the 20-anchor rig.

| Band (lvl) | Hours | Backgrounds/rooms | Scenes | Weapons | Mobs/NPCs | Pets intro | Boss |
|---|---|---|---|---|---|---|---|
| 1-5 · Forest+Parish | 5-6 | 6 (forest, camp, spirit-mirror, streets, forge, inn) — 4 EXIST as art | 8 | tier-0 ×3 LANDED | 6 named NPCs LANDED (sheet) + 5 mob kinds | 5 | camp boss |
| 6-10 · Pit+Orchard | 5-6 | 6 (pit + 5 branches) — 2 EXIST | 6 | tier-1 ×2 LANDED | +4 warden variants LANDED | +6 | Bell Warden |
| 11-15 · Golden tease | 5-6 | 3 new (thorngate 01-03 JSONs EXIST) + city RE-DRESS (palette pass, ~free) | 6 | tier-2 ×2 LANDED | +6 | +6 | faction boss |
| 16-20 · Convergence | 5-6 | 4 (keep, tomb, Forgotten Hall, cathedral vertical) | 6 | tier-3 ×1 LANDED | +4 | +6 | Dornsbane attempt + Convergence |
| **To gate** | **20-25** | **~19 unique (≈9 exist)** | **~26 (23 landed + bard 3)** | **8 (all landed)** | **~25 kinds** | **23 (data landed)** | **4+1** |

NOTE: the Act-1 corpus's level_req 0-4 = TIER steps, mapped one tier per band — no
new weapons owed before the gate. The 200-event ambient corpus covers ~10 fired
events/band without repeats.

**Asset-hour budget to the Convergence** (solo + gen pipeline; per-unit rates from the
proven quantize lane): backgrounds 10 new ×~1h = 10h · pet sprites 23 × 1.5h (top-down
sheet, idle+walk, 2 facings mirrored) = 35h · card frame 2h once (Selvarya sheet is the
prototype) + Ring-1 deck ~60 cards ×0.5h = 32h · mob/NPC sprites ~20 ×1.5h = 30h ·
weapon integration (P02 art exists) 4h · UI faces (duel glass, parish loop) 20h ·
audio (gen_thornhaven donor + the 4 dirge .dsp) 12h → **~145 asset-hours to a
complete first 20 player-hours.** The multiplier stays honest: era re-dress ≈ palette
+ roster swap, so bands 21+ reuse the same city at ~15% of band-1 cost.

## 4c3. THE DEATH CARD — the card mechanic (Sean 2026-08-18: "these are the card
mechanic fo sho")

Landed types, [PROVEN] `F:\v3\crates\forge-cart-brain-v3\src\run_dev_run.rs`:
- **DeathScar** (:95) — scar_hash (the card's deterministic id), player_hash,
  zone/room/position_mm, cycle_id, cause, killer_hash, **weapon_hash**. A death
  already carries full card provenance: who died, where to the millimetre, what
  killed them, with which weapon, on which attempt.
- **DeathReplay** (:531) — replay_hash + the captured tick ring
  (capture_death_replay off the ReplayRecorder, cold-path alloc only). THE CARD
  CONTAINS THE DEATH: playing or witnessing it replays the actual final seconds,
  deterministically. The pack-seed law applied to mortality.
- **DeathCause ×6** (:78) — Combat, Erasure, Fall, Hazard, **Sacrifice**,
  **Refusal** — the six suits of the death deck. A Refusal death is a card.
- **DeathContext** (:510) — killer name, killing-blow damage/direction/aspect,
  what YOU were doing when it landed, the tick. The card's flavor text writes itself
  from telemetry.

THE MECHANIC [AUTHORED, proposed]: dying in the Void MINTS your DeathScar as a card
as you cross. Duel verbs by suit: a Sacrifice scar spends itself to shield an ally
(the death argues its idea, thinker-card law); a Refusal scar cancels an action; a
Combat scar carries killer_hash — played against that entity it is a REVENGE card
with teeth. PvP: killing a player writes YOUR hash onto THEIR card — every death card
is also a wanted poster, tradeable over the relay like any signed event. Witness Rail
displays the parish's scars; the 2DAK Shadow already learns from the same telemetry
(blow direction/aspect) — your Shadow and your death deck are the same data.
L05 NOTE: DeathCause has a twin (run_dev_run.rs:78 vs combat/scar.rs:12, same six
variants, different order) — dedupe owed, welder card T6.

## 4c4. THE UNWIKIABLE BOSS LAW (Sean 2026-08-18: "you can't look up the boss strat —
you just need to have played long enough; not frustrating — 'I wanna raid with those
guys'")

Emergent ≠ random. Every fight stays deterministic — but over inputs no wiki can
know. Six clauses, each on a landed hinge:

1. **The boss is chosen by how you played.** boss_sieve already selects the Bell
   Warden variant from RunProfile (triggers like high_aggression_or_blood_supply —
   the 777 cascade's "missing numeric thresholds" become per-world seeded FUNCTIONS,
   not global constants). A wiki describes someone else's boss.
2. **Solution-path weights are per-world.** Each Rootless seeds its Dornsbane's
   Diplomacy/Provenance/Refusal/Combat weights from world_seed × faction clocks ×
   era. YOUR server's Receipt Eater is not their server's.
3. **Weaknesses are sensory, never numeric.** The tell is heard (Vibration law
   dissonance — resonance_delta made audible), noticed by a companion (umwelt
   Senses — "a companion notices what you cannot" is aspire-canon), or seen as the
   colour law flickering. Learnable by attention; unwritable as a number.
4. **The boss reads your scars.** DeathContext/Shadow telemetry (blow direction ×
   aspect) feeds the encounter — it counters YOUR habits. Practicing solo makes it
   better at beating YOU; a party of DIFFERENT profiles beats it. That is the
   "I wanna raid with those guys" engine: diversity is the strat.
5. **The meta lives in-world.** Witness Rail records how attempts resolved; Index
   Monks sell rumor; veterans' scars are readable. The server's knowledge economy IS
   the wiki, and it's social, local, and part of the game.
6. **The anti-frustration floor is the laban gates, generalized** — already landed
   as typed predicates (forge-geo-v3::laban): anticipation READABLE before every
   punish, recovery FAIR by weight, input feel never eaten. Plus a test-assertable
   fairness clause: every variant keeps ≥1 viable path per playstyle class
   (the Under-Orchard's four branches define the classes).

One sentence for the book's spine: **the strat isn't hidden, it's personal — and the
only place to look it up is other people.**

## 4d. Every Arch is a campaign; every Era is a game (Sean 2026-08-18)

**Arch campaigns.** Each cyoa Arch is a 20-25 hour story campaign (Sean's sizing:
Goblin 20-25h, Bard 20-25h). Seven arches are landed as scene-pool skeletons
([PROVEN] `F:\v3\crates\forge-mud-v3\src\ironroot\cyoa.rs` — glass, river, fae,
goblin, umwelt, dirge, blind); the BARD arch is the natural eighth (School of the
Bell + Sing archetype + the 15 instruments — its vocabulary is landed, its scenes are
not). Eight arches × ~22h ≈ **175h of arch campaigns** before era multiplication.

**The era realization: 4 Eras = 4 COMPLETELY DIFFERENT GAMES on one city.** The
dressing pass (§4c) was underselling it — each era swaps the RULESET, not just the
palette. Sean's four, with proposed era mapping (Sean rules the final pairing —
"Fae??" left open by him):

| Era | Game | Why it fits |
|---|---|---|
| Ancient | **Fae** (proposed) | The Fae World Overlay's own law: "human civilization is powered by restrained fae life" — the deep past is where the fae are unrestrained; raw nature, bargains, guest-law (chapter 18 is this game's rulebook) |
| Golden | **Game of Thorns** | Court intrigue at classic-MMO warmth: Widow Courts, Toll-Saints, Index Monks, guild/market politics — the faction minds ARE the players |
| Decay | **Ironroot** | The dirge core: grief, tithe, brand corruption, the bell — the game the lineage is named for |
| Void | **ShadowRun** | Cybernetic collapse — the data era; the card-hologram concept art already wears this skin; CDK terminal aesthetics native here |

Same streets, four genres: folklore-horror bargaining · political intrigue ·
grief-metal action · cyberpunk heist. The Architect spine (§1) is what makes four
games one story — each game is one layer of his mind, and finishing one era's game
recontextualizes the other three.

**RELEASE ORDER (ruled Sean 2026-08-18, distinct from era order):**
1. **IRONROOT is the first game** — monochromatic blacks and whites; colour returns
   per-thing as ledger facts, and when the colour finally comes, it's beautiful. The
   grief game ships first; its ending is the world regaining saturation.
2. **GOBLIN BARD second** — the two arches fused into one campaign: after the dirge,
   the first music is a goblin playing wreckage (the ScrapFiddle is already landed as
   its instrument). Game 1 restores colour; Game 2 restores SOUND. Sean left the
   post-Ironroot slot open ("or whatever happens after Ironroot in your world") —
   the engine's answer: mourning, then music. Goblin Bard is confirmed right.
3. Game of Thorns (the rebuilt Golden's politics) — with the Fae truth threaded
   beneath as the Architect's deepest layer.

**AMENDED (Sean 2026-08-18, later): THREE living games, and the Void is not a game —
the Void is WHERE YOU GO WHEN YOU DIE.** The fourth era becomes the death-layer that
threads the other three, and it is the PvP space. Synthesis against landed parts
[AUTHORED, mechanics proposed for ARCH000]:
- Dying in any era drops you into the VOID LAYER of the same city — the 2DAK Spirit
  Realm made canon ("some areas are ONLY accessible by dying in the right spot" —
  Correspondence made literal). Corpse-run to return; death is a mechanic, not a
  failure screen.
- The Void is where the SHADOW lives: `shadow_counterpart.rs` (landed, forge-mud-v3)
  already tracks your 8 attack directions + 8 aspects — your Shadow is your recorded
  pattern. Void PvP is players and player-Shadows hunting each other in the
  ShadowRun skin.
- THE VOID IS ONE PLACE ACROSS ALL SERVERS [proposed]: every Rootless hosts its own
  three living eras, but the Void is the Architect's single consciousness — so all
  servers' Voids are THE SAME shared layer. Inter-server PvP happens in death;
  ephemeral Mutate5D gossip (kind 21313) is its wire. You meet players from other
  worlds only by dying — which is exactly what the lore says the Void is.
- Economy note: what you carry into the Void risks Tithe (landed accrual law) —
  the EQ corpse-run tension, sovereignized.

**The corrected math:** 8 arch campaigns (~175h) × era variations + 4 era-games
(each a distinct ruleset over the one city) + systemic loops (§3) + social/arena
endgame = the thousand hours stops being a goal and becomes a sum. Production stays
1-city law: four games share one geometry, one codex, one ledger.

## 5. Drift anchors

Phrase for the seed guard: **thousand-hour city**. Live-path [PROVEN] cites above are
the drift surface; donor/tape paths are prose receipts by design and carry no tag.
