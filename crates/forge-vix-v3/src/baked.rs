//! baked.rs — prebaked (AOT-resolved) artist attributes for `.kit.vixi` slots.
//!
//! The five artist-facing keyword families from the UI-as-physical-voxel spec,
//! resolved at PARSE time (runtime_parse is forbidden — this IS the bake):
//!
//! | Keyword(s)                          | Bakes to                          |
//! |-------------------------------------|-----------------------------------|
//! | `material=` `mass=` `friction=`     | physical [`MaterialAtom`] + overrides |
//! | `vibe_scale=` `vibe_glow=` ...      | [`VibeBind`] → VibeMatrix channel |
//! | `screen_edge=` `blend=`             | chromatic reflection (ambilight)  |
//! | `motion=` `attractor=`              | [`MotionProfile`] spring          |
//! | `on_click=edict:<id>`               | world-altering edict trigger      |
//!
//! Carried on `KitDoc.baked` keyed by slot `stable_key`; consumers (render
//! dispatch, physics-on-close, vibe binder, edict router) join by key. No
//! per-frame parsing — every value here is resolved once, at load.

use forge_canvas_v3::text::FontSize;
use forge_correspondence_v3::correspondence::Material;
use forge_correspondence_v3::material_binding::MaterialAtom;

/// Which UI property an audio (VibeMatrix) channel drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VibeTarget {
    /// Drives the element's scale/radius.
    ScaleRadius,
    /// Drives an emissive glow intensity.
    EmissiveGlow,
    /// Drives alpha/opacity.
    Opacity,
    /// Drives a vertical offset.
    OffsetY,
}

/// One audio-reactive binding: a VibeMatrix channel (0..=15) → a UI property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VibeBind {
    /// VibeMatrix channel index, 0..=15.
    pub channel: u8,
    /// The UI property this channel drives.
    pub target: VibeTarget,
}

/// Screen edge to inherit colour from (Ambilight chromatic reflection).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenEdge {
    /// Left edge.
    Left,
    /// Right edge.
    Right,
    /// Top edge.
    Top,
    /// Bottom edge.
    Bottom,
}

/// Blend mode for a chromatic-reflection / overlay surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard alpha-over compositing.
    Normal,
    /// Overlay blend (contrast-preserving multiply/screen).
    Overlay,
    /// Additive blend.
    Add,
}

/// Baked deterministic spring profile (resolved from a named preset at bake time).
/// Integer gains (Permyriad-style) — consumed by `forge-canvas::SignalSpring`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionProfile {
    /// Spring stiffness, Permyriad-style gain.
    pub stiffness: u16,
    /// Spring damping, Permyriad-style gain.
    pub damping: u16,
}

/// `safe_area=<name>` — SMPTE ST 2046 / BBC 90-80 margins; HUD anchors
/// resolve inside this safe rect (`None` = full frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeArea {
    /// Title-safe margin (tightest).
    Title,
    /// Action-safe margin.
    Action,
    /// No safe-area constraint — full frame.
    None,
}

/// `hud_class=<name>` — Fagerholt/Lorentzon taxonomy: where the element
/// lives in the fiction (`None` = undeclared).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudClass {
    /// Exists inside the fiction (a character/world could perceive it).
    Diegetic,
    /// Exists outside the fiction, for the player only.
    Meta,
    /// Fiction-anchored but not perceivable by characters (e.g. waypoint arrows).
    Spatial,
    /// Undeclared / not classified.
    Non,
}

/// All prebaked artist attributes on one slot. Every ref is resolved (AOT).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BakedAttrs {
    /// `material=<name>` → resolved physical atom (albedo + reflectiveness + Mohs + mass…).
    pub material: Option<MaterialAtom>,
    /// `mass=<permyriad>` explicit override (else the atom's `mass_pmy`).
    pub mass_pmy: Option<u16>,
    /// `friction=<permyriad>` explicit override (else the atom's `friction_pmy`).
    pub friction_pmy: Option<u16>,
    /// `vibe_*=<channel>` audio-reactive binds (one slot may bind several properties).
    pub vibe: Vec<VibeBind>,
    /// `screen_edge=<edge>` chromatic reflection source.
    pub screen_edge: Option<ScreenEdge>,
    /// `blend=<mode>` for the chromatic-reflection surface.
    pub blend: Option<BlendMode>,
    /// `motion=<preset>` spring profile.
    pub motion: Option<MotionProfile>,
    /// `attractor=pointer` — the spring chases the pointer.
    pub motion_attractor_pointer: bool,
    /// `on_click=edict:<id>` — the edict/surge id to trigger.
    pub on_click_edict: Option<String>,
    /// `on_key=<chord>:edict:<id>` — key-chord edict trigger, chord baked AOT to a
    /// u32 code (low 16 = Win32 VK · bit16 Ctrl · bit17 Shift · bit18 Alt).
    /// HAND-MIRROR of `forge_input::action_map` chord packing (no cargo edge).
    pub on_key_edict: Option<(u32, String)>,
    /// `ᐍ=<milliunit>` stroke z-depth (West-Cree WE U+140D; author-time, ADR-0006 D8).
    pub stroke_z_mu: Option<i32>,
    /// `source=<field>` — live host-data bind: the engine snapshot field this
    /// kind=text/bar slot renders (STUDIO-TRANSFER M3; explicit, no path-guessing).
    pub source: Option<String>,
    /// `bind=<path>` — the authored binding word, captured VERBATIM and
    /// deliberately un-interpreted.
    ///
    /// Distinct from [`Self::source`]: `source=` names a host snapshot FIELD a
    /// `kind=text` slot renders, while authored `bind=` is used two ways across the
    /// panel corpus — a paint token on a region (`bind=palette.bg_near`) and a value
    /// binding on a widget (`bind=brush.raycast_size`, `bind=budget.limit`). Folding
    /// the two would push palette tokens into the text-source map.
    ///
    /// Neither v2's parser nor v3's had a `bind` arm, so every authored `bind=` in
    /// the corpus was silently discarded (receipt 2026-08-24: v2 parse.rs handles
    /// `source`/`role` and no `bind`; v3's did the same). This field STOPS the
    /// discard. It assigns no meaning: what a bind path resolves to is the host's
    /// call, and inventing that resolution here would settle a dialect question
    /// that belongs to the language, not to a loader.
    pub bind: Option<String>,
    /// `role=<name>` semantic role (canvas/content_band/…) — retained 2026-07-20
    /// for `audit::audit_layout` area-per-role gauging (was accepted-by-ignore).
    pub role: Option<String>,
    /// `safe_area=title|action|none` — SMPTE ST 2046 / BBC 90-80 margins;
    /// HUD anchors resolve inside the safe rect.
    pub safe_area: Option<SafeArea>,
    /// `hud_class=diegetic|meta|spatial|non` — Fagerholt/Lorentzon taxonomy.
    pub hud_class: Option<HudClass>,
    /// `focus=true|false` — a slot the pad focus walk can land on.
    pub focusable: bool,
    /// `unit=<name>` — the measurement unit a scale/ruler slot counts in
    /// (`unit=ticks` on a tick_ruler). The ruler renders labels in this unit;
    /// without it a ruler cannot say what its divisions mean.
    pub unit: Option<String>,
    /// `alpha=permyriad(N)` — slot opacity on the permyriad lattice, 0..10000.
    /// Integer because `float_in_ir` is forbidden: an overlay authored at 8500
    /// is 85% opaque with no f32 anywhere in the IR.
    pub alpha_pmy: Option<u16>,
    /// `font=<family>` — type family token for a text slot (`font=mono` pins a
    /// monospace face so hashes and telemetry columns align).
    pub font: Option<String>,
    /// `semantic=<ramp>` — the meaning-carrying colour ramp a meter reads
    /// (`semantic=green_yellow_red`), distinct from `color=` chrome.
    pub semantic: Option<String>,
    /// `primary=true` — this slot is the surface's primary focus region. More
    /// than one may be primary (two decks are co-equal); the host orders them
    /// ahead of secondary regions.
    pub primary: bool,
    /// `fixed_position=true` — the slot holds its place when siblings reflow
    /// (a centre mixer stays centred).
    pub fixed_position: bool,
    /// `searchable=true` — the region owns a searchable inventory, so the host
    /// gives it a filter affordance.
    pub searchable: bool,
    /// `ramp=type.ramp[N]` — the authored `type.ramp` stop this text slot
    /// speaks at (0=Caption..4=Display), riding `KitDoc.baked` the same way
    /// `source=` does (was accepted-by-ignore, `DEBT-RAMP-ATTR-ACCEPTED-BY-IGNORE`;
    /// the leaf-name heuristic in `diagnostics::ramp_stop_for` still answers
    /// for slots that leave this unset).
    pub ramp: Option<FontSize>,
}

impl BakedAttrs {
    /// True when the slot carried at least one artist keyword (worth a `KitDoc.baked` entry).
    pub fn has_any(&self) -> bool {
        self.material.is_some()
            || self.mass_pmy.is_some()
            || self.friction_pmy.is_some()
            || !self.vibe.is_empty()
            || self.screen_edge.is_some()
            || self.blend.is_some()
            || self.motion.is_some()
            || self.motion_attractor_pointer
            || self.on_click_edict.is_some()
            || self.on_key_edict.is_some()
            || self.stroke_z_mu.is_some()
            || self.source.is_some()
            || self.bind.is_some()
            || self.role.is_some()
            || self.safe_area.is_some()
            || self.hud_class.is_some()
            || self.focusable
            || self.unit.is_some()
            || self.alpha_pmy.is_some()
            || self.font.is_some()
            || self.semantic.is_some()
            || self.primary
            || self.fixed_position
            || self.searchable
            || self.ramp.is_some()
    }

    /// Parse `alpha=permyriad(N)` / `alpha=N` into the 0..10000 lattice. Values
    /// past the ceiling clamp rather than wrap — an author asking for 12000 means
    /// opaque, not 1.2 alpha.
    pub fn parse_alpha_pmy(v: &str) -> Option<u16> {
        let inner = v.strip_prefix("permyriad(").and_then(|s| s.strip_suffix(')')).unwrap_or(v);
        inner.trim().parse::<u32>().ok().map(|n| n.min(10_000) as u16)
    }

    /// Effective mass: explicit override, else the material atom's mass.
    pub fn effective_mass_pmy(&self) -> Option<u16> {
        self.mass_pmy.or_else(|| self.material.map(|m| m.mass_pmy))
    }

    /// Effective friction: explicit override, else the material atom's friction.
    pub fn effective_friction_pmy(&self) -> Option<u16> {
        self.friction_pmy.or_else(|| self.material.map(|m| m.friction_pmy))
    }
}

/// One slot's baked attributes, keyed by its `stable_key`.
#[derive(Clone, Debug, PartialEq)]
pub struct BakedSlot {
    /// This slot's stable identity key.
    pub stable_key: String,
    /// This slot's resolved (baked) attributes.
    pub attrs: BakedAttrs,
}

/// `material=<name>` → resolved physical atom (CE 6 groups). Unknown name → None.
pub fn resolve_material(name: &str) -> Option<MaterialAtom> {
    Material::from_name(name).map(MaterialAtom::from_material)
}

/// `on_key=` chord string → u32 chord code, resolved at BAKE time (runtime_parse
/// is forbidden — the string never survives to the frame loop). Shape:
/// `[ctrl_][shift_][alt_]<key>` where key = a-z · 0-9 · f1-f12 · space · enter ·
/// esc · tab. Code layout mirrors `forge_input::action_map::pack_chord`.
pub fn parse_chord(s: &str) -> Option<u32> {
    const CTRL: u32 = 1 << 16;
    const SHIFT: u32 = 1 << 17;
    const ALT: u32 = 1 << 18;
    let mut mods = 0u32;
    let mut key = s;
    loop {
        key = match key.split_once('_') {
            Some(("ctrl", rest)) => { mods |= CTRL; rest }
            Some(("shift", rest)) => { mods |= SHIFT; rest }
            Some(("alt", rest)) => { mods |= ALT; rest }
            _ => break,
        };
    }
    let vk: u32 = match key {
        "space" => 0x20,
        "enter" => 0x0D,
        "esc" => 0x1B,
        "tab" => 0x09,
        k if k.len() == 1 => match k.as_bytes()[0] {
            c @ b'a'..=b'z' => (c - b'a') as u32 + 0x41,
            c @ b'0'..=b'9' => (c - b'0') as u32 + 0x30,
            _ => return None,
        },
        k => {
            let n: u32 = k.strip_prefix('f')?.parse().ok()?;
            if !(1..=12).contains(&n) { return None; }
            0x70 + n - 1
        }
    };
    Some(mods | vk)
}

/// `motion=<preset>` → spring (stiffness, damping). Unknown preset → None.
pub fn resolve_motion(name: &str) -> Option<MotionProfile> {
    let (stiffness, damping) = match name {
        "snap_fluid" => (5000, 7000),
        "snap_tight" => (9000, 8500),
        "snap_loose" => (2500, 4000),
        _ => return None,
    };
    Some(MotionProfile { stiffness, damping })
}

/// VibeMatrix channel `0..=15`.
pub fn parse_channel(s: &str) -> Option<u8> {
    s.parse::<u8>().ok().filter(|c| *c < 16)
}

/// `safe_area=<name>` → the SMPTE/BBC safe rect class. Unknown name → None.
pub fn parse_safe_area(s: &str) -> Option<SafeArea> {
    Some(match s {
        "title" => SafeArea::Title,
        "action" => SafeArea::Action,
        "none" => SafeArea::None,
        _ => return None,
    })
}

/// `hud_class=<name>` → the Fagerholt/Lorentzon class. Unknown name → None.
pub fn parse_hud_class(s: &str) -> Option<HudClass> {
    Some(match s {
        "diegetic" => HudClass::Diegetic,
        "meta" => HudClass::Meta,
        "spatial" => HudClass::Spatial,
        "non" => HudClass::Non,
        _ => return None,
    })
}

/// `screen_edge=<name>` → the Ambilight source edge. Unknown name → None.
pub fn parse_edge(s: &str) -> Option<ScreenEdge> {
    Some(match s {
        "left" => ScreenEdge::Left,
        "right" => ScreenEdge::Right,
        "top" => ScreenEdge::Top,
        "bottom" => ScreenEdge::Bottom,
        _ => return None,
    })
}

/// `blend=<name>` → the overlay blend mode. Unknown name → None.
pub fn parse_blend(s: &str) -> Option<BlendMode> {
    Some(match s {
        "normal" => BlendMode::Normal,
        "overlay" => BlendMode::Overlay,
        "add" => BlendMode::Add,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_name_bakes_to_physical_atom() {
        let atom = resolve_material("iron").expect("iron resolves");
        assert_eq!(atom.material, Material::Iron);
        assert!(atom.metallic_pmy > 5000);
        assert_eq!(resolve_material("obsidian"), None, "unknown name → None");
    }

    #[test]
    fn motion_preset_resolves_and_channel_bounds() {
        assert_eq!(resolve_motion("snap_fluid"), Some(MotionProfile { stiffness: 5000, damping: 7000 }));
        assert_eq!(resolve_motion("nope"), None);
        assert_eq!(parse_channel("15"), Some(15));
        assert_eq!(parse_channel("16"), None, "only 16 VibeMatrix channels");
    }

    #[test]
    fn effective_mass_prefers_override_then_atom() {
        let mut a = BakedAttrs { material: resolve_material("iron"), ..Default::default() };
        let atom_mass = a.material.unwrap().mass_pmy;
        assert_eq!(a.effective_mass_pmy(), Some(atom_mass));
        a.mass_pmy = Some(8500);
        assert_eq!(a.effective_mass_pmy(), Some(8500), "explicit override wins");
    }
}
