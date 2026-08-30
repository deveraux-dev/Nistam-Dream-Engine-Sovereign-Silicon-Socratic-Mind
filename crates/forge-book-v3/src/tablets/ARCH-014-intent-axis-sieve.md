# ARCH-014 — THE INTENT-AXIS SIEVE (Layer 0 · 9 Prime Senses · the WHY beneath the fold)

> **Bedrock tablet.** This is Axis 0 — the ground truth that generated the 10 generators.
> It does not assign crates (that's Axes 1/2/3). It answers: **why do these 10 exist and not some other 10?**
> Status: `[UNVERIFIED]` — awaiting Sean's sign-off.

---

## §1 — The 9 Prime Senses

A human interacts with a creation engine through 9 senses: 7 receptive (intake) and 2 generative (output). Every crate in the workspace exists to serve at least one of these. If the sense doesn't exist, neither does the crate's justification.

### Receptive (intake — the human perceives/projects)

| # | Sense | What the human projects |
|---|-------|------------------------|
| 1 | **KNOW** | Memory, context — "the system remembers what I told it" |
| 2 | **HEAR** | Acoustic/linguistic — the literal words spoken or typed |
| 3 | **SEE** | Spatial/visual — what is looked at, pointed to, manipulated |
| 4 | **FEEL** | Friction, urgency, drag — lag as physical weight |
| 5 | **WANT** | Volition — the explicit goal or command |
| 6 | **EXPECT** | Determinism — "if I do X, Y happens, every time" |
| 7 | **VALUED** | Safety/principles — what is right or wrong for the project |

### Generative (output — the human externalizes)

| # | Sense | What the human projects |
|---|-------|------------------------|
| 8 | **MAKE** | Creation — the drive to author, build, externalize |
| 9 | **OWN** | Provenance — "this is mine, signed, claimed" |

---

## §2 — The Ladder (WHY → HOW / WHAT / WHOSE)

```
Axis 0 — WHY (9 Prime Senses)
  │
  ├─► Axis 1 — HOW (8 technical generators — physics, no human in loop)
  ├─► Axis 2 — WHAT (1 product generator — the ONE program)
  └─► Axis 3 — WHOSE (1 intent generator — owned/signed/meant)
```

### How Axis 0 generates Axis 1 (the 8 technical generators)

| Sense | Generator it spawns | Why |
|-------|-------------------|-----|
| SEE | `forge-gpu` | Humans see → the engine needs a vision pipeline |
| HEAR | `forge-audio` | Humans hear → the engine needs a sound pipeline |
| EXPECT | `pp-math` | Humans expect determinism → the engine needs integer-Permyriad root |
| VALUED | `forge-firewall` | Humans value safety → the engine needs immune/gates |
| KNOW | `forge-sieve` | Humans remember → the engine needs context/storage/aperture |
| WANT | `forge-daemon` | Humans want goals executed → the engine needs a door |
| KNOW + WANT | `forge-broski` | Humans want inference on context → the engine needs the NDE ladder |
| **MAKE** | `forge-vix` | Humans create → they need a language to create in |

### How Axis 0 generates Axis 2 (the 1 product generator)

| Sense | Generator it spawns | Why |
|-------|-------------------|-----|
| **MAKE** | `forge-studio` | Humans create → they need a venue to create in |

### How Axis 0 generates Axis 3 (the 1 intent generator)

| Sense | Generator it spawns | Why |
|-------|-------------------|-----|
| **OWN** | `forge-evidence` | Humans claim ownership → they need signed provenance |

### MAKE is the only sense with two organs — state it, don't hide it

MAKE crosses HOW→WHAT. This is not a leak; it's the deepest truth in the model:

- **The instrument of making** → `forge-vix` (the language you make in — Axis 1, HOW)
- **The venue/artifact of making** → `forge-studio` (the thing you've made — Axis 2, WHAT)

This is why studio and vix are the most tightly coupled pair in the workspace. CLAUDE.md already says studio embeds the vixi chatbar. The architecture was already obeying MAKE's double organ; this tablet explains why.

### FEEL spawns no generator — it IS the governor

8 of 9 senses spawn the 10 generators. FEEL is the ninth sense with no organ. It doesn't build anything — it measures the drag every built thing produces. FEEL is orthogonal to both clocks because it has no clock of its own; it reads the friction whichever clock caused it.

This is precisely why the governor/sensor machinery (§5/§6 from the prior draft) splits to its own tablet: FEEL's job is to watch the health of every other sense's organ. It's the anchor of the governor, not a domain.

### OWN and VALUED share one artifact (the receipt) — keep them distinct

VALUED (receptive: "is this right/safe?") and OWN (generative: "this is mine/signed") both cash out in the Ed25519 receipt on `forge-evidence`. The receipt is the seam where VALUED's judgment and OWN's claim converge into one bit on disk. But the modes differ — judge vs. claim — so the senses stay separate. A future editor may not collapse them; the distinction is load-bearing even though the artifact is shared.

---

## §3 — The Trait-Shim Clause (survives from prior draft — DAG resolution without uprooting intent)

> **When the compiler's DAG constraint conflicts with intent-axis placement, sink a trait/type shim to the lower layer; the implementation stays in its intent-axis domain.**

Example: `forge-vocal-corpus` serves HEAR (its intent is acoustic). It mechanically depends on `forge-ml` (brain/inference). The fix is NOT to move vocal-corpus away from audio. The fix is:

1. Extract the trait that `forge-ml` exposes (e.g. `trait Infer`) into a thin shim crate at the determinism layer (core/pp-math watershed)
2. `forge-vocal-corpus` depends on the shim (downward edge — legal)
3. `forge-ml` implements the shim (upward impl — legal, no cycle)
4. Vocal-corpus stays in audio. Its WHY is HEAR. The DAG is satisfied without lying about intent.

**Rule:** The compiler's layering is a constraint to satisfy, not a truth to obey. Intent (Axis 0) governs placement. The DAG (Axis 1 physics) governs dependency direction. When they conflict, traits bridge the gap.

---

## §4 — Two Clocks, restated through the senses

| Clock | Sense it serves | Domain |
|-------|----------------|--------|
| **DET-CLOCK** (integer, 120Hz, Drum-1) | EXPECT | Determinism — the knob turns or it doesn't |
| **CREATIVE-LANE** (float, unbounded) | SEE | Fidelity — visual richness, no ceiling |
| — (neither) | FEEL | Orthogonal — reads the drag of both, owns neither |

The Two Clocks boundary (`CollisionBridge`) is the EXPECT↔SEE membrane. FEEL observes both sides.

---

## §5 — Fast-map

- Layer-0 justification for any generator → **ARCH-014**
- 10-generator factoring / crate assignment → **§Factoring in KIRO-BACKLOG-ledger** (Brick #13)
- Governor / machine-sensor model → **future tablet** (split from this one; FEEL is the anchor)
- Circulatory / output path → **ARCH-010**
- River / context flow → **ARCH-013** (serves KNOW)
- Two Drums / Two Clocks → **ARCH-009** (serves EXPECT + SEE boundary)
- Vascular collapse module tree → **ARCH-011** (not superseded — answers WHERE; this answers WHY)

---

## Status

`[UNVERIFIED]` — pending Sean's sign-off.

No open questions remain. The 2 seat calls (Brick #13) are unchanged:
1. Human-Intent generator = `forge-evidence` (OWN) — seated.
2. Determinism generator = `pp-math` (EXPECT) — seated.
