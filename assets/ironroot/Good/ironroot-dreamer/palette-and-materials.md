# Palette and Materials

> Weld note (2026-08-16): soot-black and clay-broken re-hexed (they duplicated grave-iron and torch-dull, violating this file's own 20-point separation rule); parchment material-triad base corrected to parchment's core hex; one slate-blue hex typo fixed in the dither table. Typography is governed by *glyphs-type-and-ui.md* (EB Garamond + humanist sans, Sean 2026-08-16); nothing in this file names a font.

## CORE PALETTE (32 colors)

| Hex | Token Name | Role | Usage Rule | LoRA Caption |
|-----|------------|------|-----------|--------------|
| #0A0A0A | void-black | Ground/Void | All dark contexts; no object darker; black-background reference | ironroot void-black ground |
| #1A1A1A | slate-deep | Shadow Base | Ambient shade in unlit passages; 10% over void-black | deep slate shadow |
| #2A2A2A | ash-grey | Stone & Age | Weathered ledger stone, cracked ceramic; boundary between shadow and material | aged ash-grey stone |
| #3D2D1A | grave-iron | Metal Cold | Wrought iron, chains, execution helms, nail rings | grave-iron metal |
| #4A3020 | bell-bronze | Warm Accent | Forge mark, bell castings, warm architectural detail; the warm companion to grave-iron | bell-bronze warm metal |
| #5C4033 | marrow | Bone Decay | Aged bone turned ochre, parchment shadow, root-wood color | marrow wood & bone |
| #703820 | old-root | Organic Deep | Root tendrils, ledger bindings, old cloth; organic warmth | old-root organic tendrils |
| #2D3D2A | obsidian-dark | Deep Glass | Mirror scar base, glass-lung fractures before light strikes; barely lighter than void | obsidian dark glass |
| #8B6F47 | parchment | Ledger Surface | Index Monk pages, document backgrounds, worn cloth; the brightest warm neutral | aged parchment paper |
| #A0957F | bone | Bone Light | Skeletal detail, cracked surfaces, bone ritual items; highest neutral for no-light contexts | bleached bone white |
| #C4B5A0 | ash-pale | Far Ash | Distant dust, ash motes in Spirit routes; read-distant signal | pale ash motes |
| #E8DCC8 | chalk-whisper | Witness Faint | Barely-visible text in ledgers, ghost marks; the absolute light threshold for void-black | chalk whisper faint |
| #6B1F1F | crimson-witness | Witness Light | **MASTER LOCK**: fracture-light where sound strikes darkness; the ONLY color for being seen, revealed mark, Master Marks in stone, the grave-bell's internal light. **Reads identically in all four eras** — no tint variation; its presence is structural truth, not mood. Edge rule: appears ONLY where sound has struck or a name is exposed. | witness crimson fracture-light |
| #4A1515 | crimson-dark | Witness Shadow | Crimson shadow (when crimson is struck again; rare); inside ink-veins of deep ledgers | crimson shadow witness |
| #3D1A2A | magenta-wrong | Forbidden Altar | The "faint magenta wrongness" of the Desperate Camp. **Fence**: sourced ONLY from (1) the Camp's hidden altar — while the altar stands, its wrongness leaks as faint tint through the Camp's air and smoke, and dies with the altar; (2) the Widow's private shrine; (3) one hidden carving in the Deep Ledger. Nowhere else, ever. Visual signal: reality-break, institutional scar, the color of refusal made visible. | magenta wrongness faint |
| #1A2D3D | slate-blue | Spirit Edge | Spirit-realm tint on doors, mirror-edges, void-boundary fades; cold accent for alternate states | spirit-realm slate-blue |
| #2D1A3D | void-purple | Erasure Mark | The color of quiet spreading; appears on ledgers marked for erasure, tomb-stones in the Spirit layer; rare and ominous | erasure void-purple |
| #1F3D1F | root-alive | Growth Signal | Living root vein, places where growth resists erasure, Stonecalling response color (not light, but biological feedback) | living root-vein signal |
| #3D3D1F | torch-dull | Candlelight | Dull yellow-brown of hand-held flame; small radius, no glow; campfire, grave-lantern, shrine candles | candlelight dull flame |
| #5C5C2D | torch-warm | Candlelight Glow | Warm near-flame; second ring around torch; pairs with torch-dull for concentric dither | warm torch radius |
| #2D2D1F | torch-shadow | Candlelight Shadow | Warm shadow at torch base; smooth falloff via dither pairs | torch shadow falloff |
| #1A3D2D | phosphor-green | Terminal Glyph | Terminal text at full brightness. Green because: (1) scotopic-safe for dark-adapted eyes, (2) terminal authority without neon — muted, lived-in, (3) legible over void-black with no sRGB bloat. ONLY for active text, input prompt, active ledger entries. | terminal phosphor green |
| #0D1F0D | phosphor-dark | Terminal Shadow | Text shadow and terminal frame; one step dimmer for subtext or inactive ledger rows | terminal dark text |
| #241C12 | soot-black | Ash Residue | Ash stain after Ash-discipline burn; warmer than void-black, darker than grave-iron; environmental damage, burn scars *(re-hexed in weld)* | ash residue soot |
| #2A1A2D | salt-pale | Salt Stain | Preservation mark; faint crystalline scar from Salt-discipline sealing; corpse-containers, sealed chambers | salt preservation stain |
| #1F2D3D | water-stain | Wet Memory | Widow Canal water mark, Toll-drown fades; cold grey-blue | wet cloth water-stain |
| #553527 | clay-broken | Cracked Ceramic | Broken pottery in ash-bowls, ritual vessels; warm terracotta worn *(re-hexed in weld)* | cracked ceramic bowl |
| #2D3D3D | metal-rust | Metal Decay | Iron oxidized cold; the shadow of bell-bronze when tarnished; edge color for corroded mechanisms | metal rust oxide |
| #1A1A2D | void-edge | Dither Anchor | Darkest dither partner for void-black in gradations; fakes smooth transitions without gradients | void dither anchor |
| #3D3D3D | grey-mid | Neutral Balance | Mid-tone for fabric, paper, metal neutral; balance point between warmth and cool | neutral grey |
| #6B3D1F | slag | Material Base | Slag heap black-red; metalwork byproduct; scorched, never pristine | slag metalwork base |
| #8B1F2D | rubedo-pulse | Violence Accent | Single-frame impact pulse ONLY (see Harmonic Tier Accents); never ambient | rubedo impact pulse |
| #0D3D2D | albedo-teal | Cleansing Accent | Dither-ring around restored objects ONLY (see Harmonic Tier Accents) | albedo cleansing ring |

## ERA-MOOD RAMPS (Tint Rules)

### Golden Era (The Remembered Bloom)
- **Shifts**: parchment → warmer ochre (+30% yellow); ash-grey → lighter, more golden; torch tones brighten by 20%.
- **Never shifts**: crimson-witness (truth is unaffected by mood); void-black (darkness is absolute); phosphor-green (the terminal is the terminal).
- **Rule**: add +15% brightness and rotate +10° toward warm across the palette, except locks.

### Ancient Era (The Slow Settling)
- **Shifts**: all colors desaturate −20%; parchment → beige-grey; ash-grey → cooler; bronze → muted brown.
- **Never shifts**: crimson-witness, void-black, phosphor-green.
- **Rule**: reduce saturation by 20%, add grey (+10%), keep brightness stable. Monuments weathered; fabric faded; materials time-worn, not fresh.

### Decay Era (The Collapse)
- **Shifts**: saturation −40%; yellows sicken toward brown; parchment → mottled greyscale; bronze → rust-orange-brown; candlelight flickers erratically; obsidian-dark spreads into cracks.
- **Never shifts**: crimson-witness, void-black, phosphor-green, magenta-wrong (wrongness intensifies).
- **Rule**: extreme desaturation, +5% void-black wash over most surfaces; root-alive stays vibrant (the contrast signal); fungus geometry becomes visible in corners.

### Void Era (The Final Silence)
- **Shifts**: all non-lock colors trend toward void-black; parchment → ash-grey; ash-grey → slate-deep; bronze fades; candlelight nearly extinguishes (torch-dull → void-edge).
- **Never shifts**: crimson-witness (the last light), void-black (finality), phosphor-green (the city's last breath as text).
- **Rule**: −25% brightness; push hues toward grey-blue-void; root-alive flickers and dims to slate-blue; magenta-wrong vanishes into void (it was the wrong answer all along).

## MATERIAL SWATCHES (Triads: Base / Shadow / Worn-Edge)

| Material | Base | Shadow | Worn-Edge | Role |
|----------|------|--------|-----------|------|
| Slag | #6B3D1F | #3D1F0D | #2A2A2A | Metalwork refuse; foundry floors; dull, never reflective |
| Ichor | #5C1F1F | #2D0D0D | #1A0D0D | Institutional blood (ledger ink, ritual residue); never "fresh" — always dried, stained |
| Bone | #A0957F | #6B6B5C | #3D3D2D | Skeletal ritual items; bleached, aged; cracks into ash-grey |
| Obsidian | #2D3D2A | #1A2D1A | #0A1A0A | Deep glass, mirror-shards; fracture edges reflect crimson when struck; absorbs light |
| Marrow | #5C4033 | #3D2A1F | #2D1A0D | Aged bone-wood; parchment hue when old; supports old-root organics |
| Ash | #2A2A2A | #1A1A1A | #0A0A0A | Residue, swept dust, ritual remains; coldest neutral; almost void but visible |
| Iron | #3D2D1A | #1A1A0D | #0A0A05 | Grave-iron tools, chain links, execution metal; warm grey-black; accepts dither well |
| Parchment | #8B6F47 | #6B6B5C | #4A4A3D | Ledger pages, cloth weave base; readability requires crimson marks or phosphor text *(base corrected in weld)* |
| Black Wax | #1A1A1A | #0D0D0D | #050505 | Seal, threshold block, Spirit-route sealing; just-lighter-than-void; smooth surface marker |
| Wet Cloth | #3D4A5C | #2D3A4A | #1A2A3D | Widow Canal saturation, flood memory; cool grey-blue; dithers into slate-blue at edges |
| Old Root | #703820 | #4A2A0D | #2D1F0D | Organic vein, underpass support; warm brown; resists erasure (grows darker as decay spreads) |
| Ledger Stone | #4A4A4A | #2A2A2A | #1A1A1A | Cathedral foundation, inscribed platform; neutral grey; Master Marks appear in crimson on its surface |
| Fungus Geometry | #5C3D2D | #3D2D1A | #2D1F0D | Molt Cathedral interior growth; organic brown-grey; fractal, never even; edges blur into ash |
| Bell Bronze | #4A3020 | #2D1F0D | #1A0D05 | Bell casting, Forge mark; warm, ages into rust at worn-edge; Albedo association |
| Bell Diamond | #C4B5A0 | #A0957F | #6B6B5C | Crystal interior of Bell Vein; single-pixel high-light only (one white pixel, no bloom, no shimmer); resonates with Albedo |
| Grave Iron | #3D2D1A | #2D1F0D | #1A0D05 | Execution tool metal, chains, nail rings; cold iron; Citrinitas association |

## HARMONIC TIER ACCENTS (Resonance-Locked Colors)

### Nigredo 40Hz (Mass, Debt-Stone)
- **Accent**: #1A0D0D (deep ichor).
- **Rule**: appears ONLY when the player is under active debt pressure (broken toll counted, ledger stolen, kill-debt tallied). A pulse-glow around debt items and creditor silhouettes. No gradient — dither into surrounding color or pulse via frame-flip (appears/vanishes on even/odd frame).
- **Example**: a Toll-Saint's armor rim flickers ichor-edge when they are counting you.

### Albedo 432Hz (Stable, Cleansing, Water Memory)
- **Accent**: #0D3D2D (albedo-teal; water-stain + root-alive blend).
- **Rule**: appears when the player performs cleansing acts (free a prisoner, restore a name fragment, seal a Spirit breach). A single dither-ring around the restored object, then fades. Cleansing is quiet.

### Citrinitas (Inverse-Hz, Breath, Testimony)
- **Accent**: #3D1A2D (slate-purple; void-purple + crimson inverse).
- **Rule**: appears when the player testifies, exposes, or interrupts a ritual with Breath-discipline. Marks active testimony windows and reveals hidden ledger text. Visible only as a thin line or one-frame flicker — NPCs see it as the protagonist's breath made visible.

### Rubedo 800Hz+ (Multi-Hit Violence, Fire, Exaltation)
- **Accent**: #8B1F2D (rubedo-pulse).
- **Rule**: appears only during high-impact multi-hit sequences (execution finishers, chant-backed Ash burns, the final blow of an event). Never a glow — a single sharp frame-pulse at the moment of impact, then back to normal crimson fracture-light. The cost of violence made visible.

### Forbidden Altar Magenta
- **Color**: #3D1A2A (magenta-wrong).
- **Fence**: sourced ONLY from three sites — (1) the Desperate Camp's hidden altar: while the altar stands, its wrongness leaks as a faint tint through the Camp's smoke and air (the frames may show it camp-wide, always faint, always sourced from the altar's direction), and it vanishes the moment the altar is exposed, destroyed, or the event resolves; (2) the Widow's locked private chamber in Widow Canal; (3) one deep carving in the Deep Ledger (the pact-stone where the Erasure was first negotiated). Never in any other zone, any aura, any equipment, any UI. Presence causes a barely-perceptible pixel-shift/sub-dither texture break — reality-rupture signaled, never explained.

## LIGHTING LAW

### Crimson Fracture (Stonecalling Light)
Light does not glow or gradient. When a heartbeat-pulse strikes stone or deep water, the void-black absorbs the vibration and **fractures into crimson-witness at the impact point only**:
1. **Edge rule**: a sharp 1–2 pixel crimson line at the boundary between the sound source and the struck dark.
2. **Falloff by dither, never gradient**: full crimson-witness at the strike point (1–2 px) → 50% crimson/void checkerboard (2–3 px) → 25% crimson/slate-deep ring → void-black. Total radius 6–8 px; never beyond arm's length in-world.
3. **Persistence**: lasts only while the call is active; when the call ends, the light ceases instantly (no fade).
4. **Multiple strikes**: each strike makes a new fracture at its point; old fractures do not stack or brighten — they reset their timer.

### Candlelight Law (Mundane Fire)
Dull, warm, small. Core torch-dull; radius 3–4 tiles; ring falloff: torch-dull → 75% dither toward torch-warm → torch-warm blended 50% into surroundings → 25% blend → void-black. Constant within its radius; no flicker (flicker exists only in Decay era, as a subtle frame-skip). Warm shadows in torch-shadow; shadows reach no further than the light.

### Terminal Phosphor Law (UI & Text)
Green (#1A3D2D), not amber, not white: green sits at the rod-cells' peak sensitivity for dark-adapted vision; it carries terminal authority without neon; it holds contrast over void-black at nearest-neighbor with no anti-aliasing. Active text full phosphor-green; inactive text phosphor-dark; brightness transitions only via dither; no glows, no halos; text sits flat on void-black.

## PER-SCENE PALETTE MAPPING

### Terminal Screen
Background void-black · active text phosphor-green · input prompt phosphor-green, no glow · frame grey-mid single-line border · error = crimson-witness single-frame pulse · sub-text phosphor-dark. Tone: authority, clarity, no warmth; the institutional voice — except this institution is yours.

### Thornbell Parish (×4 eras)
- **Golden**: stone ash-grey +20% yellow; banners parchment with ochre shadow; forge mark bell-bronze at full; shrine candles amber; shadows marrow-deep, forgiving. *Memory-lit; the Parish remembers its own light.*
- **Ancient**: stone cool beige-grey; banners faded; bronze muted; candlelight tired; shadows slate-deep. *Settled, patient, time-confirmed.*
- **Decay**: stone with obsidian overlay, cracked; banners bleached and torn; bronze rusts to metal-rust; candles gutter (erratic frame-skip); void-black creeps into corners; root-alive fractures through; fungus geometry at tile edges. *Collapse visible; the Parish is being unmade.*
- **Void**: stone falls through ash-grey → slate-deep → void-black dither bands; banners monochrome; bronze fades to slate-deep; a single erratic candle pixel; void-edge everywhere. Legible: crimson-witness on surviving Master Marks, phosphor-green where a terminal still breathes. *The end.*

### The Bell Pit
Floor ledger-stone with dithered cracks (black-wax fill) · Bell Core bell-bronze base + one bell-diamond apex pixel · bell answer = interior crimson-witness flash, then crimson fracture-rings across the floor (50% dither, 6–8 px per sound) · the toll-count is read on the Witness Balcony's carved toll-board, never floating · toll refused = the Warden of Red Debt in metal-rust silhouette with fenced magenta-wrong flickers only if the altar's wrongness has been carried here (else rubedo-pulse on its strikes). Tone: the city counts here.

### The Under-Orchard
- **Root Cellar**: old-root walls with ash-grey cracks; damp soil in water-stain tint; weak candle swallowed by damp; Spirit doors edge-glow slate-blue (alternate perception, never crimson).
- **Toll Drain**: ledger-stone + black-wax seals; water-stain pools reflecting void-black; debris in bone and clay-broken; submerged chains grave-iron gone metal-rust; torch-shadow dominates.
- **Bell Vein**: bell-bronze veins through ledger-stone; rare bell-diamond ceiling points with albedo-teal dither-rings; approach = bell-bronze/crimson 50% dither as the Vein recognizes the call.
- **Spirit Fold**: obsidian-dark walls with slate-blue fractures; floor stained void-purple; the mirror-pool true void-black with crimson fracture-lines only when struck; exit portal slate-blue/void-purple dither-edge.
- **Grave-Mine**: ash-pale dust, grey-mid stone, bone fragments; no light source — the player's Stonecalling is the only light; crimson reveals Master Marks on tombs.
- **Locked Deep Door**: grave-iron frame + black-wax wards; faint erratic void-purple pulsing from the far side; opened = slate-blue flood, water-stain mixing with phosphor-green as the Deep Ledger's voice becomes audible.

### Spirit Forest Mirror (transform rule, not a new palette)
The Mirror's palette = the living palette transformed: crimson-witness stays crimson (truth persists in death) · void-black becomes void-purple (erasure-tinted void) · parchment and ash-grey become slate-blue · warm tones (bell-bronze, torch-dull, old-root) invert to slate-blue/water-stain · phosphor-green becomes phosphor-dark (the voice whispers on the dead side) · candlelight becomes faint slate-blue. All dither doubles (50% instead of 25%) — the Spirit realm is less stable, more fluid.

### Grave-Orchard Opening (rain)
Sky void-black with ash-pale rain-lines (thin, nearest-neighbor, no blur) · ground ash-grey/water-stain 50% mud · gravestones bone text on grey-mid stone · the grave-lantern torch-dull + torch-warm, small, guttering — the last light being extinguished · each Name-Shear beat reveals crimson-witness fracture-lines around the name being removed: where the institutional blade strikes stone, the light of being seen flares once, and fades as the name goes. *The color of loss is the color of being unmade.*

## QUANTIZE GATE

### Minimum Separation Rule
Any two colors assigned to different roles must differ by at least 20 points combined RGB distance to survive indexed-palette quantization: `sqrt((ΔR)² + (ΔG)² + (ΔB)²) ≥ 20`.

Example: ash-grey (42,42,42) vs slate-deep (26,26,26): distance ≈ 27.7 ✓.

### Dither Pairing Table

| Primary | May Dither With | Forbidden Pairing |
|---|---|---|
| void-black | slate-deep, ash-grey, void-edge | crimson-witness (sharp edge only, never blend) |
| ash-grey | void-black, slate-deep, parchment | magenta-wrong (institutional clean edges only) |
| crimson-witness | void-black ONLY | everything else; crimson is witness-pure |
| bell-bronze | parchment, marrow, ash-grey, metal-rust | phosphor-green (institutional vs. warm never mix) |
| grave-iron | ash-grey, slate-deep, metal-rust | torch-dull (cold vs. warm never dither) |
| parchment | bone, ash-grey, marrow, chalk-whisper | phosphor-green (text on text fails readability) |
| bone | ash-grey, parchment, chalk-whisper | crimson-witness (witness-light must be pure) |
| obsidian-dark | void-black, slate-blue, slate-deep | crimson-witness (mirror light is not witness light) |
| torch-dull | torch-warm, torch-shadow, marrow | void-black (candlelight never dithers into pure void) |
| torch-warm | torch-dull, torch-shadow, parchment | phosphor-green (terminal never touches candlelight) |
| phosphor-green | phosphor-dark, void-black ONLY | any warm or saturated color (the terminal is cold + clear) |
| slate-blue (#1A2D3D) | void-black, water-stain, slate-deep | torch-dull, bell-bronze (cold/warm never dither) |
| water-stain | slate-blue, void-purple, obsidian-dark | crimson-witness (water-light is not witness-light) |
| magenta-wrong | NONE — never dithers | all colors (wrongness is isolation) |
| old-root | marrow, ash-grey, soot-black | phosphor-green (natural vs. institutional never mix) |

## SCOTOPIC SAFETY

1. **Highest contrast** (visible in absolute darkness): phosphor-green + void-black.
2. **Secondary**: crimson-witness + void-black.
3. **Tertiary** (needs normal light): bone + void-black, parchment + ash-grey.
4. **Avoid for detail work**: dark saturated blues (rod cells barely see them; dark-blue detail vanishes in scotopic conditions). slate-blue is an *ambience* color, never a detail color.

## FINAL VERIFICATION (Every Color Earns Its Place)

| Color | Home Scenes | Status |
|---|---|---|
| void-black, slate-deep, ash-grey, grey-mid, void-edge | everywhere (ground, shadow, dither anchors) | Locked — foundational |
| crimson-witness, crimson-dark | Bell Pit, Grave-Orchard, Deep Ledger, Spirit Mirror, all Master Marks | Locked — structural truth |
| phosphor-green, phosphor-dark | terminal, ledgers, Index Ossuary | Locked — the daemon's voice |
| magenta-wrong | three fenced rooms only | Locked — fence enforced |
| bell-bronze, grave-iron, slag, metal-rust, marrow, old-root | Parish, Pit, Under-Orchard, forge, chains | Earned |
| parchment, bone, ash-pale, chalk-whisper | ledgers, graves, Spirit dust, ghost text | Earned |
| torch-dull, torch-warm, torch-shadow, clay-broken | shrines, lantern, hearths | Earned |
| slate-blue, void-purple, water-stain, obsidian-dark, salt-pale, root-alive, soot-black | Spirit spaces, canal, folds, burn scars, growth | Earned |
| rubedo-pulse, albedo-teal, Nigredo ichor, Citrinitas slate-purple | resonance moments only | Locked — event-gated |
