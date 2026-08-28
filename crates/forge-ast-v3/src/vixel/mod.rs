//! # Vixel DSL — Core Data Model
//!
//! Typed AST for the VixiScript language. Parsed at build time by
//! `grammar_bridge.rs`, routed through three semantic branches
//! (Material, Spatial, Automata) plus an Environment branch for
//! `set_*` calls.
//!
//! **Constraints:**
//! - No runtime dependencies — build-time only
//! - All numeric values in permyriad (u16, 0–10000)
//! - Parser returns `ParseError` on malformed input — never panics

pub mod ast_extractor;
pub mod ast_optimizer;
pub mod capability_index;
pub mod grammar_bridge;
pub mod physics_qa;
pub mod ui_lower;
pub mod vixel_gate;
pub mod security_gate;

/// Parse ONE `.vixel` source. `parse_vixel_dir` has always called this (line 144);
/// it just had no name at this level, so a caller that wanted per-file parsing —
/// and per-file error recovery, which the directory verb cannot give — had to fail
/// the whole directory on the first bad file.
pub use grammar_bridge::parse_vixel_source;

// ---------------------------------------------------------------------------
// Core AST
// ---------------------------------------------------------------------------

/// Root of a parsed `.vixel` file set.
///
/// Four branches:
/// - `materials`  → forge-furnace → `.forge_reg`
/// - `spatials`   → socket graph / chunk placement
/// - `automata`   → forge-shader-build → `.spv` / `.dxil`
/// - `environment`→ uniform buffer constants (`set_*` calls)
#[derive(Debug, Clone, PartialEq)]
pub struct VixelAst {
    /// Material definitions parsed from .vixel files.
    pub materials: Vec<MaterialDef>,
    /// Spatial/socket graph placement definitions.
    pub spatials: Vec<SpatialDef>,
    /// Cellular automata rule definitions.
    pub automata: Vec<AutomataDef>,
    /// Environment directive definitions (temperature, wind, gravity).
    pub environment: Vec<EnvironmentDef>,
    /// UI element definitions.
    pub ui_defs: Vec<UiDef>,
    /// Theme token cascade definitions.
    pub themes: Vec<ThemeDef>,
    /// Authored `atom { ... }` blocks — VixelAtom primitives (VixiScript's lowering target).
    pub atoms: Vec<AtomDef>,
    /// Authored `acrylic { ... }` blocks — AcrylicLoad/stamp_acrylic paint dabs.
    pub acrylics: Vec<AcrylicDef>,
    /// Authored `pressure { ... }` blocks — PressureCurve pen-feel curves.
    pub pressures: Vec<PressureDef>,
    /// Authored `layers { ... }` blocks — LayerStack paint-layer depth.
    pub layers: Vec<LayersDef>,
    /// Authored `viewport { ... }` blocks — VixelViewport camera uniforms.
    pub viewports: Vec<ViewportDef>,
    /// Authored `brush { ... }` blocks — BrushMask/MaskStamp brush tips.
    pub brushes: Vec<BrushDef>,
}

impl VixelAst {
    /// Create an empty AST.
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            spatials: Vec::new(),
            automata: Vec::new(),
            environment: Vec::new(),
            ui_defs: Vec::new(),
            themes: Vec::new(),
            atoms: Vec::new(),
            acrylics: Vec::new(),
            pressures: Vec::new(),
            layers: Vec::new(),
            viewports: Vec::new(),
            brushes: Vec::new(),
        }
    }
}

impl Default for VixelAst {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Directory-Level Public API
// ---------------------------------------------------------------------------

use std::path::Path;

/// Parse all `.vixel` files in `dir` (non-recursive) and merge into a single
/// [`VixelAst`] with globally unique IDs.
///
/// Files are sorted alphabetically for deterministic parse order. Each file's
/// IDs are offset by the running count from previously parsed files so that
/// `MaterialDef.id`, `SpatialDef.id`, and `AutomataDef.id` are unique across
/// the entire directory.
///
/// # Errors
///
/// Returns the first [`ParseError`] encountered. The error includes the
/// originating file path and line number.
pub fn parse_vixel_dir(dir: &Path) -> Result<VixelAst, ParseError> {
    let mut vixel_files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| ParseError {
            file: dir.display().to_string(),
            line: 0,
            message: format!("cannot read directory: {}", e),
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("vixel") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort alphabetically for deterministic parse order
    vixel_files.sort();

    let mut merged = VixelAst::new();
    let mut mat_offset: u16 = 0;
    let mut spatial_offset: u16 = 0;
    let mut automata_offset: u16 = 0;
    let mut ui_offset: u16 = 0;
    let mut atom_offset: u16 = 0;
    let mut acrylic_offset: u16 = 0;
    let mut pressure_offset: u16 = 0;
    let mut layers_offset: u16 = 0;
    let mut viewport_offset: u16 = 0;
    let mut brush_offset: u16 = 0;

    for path in &vixel_files {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        let source = std::fs::read_to_string(path).map_err(|e| ParseError {
            file: file_name.to_string(),
            line: 0,
            message: format!("cannot read file: {}", e),
        })?;

        let mut ast = grammar_bridge::parse_vixel_source(&source, file_name)?;

        // Offset IDs to be globally unique across files
        for mat in &mut ast.materials {
            mat.id = mat.id.checked_add(mat_offset).unwrap_or(mat.id);
        }
        for spatial in &mut ast.spatials {
            spatial.id = spatial.id.checked_add(spatial_offset).unwrap_or(spatial.id);
        }
        for rule in &mut ast.automata {
            rule.id = rule.id.checked_add(automata_offset).unwrap_or(rule.id);
        }
        for ui in &mut ast.ui_defs {
            ui.id = ui.id.checked_add(ui_offset).unwrap_or(ui.id);
        }
        for atom in &mut ast.atoms {
            atom.id = atom.id.checked_add(atom_offset).unwrap_or(atom.id);
        }
        for a in &mut ast.acrylics {
            a.id = a.id.checked_add(acrylic_offset).unwrap_or(a.id);
        }
        for p in &mut ast.pressures {
            p.id = p.id.checked_add(pressure_offset).unwrap_or(p.id);
        }
        for l in &mut ast.layers {
            l.id = l.id.checked_add(layers_offset).unwrap_or(l.id);
        }
        for v in &mut ast.viewports {
            v.id = v.id.checked_add(viewport_offset).unwrap_or(v.id);
        }
        for b in &mut ast.brushes {
            b.id = b.id.checked_add(brush_offset).unwrap_or(b.id);
        }

        mat_offset += ast.materials.len() as u16;
        spatial_offset += ast.spatials.len() as u16;
        automata_offset += ast.automata.len() as u16;
        ui_offset += ast.ui_defs.len() as u16;
        atom_offset += ast.atoms.len() as u16;
        acrylic_offset += ast.acrylics.len() as u16;
        pressure_offset += ast.pressures.len() as u16;
        layers_offset += ast.layers.len() as u16;
        viewport_offset += ast.viewports.len() as u16;
        brush_offset += ast.brushes.len() as u16;

        merged.materials.extend(ast.materials);
        merged.spatials.extend(ast.spatials);
        merged.automata.extend(ast.automata);
        merged.environment.extend(ast.environment);
        merged.ui_defs.extend(ast.ui_defs);
        merged.themes.extend(ast.themes);
        merged.atoms.extend(ast.atoms);
        merged.acrylics.extend(ast.acrylics);
        merged.pressures.extend(ast.pressures);
        merged.layers.extend(ast.layers);
        merged.viewports.extend(ast.viewports);
        merged.brushes.extend(ast.brushes);
    }

    Ok(merged)
}

// ---------------------------------------------------------------------------
// Material Branch
// ---------------------------------------------------------------------------

/// A single material definition parsed from a `material "name" { … }` block.
///
/// All physical properties are stored in permyriad (0–10000).
/// `destruction_mode`: 0 = Shatter, 1 = Splinter, 2 = Melt.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct MaterialDef {
    /// Unique material identifier.
    pub id: u16,
    /// Material name buffer (fixed 32-byte capacity).
    pub name: [u8; 32],
    /// Actual length of name string.
    pub name_len: usize,
    /// Base colour RGBA packed as u32.
    pub albedo: u32,
    /// Surface roughness in permyriad (0–10000).
    pub roughness_pmy: u16,
    /// Metallicity in permyriad (0–10000).
    pub metallic_pmy: u16,
    /// Mass in permyriad (0–10000).
    pub mass_pmy: u16,
    /// Hardness in permyriad (0–10000).
    pub hardness_pmy: u16,
    /// Flammability in permyriad (0–10000).
    pub flammability_pmy: u16,
    /// Destruction mode: 0=Shatter, 1=Splinter, 2=Melt.
    pub destruction_mode: u8,
}

impl MaterialDef {
    /// Helper: read the name as a `&str`.
    pub fn name_str(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}


// ---------------------------------------------------------------------------
// Atom Branch — VixiScript's lowering target
// ---------------------------------------------------------------------------

/// A single vixel atom parsed from an
/// `atom { coord: (x, y), material_id: N, resonance: Np, color: 0xRRGGBB }` block.
///
/// The **lowering target of VixiScript** — mirrors `forge_daemon_types::atom::VixelAtom`
/// (ColourID / MaterialID / Resonance). This block is the authoring surface for the
/// VixelAtom primitive: the `atom` keyword gives the runtime atom an `ast_arm`
/// (`grammar_bridge::parse_atom`) and an LSP mirror (`VIXEL_ATOM_KEYS`) — the join that
/// takes VixelAtom from runtime-only to authorable + terminal-audible.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AtomDef {
    /// Unique atom identifier.
    pub id: u16,
    /// Grid coordinate `(x, y)` — integer pair.
    pub coord: (i32, i32),
    /// Material registry id (matches `MaterialId`).
    pub material_id: u16,
    /// Resonance — Permyriad-scaled audio coupling (0..=10000).
    pub resonance: u16,
    /// Packed RGBA colour (`0xRRGGBBAA` / `0xRRGGBB`).
    pub color: u32,
}

/// A paint dab parsed from an
/// `acrylic { color: 0xRRGGBB, material_id: N, essence_id: N, phase: Np }` block.
///
/// Authoring surface for `forge_core::acrylic::AcrylicLoad` / `stamp_acrylic` — the
/// voxel-acrylic colour/material/essence brush load.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcrylicDef {
    /// Unique acrylic identifier.
    pub id: u16,
    /// Packed RGBA colour of the dab.
    pub color: u32,
    /// Material registry id.
    pub material_id: u16,
    /// Essence id (semantic tag baked into the atom).
    pub essence_id: u16,
    /// Deposit phase — Permyriad (0..=10000).
    pub phase: u16,
}

/// A pen-feel curve parsed from a `pressure { curve: linear|soft|hard }` block.
///
/// Authoring surface for `forge_core::pressure::PressureCurve` / `pressure_strength`
/// (the Wacom↔voxel-brush pressure seam).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PressureDef {
    /// Unique pressure definition identifier.
    pub id: u16,
    /// 0=Linear, 1=Soft, 2=Hard (mirrors `forge_core::pressure::PressureCurve`).
    pub curve: u8,
}

/// Paint layer-stack depth parsed from `layers { count: N }`.
/// Authoring surface for `forge_core::layer_stack::LayerStack` / `flatten_into`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayersDef {
    /// Unique layer stack identifier.
    pub id: u16,
    /// Number of stacked paint layers.
    pub count: u16,
}

/// Camera/viewport parsed from `viewport { w: N, h: N, zoom: Np }`.
/// Authoring surface for `forge_gpu::vixel_pass::VixelViewport`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewportDef {
    /// Unique viewport identifier.
    pub id: u16,
    /// Width in MilliUnits.
    pub w: u16,
    /// Height in MilliUnits.
    pub h: u16,
    /// Zoom — Permyriad (10000 = 1.0×).
    pub zoom: u16,
}

/// Brush tip parsed from `brush { w: N, h: N, falloff: Np }`.
/// Authoring surface for `forge_core::brush_mask::BrushMask` / `MaskStamp` (procedural
/// w×h falloff tip; the authored `.exr` source is the other path).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrushDef {
    /// Unique brush identifier.
    pub id: u16,
    /// Brush width in pixels.
    pub w: u16,
    /// Brush height in pixels.
    pub h: u16,
    /// Edge falloff — Permyriad (0..=10000).
    pub falloff: u16,
}


// ---------------------------------------------------------------------------
// Spatial Branch
// ---------------------------------------------------------------------------

/// Spatial placement definition parsed from `spawn_*` calls.
///
/// Up to 6 connection sockets (signed i8 offsets).
/// `stress_limit_pmy` is the structural stress limit in permyriad.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct SpatialDef {
    /// Unique spatial/socket graph identifier.
    pub id: u16,
    /// Connection socket array (up to 6 sockets with signed i8 offsets).
    pub sockets: [(i8, i8, i8); 6],
    /// Number of active sockets.
    pub socket_count: u8,
    /// Structural stress limit in permyriad (0–10000).
    pub stress_limit_pmy: u16,
}


// ---------------------------------------------------------------------------
// Automata Branch
// ---------------------------------------------------------------------------

/// A cellular automata rule parsed from a `rule "name" { … }` block.
///
/// `wgsl_source` is `@forge:allow_alloc` — heap allocation is permitted
/// here because this is build-time only (never reaches the hot path).
#[derive(Debug, Clone, PartialEq)]
pub struct AutomataDef {
    /// Unique automata rule identifier.
    pub id: u16,
    /// Classification of the automata rule.
    pub rule_type: AutomataType,
    /// WGSL shader source code.
    pub wgsl_source: String,
}

impl Default for AutomataDef {
    fn default() -> Self {
        Self {
            id: 0,
            rule_type: AutomataType::Custom,
            wgsl_source: String::new(),
        }
    }
}

/// Automata rule classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomataType {
    /// Fire spread automata rule.
    Fire,
    /// Fluid dynamics automata rule.
    Fluid,
    /// Gravity/falling automata rule.
    Gravity,
    /// Custom user-defined automata rule.
    Custom,
}

// ---------------------------------------------------------------------------
// Environment Branch
// ---------------------------------------------------------------------------

/// An environment directive parsed from `set_temperature`, `set_wind`,
/// or `set_gravity` calls.
///
/// - `target`: material name for targeted calls (e.g. `set_temperature("fire", 9500)`)
/// - `value_pmy`: scalar permyriad value
/// - `vector`: directional vector for `set_wind` / `set_gravity`
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentDef {
    /// Type of environment directive.
    pub env_type: EnvironmentType,
    /// Target material name buffer (fixed 32-byte capacity).
    pub target: [u8; 32],
    /// Actual length of target name string.
    pub target_len: usize,
    /// Scalar permyriad value for the directive.
    pub value_pmy: u16,
    /// 3D directional vector (used by wind/gravity).
    pub vector: [i32; 3],
}

impl EnvironmentDef {
    /// Helper: read the target name as a `&str`.
    pub fn target_str(&self) -> &str {
        std::str::from_utf8(&self.target[..self.target_len]).unwrap_or("")
    }
}

impl Default for EnvironmentDef {
    fn default() -> Self {
        Self {
            env_type: EnvironmentType::Temperature,
            target: [0u8; 32],
            target_len: 0,
            value_pmy: 0,
            vector: [0; 3],
        }
    }
}

/// Environment directive classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentType {
    /// Temperature environment directive.
    Temperature,
    /// Wind force environment directive.
    Wind,
    /// Gravity force environment directive.
    Gravity,
}

// ---------------------------------------------------------------------------
// Token & Theme Types (VixiScript design token cascade)
// ---------------------------------------------------------------------------

/// How a color value is specified — literal or token reference.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    /// Raw packed RGBA colour literal.
    Literal(u32),
    /// Token reference: `$accent_creation` — resolved at build/runtime.
    TokenRef(String),
}

impl Default for ColorValue {
    fn default() -> Self { ColorValue::Literal(0xFFFFFFFF) }
}

/// Repeat direction for list-style UI elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RepeatDir {
    /// No repetition.
    #[default]
    None,
    /// Vertical repetition.
    Vertical,
    /// Horizontal repetition.
    Horizontal,
}


/// Fill direction for progress bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum FillDir {
    /// Horizontal fill direction.
    #[default]
    Horizontal,
    /// Vertical fill direction.
    Vertical,
}


/// Particle emitter shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum EmitterType {
    /// Emit particles from edge boundary.
    #[default]
    Edge,
    /// Emit particles filling the area.
    Fill,
    /// Emit particles from a single point.
    Point,
}


/// Comparison operator for conditional rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// Less than comparison.
    Lt,
    /// Greater than comparison.
    Gt,
}

/// A runtime data binding target (e.g., `player_hp`, `moon_phase`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BindTarget(/// Name of the data binding target.
pub String);

/// Spring physics parameters (permyriad values).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpringDef {
    /// Spring stiffness coefficient.
    pub stiffness: i32,
    /// Spring damping coefficient.
    pub damping: i32,
    /// Spring scale factor.
    pub scale: i32,
}

/// Particle emitter definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleDef {
    /// Emitter shape/type.
    pub emitter: EmitterType,
    /// Particle colour (literal or token reference).
    pub color: ColorValue,
    /// Emission rate (particles per frame).
    pub rate: u16,
    /// Particle lifetime in milliseconds.
    pub lifetime: u16,
    /// Optional data binding for particle intensity.
    pub intensity_bind: Option<BindTarget>,
}

impl Default for ParticleDef {
    fn default() -> Self {
        Self { emitter: EmitterType::Edge, color: ColorValue::default(), rate: 1, lifetime: 500, intensity_bind: None }
    }
}

/// Inner fill definition (progress bars, meters).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillDef {
    /// Material name for the fill.
    pub material_name: String,
    /// Data binding for fill amount.
    pub bind: BindTarget,
    /// Fill direction (horizontal or vertical).
    pub direction: FillDir,
}

/// Inline conditional rule (state-driven overrides).
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalRule {
    /// Data binding for the condition check.
    pub bind: BindTarget,
    /// Comparison operator.
    pub op: CmpOp,
    /// Threshold value to compare against.
    pub threshold: i32,
    /// Optional colour override when condition is true.
    pub color_override: Option<ColorValue>,
    /// Optional emissive override value.
    pub emissive_override: Option<i32>,
    /// Optional particle effect when condition is true.
    pub particle: Option<ParticleDef>,
    /// Optional sound event when condition is true.
    pub sound: Option<String>,
}

/// Token cascade layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum ThemeLayer {
    /// Base token layer.
    #[default]
    Base = 0,
    /// Profile-specific token layer.
    Profile = 1,
    /// Celestial/context token layer.
    Celestial = 2,
    /// Override token layer.
    Override = 3,
}


/// A single token definition within a theme.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDef {
    /// Token name (referenced as `$name` in VixiScript).
    pub name: String,
    /// Packed RGBA colour value of the token.
    pub value: u32,
}

/// Theme definition — a named set of token values at a specific cascade layer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThemeDef {
    /// Theme name.
    pub name: String,
    /// Cascade layer this theme belongs to.
    pub layer: ThemeLayer,
    /// Tokens defined in this theme.
    pub tokens: Vec<TokenDef>,
}

// ---------------------------------------------------------------------------
// UI Definition (build-time DrawCmd lowering)
// ---------------------------------------------------------------------------

/// A declarative UI element definition with full engine capabilities:
/// layout, materials, physics, particles, sound, data binding, token colors.
///
/// All layout values are MilliUnit (1000 = 1px), all colors are packed u32
/// RGBA or `$token` references. Build-time only — lowered to static arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct UiDef {
    /// Unique UI element identifier.
    pub id: u16,
    /// Element name buffer (fixed 32-byte capacity).
    pub name: [u8; 32],
    /// Actual length of name string.
    pub name_len: usize,
    /// X position in MilliUnits.
    pub x: i64,
    /// Y position in MilliUnits.
    pub y: i64,
    /// Width in MilliUnits.
    pub w: i64,
    /// Height in MilliUnits.
    pub h: i64,
    /// Element colour (literal or token reference).
    pub color: ColorValue,
    /// Selected state colour (optional).
    pub color_selected: Option<ColorValue>,
    /// Material registry index.
    pub material_idx: u8,
    /// Material name (optional override).
    pub material_name: Option<String>,
    /// Vibe/vibration mask flags.
    pub vibe_mask: u8,
    /// Corner radius in MilliUnits.
    pub radius: u16,

    // Extended flat fields
    /// Z-depth sorting order.
    pub depth: i64,
    /// Font identifier.
    pub font: u16,
    /// Parent element name (for nesting).
    pub parent: Option<String>,
    /// List repeat direction.
    pub repeat: RepeatDir,
    /// Spacing between repeated items in MilliUnits.
    pub spacing: i64,

    // Sound events
    /// Sound to play when element is shown.
    pub sound_show: Option<String>,
    /// Sound to play when element is dismissed.
    pub sound_dismiss: Option<String>,
    /// Sound to play on hover.
    pub sound_hover: Option<String>,
    /// Sound to play on selection.
    pub sound_select: Option<String>,

    // Text content
    /// Text content (optional).
    pub text: Option<String>,
    /// Text colour override (optional).
    pub text_color: Option<u32>,
    /// Font size.
    pub font_size: u16,

    // Voxel text (3D font — degrades to flat text in CPU preview)
    /// Voxel text content (3D font).
    pub voxel_text: Option<String>,
    /// Voxel text material name.
    pub voxel_material: Option<String>,

    // Nested sub-blocks
    /// Spring physics for entry animation.
    pub spring_in: Option<SpringDef>,
    /// Spring physics for hover animation.
    pub spring_hover: Option<SpringDef>,
    /// Particle effect (normal state).
    pub particle: Option<ParticleDef>,
    /// Particle effect (selected state).
    pub particle_selected: Option<ParticleDef>,
    /// Progress bar fill definition.
    pub fill: Option<FillDef>,
    /// Conditional state-driven override rules.
    pub rules: Vec<ConditionalRule>,
}

impl UiDef {
    /// Get the name as a string slice.
    pub fn name_str(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

impl Default for UiDef {
    fn default() -> Self {
        Self {
            id: 0,
            name: [0u8; 32],
            name_len: 0,
            x: 0, y: 0, w: 0, h: 0,
            color: ColorValue::Literal(0xFFFFFFFF),
            color_selected: None,
            material_idx: 0,
            material_name: None,
            vibe_mask: 0,
            radius: 0,
            depth: 0,
            font: 0,
            parent: None,
            repeat: RepeatDir::None,
            spacing: 0,
            sound_show: None,
            sound_dismiss: None,
            sound_hover: None,
            sound_select: None,
            text: None,
            text_color: None,
            font_size: 16,
            voxel_text: None,
            voxel_material: None,
            spring_in: None,
            spring_hover: None,
            particle: None,
            particle_selected: None,
            fill: None,
            rules: Vec::new(),
        }
    }
}

// Error & Gate Types
// ---------------------------------------------------------------------------

/// Parse error with source location for diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// File path where the error occurred.
    pub file: String,
    /// Line number where the error was detected.
    pub line: usize,
    /// Error message text.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Security gate decision for Vixel AST validation.
///
/// Mirrors the shell_gate pattern: `Allow` passes, `Deny` carries a
/// human-readable reason string.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Validation passed.
    Allow,
    /// Validation failed with reason.
    Deny { /// Reason for denial.
reason: String },
}

// ---------------------------------------------------------------------------
// Optimizer & Extractor Types
// ---------------------------------------------------------------------------

/// Report produced by `ast_optimizer::optimize()`.
///
/// Counts of dead nodes removed during pruning.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PruneReport {
    /// Number of materials removed.
    pub materials_pruned: usize,
    /// Number of sockets removed.
    pub sockets_pruned: usize,
    /// Number of automata rules removed.
    pub automata_pruned: usize,
}

/// Three-branch extraction produced by `ast_extractor::extract()`.
///
/// Routes parsed definitions to their respective compilers.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedBranches {
    /// Material definitions for forge-furnace.
    pub materials: Vec<MaterialDef>,
    /// Spatial definitions for socket graph routing.
    pub spatials: Vec<SpatialDef>,
    /// Automata definitions for shader compilation.
    pub automata: Vec<AutomataDef>,
}

// ---------------------------------------------------------------------------
// Physics QA Types
// ---------------------------------------------------------------------------

/// A physics constraint violation detected at build time.
///
/// Produced by `physics_qa::physics_qa_gate()` when a material or
/// automata rule violates physical plausibility constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsViolation {
    /// Material identifier with the violation.
    pub material_id: u16,
    /// Constraint rule that was violated.
    pub rule: String,
    /// Human-readable violation description.
    pub message: String,
}

impl std::fmt::Display for PhysicsViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PhysicsViolation(mat_id={}, rule={}): {}",
            self.material_id, self.rule, self.message
        )
    }
}

impl std::error::Error for PhysicsViolation {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ast_has_no_entries() {
        let ast = VixelAst::new();
        assert!(ast.materials.is_empty());
        assert!(ast.spatials.is_empty());
        assert!(ast.automata.is_empty());
        assert!(ast.environment.is_empty());
    }

    #[test]
    fn material_name_roundtrip() {
        let mut mat = MaterialDef::default();
        let name = b"oak";
        mat.name[..name.len()].copy_from_slice(name);
        mat.name_len = name.len();
        assert_eq!(mat.name_str(), "oak");
    }

    #[test]
    fn environment_target_roundtrip() {
        let mut env = EnvironmentDef::default();
        let target = b"fire";
        env.target[..target.len()].copy_from_slice(target);
        env.target_len = target.len();
        assert_eq!(env.target_str(), "fire");
    }

    #[test]
    fn parse_error_display() {
        let err = ParseError {
            file: "world.vixel".into(),
            line: 42,
            message: "unexpected token".into(),
        };
        assert_eq!(err.to_string(), "world.vixel:42: unexpected token");
    }

    #[test]
    fn gate_decision_variants() {
        let allow = GateDecision::Allow;
        let deny = GateDecision::Deny {
            reason: "float literal detected".into(),
        };
        assert_eq!(allow, GateDecision::Allow);
        assert!(matches!(deny, GateDecision::Deny { .. }));
    }

    #[test]
    fn prune_report_default_is_zero() {
        let report = PruneReport::default();
        assert_eq!(report.materials_pruned, 0);
        assert_eq!(report.sockets_pruned, 0);
        assert_eq!(report.automata_pruned, 0);
    }

    #[test]
    fn physics_violation_display() {
        let v = PhysicsViolation {
            material_id: 7,
            rule: "burning_metal".into(),
            message: "flammability > 0 for metallic > 8000".into(),
        };
        assert!(v.to_string().contains("mat_id=7"));
        assert!(v.to_string().contains("burning_metal"));
    }

    #[test]
    fn automata_type_equality() {
        assert_eq!(AutomataType::Fire, AutomataType::Fire);
        assert_ne!(AutomataType::Fire, AutomataType::Fluid);
        assert_ne!(AutomataType::Gravity, AutomataType::Custom);
    }

    #[test]
    fn material_default_destruction_is_shatter() {
        let mat = MaterialDef::default();
        assert_eq!(mat.destruction_mode, 0); // 0 = Shatter
    }

    #[test]
    fn spatial_default_has_no_sockets() {
        let spatial = SpatialDef::default();
        assert_eq!(spatial.socket_count, 0);
        assert_eq!(spatial.sockets, [(0, 0, 0); 6]);
    }

    // -- parse_vixel_dir tests -----------------------------------------------

    use std::io::Write;

    /// Helper: create a temp dir with the given .vixel files.
    /// Returns the temp dir (auto-cleaned on drop).
    fn temp_vixel_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        for (name, content) in files {
            let path = dir.path().join(name);
            let mut f = std::fs::File::create(&path).expect("create file");
            f.write_all(content.as_bytes()).expect("write file");
        }
        dir
    }

    #[test]
    fn parse_vixel_dir_empty_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let ast = super::parse_vixel_dir(dir.path()).unwrap();
        assert!(ast.materials.is_empty());
        assert!(ast.spatials.is_empty());
        assert!(ast.automata.is_empty());
        assert!(ast.environment.is_empty());
    }

    #[test]
    fn parse_vixel_dir_single_file() {
        let dir = temp_vixel_dir(&[(
            "world.vixel",
            r#"
material "stone" {
    mass: 5000
    hardness: 7000
    flammability: 0
    roughness: 6000
    metallic: 0
    albedo: 0x808080FF
    destruction: "shatter"
}
"#,
        )]);
        let ast = super::parse_vixel_dir(dir.path()).unwrap();
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.materials[0].id, 0);
        assert_eq!(ast.materials[0].name_str(), "stone");
    }

    #[test]
    fn parse_vixel_dir_multiple_files_unique_ids() {
        let dir = temp_vixel_dir(&[
            (
                "a_materials.vixel",
                r#"
material "stone" {
    mass: 5000
    hardness: 7000
    flammability: 0
    roughness: 6000
    metallic: 0
    albedo: 0x808080FF
    destruction: "shatter"
}
material "wood" {
    mass: 3000
    hardness: 2000
    flammability: 8000
    roughness: 4000
    metallic: 0
    albedo: 0x8B4513FF
    destruction: "splinter"
}
"#,
            ),
            (
                "b_materials.vixel",
                r#"
material "iron" {
    mass: 8000
    hardness: 9000
    flammability: 0
    roughness: 3000
    metallic: 9500
    albedo: 0xC0C0C0FF
    destruction: "melt"
}
"#,
            ),
        ]);
        let ast = super::parse_vixel_dir(dir.path()).unwrap();
        assert_eq!(ast.materials.len(), 3);
        // a_materials.vixel parsed first (alphabetical), IDs 0 and 1
        assert_eq!(ast.materials[0].id, 0);
        assert_eq!(ast.materials[0].name_str(), "stone");
        assert_eq!(ast.materials[1].id, 1);
        assert_eq!(ast.materials[1].name_str(), "wood");
        // b_materials.vixel parsed second, ID offset by 2
        assert_eq!(ast.materials[2].id, 2);
        assert_eq!(ast.materials[2].name_str(), "iron");
    }

    #[test]
    fn parse_vixel_dir_ignores_non_vixel_files() {
        let dir = temp_vixel_dir(&[
            (
                "world.vixel",
                r#"
material "stone" {
    mass: 5000
    hardness: 7000
    flammability: 0
    roughness: 6000
    metallic: 0
    albedo: 0x808080FF
    destruction: "shatter"
}
"#,
            ),
            ("readme.txt", "This is not a vixel file"),
            ("notes.md", "# Notes"),
        ]);
        let ast = super::parse_vixel_dir(dir.path()).unwrap();
        assert_eq!(ast.materials.len(), 1);
    }

    #[test]
    fn parse_vixel_dir_parse_error_includes_filename() {
        let dir = temp_vixel_dir(&[("bad.vixel", "garbage_keyword")]);
        let err = super::parse_vixel_dir(dir.path()).unwrap_err();
        assert!(err.file.contains("bad.vixel"), "error should name the file: {}", err);
    }

    #[test]
    fn parse_vixel_dir_nonexistent_directory() {
        let result = super::parse_vixel_dir(std::path::Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("cannot read directory"));
    }

    #[test]
    fn parse_vixel_dir_deterministic_order() {
        // Files named to test alphabetical sorting: z before a in name,
        // but a should be parsed first.
        let dir = temp_vixel_dir(&[
            (
                "z_last.vixel",
                r#"
material "zinc" {
    mass: 7000
    hardness: 5000
    flammability: 0
    roughness: 4000
    metallic: 9000
    albedo: 0xD4D4D4FF
    destruction: "melt"
}
"#,
            ),
            (
                "a_first.vixel",
                r#"
material "amber" {
    mass: 2000
    hardness: 3000
    flammability: 5000
    roughness: 5000
    metallic: 0
    albedo: 0xFFBF00FF
    destruction: "shatter"
}
"#,
            ),
        ]);
        let ast = super::parse_vixel_dir(dir.path()).unwrap();
        assert_eq!(ast.materials.len(), 2);
        // a_first.vixel parsed first
        assert_eq!(ast.materials[0].name_str(), "amber");
        assert_eq!(ast.materials[0].id, 0);
        // z_last.vixel parsed second
        assert_eq!(ast.materials[1].name_str(), "zinc");
        assert_eq!(ast.materials[1].id, 1);
    }
}
