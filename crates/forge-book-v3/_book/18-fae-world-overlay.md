<!--
PROVENANCE
  Source: RAMUSPRIME/docs-specs/04-game-design-lore/spec-fae-overlay-2026-05-16.md
  Folded: 2026-07-01
  CITATION CORRECTED 2026-08-26 (/driver, arch021-retag-pass item 1). The line
  above read `E:/airgap/2026-05-17-dsp-hrtf-p00-loop/spec-fae-overlay-2026-05-16.md`.
  Three things were wrong with it and the aspire row only caught the first:
    1. Root: the row says fix `E:\airgap` -> `E:\.airgap`. Directionally right —
       `2026-05-17-dsp-hrtf-p00-loop` lives under `.airgap`, not `airgap`. BOTH
       roots exist and are DIFFERENT trees (6 children vs 452, neither a
       symlink), so this was never a typo.
    2. But the file is under NEITHER root. RECEIPT(claim:"spec-fae-overlay-2026-05-16.md
       exists under an E: airgap root",verdict:ABSENT,roots:[E:\airgap,E:\.airgap],
       anchor:"recursive filter spec-fae* over both = 0 matches 2026-08-26").
       Applying the row's fix alone would have turned an obviously-wrong root
       into a plausible-looking DEAD citation — harder to spot, not better.
    3. The source was in-repo the whole time, at the path now cited above
       (also copied in F:\NewRepo\_vault\output\specs\).
  So the ARCH-008 "canonical 2-copy state (1 live SoT copy here + 1 airgap
  original)" claim does not hold for this chapter: there is no airgap original.
  The two copies are this fold and the in-repo spec. Nothing was quarantined —
  that gate stays HITL-only (Sean) per the Outside-SoT Law and was not touched.
-->

# Fae World Overlay — Mid-Game Folklore Pressure Layer (12+1)

**Version:** fae_world_overlay_12_plus_1_v1
**Date:** 2026-05-16
**Status:** CANONICAL — Locked
**Depends on:** spec-world-consequence-engine-2026-05-16.md, Law Layer (same session)

---

## 1. Layer Role

Mid-game folklore pressure layer introduced after hour 20. Separate from account raids and faction world bosses.

Fae are not early-game monsters. They are ecological, legal, mnemonic, and bargain-based world actors that expose how human factions are damaging older lands.

**Late reveal:** Human civilization is not merely built on fae land. It is powered by restrained fae life.

---

## 2. Layer Placement (Cross-Layer Priority, high→low)

```
1. Weaver_Crown_OutsideWheel
2. one_shot_attempt_result
3. prevented_erasure
4. faction_world_boss_resolution
5. fae_boss_or_fae_quest_resolution    ← THIS LAYER
6. raid_first_clear
7. raid_echo_clear
8. major_faction_reform_or_collapse
9. local_quest_state
10. vendor_stock_state
11. ambient_omen_state
12. rumor_state
```

---

## 3. SolutionPathKind (17 variants, canonical order)

```rust
pub enum SolutionPathKind {
    Combat,
    Crafting,
    Hunting,
    Fishing,
    Music,
    Survival,
    DataMining,
    Diplomacy,
    Trade,
    Stealth,
    Sabotage,
    Provenance,
    WitnessBuilding,
    Ritual,
    Refusal,
    Ecology,
    RouteMastery,
}
```

### Skill Rename: Forensics → Provenance

**Tagline:** "Read where a thing came from, who owned it, what touched it, and what lies about it."

Covers: artifact origin, shimmer detection, stolen fae gifts, false receipts, Void patch truth, witness-chain inspection, relic ownership ethics.

---

## 4. Music Path — Voice & Resonance System

Music is not only audio. It is resonance instruction applied to bodies, water, colour, memory, and route-state.

| Stat Link | Maps To |
|-----------|---------|
| pitch | Resonance |
| rhythm | Momentum |
| harmony | Clarity |
| dissonance | Guilt |
| timbre | Tarnish |
| volume_pressure | Vigor |
| colour_chroma | LogicDepth |

**Fae voice tags (8):** lure, warning, grief, bargain, guest_right, glamour, debt_song, route_song

**Uses:** siren songs, fae bargains, Synthesia proc-gen voices, resonance puzzles, colour-to-stat translation, warning/lure/grief distinction.

---

## 5. Three-Formula Architecture

### Universal World Boss Formula (9+chaos)

Drives faction apex encounters only.

```
faction_pressure_q           0..10000
reputation_delta_q          -10000..10000
crime_pressure_q             0..10000
ecology_pressure_q           0..10000
economy_pressure_q           0..10000
raid_echo_pressure_q         0..10000
erasure_pressure_q           0..10000
artifact_provenance_pressure_q  0..10000
unique_trigger_flags         bitset
chaos_perturb_q             -1500..1500
```

### Fae Layer Formula (6 shared + 5 fae-specific)

Drives fae boss selection and fae quest activation.

**Shared from world:**
- faction_pressure_q
- ecology_pressure_q
- economy_pressure_q
- artifact_provenance_pressure_q
- unique_trigger_flags
- chaos_perturb_q

**Fae-specific:**
- obligation_pressure_q (0..10000)
- fae_exploitation_q (0..10000)
- consent_integrity_q (0..10000)
- replacement_quality_q (0..10000)
- source_suffering_q (0..10000)

### Living Substrate Crafting Formula (5 fae + 2 shared)

Drives ethical crafting gates.

- obligation_pressure_q
- fae_exploitation_q
- consent_integrity_q
- replacement_quality_q
- source_suffering_q
- artifact_provenance_pressure_q
- ecology_pressure_q

### Weaver Crown Formula (reads all outputs)

Override / temptation / OutsideWheel pressure model.

Reads: universal world boss outputs, fae layer outputs, living substrate crafting outputs, relic_ownership_mode, refusal_count, claimed_unique_relic_count, surrendered_or_buried_relic_count.

---

## 6. Obligation Pressure

**Field:** `obligation_pressure_q: 0..10000`
**Distinct from faction_pressure.** Faction pressure asks "who has power?" Obligation asks "what is owed?"

Affected systems: fae gifts, bargains, siren songs, guest-law, selkie coats, Murkveil gift laundering, Weaver Crown temptation.

---

## 7. Spawn Rules

| Rule | Value |
|------|-------|
| Intro window | Hour 20+ |
| All spawn in one playthrough | No |
| Max fae bosses per playthrough | 3 |
| Max fae quests per playthrough | 5 |
| Secret +1 requires | 2 fae quests resolved + 1 fae boss prevented/spared/witnessed |
| Selection method | world_seed + faction_pressure + ecology_pressure + player_solution_bias |
| No-repeat rule | Each fae boss is one-shot per root-cycle |
| Visibility | Folklore first, tracks second, bargain third, boss last |

### Selection Weights

| Condition | Weight |
|-----------|--------|
| High faction pressure | +30% |
| High ecology pressure | +25% |
| Player overuses trade/profit | +20% bargain-fae |
| Player overhunts | +35% hunt-fae |
| Player overfishes | +35% tide-fae |
| Player uses refusal | +20% secret benign fae |
| Player claims many relics | +30% hostile secret fae |
| Void leak active | +25% glamour/code fae |

### Mutual Exclusions

| Group | Members | Max/Playthrough |
|-------|---------|-----------------|
| water_fae | pearl_masked_selkie, siren_who_forgot_hunger, baptismal_hag | 1 |
| route_fae | walking_milestone, hare_with_twelve_shadows, cartwheel_king | 1 |
| grave_refusal_fae | mourning_briar, child_who_unwove_crowns, baptismal_hag | 2 |
| industrial_fire_fae | shift_whistle_dryad, hearth_that_marched | 1 |

### Non-Spawn Outcomes

- Rumor only
- Quest without boss
- Boss prevented before manifestation
- Faction suppresses folklore
- Fae leaves a gift but no encounter
- Crown converts omitted fae into later temptation

---

## 8. Global Fae Rules

- Not standard fantasy
- Not simple good or evil
- Faction interaction required
- Combat optional for most
- Unique quest reward per faction
- Reward should affect world

### Fae Item Ethics

| Action | Effect |
|--------|--------|
| Claimed | Increases human ownership pressure |
| Bargained | Increases obligation pressure |
| Gifted | Reduces Crown temptation |
| Stolen | Becomes shimmer-detectable |
| Refused | May heal ecology or open Vowless routes |

---

## 9. Faction Fae Registry (12+1)

### 1. Thornguard — The Walking Milestone

- **Court:** The Milestone Court
- **Type:** road-fae / boundary judge
- **Moral band:** stern_good_to_mischievous
- **Zone bias:** Thornhaven roads, Ironmoor causeways, old border stones
- **Folklore hint:** A stone marker changes distance depending on whether the traveler lied that day.
- **Faction interaction:** Thornguard patrols arresting people for trespass on roads that were never human roads.
- **Quest:** "Where the Road Was Promised" — road warrant claims fae path as crown road, villagers vanish following official signs.
- **Solutions:** diplomacy (oldest oath), provenance (chisel marks), route_mastery (walk original road), combat (if judging innocents), refusal (remove all signs)
- **Reward:** Milestone Without a King (relic) — reveals disputed routes, marks as fae/lawful/stolen/unowned. Risk: claiming as property angers the Court.

### 2. Murkveil — The Purse That Laughed

- **Court:** The Underleaf Exchange
- **Type:** bargain-fae / gift eater
- **Moral band:** mischievous_to_evil
- **Zone bias:** Murkveil Depths, canal markets, black-market shrines
- **Folklore hint:** A purse laughs when a gift is priced.
- **Faction interaction:** Murkveil fences laundering fae gifts as stolen goods, turning gifts into curses.
- **Quest:** "Never Sell a Gift" — child's fae-gift fenced, every resale doubles curse and erases giver from memory.
- **Solutions:** stealth (steal back without coin), diplomacy (obligation not inventory), trade (zero-price chain), provenance (shimmer reveals giver), combat (if eating names)
- **Reward:** Unpriced Purse (belt) — carries one gifted item without trade conversion. Risk: profit use makes it a curse container.

### 3. Ledger Church — The Baptismal Hag

- **Court:** The Well-Mothers
- **Type:** well-fae / grief midwife
- **Moral band:** good_to_lich_grade_evil_depending_on_pollution
- **Zone bias:** chapel wells, Grey Orchard, burial springs
- **Folklore hint:** A well repeats the name of someone who was never baptized.
- **Faction interaction:** Ledger Church baptizing old fae wells as debt fonts.
- **Quest:** "Water Does Not Owe" — debt rites poisoning a fae well that once washed grief free.
- **Solutions:** ritual (no-payment washing), crafting (debtless basin), diplomacy (surrender well), fishing (silver blindfish), combat (if drowning mourners)
- **Reward:** Debtless Well-Cup (relic) — cancels one grief-to-debt conversion. Risk: if sold, fills with black water.

### 4. Senex Convocation — The Shift-Whistle Dryad

- **Court:** The Root-Tithe Choir
- **Type:** industrialized dryad / labor-bound root spirit
- **Moral band:** tragic_evil
- **Zone bias:** Crucible Yards, Meridian Scar, worker dormitories
- **Folklore hint:** A tree grows iron leaves every time the shift bell rings.
- **Faction interaction:** Senex cut root-oaths into labor contracts, binding fae to industrial schedules.
- **Quest:** "The Tree That Punched In" — dryad forced to keep worker time, roots drag exhausted laborers back.
- **Solutions:** crafting (rest whistle), diplomacy (worker witnesses), sabotage (break time-gear), combat (sever iron leaves not root), refusal (refuse forced-labor materials)
- **Reward:** Rest-Whistle (tool) — pauses one labor/pressure chain before harm. Risk: optimizing production corrupts it.

### 5. Free Graves — The Mourning Briar

- **Court:** The Moss-Wake
- **Type:** grave-fae / thorn mourner
- **Moral band:** good_but_dangerous
- **Zone bias:** Grey Orchard, Stillborn Crypt, old battlefield graves
- **Folklore hint:** A thorn bush grows flowers only on graves with no owner.
- **Faction interaction:** Free Graves protect burial places where fae and human dead overlap.
- **Quest:** "Do Not Prune the Grave" — living briar protects unowned graves, expansion calls it infestation.
- **Solutions:** ecology (grave-moss balance), ritual (non-faction markers), crafting (thorn-safe gloves), combat (dead limbs during sleep), refusal (refuse to make grave useful)
- **Reward:** Thorn That Refused the Spade (relic) — protects one grave/witness/memory from faction claiming. Risk: blocking rightful mourning wounds with guilt.

### 6. Ironmoor Compact — The Pearl-Masked Selkie

- **Court:** The Brine Masquerade
- **Type:** selkie / tide-bargain fae
- **Moral band:** mischievous_to_good
- **Zone bias:** Ironmoor Docks, Shattered Reach, silver tide pools
- **Folklore hint:** A seal asks for its coat back in the voice of a merchant's dead wife.
- **Faction interaction:** Ironmoor traders overfished a fae tide that exchanged memory for safe passage.
- **Quest:** "The Tide Wore a Face" — stolen selkie coats as collateral, tide refuses honest fishers.
- **Solutions:** fishing (coat's reflection at silver tide), trade (collapse collateral market), stealth (steal from dock vault), diplomacy (negotiate all parties), combat (if drowning debtors)
- **Reward:** Selkie Coat Button (trinket) — breathe in tide/bargain-bound space briefly. Risk: wearing too long = borrowing skin.

### 7. Duskweald Hunt-Kin — The Hare with Twelve Shadows

- **Court:** The Antlered Unseen
- **Type:** trickster prey-fae / route teacher
- **Moral band:** mischievous_good
- **Zone bias:** Duskweald, Briarhollow, wrong-road clearings
- **Folklore hint:** A hare leaves wolf tracks when chased and child footprints when spared.
- **Faction interaction:** Hunt-Kin forgot the old rule: never kill the animal that teaches you the path.
- **Quest:** "The Prey That Taught the Hunter" — mythic hare, killing grants status but closes forest paths.
- **Solutions:** hunting (track without final shot), ecology (restore predator), crafting (blunt marker-arrow), route_mastery (twelve shadows in order), combat (shadow-pack not Hare)
- **Reward:** Twelfth Shadow Track (relic) — reveals hidden route without harming target. Risk: trophy hunting closes path permanently.

### 8. Ashhold Legion — The Hearth That Marched

- **Court:** The Ember-Wives
- **Type:** hearth-fae / war-fire widow
- **Moral band:** tragic_good_to_evil
- **Zone bias:** Ashhold, Cinderfall, burned villages
- **Folklore hint:** A campfire follows a soldier home and waits outside the door.
- **Faction interaction:** War fires built on hearth-fae bargains soldiers no longer remember.
- **Quest:** "Bring the Fire Home" — hearth-fae carried into war as morale engine, now burns homes.
- **Solutions:** ritual (household fire with names), diplomacy (admit conscription), crafting (civilian hearthstone), combat (ember-soldiers not core flame), refusal (refuse fire as banner)
- **Reward:** Civilian Hearthcoal (crafting_component) — safe fire that cannot be militarized. Risk: weapon forging turns it to ash.

### 9. Rimegate Clans — The Guest Who Was Snow

- **Court:** The Frost Table
- **Type:** hospitality-fae / winter guest
- **Moral band:** stern_good_to_lich_grade_evil
- **Zone bias:** Rimegate Peaks, snow shelters, clan halls
- **Folklore hint:** An extra bowl freezes before anyone sits down.
- **Faction interaction:** Guest-law overlaps with older fae hospitality law; breaking either summons winter judgment.
- **Quest:** "Set a Place for Winter" — clan refused shelter to unnamed guest, winter enters as creditor.
- **Solutions:** survival (keep travelers alive), diplomacy (restore guest-law), crafting (snowglass bowl), hunting (track without stepping in prints), refusal (accept exile for guest)
- **Reward:** Fourth Bowl of Snowglass (relic) — temporary guest-right in hostile cold. Risk: denying someone colder cracks it.

### 10. Shattered Reach Corsairs — The Siren Who Forgot Hunger

- **Court:** The Keening Reef
- **Type:** siren-fae / song predator turned oracle
- **Moral band:** mischievous_to_evil_but_redeemable
- **Zone bias:** Shattered Reach, reef caves, storm routes
- **Folklore hint:** A song lures ships away from rocks, then asks why sailors still fear it.
- **Faction interaction:** Corsairs using fae songs as navigational weapons.
- **Quest:** "A Song Is Not a Net" — rescue-song weaponized into wrecking lure, Siren can't tell hunger from warning.
- **Solutions:** music (counter-melody), fishing (reef-drum fish), stealth (cut echo-lines), diplomacy (wreck survivors testify), combat (hunger verses only)
- **Reward:** Warning-Note Conch (offhand) — distinguishes lure/warning/grief sounds. Risk: luring enemies corrupts tone.

### 11. Dread Lattice Nulls — The Cartwheel King

- **Court:** The Mapless Revel
- **Type:** revel-fae / spatial prankster
- **Moral band:** mischievous_to_malevolent
- **Zone bias:** Dread Lattice, wrong-map rooms, festival ruins
- **Folklore hint:** A painted wheel turns on a wall and every door becomes yesterday's door.
- **Faction interaction:** Nulls erase maps; old fae erase certainty. Not the same.
- **Quest:** "Do Not Map the Dance" — fae revel kept paths alive by changing them, Nulls pinned it into erasure.
- **Solutions:** route_mastery (navigate by rhythm), crafting (northless compass), diplomacy (stop Nulls pinning), refusal (refuse to record shortcut), combat (when King stops laughing)
- **Reward:** Northless Compass Rose (tool) — finds paths that exist only while unowned. Risk: overuse destabilizes fast-travel.

### 12. Scorn Engine / Voidwoken — The Moth That Ate a Patch

- **Court:** The Glass-Moth Procession
- **Type:** glamour-fae / code-eating moth
- **Moral band:** alien_mischievous_to_catastrophic
- **Zone bias:** Scorn Engine, Void overlays, broken lantern servers
- **Folklore hint:** A moth lands on a terminal and the error message becomes beautiful.
- **Faction interaction:** Voidwoken mistake fae glamour for corruptible code; fae mistake Void patches for iron curses.
- **Quest:** "Glamour Is Not Code" — moth eats Void patches, turns them into glamour, broken systems look healed.
- **Solutions:** data_mining (separate code from glamour), provenance (reveal fake repairs), crafting (honest lantern), refusal (decline beautiful patch), combat (false wings not moth)
- **Reward:** Honest Glamour Lantern (offhand) — shows whether repaired/blessed objects are stable. Risk: cynical use strips harmless beauty.

### 13. Outside Wheel / Weaver — The Child Who Unwove Crowns (SECRET +1)

- **Court:** The Thirteenth Thimble
- **Type:** secret fae / anti-sovereignty weaver
- **Moral band:** good_if_refused_evil_if_owned
- **Zone bias:** unowned roads, blank graves, Crown reflections, places omitted from maps
- **Folklore hint:** A child asks for a crown so they can take it apart and make socks for the dead.
- **Requires:** 2 fae quests resolved + 1 fae boss prevented/spared/witnessed
- **Faction interaction:** Weaver Crown can bind fae bargains, human law, and player ownership into one dangerous pattern.
- **Quest:** "The Thimble That Would Not Rule" — secret fae unmakes ownership spell inside crowns/relics/authority.
- **Solutions:** crafting (crownless socket), diplomacy (three factions don't claim child), witness_building (record as contradiction), refusal (let thimble remain toy), combat (only if too many claimed gifts)
- **Reward:** The Thirteenth Thimble (relic) — converts one claimed relic/crown/bargain into unowned world benefit. Risk: weaponizing unmakes legitimate bonds.

---

## 10. Living Substrate Crafting

**System name:** Living Substrate Crafting
**Core reveal:** The player thought they were finding rare materials. They were finding prisoners, bargains, organs, songs, and ghosts.

### Substrate Types

| ID | Public Name | True Label | Powers | Ethical Pressure |
|----|-------------|-----------|--------|-----------------|
| fae_blood | Red Sap | fae blood | lanterns, healing, oath ink, blood-seal weapons | high |
| fae_breath | Sweet Draft | bottled fae breath | bellows, instruments, wind gates, fire control | medium-high |
| fae_song | Harmonic Thread | captured fae song | siren lures, navigation, warding, voice locks | high |
| fae_root_spirit | Root-Stay | fae spirit holding Ironroot | roads, bridges, mines, foundations, containment | very high |
| fae_skin_or_coat | Weatherhide | stolen fae skin/coat | cloaks, sails, tents, stealth gear | severe |
| fae_bone | Hollow Ivory | fae remains | flutes, bows, charm sockets, divining rods | medium-to-severe |
| fae_dream | Soft Map | harvested fae dream | roads, fast travel, map smoothing, illusion veils | high |

### Five Crafting Paths

| Path | Result | Power | Obligation | Fae Hostility | Crown Temptation |
|------|--------|-------|-----------|---------------|-----------------|
| **Exploit** | Strongest output | +3000 | +2500 | +3000 | +2000 |
| **Bargain** | Stable output + clause | +1500 | +1500 | -500 | +500 |
| **Release** | Weaker/no item, world heals | -1500 | 0 | -3000 | -2000 |
| **Replace** | Technically demanding substitute | varies | -1000 | 0 | 0 |
| **Preserve** | Place-bound hybrid reward | +500 | 0 | 0 | 0 |

### Progressive Disclosure (matches UI Disclosure Doctrine)

| Stage | Hours | Player Understanding |
|-------|-------|---------------------|
| 1. Sensory | 20-30 | "These are rare magical materials." |
| 2. Pattern | 30-45 | "These materials have fae origin." |
| 3. Folklore | 45-60 | "Human factions have been harvesting fae sources." |
| 4. Ledger | 60-80 | "Fae life is infrastructure." |
| 5. Choice | 80+ | "Crafting is now an ethical system, not just production." |

### Crafting Tags (12)

fae_bound, fae_blooded, breath_bottled, song_captured, root_stayed, skin_borrowed, dream_harvested, bargain_clause, released_source, substitute_source, unowned_material, obligation_bearing

### Example Recipes

**Red-Sap Lantern (true: Fae-Blood Lantern)**
- Exploit: brighter, reveals hidden paths, increases fae hostility
- Bargain: warns before fae trespass
- Release: no lantern, root path heals
- Replace: alchemical lantern using mineral resonance
- Preserve: fixed sanctuary light, holds roots back without draining blood

**Rest-Whistle (true: Breath-Free Whistle)**
- Exploit: forces labor chains to pause by consuming breath
- Bargain: pauses only when workers consent
- Release: frees bound breath, ends one forced shift
- Replace: mechanical whistle powered by heat pressure
- Preserve: settlement rest bell, cannot optimize production

**Warning-Note Conch (true: Uncaged Siren Note)**
- Exploit: controls lure/warning songs
- Bargain: asks siren-note to distinguish danger
- Release: restores reef song, prevents one wreck
- Replace: crafted harmonic shell using colour/resonance math
- Preserve: fixed coastal warning shrine

**Root-Stay Brace (true: Bound Root-Spirit Brace)**
- Exploit: portable bridge/root control
- Bargain: bridge opens under agreed conditions
- Release: roots reclaim one human road
- Replace: engineered support using stone/iron resonance
- Preserve: living bridge with limited respectful passage

---

## 11. Faction Registry Fix

```
ironmoor_compact.raid_affinity: FarWound → MercyDrowned
free_graves.raid_affinity: MercyDrowned (unchanged — thematic overlap intentional)
duskweald_hunt_kin.raid_affinity: FarWound (unchanged)
```

---

## 12. Implementation Schema

```rust
pub enum SolutionPathKind {
    Combat,
    Crafting,
    Hunting,
    Fishing,
    Music,
    Survival,
    DataMining,
    Diplomacy,
    Trade,
    Stealth,
    Sabotage,
    Provenance,
    WitnessBuilding,
    Ritual,
    Refusal,
    Ecology,
    RouteMastery,
}

pub enum FaeVoiceTag {
    Lure,
    Warning,
    Grief,
    Bargain,
    GuestRight,
    Glamour,
    DebtSong,
    RouteSong,
}

pub enum SubstrateType {
    FaeBlood,
    FaeBreath,
    FaeSong,
    FaeRootSpirit,
    FaeSkinOrCoat,
    FaeBone,
    FaeDream,
}

pub enum CraftingEthicsPath {
    Exploit,
    Bargain,
    Release,
    Replace,
    Preserve,
}

pub enum FaeMoralBand {
    Good,
    GoodButDangerous,
    SternGood,
    MischievousGood,
    Mischievous,
    MischievousToEvil,
    TragicEvil,
    TragicGoodToEvil,
    AlienMischievous,
    LichGradeEvil,
    GoodIfRefusedEvilIfOwned,
}

pub struct FaeBossDef {
    pub id: &'static str,
    pub faction_id: &'static str,
    pub display_name: &'static str,
    pub court: &'static str,
    pub fae_type: &'static str,
    pub moral_band: FaeMoralBand,
    pub zone_bias: &'static [&'static str],
    pub folklore_hint: &'static str,
    pub quest_id: &'static str,
    pub solution_paths: &'static [SolutionPathKind],
    pub reward_id: &'static str,
}

pub struct SubstrateDef {
    pub id: SubstrateType,
    pub public_name: &'static str,
    pub true_label: &'static str,
    pub crafting_domains: &'static [SolutionPathKind],
    pub ethical_pressure_q: u16,
    pub corruption_risk: &'static str,
}

pub struct FaeLayerInputs {
    // Shared from world
    pub faction_pressure_q: u16,
    pub ecology_pressure_q: u16,
    pub economy_pressure_q: u16,
    pub artifact_provenance_pressure_q: u16,
    pub unique_trigger_flags: u32,
    pub chaos_perturb_q: i16,
    // Fae-specific
    pub obligation_pressure_q: u16,
    pub fae_exploitation_q: u16,
    pub consent_integrity_q: u16,
    pub replacement_quality_q: u16,
    pub source_suffering_q: u16,
}
```

---

## 13. Connections

```
WCE ← substrate as SRC material in 16-byte query
Fae Overlay ← substrate exploitation drives fae hostility + selection weights
Law Layer ← relic ownership ethics apply to substrate items
Music ← fae_song + fae_breath + fae_bone route through resonance
Provenance ← shimmer detection reveals true substrate origin
Crown ← exploit path feeds temptation, release path starves it
CCE ← crafting mastery domain tracks ethical path choices
StateCurve ← progressive disclosure stages map to precompiled thresholds
consequence_geo.rs ← spread geometry, propagation kernel, resonance table all serve fae spawn math
```

### High-Dimensional Lateral Connections (/aspire)

1. **The 13 Moons Hybrid Alignment (12+1 Isomorphism)**
   * **Subsystem Target:** `_book/pages/13-MOONS-CANON.md` / `crates/forge-book-v3/src/star_atlas.rs` / `_book/03-take-too-much.md`
   * **Lateral Connection:** The Fae overlay's **12+1 Courts** correspond 1:1 with the **13 Moons calendar system**.
   * **Execution Seam:** The 12 standard Fae bosses are paired with the 12 seasonal Moons, each corresponding to a specific `MetronomeClock` epoch. The **13th Secret Boss** (*The Child Who Unwove Crowns*, index 12) corresponds to the **Blue Moon / Intercalary Moon**, representing the paranormal *wîhtiko* boundary described in `Take Too Much`. When the player's `ownership_pressure_q` or `crown_temptation_q` exceeds the threshold, the simulation shifts the `MetronomeClock` out of normal time-flow and locks it into the intercalary moon, triggering the secret boss's spatial spawn.

2. **The World Consequence Membrane (WCE Feed)**
   * **Subsystem Target:** `crates/forge-consequence-v3` / `crates/forge-reactions-v3/src/fae_ethics.rs`
   * **Lateral Connection:** The ethical deltas in `fae_ethics.rs` (`FaeItemPressure`) act as direct input drivers for the World Consequence Engine (WCE).
   * **Execution Seam:** **The Shimmer Provenance Seal:** When a Fae relic is `Stolen`, the `shimmer_detectable: true` state triggers a low-level cryptographic provenance check inside `forge-vix/src/vibe_reg.rs`. If a stolen item is equipped in the `Belt` (`crates/forge-book-v3/src/items.rs`), the WCE elevates `fae_hostility_q` and `ecology_pressure_q` across the surrounding chunk grid. This alters the local spawn table weights inside `forge-reactions/src/spawn.rs`, causing hostile Fae units to hunt the player.

3. **The Spatial 5D Trinary Cartography (TritTree5D)**
   * **Subsystem Target:** `F:\NewRepo\crates\outland\src\trit_tree.rs` / `crates/forge-reactions-v3/src/fae_selection.rs` (`zone_bias` tags)
   * **Lateral Connection:** The Fae boss definitions in `fae_data.rs` include specific `zone_bias` strings (e.g., `Shattered Reach`, `Keening Reef`, `Ironroot Spires`). These are mapped laterally to 5D coordinates in balanced trinary space.
   * **Execution Seam:** The `TritTree5D` (traversed via the `tractor-beam` protocol) compresses 5-dimensional points (X, Y, Z, Time, SoulWord) into a packed 105-trit representation. When `select_fae` evaluates player behavior (e.g., `overfishes`), the resulting high-dimensional weights project a raycast through `TritTree5D`. This query selects physical chunks in the `MudWorld` graph that correspond to the chosen Fae boss's spatial coordinates, overlaying the folklore domain directly onto the physical map.

4. **Crematic Emission & Syn-Audit (Sonic Synthesia)**
   * **Subsystem Target:** `crates/forge-harmonics` / `crates/forge-reactions-v3/src/fae.rs` (`FaeVoiceTag`)
   * **Lateral Connection:** The Fae voice tags (`CreeFinal`, `HighWhisper`, `ResonantPitch`) map to physical pitch, scale, and tempo constraints within the `RealtimeSynth` engine.
   * **Execution Seam:** When dialogue is rendered native inside the window, the `Conductor` clock (`U2 Conductor Live`) translates the text. A `FaeVoiceTag` of `ResonantPitch` forces the `camelot.rs` vocoder to shift its carrier wave by a perfect minor third relative to the current environmental musical key. This produces a deliberate "chilling" microtonal dissonance that alerts the player of a Fae entity's presence before it is visually rendered on the canvas.

5. **Shaderbind & The Glamour Membrane**
   * **Subsystem Target:** `crates/forge-shaderbind` / `crates/forge-reactions-v3/src/fae.rs` (`Interpretation::Glamour`)
   * **Lateral Connection:** The Fae dual-nature concept—where "Glamour" and "Code" are mapped over identical byte content—is represented on the GPU via shader binding.
   * **Execution Seam:** The `Interpretation::Glamour` enum state is bound as a 1-bit push constant (`vibe_carrier_q`) through `shaderbind_dsl`. When the player's `crown_temptation_q` is high, the shader bind overrides the voxel rendering buffer, blending the material textures with Oklch chromatic aberrations. This makes the physical environment warp around the player, blending code structures into beautiful, hallucinatory folklore surfaces.

---

## 14. Post-Mortem

**What went right:**
- Flat WCE model means fae slot in without new systems — they're just pressure sources + WCE queries
- 12+1 pattern mirrors raids and world bosses — consistent architecture
- Living Substrate Crafting emerged naturally from "what are fae doing in the world" question
- Mutual exclusions prevent content soup while maintaining replayability
- Progressive disclosure doctrine applies cleanly to substrate reveal

**What could break:**
- 14 spawn inputs across two formulas means tuning complexity — mitigated by layer separation
- Mutual exclusion groups share members (baptismal_hag in water_fae AND grave_refusal_fae) — selection logic must check all groups, not just first match
- `source_suffering_q` is emotionally loaded — needs careful UI surfacing (sensation first, never raw number)
- Music stat links (pitch→Resonance, etc.) need validation against existing CCE stat domains to avoid double-mapping
