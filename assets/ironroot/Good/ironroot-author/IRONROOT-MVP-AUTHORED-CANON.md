# IRONROOT MVP — Authored Canon
### Authored against CONCEPTIRON sheets 01–07 · `/ironroot-author` · 2026-06-05

> Source art: `F:\repos\ironroot-edict\CONCEPTIRON\` (READ-ONLY port-source; deprecated, succeeded by `dirge-of-ironroot`). This canon is authored **from** the blueprints, never written back into that repo. Engine bindings reference the live `F:\repos\13forge\forge-*` substrate.
>
> **The grammar (skill spine):** Authority decides order · Threshold decides whether · Ratio decides spread · Convergence decides apex · DAG decides limit (≤3) · Root state decides volatility · Primitive priors decide fallback · Diplomacy decides meaning *after* physics already happened.

---

## 0. The Spine — One Closed Loop Across Three Spaces

The seven sheets are not seven places. They are **one consequence cycle** seen through different lenses. What the player carries out of one space is the threshold that fires in the next.

```
        ┌────────────────────── THORNBELL PARISH (01 · town) ──────────────────────┐
        │  Core Town Loop:  Return → Identify Materials → Craft/Trade →             │
        │                   one Social/Economic Choice → Advance Faction Clocks →   │
        │                   Prepare Next Run                                        │
        │   Descent Door to Bell Pit ▼            ▲ Return to Town                   │
        └───────────────┬───────────────────────────────────┬──────────────────────┘
                        ▼                                     │
        ┌──── THE BELL PIT (02 · arena) ────┐   ┌──── THE UNDER-ORCHARD (03 · dungeon) ────┐
        │ Enter → Wave → Bell-Pressure →    │   │ Enter → Choose Branch →                  │
        │ Fight/Parry/Refuse → Event vs     │   │ Fight/Sneak/Negotiate →                  │
        │ Ledger → Between-Wave Choice →    │──▶│ Extract on Record → Material gains        │
        │ Sieve Modifies Next Wave →        │   │ PROVENANCE → Return to Town              │
        │ Boss Manifests                    │   │ Branches: Root Cellar·Toll Drain·        │
        └───────────────────────────────────┘   │ Bell Vein·Spirit Fold·Grave-Mine        │
                                                 └──────────────────────────────────────────┘
```

**The interlock that makes it a game, not three minigames:** a *Social/Economic Choice* refused in town (step 4) advances the **Toll-Saints clock**, which seeds a **Warden of Red Debt** into the next arena wave (blueprint 02, Bell Warden Variants). Material extracted *without inscription* in the dungeon returns to town as **stolen-class provenance**, which the Ledger Church reads at the Witness Rail. **The loop is the consequence.**

---

## 1. The Seven Sheets (what each blueprint actually is — cited)

| Sheet | Title (as drawn) | Role | Reads back to |
|---|---|---|---|
| **Welcome** | *"Welcome to Ironroot — where every fairytale comes true"* | Diegetic **lie-surface**: cheerful tourist map (Sugarhollow, Lullaby Lake, Crowndeep). Dark panel: *"What the map doesn't show you"* — Root Veins, Branded Routes, Watched Seats. *"The real geometry — everything points inward."* | The player's Hour-0 literacy: *places that don't match, songs that repeat, don't speak the wrong name.* |
| **01** | *Thornbell Parish* — hub town, top-down | Crafting/Trade/Diplomacy/Faction-Pressure/Preparation. Bellwright Forge · Market Row · Witness Rail · Toll Gate · Parish Shrine · **Descent Door to Bell Pit**. | `forge-game-systems` zones/vendor/quest; faction clocks → `forge-sieve` |
| **02** | *The Bell Pit* — arena, top-down/hybrid | Bell Core · Player Spawn · L/R Gates · Witness Balcony · Boss Seal · Root Cracks. 5-wave sequence, Bell Wardens, **"Boss Sieve / Sieve Modifies Next Wave."** | `DirectorSieve`(20) + `BrandedSieve`(5); parry → `dirge/combat/parry.rs` |
| **03** | *The Under-Orchard* — dungeon, branch+section | Branches: Root Cellar(combat) · Toll Drain(stealth/theft) · Bell Vein(extraction) · Spirit Fold(death-route) · Grave-Mine. Material families: Bell Bronze, Bell Diamond, Grave Iron. **Locked Deep Door.** | provenance → `forge-items/ledger.rs`; extraction-call → MUSIC sieve |
| **04** | *Camera Strategy* — six lenses | *"Cameras are not views. They are vows."* Root 3D · Knife 2D · Ledger Tactical · Strategic 4X · Spirit Death · Vowless Blank. Camera-shift = **rule-shift**. | `forge-render` camera_stack/camera3d/mode/render_router |
| **05 / 06** | *Cathedral · Fortress · Palace* — vert. profiles E & D (of 7) | Ad-quadratum √2 proportion, flying-buttress thrust math, Master-Mark masonry glyphs, verticality = authority hierarchy. | `forge-architecture` gen_ad_quadratum/buttress/mason/verticality/blueprint_3d |

---

## 2. Naming-Lint Pass (applied to authored layer)

The blueprints carry one forbidden term and several near-terms. Player-facing canon renders:

| Blueprint / engine term | Player-facing canon | Note |
|---|---|---|
| `Earthcalling` (Bell Vein branch verb; engine `AbilityDomain::Earthcalling`) | **Rootcalling** (organic vein) / **Stonecalling** (mineral seam) | ⚠ Engine enum may stay `Earthcalling` *internally*, but **must remap before any UI surface**. Flagged in §6. |
| "Thirteen Bells" / implied 13-cycle | **Thirteen Tolls** (calendar) / **Thirteen Bells Warden** (the boss is native, keep) | |
| Rival/echo boss | **Shadowmirror** (never "Nemesis") | from `MirrorSieve` |
| Apex corruption entity | **The Branded** (never "Windigo") | "Branded Route" already on the lie-map ✓ |

No aurora, no bone-whistle, no Cree surface terms present in the art — floor intact.

---

## 3. Flagship Consequence — TOWN · "The Toll That Comes Due"

*The Toll Gate does not raise its arm in anger. It simply reads what you carried home, and counts.*

When you return from a run, Toll-Sister Vey's ledger reconciles what left the Parish against what came back inscribed. A gap is not theft — Ironroot has no such word at the gate. It is **unrecorded weight**, and unrecorded weight tolls.

```
[WCE Query]
  toll_reconcile(carried_value: N, inscribed_value: N, gap := carried - inscribed)

[Authority & Threshold]
  authority: Toll-Saints (chosen authority — they elect to collect)
  threshold: gap > toll_threshold(faction_clock_phase)   # rises as the clock advances

[Spread Consequentials  (DAG depth 3)]
  Diplomacy : witness_liability(Ledger Church, intent: unrecorded_weight, witness_count: Witness Rail)
  Economy   : price_change(Market Row, +debt_premium)            # Mara of the Free Guilds reprices
  Faction   : advance_clock(Toll-Saints, +1)  ──▶  seeds Warden of Red Debt into next Bell Pit wave
  (STOP — level-4 root-volatility shift blocked by DAG)

[Readable Tells ≥3]
  • Hour 0  — Toll-Sister Vey stops mid-count and looks at your hands, not your face.
  • Hour 0  — the gate bell tolls a *number*, not a chime. The number is your gap.
  • Town    — Market Row prices tick up overnight; Index Clerk Oth leaves a margin blank in his book.

[Counterplay ≥2]
  1. Pay the toll        — costs coin/labor now; clock does not advance.
  2. Retro-inscribe      — Index Clerk Oth back-dates provenance (costs Index-Monk standing + time).
  3. Accept Red Debt     — refuse; the debt is witnessed. The Warden of Red Debt manifests in the arena.

[Primitive Prior fallback]  #6 Obligation Inscribes — the gap does not fade; it accumulates and transmits.
[Provenance]  blake3(descriptor) → store at inscription time
```

**Engine bind:** `WitnessSieve`(17) + `DiplomacySieve`(15) + `event_ledger.rs`; the clock is a `forge-sieve` instance advancing in 4X (sheet 04, Faction Clocks) and resolving in all lenses.

---

## 4. Flagship Consequence — ARENA · "The Bell That Counts the Living"

*The Bell Core does not ring for the dead. It rings to find out who is still standing — and each ring narrows the question.*

```
[WCE Query]
  bell_pulse(wave: N, interval_ms := f(intensity), count: 0..13)

[Authority & Threshold]
  authority: imposed (the Bell Core is root-bound; no one chooses the pulse)
  threshold: count == 13  →  Thirteen Bells Warden manifests   # convergence apex (Boss Seal opens)

[Spread Consequentials]
  Sound/Color/Physics : grave-bell motif (descending minor third) — the interval audibly shortens
  Structure           : Root Cracks widen each pulse (severity += 1)
  Diplomacy           : Witness Balcony records who fought / who fled (feeds town ledger)
  Spirit              : a death here does not end — it folds into the Spirit Fold (dungeon 03)

[Readable Tells ≥3]
  • the pulse interval shortening — the same descending two-note bell, faster (you can count ahead).
  • Root Cracks throwing red light up the Witness Balcony.
  • between-wave silence getting *shorter* — the Sieve is tightening.

[Counterplay ≥2]
  1. Perfect Parry the pulse — Knife 2D lens, frame-window timing; clears pressure, no debt.
  2. Refuse → Vowless Blank   — step out of the vow entirely; null outcome, no ledger write, no reward.
  3. Spend a clear            — sacrifice this wave's loot so the Sieve softens the next (Director backs off).

[Primitive Prior fallback]  #1 Damage Model — pain & cost proportional to severity, no exceptions.
```

**Engine bind:** wave pacing = `DirectorSieve`(20) `evaluate → DirectorIntensity`; **"Boss Manifests"** = `BrandedSieve`(5) `evaluate → BrandedManifests` once convergence conditions meet. The bell motif is `PHRASE_KIND_MINOR_THIRD_DESCENT` (forge-sieve/forge-harmonics) — the same grave-bell phrase the dispatch fabric already routes. Parry → `dirge/combat/parry.rs`.

---

## 5. Flagship Consequence — DUNGEON · "Provenance, or the Orchard Remembers"

*You can take iron from the Grave-Mine in silence. The iron will remember the silence.*

```
[WCE Query]
  extract(branch: BellVein|GraveMine|RootCellar|..., material: family, inscribed: bool)
  source_class := inscribed ? Recorded : Stolen

[Authority & Threshold]
  authority: chosen (you swing) — but the Root is the witness, and the Root is also authority
  threshold: extraction_volume > vein_capacity  →  vein collapse → Toll Drain floods

[Spread Consequentials]
  Root      : #5 Root Remembers — seed/heritage of the material persists; source_class is permanent
  Diplomacy : Stolen-class → witness_liability travels home to the Toll Gate (links §3)
  Economy   : Stolen material reprices down / flags at Market Row
  Structure : over-extraction collapses the Bell Vein; a Spirit Fold seam can breach

[Readable Tells ≥3]
  • the vein's resonance — a sub-bass tone you must *sound back* to extract cleanly (Rootcalling).
  • Grave-Mine iron-smell sharpening as you near capacity.
  • the Locked Deep Door's seal-light brightening when you take more than the Orchard will forgive.

[Counterplay ≥2]
  1. Inscribe at extraction — sound the Rootcalling note; material is Recorded (costs the held tone + time).
  2. Extract-and-flee       — Toll Drain stealth route; accept Stolen-class for speed.
  3. Controlled collapse    — sacrifice the vein to seal a Spirit Fold breach (lose the seam, save the run).

[Primitive Prior fallback]  #7 Knowledge Precedes Mastery — sensing the vein before cutting it is the advantage.
```

**Engine bind — this is the through-line I built this session:** the **Rootcalling extraction note** is literally `SieveEvent::Note(HarmonicEvent{ pitch < 36, on: true })` → `MusicSieve` → `SieveAction::AbilityPromotion { domain: AbilityDomain::Earthcalling }` in `forge-sieve`. A sub-bass note sounded into the Bell Vein **is** the inscription act. Provenance hashing → `forge-items/ledger.rs`.

---

## 6. The Cathedral as Authority Made Stone (sheets 05/06)

The Cathedral·Fortress·Palace is the **Ledger Church's body**. It is not decoration — it is the Witness-Liability Ledger built at architectural scale: every course of stone bears a **Master Mark** (a mason's inscribed name), and *names, once inscribed, cannot be forgotten — only inherited* (Primitive Prior #4). Its **verticality is the authority hierarchy**: how high you are permitted to stand is how much you are trusted to witness.

- **Ad quadratum √2** proportion + **flying-buttress thrust math** → `forge-architecture` `gen_ad_quadratum.rs`, `buttress.rs`, `verticality.rs`, `blueprint_3d.rs`.
- **Master Marks** (the masonry glyph set drawn bottom-of-sheet) → `forge-architecture/mason.rs` master-mark geometry; each mark is a diegetic inscription = a ledger entry.
- Authored hook: ascending the Cathedral is a **counterplay vector for diplomacy** — to reach the Index above the Clerestory and have a Stolen-class material re-witnessed costs you a climb past every faction's permitted altitude.

⚠ **Reconciliation flag for Sean:** engine `AbilityDomain::Earthcalling` (code, internal, fine) ↔ player-facing **Rootcalling/Stonecalling** (lint). One small remap shim needed *only* if/when that domain surfaces in UI text. Not a code change today — a noted seam.

---

## 7. Six Lenses, Six Vows (sheet 04) — the meta-grammar

*"Camera Shift means Rule Shift. No free conversion."*

| Lens | The vow (diegetic) | Rule-shift | Space | Engine surface |
|---|---|---|---|---|
| **Root 3D** | *I walk and I am seen* | exploration/navigation, full physics | Town·Arena·Dungeon | `camera3d.rs` |
| **Knife 2D** | *I close in and cut clean* | side-on precision, parry frames | Arena·Dungeon | `mode.rs` 2D path |
| **Ledger Tactical** | *From death to decisions* | isometric tactics, encounter play | Town·Arena | `render_router.rs` |
| **Strategic 4X** | *I weigh the Parish* | overmap, faction clocks advance | Town | 4X seed (sheet 04) |
| **Spirit Death** | *When death occurs, enter the echo* | death/aftermath, Spirit Fold | Dungeon (death) | `DreamSieve`(23) + `dirge/death_scar.rs` |
| **Vowless Blank** | *When I refuse, enter the blank* | null space — no ledger write, no reward | any (refusal) | the refusal mechanic |

The four **Faction Clocks** (Ledger Church · Toll-Saints · Free Guilds · Index Monks) advance in **Strategic 4X** and *resolve in all lenses* — the strategic layer is where consequence is scheduled; the other lenses are where it lands.

---

## 8. Mastery Curve (skill §Masterwork, mapped to the three spaces)

| Hours | Literacy | Where it's taught |
|---|---|---|
| 0–10 | sensory world-reading | the lie-map's tells; the bell that tolls a number |
| 10–25 | pattern recognition | town clock ↔ arena warden link becomes legible |
| 25–45 | consequence tracing | dungeon provenance → town ledger → next-run warden |
| 45–70 | authority-seeking | climbing the Cathedral; faction-clock manipulation |
| 70–90 | ledger literacy | reading the Witness Rail to pre-empt liability |
| 90–100+ | prevention mastery | breaking a consequence chain before Hour-0 tell |

---

## 9. Validation Checklist (skill)

- [x] Consequences lyrical + mechanical (prose embeds systems) — §3–5
- [x] Authority & threshold documented (who, when, measurable) — all three
- [x] Spread consequentials listed (W/F/St/E/D/R) — all three
- [x] Readable tells ≥3 (sensory, progressive) — all three
- [x] Counterplay ≥2 (player agency) — all three
- [x] Cascade depth ≤3 (DAG respected) — town descriptor shows the level-4 block
- [x] Naming lint passed — §2 (Earthcalling→Rootcalling flagged for UI)
- [x] Primitive-prior fallback defined — #6/#1/#7
- [x] Diplomacy integration (witness counts, covenants) — Witness Rail/Balcony, Toll-Saints
- [x] Mastery curve visible — §8
- [ ] Provenance hash *stored* — descriptors specify blake3; **emission is a build task** (see §10)

---

## 10. What this unlocks (next, if released — recon stops here)

1. **Asset estimate** (your earlier parked question): these 7 sheets *are* the "1 MVP loop · 1 town · 1 arena · 1 outer area" scope. The 4-era / 4-music / 4-story multiplier rides the **4X seed** + **Faction Clocks** — same spaces re-vibed (vibe_matrix), not 4× the geometry.
2. **Engine wiring already live:** arena Boss-Sieve, grave-bell phrase, and Rootcalling extraction note are all on real `forge-sieve` substrate — the dungeon extraction consequence is *playable today* against the MUSIC-sieve through-line.
3. **One real seam to close:** the `Earthcalling→Rootcalling` UI remap shim (§6).

*Authored from the blueprints. The world reads back.*
