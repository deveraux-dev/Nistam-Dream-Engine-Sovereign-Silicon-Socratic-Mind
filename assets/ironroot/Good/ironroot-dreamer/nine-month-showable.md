# NINE-MONTH SHOWABLE & MEDIA/LoRA PLAN — IRONROOT

## 1. DEMO SCRIPT: The Proof (3–5 min, terminal + Knife 2D)

**What the showable demonstrates:**
- Terminal ritual: player opens a Book, inscribes a vow ("never abandon the grave-name"), sees the daemon respond with a SoulWord hash.
- The crack: cut to Thornbell Parish (Knife camera). The grave-orchard stands marked by a small hand-built lantern. Wind shifts. Toll-Sister Vey reads a ledger entry: "*Debt Unpaid: Burial of Morrow. Iron tithe due.*"
- First loop: player engages The Bell Pit (arena, 13-bell rhythm). Parry the toll-pressure. A Shadow forms at the player's back—a stain in the air, scarred with chain-marks. Fight ends: the 13th bell does not ring.
- Toll: cut to the grave-site. The lantern extinguishes. A Name-Shear sound (blade-through-bone transient) tears across the audio space. The spouse-name chisel-mark vanishes from the grave-stone.
- Hearth return: player re-enters The Under-Orchard Root Cellar (nested refuge). Kindles a new fire in a cracked ceramic bowl. A debt-spirit manifests—neither enemy nor ally. "What name do you carry now?"

**What it does NOT show:**
- No world-scale plot; no exposition; no ledger tutorials.
- No faction warfare or city politics (Ledger Church, Index Monks stay off-stage).
- No Spirit-realm dive or endgame cleanse-audits.
- No mercy-ledger resolution; no weapon crafting.
- Silent on the 12 Alchemical Gates; progression implied, not explained.

**Gate:** Player can parry at least two toll-pressure attacks and survive the Name-Shear audio/visual punch without breaking silhouette.

---

## 2. STILLS MANIFEST: Key Frames + Loops

| Working Title | Source Frame Series | Purpose | Palette Mood | Gate |
|---|---|---|---|---|
| **Grave-Lantern Vigil** | T01 / Opening monologue | Press lead; hero image; "what will they protect?" | Ancient gold on void-black, hand-wrought iron lantern detail | Silhouette reads at 1-bit; lantern flame is crimson witness-light only |
| **The Toll-Sister's Ledger** | T03 / Ledger reading | Storefront / pitch; institutional dread | Decay grey, black wax threshold, page-spine detail | No anti-aliasing on page edges; indexed to master palette |
| **Bell Pit Descent** | F07 / Arena entry | Cinematic beat; "the city has clocks" | Void-black floor, bell-bronze ceiling mechanism, descending architecture | Bell geometry reads as integer grids; no gradients |
| **Shadow Stain: First Scar** | L15 / Combat aftermath | Marketing moment; the price of fighting | Crimson fracture-lines in a boiling absence | Scars follow player's parry-rhythm; deterministic |
| **Under-Orchard Root Cellar** | F22 / Hearth rest | Refuge visual; "there is still warm ash" | Black earth-root, cracked ceramic bowl, low flame orange (not golden) | Bowl edges show deliberate dither; no smooth gradients |
| **The Name-Shear Prelude** | T06 / Chisel-mark | Dread beat; "administratively unhappened" | Void-black stone, weathered chisel-point catch, grave-marker edge | Type-readback: chisel mark disappears pixel-exact; no animation blur |
| **Thornhaven Market Row** | E04 / Daytime terrace | Breadth of the MVP zone | Bell-bronze market poles, cloth awnings in bone white / grave-iron stripe | 16:9 horizontal; horizon line at 1/3; no people in frame |
| **Cathedral-Fortress Overhang** | E07 / Upward approach | Scale; the Molt Cathedral in distance | Decay ash-grey stone, root fracture patterns, void-black shadow underside | Fractals stop at 3 iterations; no infinite detail |

**Three Short Loops / Animatics (20–40 sec each):**
1. **"The Tolled Heartbeat"** (L-series frames 10–20): Toll-Saint chain-rattle cut short; Bell Pit chains swing; player's sovereign heartbeat pulses the stone glow to crimson; 120Hz tick made visible as light-tick.
2. **"Ash-Bound Ascension"** (T-series 12–18): Knife ascends (toggle state); silhouette expands with ash-veil, vow-suture lines trace up the armor; NO beast form, NO horn-crown, just ash-split cloth and iron. Toggle off: silhouette snaps clean.
3. **"The Book Writes Itself"** (T01–T04): Daemon loop: `OPEN BOOK → SET LAW → RUN GAUGE` commands trace as phosphor text on void-black terminal; SoulWord hash glows, event ledger appends, a small Tombstone marker appears in the world. Player sees their action recorded in the game's own ledger.

---

## 3. LoRA PLAN: Training Set, Captions, Iteration Ladder

### 3.1 Training Set Manifest (from on-disk corpus)

**ANCHOR ASSETS (style-locked, enter training as-is):**
- All 2dak palette sheets (master palette is the gate; see quantize-lock §3.3)
- C01–C08 character concepts (hand-studied poses: guard, hitstun, casting, dash; no zodiac context; Root-Pruner silhouettes only)
- E01–E08 environment plates (forest / camp / spirit mirror / boss arena / celestial bastion / canopy vista; void-black backgrounds only)
- Thornhaven 4-era renders: gate, inn, market, courtyard, cellar (golden / ancient / decay / void moods; all 16:9)
- Cathedral-fortress + Under-Orchard blueprint diagrams (orthographic, no perspective cheat)
- Faction banners (Ledger Church bell cracking, Toll-Saints chain rhythm, Root-Pruners blade transient; geometry only, no animals)

**DONOR-ONLY ASSETS (re-skin before training):**
- Zodiac-named sprite folders (aries–pisces; pose/silhouette reference only)
  - Map to: rooted deserters (ash-bound human posture), fungal oath-effigies (cloth-wrapped forms), ash-warped debtors (labor-hunched silhouettes)
  - Animator's eye: keep attack/dodge/cast rhythm; strip animal markers; substitute material damage (cracked armor, trailing ash, sutured wounds)
- Any wolf / beast art (same rule: re-skin as stitched armor husks with trailing root-matter; pose carries forward, subject does not)

### 3.2 Caption Schema (stable vocabulary for LoRA training)

**Required fields per image:**
```
ironroot style, hi-bit pixel art, 
void-black ground, [mood: golden | ancient | decay | void],
[subject: character silhouette | architectural plate | faction glyph],
crimson witness-light (where applicable), dithered indexed palette, nearest-neighbor, 
dark fantasy, handcrafted, [specific detail: toll-chain geometry | root fracture | bell-bronze seam | cracked ceramic],
NO anti-aliasing, NO neon, NO sparkle, NO gradient, NO aurora
```

**Forbidden tokens in ANY caption (hard ban for LoRA):**
- zodiac, constellation, animal, beast, wolf, eagle, horned, aurora, neon, sparkle, pristine, watermark, text-overlay, anti-alias, gradient, sRGB

### 3.3 Iteration Ladder (budget-gated, stage gates mandatory before spend)

**STAGE 1: Cheap / Local Exploration (FREE, immediate, throwaway)**
- Objective: Composition drafts, mood verification, beat-frame mapping
- Tools: free local models (ollama / comfy-ui), low-res (512×512), runs per image <2 min
- Gate before: None; rapid iteration; delete after approval
- Iteration: Max 50 draft images across the 8 stills + 3 loops; one pass per working title
- Acceptance: Sean eye-judges 1-bit silhouette legibility; mood hits the era-tint target

**STAGE 2: LoRA Pre-training on VertexAI GenAI Credits (PAID, once per phase, deterministic)**
- Objective: Style corpus quantize-lock and LoRA training run
- Gate BEFORE spend: Every training image must pass Sean's hand-approval; image must survive reduction to master palette with no loss of recognizability
- Corpus prep: C01–C08 + E01–E08 + Thornhaven 4-era + blueprints + banners, re-skinned donor assets, master-palette quantized in-place, captions finalized
- LoRA train: 500–1000 training steps, batch size 1, learning rate 1e-4, adapter rank 32; checkpoint saved
- Credit burn: ~$12–18 per run (estimated)
- Acceptance: Inference test on a held-out prompt (e.g., "a cracked ceramic bowl on void-black, ironroot style") must return indexed, dithered, no-gradient result
- Result: Frozen LoRA weights; locked for the 3-month phase; any request for re-training requires a new month-3 / month-6 milestone gate approval

**STAGE 3: MVP-Quality Final Passes (PAID, stills-only, high-resolution)**
- Objective: 8–12 final stills at 1080×1920 for press, storefront, pitch materials
- Budget: 30 VertexAI high-res inferences per still (max 360 credits reserved)
- Gate BEFORE spend: Stills Manifest locked; captions reviewed; LoRA weight frozen; Sean confirms mood-target and source-frame mapping
- Process: Use frozen LoRA; iterate on prompt variation and negative-prompt tuning; semantic-search the best 3 outputs per still; Sean hand-reviews pixel-for-pixel (quantize score, silhouette clarity, banned-token absence)
- Acceptance: All 8–12 stills pass (1) quantize-lock to master palette, (2) silhouette reads at 1-bit, (3) no banned imagery, (4) caption round-trips (given final image, captions can be auto-generated and match the original)
- Rejection gate: Any still that fails (1) or (2) burns one full re-iteration budget; three rejections on one still = roll forward to next phase or substitute with a Stage 1 hand-painted draft

**CRITICAL: No Stage 3 budget is spent before Stage 2 is locked. No Stage 2 is trained before Stage 1 drafts are Sean-approved.**

---

## 4. CONSISTENCY GATES: Proof of On-Look

An image is LIVE and ship-ready only if ALL five pass:

1. **Quantize-Lock**: Reduce to master palette (void-black, crimson, bell-bronze, grave-iron, bone, ash) using nearest-color quantization. No visible banding or loss of silhouette legibility. ✓ = passes
2. **1-Bit Silhouette**: Render at 1-bit (black/white only); character, object, or architectural edge must remain recognizable and anatomically plausible (no alien limbs, no floating parts). ✓ = passes
3. **Banned-Imagery Clean**: Scan for aurora, neon, sparkle, gradient, anti-alias artifacts, watermarks, text overlays. Zero hits = pass.
4. **Caption Round-Trip**: Given only the final image, generate captions from stable vocabulary. Auto-captions must semantically match the original brief (mood, subject, detail). Mismatch signals training-set leakage or LoRA collapse.
5. **Type-Readback** (terminal frames only): If frame contains terminal phosphor text (daemon commands, ledger inscriptions), render at intended resolution and verify pixel-exact character shapes against the terminal face defined by the typography law in *glyphs-type-and-ui.md* (EB Garamond + humanist sans; mono seam pending Sean's pick). ✓ = passes

> Note: the "Source Frame Series" mappings in §2 are working references authored in parallel with the frame lanes; re-anchor each still to its final frame number during the Month-1 review.

Failure of ANY gate = image is rejected and removed from final manifest. Rejection of >30% of a stills batch triggers a phase re-plan (rolling to next month or reducing scope).

---

## 5. MONTH BEATS: Authoring Milestones

| Month | Deliverable | Gate | Success Proof |
|---|---|---|---|
| **Month 3** | Stage 1 drafts (all 8 stills + 3 loops sketched; 50 throwaway images explored) | Sean approves silhouettes and mood-tints on ≥6 of 8 stills; 1 loop animatic plays ≥24 fps clean | Review session with Sean; approved drafts locked in dated directory |
| **Month 6** | Stage 2 LoRA trained; Stage 3 preview stills (3 of 8 final stills at high-res; MVP frame only) | LoRA inference passes acceptance test; 3 preview stills each pass quantize-lock + 1-bit + caption round-trip; Sean eyes each | LoRA weight file signed; preview stills in project deliverables folder; Sean sign-off email |
| **Month 9** | Stage 3 complete (all 8–12 final stills + 3 loops finalized; press-kit ready) | All stills pass all 5 consistency gates; loops render >30 fps on target hardware; Sean final review | Showable ships; stills folder ready for press/storefront; media links in game.toml |

---

## 6. BUDGET SUMMARY & RISK GATES

**Credit Budget (VertexAI GenAI):**
- Stage 2 LoRA train: $12–18 (once per 3-month phase) × 3 phases = $36–54 reserved
- Stage 3 high-res stills: $250–360 (360 inferences; only months 6 & 9) = $250–360 reserved
- **Total: $286–414 across 9 months**

**Risk Gate: Quantize-Lock Failure**
If >30% of Stage 1 drafts fail to quantize to master palette, the palette itself is re-examined (signal: model has learned colors outside the canonical set; either restrict training set further or expand palette). Decision point: end of Month 1. Cost: +1 week planning, +$12 for re-train.

**Risk Gate: LoRA Collapse**
If Stage 2 inference produces images that fail caption round-trip (e.g., generates astrology symbols despite captions banning them), the training set is contaminated. Action: audit donor-asset re-skinning; re-train Stage 2 with corrected corpus. Cost: +$18, +1 week.

**Risk Gate: Silhouette Ambiguity**
If >2 stills fail 1-bit readback (silhouette becomes illegible), composition is fundamentally off-target. Action: revert to Stage 1, iterate on framing. Cost: 10 draft days; re-run Stage 2 test.

---

## 7. Sean's Final Gate (MANDATORY)

Every image is delivered to Sean for **hand review** before ship. He judges:
- Does this image land the mood of the era (golden / ancient / decay / void)?
- Does silhouette scream "Ironroot," not "generic dark fantasy"?
- Are there any hints of the banned list (aurora shimmer in the corner, a subtle gradient, an anti-alias halo)?
- Does the image feel **painted**, not **computed** (craft visible, not noise)?

Rejection path: Image is pulled; Stage 1 alternate drafted or prompt is tuned; re-run inference and re-check. No image ships unsigned by Sean's eye.

