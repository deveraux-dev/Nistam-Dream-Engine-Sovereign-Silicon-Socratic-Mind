# TRIPTYCH — Dark Side of the Oz tribute, 3 scripts, 1 timeline
Sean Morin / 2026-08-20. Runtime 3:00 each, lockstepped to one shared timecode grid.
Thesis: Haha (laughter) / Ah! (introspection) / Ahhh! (shock) — three faces of one side, one at a time.

## Ground truth used (receipts)
- Audio bed: `F:\NewRepo\tools\youtube-forge\youtube-projects\chapter-1\mixed-audio.mp3` (44MB, 2026-07-14) — Sean's own Chapter-1 narration. NOT re-listened this session; beat timestamps taken from `VIRAL-VIDEO-PLAN.md` in the same dir (47:32, 48:22, 53:29, 1:00:47) and `Chapter 1.txt` transcript. [INFERRED] exact wording at each beat — re-cue against the transcript before cutting.
- Visual bed (Video B): `C:\Users\seanm\Desktop\Proofs.mp4` (23.4MB, 2026-08-20). Content unwatched by tooling — Sean cues.
- Verified numbers only (this session): 49/49 tests green; 12.05M arbitrations/s @ 37.4ns; 0 hot-path heap bytes; 40,000/40,000 sabotage repudiated; live gemini-2.5-flash audit @ $0.0004. The 6.42 Gtok/s figure is UNRECEIPTED — do not put it in any judge-facing cut.

## The trit mechanics
Three videos, one 3:00 timeline, same section boundaries. Each video is one channel of the same event.
Played alone: one complete lesson. Played together (A+B synced): the collisions land on the section
boundaries and the third lesson appears in the gaps — that IS video C's subject. Like the trit:
{-1, 0, +1} are not three things, they are one cell read three ways. You cannot occupy two at once.

SHARED TIMECODE GRID (all three scripts cut on these boundaries):
  0:00  COLD OPEN      (7s)
  0:07  THE CLAIM      (38s)
  0:45  THE TURN       (60s)   — kishotenketsu "ten"
  1:45  THE PROOF      (60s)
  2:45  THE SEAL       (15s)
  3:00  out

---

## SCRIPT A — "The Inspector" (judges · Haha · laughter)
The story cut. Warm, self-deprecating, zero jargon. Audio: mixed-audio.mp3 narration excerpts + music.

0:00 COLD OPEN — Black. One line of white stencil type: "I spent 23 years watching paint dry."
     Beat. "Professionally." (the laugh — earn it in the first 7 seconds)
0:07 THE CLAIM — B-roll: bridges, rust, gloved hand with a mil gauge. Narration bed (cue near 47:32):
     the 23-years-of-painting → building-software line, verbatim from the tape. On-screen:
     "Inspection reports are the only thing between a coat of paint and a structural failure."
0:45 THE TURN — The problem, told as a job story: the backdated log, the photo that migrates between
     jobs, the report edited after the fact. "The steel never lies. The paperwork does."
     Cut to: the Surface Ledger page, the FAIL specimen card. "So I built paperwork that can't."
1:45 THE PROOF — Screen capture, real terminal: `cargo test -p forge-envelope` → 49/49 green, live.
     The browser chain demo: attest three notes, click TAMPER, every seal after it breaks red.
     One number allowed on screen at a time: 40,000/40,000 forgeries repudiated. $0.0004 per audit.
2:45 THE SEAL — Sean, direct to camera or voice over the stamped footer: "Prep before paint.
     Seal before storage. No ending is silent." Title card: SURFACE LEDGER + DOI.

## SCRIPT B — "The Machine" (architects · Ah! · introspection)
The mechanism cut. Quiet, dense, no music until the seal. Visual bed: Proofs.mp4 captures + code.
Same boundaries, same narration bed underneath at -18dB (it's the same event, different channel).

0:00 COLD OPEN — Terminal only. `cargo test --test scale_test -- --nocapture` starts. No title.
0:07 THE CLAIM — The pipeline, one stage per breath: envelope → byte-sieve (5 trits/byte, 243+13
     sentinels) → Gemma triage → Gemini schema-locked verdict → weaver cross-check → SHA-256 fold.
     Code on screen: s13.rs packing, the sentinel range 243..255. No adjectives. Line numbers visible.
0:45 THE TURN — The design inversion: the intelligence is disposable, the *ledger* is sovereign.
     Zero-retention as a feature: EphemeralEnvelope tick-deadline, zeroize-on-resolve. Why no_std,
     why integer ticks, why the hot path allocates nothing (show: heap_allocations_in_hotpath_bytes: 0).
1:45 THE PROOF — live_scale_telemetry.json raw on screen: 1,000,000 arbitrations, 0.083s, 37.4ns.
     scale_test.rs attack classes scrolling: tick forgery, hash swap, genesis breach, expiry snoop —
     each with its assert. The real Gemini receipt JSON, unedited, gemini-2.5-flash, tick 1001.
2:45 THE SEAL — The chain-fold formula alone on black: H(prev ∥ tick ∥ disposition). "The chain
     only moves forward." Music enters for the first time. Same title card as A, same frame.

## SCRIPT C — "The Doorway" (the third · Ahhh! · shock)
Not a pandering explainer — the sync artifact. C is what exists only because A and B are locksteped.
Form: split screen, A left channel / B right channel, both audio beds live, mixed. At each shared
boundary the two frames rhyme (the mil gauge cut lands on the trit-packing cut; TAMPER-red lands on
the assert lines). C's own overlay is minimal — a third voice, sparse text cards in the gaps:

0:00 — both cold opens together. Card: "You can only watch one."
0:45 — at the shared TURN: "The story and the machine are the same event."
1:45 — as A's tamper-red and B's asserts fire on the same frame: "Laughter, introspection, shock.
        Three states. One cell. You cannot hold more than one at a time."
2:45 — both seals land on the identical title frame — the only moment A and B are the same video.
        Card: "That's a trit." Hold. Out.

Shock is not a jump-scare: it's the viewer realizing at 2:45 they were watching one video all along.
The Divine Comedy mapping: A=Paradiso register (light), B=Purgatorio (work), C=Inferno's function —
the descent that makes the other two legible. Dark Side of Oz rule honored: neither A nor B
references the other; the lockstep is discoverable, never announced inside A or B.

## Production notes
- Cut A and B against the SAME .mp3 bed so sync is free; C is an OBS/ffmpeg side-by-side composite,
  audio: A full left, B full right (headphone version), or A bed + B bed ducked for speakers.
- Judge submission gets A alone (3:00 limit-safe). C goes in the blog/social lane and the repo.
- Every on-screen number must appear in the MEASURED rows of surfaceledger/index.html. If it's not
  on that ledger, it's not in the video.
