# HANDOFF — All Things Agentic Hackathon

**Written 2026-08-21 · Submission closes 2026-08-31 17:00 PT · 10 days**
Companion: `Rules-Benchmarks.md` (same directory) — full rules compression, 14 sections.

**Cold-start summary.** Sean (Cree, Alberta, 2748684 Alberta LTD, heysunny.ca) is entering a Google/Devpost agentic-AI contest. The submission is a **polysynthetic morphological lexer** for Plains Cree, wired into HeySunny (his existing eldercare voice companion) as new in-window work. Nothing is built. The architecture is settled. The blockers are administrative.

---

## 1. THE ARTIFACT

**Polysynthetic Morphological Lexer** — an FST-based lexer that segments Cree words into morphemes, because in a polysynthetic language a whitespace token is an entire clause and every pipeline downstream of a whitespace tokenizer is parsing garbage.

One component, four functions:

| Function | What the lexer provides |
|---|---|
| Anti-hallucination | defines the legal emission space |
| 5D lattice | each analysis *is* a coordinate |
| Clinical biomarker fix | TTR, pronoun/noun ratio, pause boundaries computed on morphemes not whitespace |
| Multimodal | shared alignment across SRO, syllabics, audio |

**The Twist (5A, 40% weight):** vocal/linguistic biomarkers for cognitive decline are calibrated on English morphology. A Cree-speaking elder screened with that instrument is measured with the wrong ruler. The lexer closes that gap.

## 2. ARCHITECTURE — corrected, buildable

**⚠ Vertex/Gemini does NOT support GBNF or per-token logit masking.** It supports controlled generation via `responseSchema` (structured JSON) only. Any plan built on "logit-masked Gemini emission" is not implementable on the required platform. This invalidated the original design.

**Corrected pipeline — buildable on Vertex today:**

```
Gemini 3.5 (Vertex, responseSchema)  →  emits a COORDINATE, never Cree characters
       {"class":"VTA","actor":"1SG","goal":"3AN","order":"independent"}
                   ↓
Clingo / ASP                          →  integrity constraints reject illegal coordinates
                                          (obviation, direction on 2>1>3>3′ hierarchy)
                   ↓
FST (generation mode)                 →  the ONLY thing that emits Cree text
                   ↓
                niwâpamâw
                   ↓
.glb lattice                          →  witness surface — legal cells lit, holes dark
```

Hallucinated Cree is impossible because **Gemini is never in the Cree character-generation path.** Stronger than masking, and it ships.

**Division of labour is compiler-theoretic:** lexer (FST, regular) + constraint solver (ASP, beyond-regular). Obviation and direction are long-distance dependencies *within* a single token — past what regular power handles. That's why Clingo earns its place.

**`.glb` is transport, not constraint.** Constraints are discharged by the solver before anything reaches the container. Satisfies WAVE_CLOSE=PHOTON.

## 3. VERIFIED — receipts exist

| Fact | Receipt |
|---|---|
| Rules: 20 hard gates, 8 deadlines, 3 mandatory stack items, 13 deliverables, 16 prizes | `Rules.txt` L29–504 → `Rules-Benchmarks.md` |
| `heysunny.ca` = 3,373 files / 1,170,663 lines, last commit `a72e024` 2026-04-06 | `…\manifests\heysunny.ca.json` |
| Plains Cree FST exists: ~16,500 stems, ⅔ verbs, ALTLab + Giellatekno | github.com/giellalt/lang-crk |
| FST ships **prebuilt** `.bhfst`/`.zip` releases — no HFST/Foma/autotools build | same, releases page |
| **lang-crk license UNRESOLVED** — README reads "licenced under LICENSE licence" (unfilled template), no SPDX | verified by fetch |
| `morphodict` = **Apache-2.0** | github.com/UAlbertaALTLab/morphodict |
| itwêwina partners: ALTLab + First Nations University + **Maskwacîs Education Schools Commission**; sources = Wolvengrey + Maskwacîs + Alberta Elders' dictionaries | altlab.ualberta.ca/itwewina |
| `letter_morphometrics()` — glyph stroke anatomy from alpha mask, zero ML, **12 tests** | `morphometric_smooth.rs:187–314`, tests `:316–454` |
| `GlyphRegion` enum declared `:172`, **never used** (function pushes strings) | same file |
| `CREE.md` is NOT a morphological derivation — doctrine-symbol scheme, self-shelved HORIZON | `CREE.md:87` |

## 4. UNVERIFIED — do not build on these

- **`cree_syllabics.rs`** — cited in `CREE.md:5`, never located. L22b violation. Find it or drop it.
- **All effect sizes** — Cohen's d=0.96, SMD=1.56, 81.69%, $4.43/$1, 19%/12%. Via NotebookLM over 14 unread sources. **Cite primaries or don't cite.**
- **Morpheme-level biomarker novelty** — plausibly the strongest IP claim, never searched for prior art. Search before writing it down.
- **pexil / penteract** — no implementation seen. `gamedev-drive` marks Ump128/Morton key UNPROVEN; Pose5D PROVEN.
- `morphometric_smooth.rs` — read from a July `E:` snapshot, never compiled in a live tree.
- **G2 residency** — the gate that voids everything else.
- Whether the HeySunny research corpus contains `sunny-sessions` transcripts (blocks any Vertex upload — Alberta HIA makes vendors affiliates; patient data into commercial model training is expressly prohibited).

## 5. DECISIONS

| Decision | Rationale |
|---|---|
| **Category: C3 Fortified Enterprise Fleet** (flipped from C2) | 12-week Life Review = "weeks of async"; TELUS CHR = "official enterprise infrastructure"; HIA/PIPA/#DataBack = "data sovereignty". Weak link: cross-department cataloging. Fall back to C2 if the demo collapses to one conversation. |
| **G8: HeySunny = disclosed pre-existing dependency; Cree layer = the new work** | Line 118 permits frameworks/libraries; disclosure is not a penalty. Cannot submit HeySunny itself. |
| **Gemini emits coordinates, FST emits Cree** | Vertex has no GBNF. See §2. |
| **Drop Echo/Alexa/AWS front-end** | Reports 5 & 11 are an Amazon pitch. A16 bars third-party branding. Use Pi 5 + ReSpeaker + Coral. |
| **Wellness positioning only** | Never diagnostic (Class II device risk). Beacon supplementary to 911, never a replacement. |
| **No elders on camera** | Their voices would enter Google's perpetual promotional license (§12). Use Sean's own voice. Keeps community audio out of the submission entirely. |
| **Fund: spiral = growth schedule, not recipient lottery** | Geographic selection under-serves clustered need. ZKP is privacy, not governance — the credential *issuer* is the real governor and must be named. Forever needs corpus, not 2% flow. |
| **Repo compression rejected** | 5 of 6 submission components are net-new. `CREE.md:78–87` already ran this experiment: chars ≠ tokens, one-shot is a net loss. |

## 6. PITCH BOUNDARIES

**Cite as prior art:** two-level finite-state morphology, Giellatekno/ALTLab FSTs, 20+ years across dozens of languages. Claiming the FST as novel fails instantly against a domain-literate judge.

**Claim as novel:** (1) FST-derived constraint over polysynthetic morphology in a low-resource setting — *narrow*, since grammar-constrained decoding is itself published work; (2) morpheme-level acoustic/linguistic biomarker extraction — **strongest claim, unverified**; (3) the spatial-linguistic coordinate mapping — *weak, demote to design note*.

**The isomorphism table is 2-of-5.** Real: *Lexer* (deterministic segmentation of a continuous signal — `letter_morphometrics` does this on glyphs, FST on morphemes) and *Constraint Solver* (literally the same Clingo on two domains). Forced: *Raw Input* (vacuous), *AST/Lattice* (equivocates discrete categorical tuple with continuous vertex algebra), *Decoder Mask* (a render pipeline draws, it doesn't constrain — position in a pipeline isn't structural identity). **Say "shared front-end discipline," not "isomorphism."**

**Overclaim risk:** framing this as "fundamental AI infrastructure" makes judges evaluate it *as* infrastructure — benchmarks, evals, generalization. A 10-day demo can't answer that. Say the lexer solves a real structural problem here and the approach generalizes. Same trap as "proven 100%," better suit.

## 7. BLOCKERS — asked 3×, unanswered

1. **Pi 5 + ReSpeaker in hand?** No shipping room.
2. **Is Alexa on the team?** A12: all members on Devpost, one Representative appointed. Also a G8 question if HeySunny is partly their work.
3. **G2 residency confirmed?** Quebec excluded. An Alberta entity is not proof of personal residence.

## 8. NEXT ACTIONS — ordered

1. **Confirm G2.** Everything else is void if this fails.
2. **Send the ALTLab email** → `altlab@ualberta.ca` (Dr. Antti Arppe is the principal; they prefer the general address). Draft exists — discloses the Google promotional license deliberately. **Do not also email `21c.tools@ualberta.ca` yet** — partnership is a conversation, not a cold email. Get the narrow license answer first.
3. **D1 credit form** — Aug 28 12:00 PM PT cutoff, 72 *business* hours review → effectively overdue.
4. **Stand up `sean@heysunny.ca`** — sole remaining gate on Startup Excellence ($20k + $5k).
5. **Vertex hello-world returning one JSON coordinate.** An afternoon. Converts the whole design into a spine.
6. **S1+S2+S3 deployed** — Gemini 3.5 via Vertex + ADK + Cloud Run, live, before any feature work. Missing one = Stage One fail regardless of quality.
7. **Then build:** one VTA verb paradigm, Clingo-enumerated, FST-realized, lattice baked to GLB, lighting on match, refusing on illegal.

## 9. SCORE MATH

```
B = 0.40·Innovation + 0.30·Architecture + 0.30·Demo      B ∈ [1,5]
F = B + bonus                                             bonus ≤ 1.0,  F ≤ 6
```

- **Full bonus (+1.00) = the exact delta from straight 4s to straight 5s.** One is a blog post, a tweet, and 3 model integrations. The other is beating 6,296 people on every axis.
- **Base caps at 5.00; bonus stacks past it.** Straight 5s with no bonus (5.00) loses to 4.2 with full bonus (5.20). Skipping bonus removes you from the winning range arithmetically.
- **Innovation is double-counted** — 40% weight AND first tiebreak.
- **Target: 4.5 / 4.5 / 4.5 + 1.00 = 5.35.** Winnable in 10 days. Straight 5s is not.

## 10. FILE ANCHORS

```
C:\Users\seanm\Desktop\Rules.txt                                    source rules
C:\Users\seanm\Desktop\Rules-Benchmarks.md                          compressed, 14 §§
F:\NewRepo\_vault\output\research\context\manifests\heysunny.ca.json
F:\NewRepo\_vault\output\research\research\heysunny-elder-research-dump.md
F:\NewRepo\_vault\output\research\research\heysunny-reports-2-through-6.md
F:\NewRepo\_vault\output\research\research\heysunny-reports-7-through-10.md
E:\.airgap\NewRepo-source-2026-07-01\crates\forge-vision\src\scan\morphometric_smooth.rs
E:\.quarantine\md-dedup-2026-07-14\E\13forge-super\_plans\archive\CREE.md
E:\.quarantine\…\root-docs\OMRlogic.md                              GBNF Crucible @ L38
E:\13forge-super\_merged\airgap\2026-06-07-memory-repair\shadowseer-is-ml-observer-sightmap-is-morphometric.md
E:\13forge-super\_scratch\py\morphometrics_scraper.py                12 Treaty-6 species
github.com/giellalt/lang-crk                                        FST — license unresolved
github.com/UAlbertaALTLab/morphodict                                Apache-2.0
```

## 11. ERRORS THIS SESSION — read before trusting anything above

- Cited `cree_syllabics.rs` as salvageable infrastructure without ever locating the file.
- Framed "five prize lanes" as if eligibility were proximity to winning. Eligible for five with nothing built is worth zero.
- Called Best Multimodal UX "the softest target." No rubric means *unpredictable*, not easy. Inverted.
- Said `morphometric_smooth.rs` has 14 tests. It has 12.
- Let architecture accumulate across five expansions (FST → GBNF → morphometrics → 5D lattice → lexer framing) with **zero implementation**. Generated design and called it progress.
- Treated Cree provenance as an outsider-consent question before learning Sean is Cree. Community-protocol framing was wrong as applied; ordinary third-party software licensing still stands.

**Every turn closed STATIC: none / RUNTIME: none except the FST verification.** A successor should refuse to add architecture until §8 item 5 has a receipt.
