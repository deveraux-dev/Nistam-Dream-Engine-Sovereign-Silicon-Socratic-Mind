# 19. Visual Artifact Remediation

<!-- [BOARD: VIXI-LOWER] -->

Panels collapse into ugly, broken, or squished visual artifacts when they lack explicit viewport constraints, flex-shrink rules, and theme token bindings. When a `.kit.vixi` panel is loaded without an active container context, it defaults to zero-width bounds, collapses its content, and renders un-styled wireframe placeholders.

Here is how we turn the visual presentation from raw placeholders into clean, production-grade UI.

---

## 1. The Container Wrapper (`.kit.vixi` Standard)

Every panel needs a standardized root wrapper with explicit bounding, padding, and tokenized colors (`tokens.css`). Wrap the raw panel definitions in this clean container geometry:

```vixiscript
// Panel Layout Shell Standard
container root {
    min_width = 320
    min_height = 240
    flex_grow = 1
    padding = [12, 16]
    gap = 8
    background = "var(--bg-surface-0)"
    border = "1px solid var(--border-subtle)"
    corner_radius = 6
    overflow = "clip"

    header panel_title_bar {
        height = 28
        layout = "row"
        align = "space-between"
        border_bottom = "1px solid var(--border-muted)"
        
        text label {
            color = "var(--fg-primary)"
            font = "var(--font-mono-bold)"
            size = 12
        }
    }

    content_area main_viewport {
        flex_grow = 1
        layout = "column"
        // Active component body injects here
    }
}
```

---

## 2. Eliminating the 3 Main Visual Bugs

| Visual Bug | Root Cause | Fix |
| :--- | :--- | :--- |
| **Squished / Collapsed Text** | Unbounded Flex Shrinking | Set `flex_shrink = 0` on label and button elements inside panel rows. |
| **Garbage / Unstyled Colors** | Hardcoded or missing RGBs | Bind all background/foreground parameters to `TokenSheet` / `tokens.css` design variables. |
| **Clipped Canvas (`scan`, `world_node`)** | Missing parent container bounds | Enforce explicit `min_width` / `min_height` on the panel's root node during the lowering pass in Rust. |

---

## 3. Rust Bounding Guard (`crates/forge-vix-v3`)

In `lower_panel()`, inject automatic layout guards so even incomplete or unrendered panels automatically inherit minimum viewport dimensions and clean fallback styles instead of breaking the Studio layout:

```rust
pub fn sanitize_panel_bounds(node: &mut VixiNode) {
    // Enforce default fallback dimensions if panel specifies none
    if node.min_width.is_none() {
        node.min_width = Some(280.0);
    }
    if node.min_height.is_none() {
        node.min_height = Some(180.0);
    }
    
    // Ensure overflow doesn't spill into adjacent Studio viewports
    node.style.overflow = Overflow::Clip;
}
```

---

## 4. Topographic Mapping Standards (The Latent Lateral Connection)

Connecting raw backend state to visual elements requires tracing a deterministic event and data flow:

$$\text{Data Source} \longrightarrow \text{Spatial Index} \longrightarrow \text{State Channel} \longrightarrow \text{UI Binding} \longrightarrow \text{Audio/Telemetry Readback}$$

### A. Parser-to-UI Translation (CST/AST)
*   **The Source:** Raw VixiScript parsed via `forge-ast` (and `grammar_bridge.rs`) yields a CST (Concrete Syntax Tree) representing exact source coordinates, whitespace, and comments, which is simplified into an AST.
*   **The UI Binding:** This AST compiles into a `KitDoc` representing the AST's document model, containing a vector of `baked` slots with event bindings like `on_click_edict` and `on_key_edict`.
*   **The Lowering Seam:** The `lower()` pass in `forge-vix` maps `KitDoc` nodes to layout boxes (`LoweredUi`), joining `baked` event keys with actual viewport geometry via their `stable_key` (e.g., `root.face.portals.press_page`).

### B. 5D Spatial Indexing
*   **Coordinate Standard:** Layout geometry utilizes absolute integer-based MilliUnits (MU) where $100\% = 1,000,000\text{ MU}$ (e.g., $1920\text{ px} = 1,920,000\text{ MU}$).
*   **Bounding:** When 5D spatial vectors or pickers are drawn, they must map to a `role=canvas` or `role=brush` box. The click center coordinates `(cx, cy)` are evaluated relative to the absolute bounding box of the active viewport (e.g., subtracting `TOPBAR_H_MU` to prevent hit-testing drifting).

---

## 5. The Weld & Wire Workflow (Integration)

Integrating a new visual panel into `forge-studio` follows a strict 5-step workflow to guarantee compilation, validation, and layout safety:

### Step 1: Author the `.kit.vixi` File
Write the visual panel structure inside `F:\NewRepo\crates\forge-vix\panels/<name>.kit.vixi`, ensuring it conforms to the standard wrapper and uses flex-shrink and tokenized variables.

### Step 2: Register in `STUDIO_PANELS`
Embed the file into the central registry inside `crates/forge-vix-v3/src/loader.rs`:
```rust
pub const STUDIO_PANELS: &[(&str, &str)] = &[
    ...
    ("<name>", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/panels/<name>.kit.vixi"))),
];
```

### Step 3: Add a Proof Spec
Add a layout validation entry to `PANEL_PROOF_SPECS` inside the `every_panel_has_a_readback_proof` test in `loader.rs`:
```rust
const PANEL_PROOF_SPECS: &[(&str, &str)] = &[
    ...
    ("<name>", "root.some_identifying_element_stable_key"),
];
```
This forces validation, verifying that the panel lowers without parser errors and contains the expected control layout.

### Step 4: Write the Rust Stranglers
Implement a thin strangler in `F:\NewRepo\crates\forge-gui\src/<name>_kit.rs` or `crates/forge-studio-v3/src/<name>_kit.rs` to handle loading, data-binding, and visual state injection:
```rust
pub fn render_custom_kit(viewport: IrRect, state: &State) -> LoweredUi {
    let src = forge_vix::loader::studio_panel("<name>").unwrap();
    let ctx = TokenCtx::comfy();
    let mut loaded = forge_vix::loader::load_kit(src, &ctx, viewport, 1).unwrap();
    
    // Programmatic Bounding Guard
    sanitize_panel_bounds(&mut loaded.ui.root);
    
    // Bind dynamic data (e.g., telemetry values) to specific slots
    bind_telemetry_fields(&mut loaded.ui, state);
    
    loaded.ui
}
```

### Step 5: Wire the Edicts
Map user click and keystroke actions in `F:\NewRepo\crates\forge-studio\src\edicts.rs` (`dispatch` & `execute` arms) to trigger state transitions or trigger actions.

