# MASTER REEL, 80 seconds, dual-track kishotenketsu
Sean Morin / 2026-08-20. 1920 frames at 24 FPS, 60 BPM metronome. Supersedes the 3:00 cut.
Left = money. Right = metal. Center = the channel the collision makes, over the 1200s drone.
Acts: Ki-Sho 0-48s (#1AE0FF), Ten 48-68s (#FF3B6E), Ketsu 68-80s (#4DFFB0).
Ratio: 48 / 20 / 12 = 60 / 25 / 15. Kishotenketsu holds.
FPS: 24, not the Drop Law 20. A 500 ms kept column is 12 frames. Every cut lands on a multiple of 12.
Register: lived, present tense, plain. No jargon on any channel. The trade talks, the trade wins.

## ACT 1 · KI-SHO

### S1 The Creep · 0.0-12.0 · frames 0-288
- L: "An audit costs money because nobody trusts the paperwork."
- R: "Steel gives up slowly, under the coating, where nobody looked."
- C: "Nobody trusts the paperwork, and the steel gives up underneath it."
- V: aerial over the Walterdale arches. Wireframe grid, a red counter climbing in dollars.
- Asset: `assets/shots/20170624_104506 (1).jpg`, `assets/shots/20251201_120106.jpg`.

### S2 The Bedrock · 12.0-24.0 · frames 288-576
- L: "A small sentry sits on the asset and reads it before anything leaves."
- R: "We listen to what the steel and the concrete are doing right now."
- C: "A sentry sits on the steel and reads what it is doing right now."
- V: extreme close-up, sensor clamped to sandblasted steel. 120 Hz wave over raw bitfields.

### S3 The Collapse · 24.0-36.0 · frames 576-864
- L: "The photo stays on the device. Only the state travels."
- R: "Five trits fill a byte. The picture becomes a number."
- C: "The photo stays. The picture becomes a number."
- V: a site photo collapses into a 16-byte UmpWord. Sieve-13 shows [0, +1, -1, 0...].
- BADGE: `5 TRITS / BYTE · 243 STATES · 13 SENTINELS`. Measured. See NUMBERS.

### S4 The Escalation · 36.0-48.0 · frames 864-1152
- L: "When something is wrong, it goes up to Gemini."
- R: "The buffer wipes itself when its time runs out. The link is what remains."
- C: "The buffer wipes. The link goes up to Gemini."
- V: buffer flashes to 0x00 via SIMD `.zeroize()`. A line runs to the Vertex AI gateway.

## ACT 2 · TEN, the sentinel breach

### S5 The Threshold · 48.0-58.0 · frames 1152-1392
- L: "The handbook is cached, so we do not pay to read it twice."
- R: "The freeze comes in, and the steel starts working against itself."
- C: "The freeze comes in while the handbook is already open."
- V: thermal flash. Byte 252, Kaskatinowipisim, Freeze-up Moon, trips on the stream.
  Asset: `assets/shots/20260131_183819.jpg`.

### S6 The Halt · 58.0-68.0 · frames 1392-1632
- L: "One audit costs four ten-thousandths of a cent."
- R: "The sentinel halts the stream and compiles a word in thirty-seven nanoseconds."
- C: "The sentinel halts, and the audit costs almost nothing."
- V: split terminal. Right: `chaos_monkey.rs` halts. Left: the real receipt, `cost_usd: 0.0004`,
  project `nde1-493505`, `us-central1`.

## ACT 3 · KETSU

### S7 The Chain · 68.0-74.0 · frames 1632-1776
- L: "The answer comes back typed and checked."
- R: "Every resolution folds into the chain, and the chain only moves forward."
- C: "The answer folds into the chain, and the chain only moves forward."
- V: Vertex returns the typed PhysicalInspectionAudit block. The hash seals onto genesis, `attest.rs`.

### S8 The Witness · 74.0-80.0 · frames 1776-1920
- L: "The trust is verifiable, and Google carries the reasoning."
- R: "The proof rides on the work itself, on ground you can stand on."
- C: "The proof rides on the work, on ground you can stand on."
- V: black. SURFACE LEDGER AND FORGE-ENVELOPE. Vertex AI and Rust badges aligned.
  Final rule: "No ending is silent. Every erasure is witnessed."

## NUMBERS · what may appear on screen
A 20-file log sweep ran this session over GEMMAPROOF.txt, Google.txt, ShaderB.txt, Trit-Moe.md,
FUCK.txt, Untitled.txt, Sweet.txt, googls.txt, GEMMAPROOF2.txt. Result: no criterion tables, no
cargo bench output, no timing harness prints, no timestamps. Every headline throughput number
traces to hardcoded Tailwind divs in a dashboard mockup (ShaderB.txt:593-691) or to projection
tables in chat logs (GEMMAPROOF.txt:1373-1384).

BANNED FROM SCREEN, no receipt exists:
- 35 ns and 35.12 ns. GEMMAPROOF.txt:1373 lists 35 ns as a PROJECTION for Tier 1 S13 DFA.
  Measured is 37.3633 ns, live_scale_telemetry.json:8. Say thirty-seven.
- 6.42 Gtok/s, 856.16 Mtok/s, 40.66 Gtrits/s, 57.48 GB/s, 879.51M plans/s, 17.89 ns, 1.17 ns.
  All hardcoded UI text. GEMMAPROOF's own table says 228.5 Mtok/s, off from 856.16 by 3.7x.
  The numbers do not agree with each other.
- 1,562,500x compression. No bench.
- 450,000 tokens, 75 percent discount, 60 Million audits, 562 dollars. Projected from a pricing
  model, never a billing console read. `verify_billing_draw.py` exists and is unrun.

CLEARED FOR SCREEN, measured this session:
- 37.36 ns per arbitration, 12,048,323 arbitrations per second, 1,000,000 in 0.083 s.
- 0 heap bytes on the hot path.
- 40,000 sabotage attempts injected, 40,000 repudiated.
- 0.0004 dollars per Gemini audit, gemini-2.5-flash, project nde1-493505, us-central1.
- 49 of 49 tests green.
- 5 trits per byte, 243 states, 13 sentinels.

## GRAMMAR
Right channel is the verb bed. Terminal frames carry imperatives only, never nouns:
HALT, COMPILE, ROUTE, ZEROIZE, SEAL. Left may carry nouns and money.
Center is where the adjective renders. Nobody says the adjective.
