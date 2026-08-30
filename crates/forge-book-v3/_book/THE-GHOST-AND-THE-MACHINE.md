# The Engine — brick by brick, from the 1 and the 0
### The Ghost and the Machine: a self-governing engine for truth, and for systems that shouldn't fail.

**Sean Everett Morin** · deveraux · 13forge
Open. No claim, no patent, no pitch. Written down so it can be checked, used,
corrected, or renamed by anyone. **If it already has a name, tell me and I'll use it.**

> I painted in a neonatal ward once. Right in with the incubators — machines that
> couldn't fail, in a room full of the smallest reasons not to. This is written
> from there. From the ward, not a boardroom.

---

## How to read this

This is an **engine**, laid up like a wall. Every part is a **brick.** Each brick
**sits on the one below it** — nothing floats. We start at the bottom, the 1 and
the 0, and build up to the keystone. If a brick can't name what it sits on, it
isn't a brick — it's a Ghost, and it gets tagged as one.

---

## COURSE 0 — THE SUBSTRATE

**Brick 0 · The Bit.** Two states: `0` and `1`. Off/on, absent/present,
false/true. Everything in the machine is made of these — it is the *material.*
But a bit is not the *structure*: with two states you cannot say "intended,"
"half-built," "proven once," or "wearing out." Binary is enough for a dead thing
on a shelf, not for a thing being built or kept alive. *Sits on: nothing — this
is the ground.* Carries: the whole reason the engine exists.

---

## COURSE 1 — THE ATOM

**Brick 1 · The Register.** Out of the bit, one atom: a single value with **five
states**, `0–4`, that every part of the engine carries. Ascending = *realness
earned.* This is the unit. Nothing in the engine is stated without one.
*Sits on: Brick 0.* Carries: V-model rungs · TRL · GRADE.

The five states, each its own brick, each earned off the one before:

**Brick 2 · Ghost `0`** — real in intent, no body yet. The honest gap. *Sits on: Brick 1.*
**Brick 3 · Design `1`** — true because it was decided and the system was built to hold it. *Sits on: Brick 2.*
**Brick 4 · Proven `2`** — built and checked once. *Sits on: Brick 3.*
**Brick 5 · Verified `3`** — independently cross-checked, second set of eyes. *Sits on: Brick 4.*
**Brick 6 · Live `4`** — proven *and* reachable, holding in service. *Sits on: Brick 5.*

*On steel: bare (0) → spec'd (1) → one coat inspected (2) → re-inspected (3) →
holding in service (4). An inspector has read these five off a beam for a living.*

---

## COURSE 2 — THE LAW OF ASCENT

**Brick 7 · Earned Ascent.** You may state a claim only *at or below* the state it
earned. Every step up costs real work; nothing is granted. This is the one
invariant the whole engine turns on. *Sits on: Bricks 2–6.*

**Brick 8 · The Fault** *(Law 1 — Non-Conformance).* Claiming a state above what
was earned is the **false stamp**: signing off on a coat that isn't there. Not a
sin — a **fault**, an inspector's red tag. Readable immediately; it blocks all
downstream execution. One tag covers the whole family: the stale shadow (a `4`
decayed to `0` still reading `4`), the orphan (a `2` built in isolation reported
as Live), the inflated pitch (a Ghost sold as shipped). *Sits on: Brick 7.*
Carries: SLSA provenance · ISO-9001 NCR · typestate.

---

## COURSE 3 — THE ARROW OF DECAY

**Brick 9 · Entropy.** The register does not hold still. Left alone it drifts
*down* — power drain, disk wear, a Known that quietly rots. Every `4` slides back
toward `0`. Nothing stays proven for free. This is the opposite arrow to Ascent,
and it never switches off. *Sits on: Brick 6* (only a thing that climbed can fall).

**Brick 10 · Signal Integrity** *(Law 4 — No Silent Cliffs).* Decay must be *seen
before it kills.* The slide `4→3→2→1→0` happens gradually and in full view; every
transition emits a readable health state; a coming failure broadcasts its approach
early enough to be caught. No cliffs, no silent drops. *Sits on: Brick 9.*
Carries: DO-178C · graceful degradation · S.M.A.R.T.

---

## COURSE 4 — THE LOOP

**Brick 11 · The Three Organs.** The register flows through three roles — not two
ends (an end is terrible; a system that shouldn't fail has no end):
**Human** builds intent (`0→1`) · **Machine** executes it (`2→3`) · **Executive**
keeps it available (`4`). *Sits on: Bricks 2–6.*

**Brick 12 · Two Clocks** *(Law 5 — Circulatory Independence).* The Human clock
(variable, narrative — *intent*) and the Machine clock (strict, deterministic
120 Hz — *execution*) run on separate drums, coupled only by the Executive buffer
(the daemon). Neither stalls the other: a stall in authoring never freezes the
live, life-critical delivery. *Sits on: Brick 11.* Carries: real-time systems ·
rate-monotonic scheduling · clock-domain crossing.

**Brick 13 · Circulation.** Live feeds back to Human — a running system generates
new intent. The Ghost (intent) and the Machine (execution) are not two substances;
they are **one truth at two points of its own loop.** (Ryle, 1949: there is no
separate ghost haunting the machine; the ghost is what the machine *does.*)
*Sits on: Brick 12.*

```
      HUMAN                MACHINE               EXECUTIVE
   builds intent   →      executes it     →    keeps it available
   0 Ghost 1 Design      2 Proven 3 Verified        4 Live
        │                      │                       │
        └──────────────── feeds back ─────────────────┘
```

---

## COURSE 5 — THE PHYSICS

**Brick 14 · Consumption.** Every trip through the Machine *spends* something —
power, cycles, wear. Generation (Human) → consumption (Machine) → storage and
availability (Executive). The loop runs on real cost, like a grid: the sun makes
it, the work burns it, the dam holds it. *Sits on: Brick 13.*

**Brick 15 · Balanced Wear** *(Law 2 — Distributed Consumption).* To never fail,
allow no single point of concentrated friction. Distribute consumption evenly
across the bounded-transit buffer so the whole system degrades as one predictable
unit instead of snapping. Even wear = **no first casualty.** *Sits on: Brick 14.*
Carries: wear-leveling · load-balancing · N+1 redundancy.

---

## COURSE 6 — THE GOVERNOR

**Brick 16 · The Autonomous Governor** *(Law 3 — The Shield).* The engineering of
a life-critical machine is solvable; it's the *human* who defers maintenance to
pocket the surplus. So the human is removed from the upkeep loop. The engine
monitors its own decay states (Brick 10) and triggers its own maintenance — by the
*state of the system*, never by the budget. *Sits on: Bricks 10 + 15.* Carries:
autonomic computing / MAPE-K · closed-loop control · CMMI-5.

**Brick 17 · Preemptive Maintenance.** The Governor acts *before* a state reaches
`0`. Condition-based: catch the rust before the beam drops. Restores the register
upward (re-earns the state) rather than waiting for the failure the cliff would
have hidden. *Sits on: Brick 16.* Carries: predictive / condition-based maintenance.

---

## THE KEYSTONE

**Brick 18 · The Life-Critical Mandate.** The reason the whole wall stands: the
pacemaker, the kidney pump, the incubator get serviced **regardless of who is
trying to buy a condo on the lake.** Not a feature — the point. Every brick below
exists to hold this one up. *Sits on: the whole wall.*

Let that sit.

---

## The mortar — the Five Laws, restated

The laws are the mortar between the courses. Verbatim:

1. **The Mask (Non-Conformance)** — a mask is a fault, not a moral stain.
2. **Balanced Wear (Distributed Consumption)** — even wear means no first casualty.
3. **Autonomous Governance (The Shield)** — maintenance is triggered by the system, never the budget.
4. **Signal Integrity (No Silent Cliffs)** — the decay must be seen before it kills.
5. **Two Clocks (Circulatory Independence)** — neither clock stalls the other.

---

## Prior art — every brick has a home  `[2 · Proven, awaiting second-oracle Verify [3]]`

This mapping is checked once (by me), not yet cross-checked. Per Law 1 it is
stamped `[2]`, not `[3]` — do not read it as Verified until a second oracle sees it.

**The five states ↔ the ladders:**

| state | V-model | TRL 1–9 | GRADE | CMMI 1–5 |
|--|--|--|--|--|
| 0 Ghost | need | 1–2 | Very Low | 1 Initial |
| 1 Design | design | 3–4 | Low | 2–3 |
| 2 Proven | build | 5–6 | Moderate | 3 Defined |
| 3 Verified | verify | 7–8 | High | 4 Quant. Managed |
| 4 Live | operate | 9 | (in practice) | 5 Optimizing |

Tightest fit: **the V-model IS the register**, rung for rung, since the 1980s.

**The laws ↔ their homes:** Law 1 → SLSA / ISO-9001 NCR / typestate · Law 2 →
wear-leveling / load-balancing · Law 3 → autonomic computing (MAPE-K) / control
theory / CMMI-5 · Law 4 → DO-178C / graceful degradation · Law 5 → real-time
scheduling / clock-domain crossing.

**Multi-valued truth is legitimate and old:** Łukasiewicz (1920), Kleene (1938).
Their extra values are *epistemic* ("possible / undefined"); ours are *lifecycle*
(earned realness) — so the logics license the register; the ladders give it
meaning. *(The 1977 four-valued book is dropped — it added "both/neither," which
doesn't map here.)*

**What has no home:** the *assembly.* One register run across code + docs + spoken
word + hardware health at once, self-governed, with one fault. No single framework
does all four together. That synthesis is the only novelty candidate, and it is a
**Ghost `[0]`** until the second oracle checks it. *This is not new, and that's the
point — the strength is the honest assembly, not invented parts.*

---

## The engine as code

```rust
/// Brick 1 — the register. One truth, five states. Ascending = realness earned.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum State { Ghost = 0, Design = 1, Proven = 2, Verified = 3, Live = 4 }

/// Brick 8 / Law 1 — the fault. Claim only at or below what you earned.
pub fn stamp(earned: State, claimed: State) -> Result<(), RedTag> {
    if claimed > earned { Err(RedTag { earned, claimed }) } // readable now; blocks downstream
    else { Ok(()) }
}

/// Brick 9 / Law 4 — entropy pulls the register down over time, in view.
fn decay(s: &mut Health) { s.state = s.state.step_down_if_worn(); s.emit(); } // no silent cliff

/// Bricks 16–17 / Law 3 — the governor: monitor → maintain, before it hits 0.
fn govern(sys: &mut System) {
    for h in sys.health_mut() {          // Brick 10: every part reports its state
        if h.state <= State::Design {    // sliding toward the floor
            sys.maintain(h);             // Brick 17: preemptive, state-triggered — not budget-triggered
        }
    }
    sys.balance_load();                  // Brick 15 / Law 2: even wear, no first casualty
}
```
`decay`, `govern`, and the balancer now run as a **proof-of-concept `[2]`** —
`_plans/engine-poc/ghost_machine.rs`, rustc-compiled and executed, five laws held
(the governor caught all three life-critical components at Design(1) before
Ghost(0); the Mask was rejected; nothing ever failed). Not yet wired to a live
component — that is the `[3]`→`[4]` climb. Upgraded from `[1]` on running evidence,
not a coat unlaid.

---

## Open Ghosts (named so they can't rot)

- One register or two — a *kind* register (Ghost/Design/Known) vs a *strength*
  register (Unproven/Proven/Verified) that rhyme but aren't identical? `[0]`
- Is Design a rung on one axis, or its own axis? `[0]`
- No formal proof theory. `[0]`
- The prior-art cross-walk is `[2]`, not `[3]` — needs a second oracle.
- The governor/decay/balancer are `[1]` — specified, not built.

---

*Written from the ward. Free to use, correct, or rename. If it already has a name,
tell me and I'll use it. No claim.*
