# Nistam Dream Engine — the read

**372 words. Cap 185s.** Narrate from `docs/narrate.html`.
Voice rebuilt 2026-08-28 from `_Hamster.16k.txt` and `_30mPODCAST.16k.txt` (whisper base.en, ~46 min).
Structure and receipts unchanged from the prior draft. Register replaced.

---

## WHAT THE TRANSCRIPTS SAID

Measured from your own speech, not guessed.

| Trait | Evidence | Now used |
|---|---|---|
| **You say it twice** | "It's wild. It's actually wild." · "same idea, same idea." · "It's always this time. It's always this time." · "It goes down to money. It goes down to money." · "It's like a hamster wheel. It's like a hamster wheel." | Four doublings, placed on the lines that carry weight |
| **"Here's the thing" / "You know"** | Constant connective tissue | Kept, sparingly — enough to sound spoken |
| **Direct challenge to the listener** | "You got to think about that." · "Let that sink in a little bit." | Opens the peak |
| **Concrete jobsite specifics** | Bruce and the power tools · steel off a boat that sat in salt · Sandra not showing up | The steel is in the script |
| **You land on a humble image, not a slogan** | "It's like a hamster wheel. If I had to call it anything." | The close is now an image, not a carved line |
| **Contractions, always** | "they're", "I'm", "doesn't", "can't" | Throughout. The old draft had almost none, which is why it read like stone |
| **The math confession** | "geometry is math... I can touch it, I can feel it, I can see it... 62.34 divided by 6.92 to the power of — that's exactly when I got lost" | **This is now the centre of KI.** It explains balanced ternary better than the old technical version did |

**What I got wrong before.** I built the register out of `author-dossier.md` and `THE-DROP-LAW.md` — doctrine files where every line is deliberately compressed to bone. I matched the density of your *notes* instead of the cadence of your *speech*. The result was three minutes of carved aphorism and nobody talks that way.

**Pace.** Your measured rate across both files is ~125 wpm *including* your natural pauses. That's conversational rambling, though — reading prepared text you'll run faster, likely 140–150. At 145 wpm this lands **166s**. At 125 wpm it lands **191s and breaches the cap**. Rehearse once and read the drift number in `narrate.html`; it computes from your actual rate. The marked OPTIONAL CUT below is the release valve.

**Holds.** Mostly gone. Your speech already contains its own pauses — that's what makes 125 wpm gross. Stacking 26 seconds of authored silence on top of a speaker who already breathes was double-counting. Three pillows remain as the only true silence.

---

## KI · establisher

**FRAME 1** · black, then the VRAM ledger filling. Five rows.

> Five models. One card.
> All loaded at once. None of them waiting.
>
> That shouldn't fit.
> That should not fit.

**FRAME 2** · the roster, one row lighting per line.

> A nine billion one doing the heavy thinking.
> A small one that answers fast.
> Its mirror, trained to disagree with it, carrying no weights of its own.
> A codec.
> And a sentry, and all it does is watch.

**FRAME 3** · a page of floating-point notation. Then it clears.

> Here's the thing. I never got the math.
> I got lost the second somebody put a decimal, to the power of something, up on a page.
>
> But geometry I got.
> Because I can touch it. I can feel it. I can see it.

**FRAME 4** · three states, drawn big. Then five of them filling a byte.

> So I stopped writing the weights as numbers.
>
> Minus one. Zero. Plus one.
> You can hold that in your hand.

**FRAME 5** · 243 slots lit. 13 stay dark, then turn red.

> Five of them fit in a byte.
> Two hundred forty three combinations, and a byte holds two hundred fifty six.
>
> So there's thirteen left over.
> Thirteen values that can never be a weight. Not ever.
>
> So I made them alarms.

---

## PILLOW · 4s · tag `descent`

> seven gates, each one
> takes a garment — by the floor
> she is only voice

---

## SHŌ · initial

Three gates. Action-to-action. The count is load-bearing and resolves on the third.

**GATE 1** · the sentinel fires. Inference halts.

> Bad byte comes in. There's no state it can be.
> Nothing gets in.

**GATE 2** · refused at a fixed header offset.

> Malformed envelope comes in. Refused at a fixed spot in the header,
> before anything's parsed, before anything's allocated.
> Nothing gets in.

**GATE 3** · a phrase table. The rows for light aren't greyed out. They're absent.

> A body with no eyes has no words for light.
> Not forbidden. Not checked. There's just no branch to get to.
> Nothing gets in.

**FRAME 6** · a steel beam. Rust under the coating.

> I spent twenty three years around guys who'd sign off on steel
> that sat in salt on a boat. And guys who wouldn't.
> The paperwork looked the same either way.
>
> So I don't want a system that checks.
> I want one that can't do it wrong.
>
> A rule that something has to check is a suggestion.
> That's a shape.

**FRAME 7** · the suppressor with nothing left to act on.

> And turn it around, it's not even a defence.
> You can't blind something that doesn't see.
> You can't leak what you never kept.

**FRAME 8** · `./scripts/demo_cloud_agent.ps1` running. Bucket, sieve, Vertex AI, Firestore, scrub.

> There's a watcher sitting on a bucket. It's not a chatbot.
> It filters before it spends a token, asks Gemini 2.5 Flash on Vertex AI
> at temperature zero, writes the chain head to Firestore,
> then wipes its own staging.
>
> Nobody's in the loop.

---

## PILLOW · 4s · tag `samizdat` — **FIRST CUT if you run long**

> copied by hand, twice
> wrong — so I say it three ways;
> one survives the fall

---

## TEN · PEAK

**FRAME 9** · `docs/BENCHMARKS.md`. A table of commands. The Result column is empty.

> And then the same thing came back around on me.
> You got to think about that for a second.
>
> I'm not putting a benchmark number on this screen.

**FRAME 10** · the empty column, held.

> Every number I could show you was true on my machine.
> One day. One temperature.
> In a pitch it stops being a measurement. It becomes a claim.
>
> I didn't delete the old ones. They're in the repo, dated.
> Superseded, not removed.
>
> So the write up has no figures. It's got commands.

**FRAME 11** · split three ways: the sentinel byte, the absent phrase row, the empty column.

> A byte that can't be a weight.
> A body that can't say light.
> A number I can't keep true.
>
> Three gates, same shape. Three gates, same shape.
> And the third one's me.

---

## PILLOW · 4s · tag `openframe`

> outer frame left open
> on purpose — the moral walks
> out and finds the reader

---

## KETSU · release

**FRAME 12** · `cargo test --workspace` scrolling. Music enters.

> Five models. One card.
> Thirty eight crates, and the core doesn't allocate anything.
>
> Built by one guy in a room.

**FRAME 13** · dark. Title card and DOI.

> Don't take my word for it. Clone it, run it.
> The only number you have to trust is the one your machine prints.
>
> That's the whole thing. If I had to call it anything.

---

## WHAT SURVIVED FROM THE OLD DRAFT

Everything structural. Only the voice changed.

- **The three gates**, resolving on the third — Drop Law's load-bearing repeat count
- **Peak in TEN, initial in SHŌ** — Peak never opens, initial never closes
- **The turn is a new element** — the author under his own gate, not an argument against acts one and two
- **The close re-reads the open** — "five models, one card" returns after the meaning moves
- **Three pillows**, ORDER positions 1 / 4 / 8, never adjacent, closing on `openframe`
- **Every receipt** — unchanged, listed below

## WHAT CHANGED BEYOND VOICE

| Change | Why |
|---|---|
| The math confession is now the centre of KI | Your own explanation beats mine. "I can touch it, I can feel it, I can see it" *is* why balanced ternary. It arrives from your life instead of from a spec |
| The steel-off-the-boat story enters SHŌ | Your own analogy for signed-off bad material. It sets up "I don't want a system that checks" with something a judge can picture |
| "Three gates, same shape" is doubled | Your most distinctive verbal move, placed on the line that carries the most |
| The close is an image, not a slogan | "If I had to call it anything" is how you actually end a thought. Compare your hamster-wheel close |
| Authored holds removed | Your 125 wpm already contains your pauses. Adding 26s of silence on top double-counted them |

## RECEIPTS

| Line | Source |
|---|---|
| Five models, one card | `crates/gemma-s13/src/vram_budget.rs:196-204` · `DEMO_FLEET: [FleetMember; 5]` |
| The roster, seat by seat | same file:196-197 — "9B backbone, direct 2B, its anti-expert mirror (same directory, shared weights), the manifold codec, and the sentry" |
| "carrying no weights of its own" | same file:198-204 `shares_weights: true`; :226-232 counts a shared slot once. It still holds its own KV cache, :234-237 |
| All five at once | `crates/studio-tauri/src/main.rs:184-195` — "all five models resident, 4k of shared context, i8 KV" |
| 5 trits to a byte, 243 states, 13 spare halt inference | `docs/DEVPOST.md:12-16` |
| Gate 2, fixed header offset before alloc or parse | `docs/DEVPOST.md:29-31` |
| Gate 3, no words for light | `umwelt-sense.rules.md:109-127` · `gating.is.structural.not.checked`, gate `a_blind_body_has_no_words_for_light` |
| "A rule that something has to check is a suggestion" | same file:114 — "a prohibition nothing verifies is a suggestion" |
| "That's a shape" | `author-dossier.md:22` — "enforced by SHAPE, not policy" |
| "You can't blind something that doesn't see" | `forge-envelope/docs/umwelt.txt:13` — "the dark is simply not a category that applies to her" |
| Landed as Rust | `forge-mud-v3/src/magic/umwelt.rs:294-301` — `Form::Lich` sets `shadowed_q: 0` when it gives up `sightline_q` |
| "You can't leak what you never kept" | `docs/DEVPOST.md:16-18,44` — ADR-0026 zeroize on tick deadline, staging wiped after ack |
| The watcher | `docs/DEVPOST.md:35-45` |
| "Superseded, not removed" | `docs/BENCHMARKS.md:7-9` |
| The empty column | `umwelt-sense.rules.md:29-47` · `absence.is.never.asserted` |
| 38 crates, core is `#![no_std]`, zero heap | `docs/DEVPOST.md:70` |
| Three haiku, verbatim | `F:\NewRepo\tools\storydrop-forge\haiku.py:2,5,9` · ORDER at :12 |
| The math confession, the steel, the doublings | `C:\Users\seanm\Desktop\_Hamster.16k.txt:41-45,56-63` and `:9,26,96,113,137-138` |

## THE MODEL STRING — SWEPT 2026-08-28

Real models are 2.5 Flash and 2.5 Flash Lite. The 3.7 string names nothing; neither do 3.5 or 3.1.
This section avoids writing the dead ids literally so a future sweep doesn't rewrite its own account
of the defect — which is exactly what happened on the first pass.

Two live call sites defaulted to a model that does not exist: `scripts/vertex_flash_cache.py:52` and
`crates/forge-envelope/scripts/agent_loop.py:177`. Both read an env var first, which is set on Sean's
machine, which is why `audit_receipt_to_attest.json:1` shows a real `gemini-2.5-flash` call and why
this never surfaced locally. A judge cloning the repo has no such variable. `demo_cloud_agent.ps1`
runs with `--require-cloud`, which disables every fallback by design — so the 1-click demo hard-failed
rather than degraded. Both now default to `gemini-2.5-flash`.

51 files, 119 lines. Manifest: `docs/MODEL-STRING-SWEEP-2026-08-28.md`. Mapping: anything the code
already marked `-lite`/`Lite` → `gemini-2.5-flash-lite`, everything else → `gemini-2.5-flash`.

`crates/forge-envelope/scripts/forge_lint.py:25-26` excluded on purpose — it is the banned-string
detector for this exact defect and its keys must stay wrong. It already existed and would have caught
all of this. It was never run over these files. Wiring it into the gate is the real fix.

**Verify, don't assume.** `surfaceledger/live_scale_telemetry.json:15` is a generated receipt whose
model field was hand-edited in the sweep. An edited receipt is not a receipt — regenerate it from a
real run. And `atg.rs:17` plus its asserts at `atg.rs:138` / `main.rs:92` moved together so tests
should pass, but run `cargo test --workspace` rather than taking that from this page.
