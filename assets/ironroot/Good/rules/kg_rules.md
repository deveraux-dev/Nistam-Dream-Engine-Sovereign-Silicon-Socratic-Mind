# Knowledge Rules

## KG_RULE: brand.executable.allegiance

CATEGORY: Faction / AI  
SOURCE: Dirge Brand System  
CLAIM: A brand is a symbolic runtime contract that modifies allegiance and behavior.  
THEORY_BASIS: Semiotics, ritual authority, faction identity, executable metadata.  
ENGINE_TRANSLATION: Apply brand components to faction relation tables, AI goal stacks, audio motifs, shader overlays, and animation biases.  
CV_PRIOR: Recognize brand glyph/motif as candidate allegiance marker.  
MESH_PRIOR: Brand must bind to valid surface region or entity attachment socket.  
SDF_PRIOR: Brand depth determines surface/memory/material penetration.  
ATLAS_PRIOR: Brand has icon/glyph/mask atlas entry.  
MATERIAL_PRIOR: Material affects brand adhesion, decay, and spread.  
ANIMATION_PRIOR: Branded entities may receive posture/gait/gesture overrides.  
AUDIO_PRIOR: Carrier motif activates or strengthens brand.  
FLOW_PRIOR: Player must be able to read brand state quickly.  
TRAINING_TOKEN: `<brand_executable_allegiance>`  
CONFIDENCE: 0.94  
FAILURE_MODE: Brand becomes invisible mind-control with no counterplay.  
QUALITY_GATE: Every brand must expose carrier, target filter, resistance, decay, and counter-rule.  
[TAG:FACTION] [TAG:AUDIO] [TAG:BEHAVIOR] [TAG:QUALITY]

## KG_RULE: music.faction.doctrine

CATEGORY: Faction / Audio / Behavior  
SOURCE: Dirge Music-Faction System  
CLAIM: Each faction expresses world control through a distinct musical grammar.  
THEORY_BASIS: Music as identity, ritual performance, symbolic control, orchestral role mapping.  
ENGINE_TRANSLATION: Faction doctrine maps to motifs, carriers, rhythms, AI behaviors, shader palettes, zone effects, and combat timing.  
CV_PRIOR: Faction visuals should align with doctrine symbols but not determine behavior alone.  
MESH_PRIOR: Faction architecture may encode rhythm, symmetry, density, or resonance nodes.  
SDF_PRIOR: Faction terrain modifications have recognizable resonance signatures.  
ATLAS_PRIOR: Faction glyphs and instrument icons must remain readable at small sizes.  
MATERIAL_PRIOR: Material choice supports but does not define faction doctrine.  
ANIMATION_PRIOR: Faction units move according to doctrine: bound, free, direct, indirect, syncopated, choral, ruptured.  
AUDIO_PRIOR: Motif and carrier define activation pathway.  
FLOW_PRIOR: Player should learn faction behavior through repeated musical cues.  
TRAINING_TOKEN: `<music_faction_doctrine>`  
CONFIDENCE: 0.90  
FAILURE_MODE: Factions become aesthetic playlists with no mechanical distinction.  
QUALITY_GATE: Every faction doctrine must affect at least three runtime surfaces: AI, audio, environment, combat, brands, or materials.  
[TAG:MUSIC] [TAG:FACTION] [TAG:BEHAVIOR]

## KG_RULE: topology.resonant.fracture

CATEGORY: Topology / Terrain / Audio  
SOURCE: Dirge Topological Resonance System  
CLAIM: Terrain can be reshaped when the correct resonance phrase matches a validated material/topology signature.  
THEORY_BASIS: Resonance, material stress, cymatics analogy, fault-line completion, geologic substrate as instrument.  
ENGINE_TRANSLATION: SongPhrase resolves against ResonanceTarget; if interval/rhythm/intensity match and thresholds pass, terrain state changes.  
CV_PRIOR: Visible cracks, stress lines, water seepage, iron veins, and echo behavior suggest resonance candidates.  
MESH_PRIOR: Terrain deformation must route through approved topology operation: split, collapse, raise, sink, shear, open, seal.  
SDF_PRIOR: SDF stress field determines fracture plausibility.  
ATLAS_PRIOR: Map/minimap must update topology state and route access.  
MATERIAL_PRIOR: Stone, iron, root, bone, and water all alter resonance thresholds.  
ANIMATION_PRIOR: Terrain change must have anticipation, tremor, fracture, and settle phases.  
AUDIO_PRIOR: Fundamental tone and overtones communicate progress toward fracture.  
FLOW_PRIOR: Major topology changes must be authored or strongly gated to avoid breaking progression.  
TRAINING_TOKEN: `<topology_resonant_fracture>`  
CONFIDENCE: 0.86  
FAILURE_MODE: Music becomes arbitrary terrain-delete button.  
QUALITY_GATE: No topology mutation without validated ResonanceTarget, material profile, route-impact check, and progression gate.  
[TAG:MUSIC] [TAG:SDF] [TAG:MESH] [TAG:TOPOLOGY] [TAG:QUALITY]

## KG_RULE: laban.effort.perceived_impact

CATEGORY: Animation / Combat Feel  
SOURCE: Theory-to-Engine Prior Compiler  
CLAIM: Laban Weight influences perceived force, intention, and impact, but must not determine canonical physical mass.  
THEORY_BASIS: Laban Movement Analysis, Effort category: Weight.  
ENGINE_TRANSLATION: Strong Weight maps to anticipation frames, hit-stop, recovery duration, camera impulse, contact audio, and easing curves. Canonical mass remains material/volume-authoritative.  
CV_PRIOR: Heavy-looking motion suggests perceived impact candidate.  
MESH_PRIOR: Mesh volume may support or contradict perceived heaviness.  
SDF_PRIOR: SDF volume contributes to actual mass calculation.  
ATLAS_PRIOR: Attack iconography may encode heavy/light swing type.  
MATERIAL_PRIOR: material_id + volume determine mass.  
ANIMATION_PRIOR: Strong = slower anticipation/recovery; Light = faster recovery/lower hit-stop.  
AUDIO_PRIOR: Strong = low-frequency impact emphasis.  
FLOW_PRIOR: Strong actions require readable windup and fair counter-window.  
TRAINING_TOKEN: `<laban_perceived_impact>`  
CONFIDENCE: 0.95  
FAILURE_MODE: Fast-moving truck gets feather mass because motion looks light.  
QUALITY_GATE: Movement priors cannot write rigid body mass.  
[TAG:MOVEMENT] [TAG:FRICTION] [TAG:PHYSICS]

## KG_RULE: gttm.physics.rhythm_boundary

CATEGORY: Rhythm Combat / Physics / Audio  
SOURCE: Theory-to-Engine Prior Compiler + Dirge Rhythm Combat  
CLAIM: Musical meter may structure telegraphs, Foley, and boss cues, but must not override major physical impacts.  
THEORY_BASIS: GTTM metrical hierarchy, rhythm readability, game feel latency constraints.  
ENGINE_TRANSLATION: Minor audio/animation events can quantize to grid within a small delay window. Major collisions, ragdolls, falling rocks, and topology fractures resolve by physics first.  
CV_PRIOR: Beat-aligned visual cues suggest intended timing windows.  
MESH_PRIOR: Physical collisions use collision geometry, not score grid.  
SDF_PRIOR: Terrain impact/fracture timing uses SDF/physics state.  
ATLAS_PRIOR: Combat lane indicators may be beat-aligned.  
MATERIAL_PRIOR: Material affects impact audio and response.  
ANIMATION_PRIOR: Telegraphs and anticipation may lock to rhythm.  
AUDIO_PRIOR: Minor Foley can micro-quantize; major impacts do not.  
FLOW_PRIOR: Timing correction must not exceed player-noticeable causality thresholds.  
TRAINING_TOKEN: `<friction_metrical_physics>`  
CONFIDENCE: 0.93  
FAILURE_MODE: Falling objects pause unnaturally to land on beat.  
QUALITY_GATE: Minor quantization max 40 ms; major physics events ignore grid.  
[TAG:MUSIC] [TAG:PHYSICS] [TAG:FLOW] [TAG:FRICTION]

## KG_RULE: gestalt.grouping.material_gated

CATEGORY: Perception / Geometry  
SOURCE: Theory-to-Engine Prior Compiler  
CLAIM: Proximity suggests grouping, but material and interactivity checks decide whether geometry actually merges.  
THEORY_BASIS: Gestalt proximity principle with engine validation.  
ENGINE_TRANSLATION: Nearby parts may cluster visually or procedurally if material continuity, sockets, and semantic class agree.  
CV_PRIOR: Spatial proximity suggests candidate grouping.  
MESH_PRIOR: Mesh merge requires socket/material compatibility.  
SDF_PRIOR: SDF union allowed only when topology and material gates pass.  
ATLAS_PRIOR: Proximate parts may share atlas chart only if semantic grouping is valid.  
MATERIAL_PRIOR: Material discontinuity blocks automatic merge.  
ANIMATION_PRIOR: Separate interactables retain separate animation/physics identities.  
AUDIO_PRIOR: Grouped objects may share resonance if material continuity passes.  
FLOW_PRIOR: Player-readable interactables must not be visually swallowed into background clusters.  
TRAINING_TOKEN: `<gestalt_grouping_material_gated>`  
CONFIDENCE: 0.91  
FAILURE_MODE: Loot, traps, or doors merge into static scenery because they are nearby.  
QUALITY_GATE: No geometry merge without semantic/material/socket agreement.  
[TAG:GESTALT] [TAG:MESH] [TAG:MATERIAL] [TAG:FRICTION]

## KG_RULE: raga.mode.fictional_resonance

CATEGORY: Music / Mode / World State  
SOURCE: Raga-inspired Dirge Mode Logic  
CLAIM: Raga-like modal grammar can define lawful phrase direction, emotional effect, time-gate, and target compatibility.  
THEORY_BASIS: Modal grammar, rasa association, time/season linkage, phrase identity, drone anchoring.  
ENGINE_TRANSLATION: Fictional modes define allowed rising/falling phrases, primary tones, signature motifs, ornaments, valid targets, and runtime effects.  
CV_PRIOR: Visual/aura state may indicate mode compatibility but cannot authorize effect alone.  
MESH_PRIOR: Architecture may encode modal intervals in spacing/repetition.  
SDF_PRIOR: Topology targets may require a mode-specific resonance path.  
ATLAS_PRIOR: Mode icons must distinguish primary tone, time gate, and target type.  
MATERIAL_PRIOR: Material resonance modifies mode effect strength.  
ANIMATION_PRIOR: Phrase completion alters gesture, posture, and timing windows.  
AUDIO_PRIOR: Drone, primary tone, and signature phrase are required for activation.  
FLOW_PRIOR: Player learns mode through repeated, readable response loops.  
TRAINING_TOKEN: `<raga_fictional_mode_prior>`  
CONFIDENCE: 0.84  
FAILURE_MODE: Real cultural music is flattened into exotic spell loot.  
QUALITY_GATE: Use fictional mode names unless reviewed; include cultural safety metadata.  
[TAG:MUSIC] [TAG:CULTURAL] [TAG:QUALITY]

## KG_RULE: forge.glyph.executable_script

CATEGORY: Glyph / Scripting / Faction AI  
SOURCE: Forge Steganographia Glyph Language  
CLAIM: A glyph cluster can encode aesthetic, phonetic, mnemonic, and executable layers at once.  
THEORY_BASIS: Steganographic notation, visual scripting, ritual memory systems, symbolic compression.  
ENGINE_TRANSLATION: Parse glyph shape, orientation, color, sound, placement, and grouping into faction behavior, material operations, shader states, audio cues, or AI goals.  
CV_PRIOR: Glyph features are candidate clauses, not final instructions.  
MESH_PRIOR: Glyph placement on surface may bind script to local object/zone.  
SDF_PRIOR: Engraving depth affects persistence/resistance.  
ATLAS_PRIOR: Glyphs require readable atlas variants and masks.  
MATERIAL_PRIOR: Material controls legibility, persistence, and resonance.  
ANIMATION_PRIOR: Stroke order may encode gesture sequence.  
AUDIO_PRIOR: Syllable/tone activates or validates glyph.  
FLOW_PRIOR: Symbols must be teachable and not become unreadable ornamental noise.  
TRAINING_TOKEN: `<forge_glyph_executable_script>`  
CONFIDENCE: 0.87  
FAILURE_MODE: Cool-looking nonsense with no parseable semantics.  
QUALITY_GATE: Every executable glyph must have visible layer, phonetic/mnemonic notes, parse result, and validator path.  
[TAG:GLYPH] [TAG:SCRIPTING] [TAG:QUALITY]
