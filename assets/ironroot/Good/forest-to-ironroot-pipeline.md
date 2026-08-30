# Implementation Plan: AI-to-Authored Forest Asset Pipeline Integration

This document maps out the end-to-end pipeline for taking eight (8) AI-generated forest environment images from the artist's desktop, refining them into production-ready pixel assets using the `13forge-studio.exe` PAINT surface, and wiring them as parallax layers in the sovereign game engine (`ironroot.exe`).

---

## 1. Objectives & Context

We have eight high-quality concept art environment PNGs located on the desktop:
*   `E01_forest_entry.png`
*   `E02_forest_deep.png`
*   `E03_camp_exterior.png`
*   `E04_camp_interior.png`
*   `E05_spirit_forest.png`
*   `E06_boss_arena.png`
*   `E07_celestial_bastion.png`
*   `E08_forest_canopy_vista.png`

Our goals are:
1.  **Remediation & Quantization:** Process raw AI images to strip visual anomalies, map colors to compliance-tested palettes, and establish digital provenance via cryptographic signing (proving manual authorship).
2.  **Engine Ingestion:** Port these newly authored assets into the native `ironroot` rendering engine as high-fidelity multi-layer parallax scrolling backgrounds.

---

## 2. Key Files & Context

*   **Sovereign Game Binary:** `ironroot.exe` (compiles from `crates/ironroot`)
*   **Asset Ingestion Interface:** `crates/ironroot/src/cartridge.rs` (the `init_render` and `render` pipeline)
*   **Game Cartridge Definition:** `crates/ironroot/ironroot.toml`
*   **Parallax Layout Engine:** `crates/ironroot/src/platform.rs` (`default_parallax` layer configuration)
*   **Authoring Surface Binary:** `13forge-studio.exe` (`.forge/bin/13forge-studio.exe`)
*   **Target Asset Directory:** `crates/ironroot/assets/textures/backgrounds/` (or `crates/ironroot/assets/concept/concepts-v3/`)

---

## 3. Part 1: The Studio PAINT Remediation (Artist Workflow)

To transition raw AI concepts into verified "authored" pixel art, use the following interactive paint loops in the `13forge-studio` canvas:

### Step 1.1: Launch Studio Paint Mode
Open your PowerShell console and run the studio:
```powershell
& "F:\NewRepo\.forge\bin\13forge-studio.exe" paint
```

### Step 1.2: Canvas Import and Layer Assembly
1.  Press **`C`** to clear the artboard and establish a blank project canvas.
2.  Drag-and-drop the target environment PNG (e.g., `C:\Users\seanm\Desktop\Good\E01_forest_entry.png`) onto the running window. 
    *   This scales the image and places it as a new layer (**"Import 1"**) on top of your project stack.
3.  Press **`Home`** to auto-fit the screen or **`Shift + Home`** for a strict 1:1 pixel zoom.

### Step 1.3: Authoring, Quantization & Touch-ups
1.  Press **`L`** to create a blank refinement layer on top of the imported image.
2.  Press **`Tab`** to cycle active layer selection.
3.  Use the following hotkeys to manually touch up, dither, or draw:
    *   **Brush:** `V` (Select brush tool).
    *   **Line:** `K` (Draw straight boundaries / platforms).
    *   **Fill:** `F` (Fill areas).
    *   **Erase:** `E` (Toggle eraser mode).
    *   **Branding Swatches:** `1` through `5` for active era-compliant colors.
    *   **Tonal Nudges:** Saturation `H`/`J` | Value/Brightness `U`/`I`.
    *   **Opacity:** `O` (Cycles through `64`, `128`, `192`, `255`).

### Step 1.4: Provenance Signature Export
Once complete, save and export the flattened work-product:
1.  Press **`Ctrl + S`** to save the project. This triggers compiled authoring checks (`authoring_gates_forge13`) to ensure no color-space violations remain.
2.  Press **`Ctrl + E`** to export the signed, production-ready PNG.
    *   This signs the output bytes with your local studio identity (`.forge/keys/studio.ed25519`) and writes an Ed25519 cryptographic authenticity proof beside it (`exports/canvas-<secs>.receipt.json`).
3.  Copy the exported PNG into the engine's assets:
    ```powershell
    Copy-Item "F:\NewRepo\exports\canvas-<secs>.png" "F:\NewRepo\crates\ironroot\assets\textures\backgrounds\forest_entry.png" -Force
    ```

---

## 4. Part 2: Sovereign Engine Integration (Engineer Workflow)

To consume your authored parallax scrolling layers within the active game loop of `ironroot.exe`, execute these integration steps:

### Step 2.1: Register a New Zone in `ironroot.toml`
Add a custom static zone designated for your forest environment. By declaring `"spirit"` as its element, we can dynamically link it to our custom background layers.

```toml
[[arena_zones]]
id = "spirit_forest"
name = "The Spirit Forest"
[[arena_zones.phases]]
element = "spirit"
name = "Grave-Orchard Forest"
weather_override = "mystical"
hazards = ["thorn_spikes", "spirit_wisps"]
```

### Step 2.2: Load Textures Into GPU VRAM (`cartridge.rs`)
Integrate the raw asset binary loads inside `crates/ironroot/src/cartridge.rs` within `init_render`.

1.  Embed the authored forest PNG files using `include_bytes!`:
    ```rust
    let canopy_png = include_bytes!("../assets/textures/backgrounds/forest_canopy_vista.png");
    let entry_png = include_bytes!("../assets/textures/backgrounds/forest_entry.png");
    let deep_png = include_bytes!("../assets/textures/backgrounds/forest_deep.png");
    ```
2.  Upload textures via the device's `TextureManager` and assign specific handle IDs:
    ```rust
    let tex_canopy = texture_manager.load_from_png(device, queue, canopy_png, FilterMode::Linear)
        .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[30, 40, 50, 255], FilterMode::Linear));
    let tex_entry = texture_manager.load_from_png(device, queue, entry_png, FilterMode::Linear)
        .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[20, 60, 30, 255], FilterMode::Linear));
    let tex_deep = texture_manager.load_from_png(device, queue, deep_png, FilterMode::Linear)
        .unwrap_or_else(|_| texture_manager.load_rgba(device, queue, 1, 1, &[10, 80, 20, 255], FilterMode::Linear));

    const SPRITE_FOREST_CANOPY: MeshHandle = 301;
    const SPRITE_FOREST_ENTRY: MeshHandle = 302;
    const SPRITE_FOREST_DEEP: MeshHandle = 303;

    pipelines.sprites.insert(SPRITE_FOREST_CANOPY, renderer::upload_sprite(device, tex_canopy));
    pipelines.sprites.insert(SPRITE_FOREST_ENTRY, renderer::upload_sprite(device, tex_entry));
    pipelines.sprites.insert(SPRITE_FOREST_DEEP, renderer::upload_sprite(device, tex_deep));
    ```

### Step 2.3: Map Parallax Backgrounds to the Spirit Element (`platform.rs`)
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
        ParallaxLayer { texture_ref: bg.0.into(), scroll_scale_permyriad: 0, z_order: -3 },     // Vista (Static)
        ParallaxLayer { texture_ref: bg.1.into(), scroll_scale_permyriad: 1000, z_order: -2 },  // Exterior (Slow)
        ParallaxLayer { texture_ref: bg.2.into(), scroll_scale_permyriad: 5000, z_order: -1 },  // Deep (Mid)
        ParallaxLayer { texture_ref: "foreground_dust".into(), scroll_scale_permyriad: 13000, z_order: 1 },
    ]
}
```

---

## 5. Verification & Testing

To confirm compile and execution liveness:
1.  **Cargo Check:** Validate types and syntax are fully compiling:
    ```powershell
    cargo check -p ironroot
    ```
2.  **Platform Layout Tests:** Run built-in structural checks to verify that the four parallax layers parse, sort, and yield correct scroll scales:
    ```powershell
    cargo test -p ironroot --test pbt_new_systems
    ```
3.  **Run Sovereign Engine:** Launch `ironroot.exe` and select **The Spirit Forest** zone to view live parallax scrolling integrated with procedural voxel terrain and player movement:
    ```powershell
    cargo run -p ironroot --bin ironroot-sovereign
    ```
