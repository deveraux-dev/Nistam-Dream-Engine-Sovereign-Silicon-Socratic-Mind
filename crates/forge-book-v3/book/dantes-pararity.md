---
title: Dante's Pararity
dated: 2026-08-16 04:53 (Edmonton)
proof_state: mixed -- see inline tags, per L12 (Proven | Authored | Estimate | Unproven)
sources: Sean Morin (session thread) + a Gemini cross-check, folded and re-typed against
  this repo's own truth-gate before landing as a book/ chapter -- an external model's essay
  is not automatically canon here just because it agrees with the human
---

# Dante's Pararity

## The real thing this chapter is anchored to [Proven]

`crates/forge-engine-v3/src/tick.rs:28-36` (ARCH000 seat ruling, 2026-08-11):

```rust
pub const REGISTER_INFERNO: i8 = -1;
pub const REGISTER_PURGATORIO: i8 = 0;
pub const REGISTER_PARADISO: i8 = 1;
```

Packed through `EngineTick8::encode`'s `pack5` call (`tick.rs:131`) as trit1 of the mode
byte, alongside `run_state` at trit0. This is a real, tested, ARCH000-ruled instance of
`PARARITY.md`'s n=3, k=1 construction -- the one this whole chapter traces backward, not a
narrative retrofit onto code that doesn't carry it.

## The word chain [Proven, etymology checked independently]

**Parity** -- Latin *paritas*, from *par* ("equal"). **Arity** -- not ancient at all: a
20th-century back-formation (logic/CS, ~1970s) stripped off "un-ary/bin-ary/tern-ary,"
keeping the suffix (Latin *-arius*, "pertaining to"). **Pararity** -- this repo's own
coinage (`PARARITY.md` §6-7: "only the term *pararity* -- naming *k* -- is offered as
new"), Greek *para-* ("beside") + *parity*. **Parody** -- a false cognate: Greek *parodia*
= *para-* + *oide* ("song"), sharing only the prefix with *parity*, not the root. **Comedy**
-- Greek *komoidia* = *komos* ("revel") + *oide* (same "song" root as parody). Dante titled
his poem a *Comedia* because it ends well, not because it is funny.

## The 13th century [Proven as history, Authored as connection]

**Fibonacci's *Liber Abaci*, 1202** [Proven]: the book that carried the Hindu-Arabic
*zephirum* into Latin Europe as a real positional digit, not merely a placeholder for
absence. Before this, in the inherited Aristotelian frame, zero was *privatio* -- a lack,
not a value.

**Thomas Aquinas's *Summa Theologiae*, compiled through the 1260s-1274** [Proven as
dating]: scholastic ethics built heavily on Aristotle's doctrine of the mean (virtue as a
balance between excess and deficiency, via Aquinas's own commentaries on the
*Nicomachean Ethics*). [Authored, not a precise historical citation]: calling this a "via
media" doctrine borrows a phrase more associated with later (Anglican-era) usage; the
underlying Aristotelian-mean concept is real, the specific Latin label applied to it here
is this chapter's framing, not Aquinas's own vocabulary, and should not be repeated as a
direct quotation.

**Dante Alighieri, born 1265** [Proven]: raised inside exactly this scholastic milieu,
writing the *Divine Comedy* c. 1308-1320 -- early 14th century, though the intellectual
formation is 13th-century. His *Purgatorio* is the only one of the three realms with time
in it: Inferno and Paradiso are both eternal and unchanging (the damned do not worsen, the
blessed do not further ascend); only in Purgatorio do souls actually move, climb, and
transform. [Authored]: this makes Dante's "middle" not a passive resting point between two
poles but the *sole zone where change is possible at all* -- a stronger reading than plain
"equilibrium."

## The fold back onto `tick.rs` [Authored interpretation of Proven code]

`PARARITY.md` §0 describes the balanced-ternary zero as "the fulcrum... the place a
system returns to" -- language that reads as a passive rest state. Dante's version, read
literally against his own text, is more specific: the zero-state is where the *value is
still mutable*; ±1 are the states nothing further happens to. Whether
`REGISTER_PURGATORIO=0` in `tick.rs` is USED that way anywhere in this codebase --
i.e. whether register ever actually transitions through 0 as a deliberate intermediate
step, rather than 0 simply being one of three static tags a caller picks once -- is
**[Unproven]** and not checked as part of this chapter. That would be the next real
question, not a conclusion this chapter is entitled to draw for free.

## What this chapter is and isn't

This is lore with a code anchor, not a design document. It explains *why* the register
trit is named the way it is and gives that naming a real intellectual lineage. It does not
claim the Dante mapping was load-bearing at design time (`ARCH000 2026-08-11`'s own ruling
language doesn't cite Dante's temporal structure, only the register-as-narrative-canon
decision) -- that specific "Purgatorio is the mutable one" reading is this chapter's own
contribution, dated tonight, not retroactively assumed to have been the original intent.
