---
name: ironroot-animation-communication
description: compile ironroot combat animation, scripted scenes, bardic performance, faction rituals, shadow echoes, fae motion, dialogue gestures, and environmental motion into deterministic communication contracts with motion phrases, timing windows, sound sync, worldstate effects, friction checks, and validation gates. use when asked to author or audit animation that communicates intent, threat, refusal, identity, faction pressure, romance, memory, shadow habit, or story beats for the dirge of the ironroot or compatible game scenes.
---

# Ironroot Animation Communication

## Purpose

Use this skill to treat animation as communication rather than decoration. Convert rough combat, scripted-scene, dialogue, ritual, Bardic, Shadow, fae, faction, romance, or environmental motion ideas into bounded animation contracts that can be paired with the Ironroot dialogue/communication compiler.

## Operating doctrine

Animation must communicate before dialogue explains. Motion should express intent, threat, refusal, memory damage, faction pressure, Bardic action, Shadow habit, identity state, and worldstate mutation.

Do not infer hard truth from motion alone. Motion is a prior until confirmed by context, state, dialogue, sound, player input, or a validator.

## Core workflow

1. Frame the scene: identify actor, scene kind, communication goal, active pressure, combat aspect, identity state, and disclosure level.
2. Bind to communication: connect the animation to a dialogue line, Bardic response, silence/refusal, sound primitive, faction signal, or environmental cue when available.
3. Choose motion grammar: select combat, dialogue gesture, scripted scene, faction ritual, Bardic performance, Shadow echo, fae movement, romance, or environmental motion.
4. Define the motion phrase: start pose, anticipation, action, impact/turn, recovery, hold/afterimage.
5. Define timing: anticipation ticks, active ticks, recovery ticks, cancel windows, input lock, readability at camera distance.
6. Attach state: event ledger write, relation shift, identity scar, faction suspicion, Shadow learning, worldstate mutation, or no-op if intentionally cosmetic.
7. Run friction checks using `references/friction-map.yaml`.
8. Validate with gates from `references/quality-gates.yaml`.
9. Emit a human-readable scene pass plus a machine-readable contract using `references/schema.json`.

## Required outputs

For full scene authoring, return:

- scene summary
- communication goal
- motion phrase
- timing contract
- dialogue/sound sync if any
- worldstate effects
- friction points
- validation gates
- machine-readable YAML or JSON contract

For quick critique, return:

- strongest motion read
- likely confusion
- missing state/timing data
- highest-risk friction point
- recommended patch

## Reference files

- `references/compiler.yaml`: full compiler doctrine, compile order, enums, and output contract.
- `references/rules.yaml`: knowledge rules for combat readability, Vowless refusal, Shadow habit, faction motion, Bardic input, dialogue gesture, Quincunx blocking, and scripted-scene state writes.
- `references/friction-map.yaml`: failure modes and resolution rules.
- `references/quality-gates.yaml`: required validation gates.
- `references/schema.json`: machine-readable output schema.
- `references/examples.yaml`: example combat and scripted-scene rows.
- `references/generic-transfer.md`: how to adapt the procedure outside Ironroot.

## Style constraints

- Prefer concrete state/timing contracts over cinematic prose.
- Keep player-facing animation readable and diegetic.
- Preserve Ironroot-specific style anchors: Bardic action, refusal, identity, debt, witness, faction pressure, Shadow habit, deterministic replay, and worldstate ledger.
- For generic reuse, replace Ironroot nouns with project-specific communication axes while preserving the same compiler stages and gates.
