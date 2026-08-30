# The Five Legs

A material in this engine is not a colour with a name stapled to it. It is one
index, and everything true about that index has to agree.

The law was written before the parts were: `colour_id ≡ material_id ≡ essence_id
≡ resonance_id`. One number, `0..=63`, and every organ reads the same one. Paint
picks a slot. The physics reads that slot's density. The audio reads that slot's
ring. The stat engine reads that slot's essence. Nothing translates, because
there is nothing to translate — it is the same integer the whole way down.

That is the promise. For a long time it was only three-quarters kept.

## What was actually there

Four legs stood on their own tables.

**Colour** — `forge_core::correspondence::palette_rgb`, the 2DAK 64-colour
palette. The albedo you see.

**Material** — `material_registry.rs`, sixty-four slots with five integer axes:
Mohs hardness, metallic, roughness, density, restitution. Gold at 0. Obsidian at
20. Void at 62, every axis zero, the silence a fresh stroke paints over.

**Essence** — `essence_registry.rs`, the semantic mirror: potency, volatility,
polarity, affinity, spread, tier. Sixty-four again, eight families of eight.

**Resonance** — derived in `material_atom`, the note a material makes when it is
struck. Hard bodies ring by stiffness. Thin bodies carry a flow floor so water
and lava are not both a dead zero. Mass drags the pitch down. It was hardcoded to
zero once; that clamp to `150..=10000` is the fix, and the test that says
`slot {idx} rings at 0` is the reason it stays fixed.

Four legs, and the fifth was missing.

## The missing leg

**Surface** — the texture. A library of 220 PBR sets sat under `assets/textures`,
discovered by a registry keyed on strings like `pbr/stone/cobble/PavingStones046`,
with no way to say which slot wore which. `init()` had no callers. The registry
was built and never mounted. The material table said "Obsidian" and the texture
library said `Rock030`, and nothing in the engine knew those were about the same
thing.

Binding by name gets you ten. The library is named the way photogrammetry
libraries are named — `Wood051`, `Metal032`, `Concrete033` — not the way a
material table is named. Fifty-four slots have no set carrying their word.

So the law answers its own question. If `colour_id ≡ material_id`, then a slot's
colour *is* its identity, and the honest way to pick its surface is the surface
whose own mean albedo sits nearest that colour. Decode each set's colour map once
at thumbnail scale, average it, cache the result. Then every slot picks by the
one thing it already knows about itself.

Ten by name, fifty-four by colour, sixty-four bound.

## Why distinct matters

Nearest-colour alone collapsed sixty-four identities onto thirty-five surfaces.
Twenty-nine slots doubled up, which looks less like a rich palette and more like
a broken one.

The fix is greedy by confidence. Name matches claim first — they are the
strongest evidence available. Then colour matches, best-distance first, so a slot
with a strong match picks before a slot whose nearest is far. Each takes the
nearest set not yet claimed. Sixty-four distinct surfaces, and the assignment
only shares once the library is genuinely exhausted, because a shared surface
still beats a hole.

## The capstone

None of the above is worth anything if a leg can drift quietly. So the legs are
resolved together, at one index, by one function, and a test walks all sixty-four
and asserts that each leg equals what its own home holds at that same index. A
material table reordered without the palette breaks it. An essence renamed breaks
it. A ring clamped back to zero breaks it.

The receipt is on disk: sixty-four rows, index, material, colour, essence, ring,
how it bound, and the surface it wears. You can read what every slot is without
running anything.

```
0   Gold            170D09  Fire      1566  name    pbr/metal/gold/Metal048A
20  Obsidian        …       …         …     …       …
62  Void            CEC9C0  Aether    3333  name    pbr/special/void/Concrete033
63  Echo-Residue    D8D6D0  Null      3333  colour  13moons/terrain/gravel/Gravel023
```

One thing that table exposes and does not fix: the palette reads as a luminance
ramp. Gold is `170D09`, near black. Void is `CEC9C0`, near white. Colour-binding
quality is capped by that — the fifty-four are matching on brightness more than
on hue. A perceptual distance metric will not fix a ramp. A hue-bearing palette
would, and that is a decision, not a patch.

## What it is for

A texture browser cycles the library and binds a set to the slot you are holding.
Because the slot is the shared handle every creation surface already reads, the
pick lands everywhere at once — the paint rail, the canvas, the sprite pipeline,
the world. You are not assigning a texture to an object. You are saying what a
material *is*, once, and the engine agrees with you about it from then on.

That is the whole point of one index.
