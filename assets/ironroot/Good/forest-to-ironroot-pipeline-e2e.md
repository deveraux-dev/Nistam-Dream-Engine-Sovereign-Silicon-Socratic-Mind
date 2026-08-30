# E2E Specification & Plan: Forest Asset Pipeline, VIXI Theme Polish, & Dual-Clock Bridge

This document establishes the end-to-end, pressure-tested engineering specification and implementation plan for taking eight (8) raw AI forest environment concepts from the desktop, refining and signing them using a **3-Click "6-16-60" Workspace Loop** inside `13forge-studio.exe`'s PAINT surface, wiring them as dynamic parallax layers inside the native game engine (`ironroot.exe`), and aligning the dual clocks (`MetronomeClock` 120Hz and microsecond UMP clock) for seamless bi-directional `.vixi` transferability.

---

## 1. Objectives & Architectural Context

We are processing eight raw environment PNG assets located on the desktop:
*   `E01_forest_entry.png`
*   `E02_forest_deep.png`
*   `E03_camp_exterior.png`
*   `E04_camp_interior.png`
*   `E05_spirit_forest.png`
*   `E06_boss_arena.png`
*   `E07_celestial_bastion.png`
*   `E08_forest_canopy_vista.png`

Our goals are:
1.  **3-Click "6-16-60" Creation-to-Game Loop:** Design a tactile, frictionless UIUX workflow that lets anyone (from a 6-year-old child to a 16-year-old hacker or a 60-year-old grandparent) import, refine, sign, and hot-reload assets in-game in exactly 3 clicks.
2.  **VIXI Panel Specification:** Author a fully-compliant, stub-free VIXIScript workspace panel layout (**`zone_builder.kit.vixi`**) adhering strictly to VIXI core laws (no region exceeding $\le 4 \pm 1$ children, size-based preattentive hierarchy, and strict contrast ratios).
3.  **Engine Ingestion & Parallax Mapping:** Weld these newly authored assets into the native `ironroot` rendering engine as high-fidelity multi-layer parallax scrolling backgrounds, associated with a new `"spirit"` element theme.
4.  **VIXI Theme Polish:** Map the active element color to the spectral violet token (`accent_wonder` / `#C46BFFFF`) inside `ironroot_glass.profile.sheet.vixi`.
5.  **Double-Clock Calibration & .vixi Bridge:** Establish lossless bi-directional timeline transfers by aligning the 120 Hz deterministic Metronome frame clock with the microsecond UMP timeline clock.
6.  **E2E Validation:** Verify the entire pipeline with a 5-tier automated testing suite.

---

## 2. Key Files & Context

*   **Sovereign Game Binary:** `ironroot.exe` (compiles from `crates/ironroot`)
*   **Asset Ingestion Interface:** `crates/ironroot/src/cartridge.rs` (the `init_render` and `render` pipeline)
*   **Game Cartridge Definition:** `crates/ironroot/ironroot.toml`
*   **Parallax Layout Engine:** `crates/ironroot/src/platform.rs` (`default_parallax` layer configuration)
*   **Authoring Surface Binary:** `13forge-studio.exe` (`.forge/bin/13forge-studio.exe`)
*   **VIXI Console Interface:** `crates/ironroot/vixi/ironroot_console.kit.vixi`
*   **VIXI Color Theme:** `crates/forge-canvas/themes/ironroot_glass.profile.sheet.vixi`
*   **VIXI Script Validator:** `.claude/skills/vixi-uiux/check.ps1`
*   **Studio Conductor Host:** `crates/forge-gui/src/conductor_host.rs`
*   **Semantic Clock Conductor:** `crates/forge-semantic/src/quad_lane.rs`
*   **VIXI Timeline Parser:** `crates/forge-vix/src/timeline.rs`
*   **Target Asset Directory:** `crates/ironroot/assets/textures/backgrounds/`

---

## 3. UIUX Wireframe Specification: The 3-Click Workspace Loop

To ensure immediate usability across the "6-16-60" user spectrum, we define the following 3-Click Creation-to-Game loop:

```
+─────────────────────────────────────────────────────────────────────────────+
| [HUD] Spirit Forest -- Grave-Orchard Depths                [PLAY IN STUDIO] |
+─────────────────────────────────────────────────────────────────────────────+
|                                                      |  [RIGHT CONTROL RAIL] |
|  [LEFT CANVAS WORKSPACE]                             |                       |
|                                                      |  +─────────────────+  |
|  * Artboard: Displays active multi-layer backgrounds  |  | CLICK 1: IMPORT |  |
|  * Sandbox: Drag-and-drop reference image to import  |  | [ Drop PNG File ] |  |
|  * Interactive: Left-click places terrain platforms  |  +─────────────────+  |
|  * Tooltips: Hover reveals voxel material depth      |  | CLICK 2: SIGN   |  |
|                                                      |  | [ Seal & Proof ] |  |
|                                                      |  +─────────────────+  |
|                                                      |  | CLICK 3: SHIP   |  |
|                                                      |  | [ Hot-Reload ]  |  |
|                                                      |  +─────────────────+  |
+──────────────────────────────────────────────────────┴──────────────────────+
| [INFO BAR]  Legend: [Platform] [Ground] [Pit] [Player Spawn]                |
+─────────────────────────────────────────────────────────────────────────────+
```

### The 3-Click Pipeline Interaction Matrix:
*   **Click 1: Import Concept (Frictionless):** Dragging-and-dropping any desktop PNG file (e.g. `E01_forest_entry.png`) onto the canvas instantly loads the asset. The studio executes `import_rgba_as_layer()`, automatically dither-quantizing the image colors into the compliant palette and scaling it to fit the background layer slots.
    *   *6-year-old path:* Immediate visual feedback of their custom image as the background.
    *   *16-year-old path:* The new layer appears in the `.vixi` code panel as an editable layer node with direct control over opacity and scroll-speed scalars.
    *   *60-year-old path:* Clear, legibly labeled "Import" file-drop field with a text description of the canvas contents.
*   **Click 2: Seal & Proof (Cryptographic Signature):** Click the "Seal & Proof" button (or press `Ctrl+E` / `F6`). This runs the compliance verification gates, signs the final flattened bytes with the local studio key (`.forge/keys/studio.ed25519`), and writes an Ed25519 authenticity receipt (`exports/canvas-<secs>.receipt.json`).
*   **Click 3: Ship & Play (Live Sandbox Refresh):** Click the "Ship to Game" button (or press `Ctrl+Shift+S`). This copies the signed PNG to the target asset folder, automatically configures the zone metadata in `ironroot.toml`, and signals **hot-reloading** inside the running game engine.
    *   The background immediately swaps on the screen, letting the user test character physics and platforms in front of their newly authored forest background in real-time.

---

## 4. Part 1: VIXI Workspace Panel Layout (`zone_builder.kit.vixi`)

We author the complete, compliant, stub-free VIXIScript kit representing this 3-click workspace panel.

We write this file to: **`crates/forge-vix/panels/zone_builder.kit.vixi`**:
```vixi
#vixi:kit v1
surface: zone_builder
profile: forge_studio
classification: workspace_surface

# ZONE BUILDER — The 3-Click "6-16-60" creation-to-game workspace panel.
# Features a left canvas displaying the artboard/platforms, and a right
# control rail containing the 3 tactile action buttons for importing,
# sealing/signing, and hot-shipping/playing the level assets.
#
# Cognitive load rules: every region holds <= 4+-1 children. Visual hierarchy
# is driven by a single preattentive attribute (Size). Contrast ratio is >= 4.5.

slot root kind=region layout=stack_h gap=mu(8) padding=mu(4)

# ── THE LEFT CANVAS WORKSPACE (65% width) ────────────────────────────────────
slot root.canvas_workspace kind=region layout=stack_v gap=mu(4) size=mu(650) priority=primary
slot root.canvas_workspace.hud_overlay kind=region layout=stack_h size=mu(44)
slot root.canvas_workspace.hud_overlay.title kind=text ramp=type.ramp[0] color=palette.accent_wonder name=zone_title
slot root.canvas_workspace.hud_overlay.element_tag kind=text ramp=type.ramp[1] color=palette.accent_wonder name=zone_element
slot root.canvas_workspace.hud_overlay.indicators kind=region layout=stack_h size=mu(120) name=status_indicators

# The artboard itself where dcomp and wgpu textures are rendered.
slot root.canvas_workspace.artboard kind=region layout=overlay role=canvas priority=primary

# ── THE RIGHT CONTROL RAIL (35% width) ──────────────────────────────────────
slot root.control_rail kind=region layout=stack_v gap=mu(8) padding=mu(4) size=mu(310) bind=palette.bg_near curve=chrome.curvature thick=chrome.thickness color=palette.border name=control_rail

# CLICK 1: IMPORT CONTAINER (tactile drop field)
slot root.control_rail.import_box kind=region layout=stack_v gap=mu(2) padding=mu(2) bind=palette.bg_active curve=chrome.curvature thick=chrome.thickness color=palette.separator name=import_box
slot root.control_rail.import_box.btn_import kind=widget name=icon_button on_click=edict:import.load_concept size=mu(44)
slot root.control_rail.import_box.label kind=text ramp=type.ramp[1] color=palette.text_primary name=import_label

# CLICK 2: SIGN & PROOF CONTAINER (Ed25519 signature generator)
slot root.control_rail.sign_box kind=region layout=stack_v gap=mu(2) padding=mu(2) bind=palette.bg_active curve=chrome.curvature thick=chrome.thickness color=palette.separator name=sign_box
slot root.control_rail.sign_box.btn_sign kind=widget name=icon_button on_click=edict:paint.seal_signature size=mu(44)
slot root.control_rail.sign_box.label kind=text ramp=type.ramp[1] color=palette.text_primary name=sign_label

# CLICK 3: SHIP & PLAY CONTAINER (copies assets and triggers engine hot-reload)
slot root.control_rail.ship_box kind=region layout=stack_v gap=mu(2) padding=mu(2) bind=palette.bg_active curve=chrome.curvature thick=chrome.thickness color=palette.separator name=ship_box
slot root.control_rail.ship_box.btn_ship kind=widget name=icon_button on_click=edict:ship.deploy_and_play size=mu(44)
slot root.control_rail.ship_box.label kind=text ramp=type.ramp[1] color=palette.text_primary name=ship_label

# UI law gates
gate contrast_min = 4.5
gate hit_target_min = mu(44)
gate runtime_parse = forbidden
gate alloc_steady = forbidden
gate float_in_ir = forbidden
```

---

## 5. Part 2: Sovereign Engine Integration & Texture Loading

We trace the complete, stub-free engineering wiring to ingest and scroll the authored backgrounds inside `ironroot.exe`.

### Step 2.1: Cartridge Zone Registration (`ironroot.toml`)
Update `crates/ironroot/ironroot.toml` to register the **Spirit Forest** zone:

```toml
[[arena_zones]]
id = "spirit_forest"
name = "The Spirit Forest"
[[arena_zones.phases]]
element = "spirit"
name = "Grave-Orchard Forest"
weather_override = "mystical"
duration_ticks = 2400
hazards = ["thorn_spikes", "spirit_wisps"]
```

### Step 2.2: Compile-time Ingestion (`cartridge.rs`)
Embed and load your authored PNG files inside `crates/ironroot/src/cartridge.rs` within the `init_render` initialization:

```rust
// 1. Embed raw authored forest PNG files into the sovereign binary
let canopy_png = include_bytes!("../assets/textures/backgrounds/forest_canopy_vista.png");
let entry_png = include_bytes!("../assets/textures/backgrounds/forest_entry.png");
let deep_png = include_bytes!("../assets/textures/backgrounds/forest_deep.png");

// 2. Upload assets to VRAM with Linear filtering for smooth scroll-scaling
let tex_canopy = texture_manager.load_from_png(device, queue, canopy_png, FilterMode::Linear)
    .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[30, 40, 50, 255], FilterMode::Linear));
let tex_entry = texture_manager.load_from_png(device, queue, entry_png, FilterMode::Linear)
    .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[20, 60, 30, 255], FilterMode::Linear));
let tex_deep = texture_manager.load_from_png(device, queue, deep_png, FilterMode::Linear)
    .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[10, 80, 20, 255], FilterMode::Linear));

// 3. Register identifiers in the renderer sprites map
const SPRITE_FOREST_CANOPY: MeshHandle = 301;
const SPRITE_FOREST_ENTRY: MeshHandle = 302;
const SPRITE_FOREST_DEEP: MeshHandle = 303;

pipelines.sprites.insert(SPRITE_FOREST_CANOPY, renderer::upload_sprite(device, tex_canopy));
pipelines.sprites.insert(SPRITE_FOREST_ENTRY, renderer::upload_sprite(device, tex_entry));
pipelines.sprites.insert(SPRITE_FOREST_DEEP, renderer::upload_sprite(device, tex_deep));
```

### Step 2.3: Parallax Mapping to the Spirit Element (`platform.rs`)
In `crates/ironroot/src/platform.rs`, update `default_parallax()` to resolve the `"spirit"` element phase to your uploaded background texture keys:

```rust
fn default_parallax(element: &str) -> Vec<ParallaxLayer> {
    let bg = match element {
        "fire"   => ("ember_sky", "ash_mountains", "charred_trees"),
        "water"  => ("storm_sky", "fog_mountains", "kelp_trees"),
        "earth"  => ("dusk_sky", "stone_mountains", "oak_trees"),
        "air"    => ("pale_sky", "cloud_mountains", "wind_grass"),
        "spirit" => ("forest_canopy_vista", "camp_exterior", "forest_deep"),
        _        => ("void_sky", "void_mountains", "void_trees"),
    };
    vec![
        ParallaxLayer { texture_ref: bg.0.into(), scroll_scale_permyriad: 0, z_order: -3 },     // Canopy Vista (Static)
        ParallaxLayer { texture_ref: bg.1.into(), scroll_scale_permyriad: 1000, z_order: -2 },  // Camp Exterior (Slow)
        ParallaxLayer { texture_ref: bg.2.into(), scroll_scale_permyriad: 5000, z_order: -1 },  // Forest Deep (Mid)
        ParallaxLayer { texture_ref: "foreground_dust".into(), scroll_scale_permyriad: 13000, z_order: 1 },
    ]
}
```

---

## 6. Part 3: Double-Clock Calibration & .vixi Bridge

To prevent timing drift and ensure timeline assets transfer losslessly between the in-studio Conductor and the game editor engine:

### Step 3.1: Clock Specifications
*   **The Deterministic Clock (`MetronomeClock`):** An absolute-integer sequencer (`u64` ticks) at **120 Hz**, driving L1 (physics) and L2 (rendering).
*   **The Microsecond UMP Clock (`universal_tick_us`):** A high-resolution timeline (`i64` microseconds) with a 32-us stamp precision, used inside the binary payload files and `.timeline.vixi` TOML sections for L0 (audio) and L3 (inference).

### Step 3.2: Translation Bridging Formulas
To maintain synchronization across both sides of the determinism firewall, apply the **120 Hz Calibration Quotient ($10^6 \text{ us} / 120 \text{ Hz} \approx 8333$ us per tick)**:

1.  **Import Timeline (`.vixi` $\to$ Conductor Tick):**
    Convert microsecond timestamps to Metronome frames for the tick scheduler:
    $$\text{Metronome Tick} = \frac{\text{universal\_tick\_us}}{8333}$$
2.  **Export Timeline (Conductor Tick $\to$ `.vixi`):**
    Translate absolute editor frame markers back to precise microsecond stamps inside the TOML sequence:
    $$\text{universal\_tick\_us} = \text{Metronome Tick} \times 8333$$

---

## 7. VIXI UI & Theme Polish

To provide "spectral" visual indicators when inside the **Spirit Forest** zone:
1.  **VIXI Token Map:** Map the active element color inside the game HUD's stylesheet using the **`ironroot_glass.profile.sheet.vixi`** palette.
    *   The element `"spirit"` maps directly to **`accent_wonder`** (`#C46BFFFF` - GHOST-400 spectral violet).
2.  **HUD Alignment:** Update the active zone indicator inside `ironroot_console.kit.vixi` so that its title or element status is styled using this signature color:
    ```vixi
    slot root.hud.stage.title kind=text ramp=type.ramp[0] color=palette.accent_wonder
    ```

---

## 8. End-to-End Pressure Testing

We execute a comprehensive 5-tier validation suite to ensure full compilation, layout safety, clock synchronization, and resource boundary compliance:

### Tier 8.1: Rust Structural Compiler Check
Execute a strict type and cargo liveness pass:
```powershell
cargo check -p ironroot --all-targets
```

### Tier 8.2: VIXIScript Layout & Law Validation
Validate the new `zone_builder.kit.vixi` and `ironroot_console.kit.vixi` layouts against the VIXI UX compiler rules.
*   **The Laws Checked:** Children density limit ($\le 4 \pm 1$ per group), single preattentive visual attribute (Size), and color contrast ratio ($\ge 4.5$).
```powershell
pwsh -NoProfile -File .claude/skills/vixi-uiux/check.ps1 crates/forge-vix/panels/zone_builder.kit.vixi
pwsh -NoProfile -File .claude/skills/vixi-uiux/check.ps1 crates/ironroot/vixi/ironroot_console.kit.vixi
```
*   **Pass Criteria:** `EXIT_0` and `0 warnings` on both.

### Tier 8.3: Conductor Clock-Sync Unit Testing
Verify that importing, mapping, and exporting timelines back and forth through the double-clock bridge yields exactly zero timing drift.
```powershell
cargo test -p forge-gui --test conductor_audio_e2e
```

### Tier 8.4: Resource Boundary and Memory Check
Check that the newly bundled PNG assets do not breach size constraints or contain un-quantized colors.
*   **Size Check:** Assets must be compressed and dithered to remain under the target boundary (**~2.5 MB** combined raw size to keep the executable lightweight).
*   **Format Check:** Strictly RGBA8-encoded PNG files.

### Tier 8.5: Interactive Sovereign Run
Run the executable and load **The Spirit Forest** zone to observe the parallax background layers scrolling smoothly in response to player physics:
```powershell
cargo run -p ironroot --bin ironroot-sovereign
```
*   **Visual Proof:** Observe the canopy vista static backplate, mid camp exterior scrolling slowly, and the near forest deep scrolling in harmony, with the active HUD tinted spectral violet.
