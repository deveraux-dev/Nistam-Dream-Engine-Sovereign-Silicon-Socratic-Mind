# XVI · Sovereign Pipeline Lifecycle and End-to-End Orchestration

The 13Forge Sovereign Stack operates a unified, five-stage offline compilation and alchemical planning pipeline. By grounding content, geometries, constraints, compilation targets, and alchemical optimization paths into a single continuous flow, the system achieves total mathematical determinism and robust offscreen playback.

## Stage 1: The Unified Codex (Book -> Chapter -> Page)
The pipeline begins at the highest level of authoring. A Book organizes content into standard structural Chapters which organize content into standard Pages. Pages hold structured Blocks containing text, inks, and emphasis styles. This pure text and content are either imported dynamically from raw markdown files via `import_md` or assembled centrally via the master `full_atlas` builder.

## Stage 2: Geometric Projection (Book -> layout.rs)
To transition from abstract text blocks to safe physical layouts, `layout.rs` takes over. It models pages and text columns as explicit bounding boxes measured in integer-only i64 MilliUnits, establishing standard printable margins and calculating safe, non-overlapping columns via `columns(n, gutter)`.

## Stage 3: Declarative Page Solving (Layout -> ASP-Clingo)
When pages must be assembled dynamically (for instance, customizing page layouts based on historical eras or gating chapters behind completed world-quests), the engine invokes an in-process Answer Set Programming (ASP) rule solver. The solver processes available sections as spatial facts and applies recursive-free rules to prove exactly WHICH sections are allowed to fit onto a physical page without violating layout or content boundaries.

## Stage 4: Standalone Compilation (Book -> export_book / to_vixi)
Once the layout is mathematically solved, the Book compiles it into specific "folding faces" via `compile_faces`.
- **The HTML Face:** `export_book` converts the layout and styled blocks into a single standalone HTML document containing the typeset layouts, CMYK colors, and interactive page folds.
- **The Vixi Face:** `to_vixi` compiles the structures into a binary VixiScript intermediate representation, mapping layout elements directly to intermediate UI draw lists.

## Stage 5: Headless Compositing & Sequential Planning (VixiScript -> Fable5)
The compiled VixiScript bytes are loaded directly by the headless rendering layers:
- **Offscreen Readback:** `technothesia` parses the VixiScript and lowers it through `build_vixi_scene()` to compile a flat list of intermediate draw operations (`DrawList`). The `wgpu` FrameComposer composites these operations off-screen, writing a deterministic, pixel-exact BMP file.
- **Sequential Planning Loop:** The stateless WASM MUD engine (`sf-wasm`) leverages this exact same text-to-pixel compilation pipeline. The Fable5 sequential planning agent parses these compiled states through `planning_observation` and `seehear.rs` to analyze material colour classifications and solve the shortest command sequence to advance the game's alchemical gates without collapsing the surrounding zones.

## Verified End-to-End Integration
To prove the structural integrity of this multi-stage topology, an integration test executes all 5 stages in sequence. From Codex creation and column projection to ASP-Clingo layout constraint-solving, compiling to themed HTML, and running the sequential Fable5 alchemical planning optimizer over `sf_wasm::integration::Evaluator`, every boundary is verified.
