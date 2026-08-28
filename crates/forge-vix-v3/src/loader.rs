//! `.kit.vixi` source → renderable panel — the load path a host calls.
//!
//! Ported 2026-08-24 from `F:\NewRepo\crates\forge-vix\src\loader.rs` (90,232 B).
//! Only the MECHANISM crossed: that file is ~89 KB of baked v2 panel source
//! constants (`STUDIO_PANELS`, `AUTHORING_TEMPLATES`, `AUTHORING_VIBES`,
//! `HUD_KIT_SRC`) wrapped around ~150 lines of loader. v3 authors its own
//! surfaces, so the constants stayed behind.
//!
//! NOT ported: `load_kit_live` / `load_kit_variant_live`. Both call the donor's
//! `crate::live::ctx_for(profile)` to pick a token context from the kit's own
//! `profile:` header, and v3 has no `live` module — `layout.rs:35` names the seam
//! but nothing implements it. The profile is parsed and reachable
//! (`LoadedKit::profile`); choosing a `TokenCtx` from it is the open work, and
//! guessing that mapping here would invent a design v2 already settled elsewhere.
//!
//! NAMED `LoadedKit`, not the donor's name for it: `forge-core-v3`'s
//! `organs/dungeon_master_kit.rs:67` already declares a `#[doc(hidden)]` struct of
//! that donor name holding a single `ui` field — a STUB standing in for this very
//! type, alongside stub `IrRect`/`LoweredUi` siblings. Two live homes for one name
//! is an L05 defect (the one-home hook blocked the first write on exactly this), so
//! the real thing takes a distinct name. That stub is now superseded and wants
//! deleting, but it lives in another crate and removing it is its own weld with its
//! own gate — flagged here, not done silently.

use crate::ir::{IrRect, LoweredUi};
use crate::layout::{lower, TokenCtx, WidgetSpec};
use crate::parse::{parse_kit, parse_kit_variant, KitDoc, ParseError};
use crate::tokens::Palette;

/// A parsed + lowered panel: the doc (for gate/variant/profile inspection) and
/// the renderable UI.
#[derive(Clone, Debug)]
pub struct LoadedKit {
    /// The parsed kit — gates, baked attrs, profile header, spec tree.
    pub doc: KitDoc,
    /// The lowered, renderable UI.
    pub ui: LoweredUi,
}

impl LoadedKit {
    /// The kit's OWN word for a slot (`text="…"`).
    ///
    /// Static labels live in the `.kit.vixi` and a host only PLACES them: a host
    /// keeping its own const label table has two homes for one string, and the
    /// kit's copy silently stops being the truth. `None` = the slot is a live
    /// value the host fills, or the key is unknown — never a panic.
    pub fn text(&self, key: &str) -> Option<&str> {
        fn walk<'a>(spec: &'a WidgetSpec, key: &str) -> Option<&'a str> {
            if spec.stable_key == key {
                return spec.text.as_deref();
            }
            spec.children.iter().find_map(|c| walk(c, key))
        }
        walk(&self.doc.root, key)
    }

    /// The `profile:` header the kit asked for, if it declared one. v3 cannot yet
    /// act on it (no `live::ctx_for`); a host that knows its own profile table can.
    pub fn profile(&self) -> Option<&str> {
        self.doc.profile.as_deref()
    }

    /// The authored `bind=` path on a slot, by stable key. `None` = the slot
    /// declared no bind, or the key is unknown — never a panic.
    ///
    /// The loader assigns this string no meaning: it hands back the word the kit
    /// authored so the HOST can decide what `brush.raycast_size` or
    /// `palette.bg_near` resolves to. See [`crate::baked::BakedAttrs::bind`].
    pub fn bind(&self, key: &str) -> Option<&str> {
        self.doc
            .baked
            .iter()
            .find(|b| b.stable_key == key)
            .and_then(|b| b.attrs.bind.as_deref())
    }

    /// Every authored `(stable_key, bind path)` pair in the kit, in slot order.
    /// A host walks this once at load to discover what the surface asked to be
    /// wired to, instead of hard-coding slot names.
    pub fn binds(&self) -> Vec<(&str, &str)> {
        self.doc
            .baked
            .iter()
            .filter_map(|b| b.attrs.bind.as_deref().map(|v| (b.stable_key.as_str(), v)))
            .collect()
    }
}

/// Carry the kit's authored binds onto the lowered UI, so a renderer holding only
/// `LoweredUi` still honours what the kit declared.
fn attach_source_binds(doc: &KitDoc, ui: &mut LoweredUi) {
    ui.source_binds = doc
        .baked
        .iter()
        .filter_map(|b| b.attrs.source.clone().map(|s| (b.stable_key.clone(), s)))
        .collect();
    ui.ramp_binds = doc
        .baked
        .iter()
        .filter_map(|b| b.attrs.ramp.map(|r| (b.stable_key.clone(), r)))
        .collect();
    // The live DRIVE. A slot may author more than one (`vibe_glow=` AND
    // `vibe_scale=`), so this flattens rather than picking one.
    ui.vibe_binds = doc
        .baked
        .iter()
        .flat_map(|b| b.attrs.vibe.iter().map(|v| (b.stable_key.clone(), *v)))
        .collect();
    // Authored `text="…"` rides the SPEC TREE, not `baked`, so it is walked
    // rather than filtered — same contract: a renderer holding only the lowered
    // UI still shows the words the kit authored.
    fn walk(spec: &WidgetSpec, out: &mut Vec<(String, String)>) {
        if let Some(t) = &spec.text {
            out.push((spec.stable_key.clone(), t.clone()));
        }
        for c in &spec.children {
            walk(c, out);
        }
    }
    let mut lits = Vec::new();
    walk(&doc.root, &mut lits);
    ui.text_literals = lits;
}

/// Parse + lower a `.kit.vixi` source into a renderable panel.
pub fn load_kit(
    src: &str,
    ctx: &TokenCtx,
    viewport: IrRect,
    version: u32,
) -> Result<LoadedKit, ParseError> {
    let doc = parse_kit(src)?;
    let mut ui = lower(&doc.root, viewport, ctx, version);
    attach_source_binds(&doc, &mut ui);
    Ok(LoadedKit { doc, ui })
}

/// [`load_kit`] on the `comfy` token floor — the profile-less convenience.
///
/// This is NOT the donor's `load_kit_live`: it does not read the kit's `profile:`
/// header, because v3 has no profile→`TokenCtx` table yet. A kit that declared a
/// profile still loads; it just wears the floor until that seam lands.
pub fn load_kit_comfy(src: &str, viewport: IrRect, version: u32) -> Result<LoadedKit, ParseError> {
    load_kit(src, &TokenCtx::comfy(), viewport, version)
}

/// Parse + lower a kit ON THE PROFILE IT ASKED FOR — the seam this module's own
/// doc called "the open work" until 2026-08-26.
///
/// The kit's `profile:` header picks the sheet ([`crate::tokens::profile_by_name`],
/// ported from v2's `tokens.rs:601-630`); an absent or unknown name falls to the
/// comfy floor, exactly as v2's `live::ctx_for` (`live.rs:75-83`) cascades. The
/// returned [`Palette`] is the one the kit ASKED for, so a caller emitting themed
/// HTML no longer has to guess which sheet a surface wanted.
///
/// This is the whole reason `profile: molten` was authorable but inert.
pub fn load_kit_profiled(
    src: &str,
    viewport: IrRect,
    version: u32,
) -> Result<(LoadedKit, Palette), ParseError> {
    let doc = parse_kit(src)?;
    let base = doc
        .profile
        .as_deref()
        .and_then(crate::tokens::profile_by_name)
        .unwrap_or_else(crate::tokens::BaseProfile::studio_dark);
    let tokens = base.to_tokens();
    let ctx = tokens.to_token_ctx();
    let mut ui = lower(&doc.root, viewport, &ctx, version);
    attach_source_binds(&doc, &mut ui);
    Ok((LoadedKit { doc, ui }, tokens.palette))
}

/// Parse + lower with an active variant (`template_grammar` variant blocks).
pub fn load_kit_variant(
    src: &str,
    active_variant: &str,
    ctx: &TokenCtx,
    viewport: IrRect,
    version: u32,
) -> Result<LoadedKit, ParseError> {
    let doc = parse_kit_variant(src, active_variant)?;
    let mut ui = lower(&doc.root, viewport, ctx, version);
    attach_source_binds(&doc, &mut ui);
    Ok(LoadedKit { doc, ui })
}

/// Which edict a press landed on: the first baked `on_click=edict:<id>` slot whose
/// LOWERED box contains `(px, py)` in MilliUnits.
///
/// A pure geometry join — `doc.baked` × `ui.layout` by `stable_key`. The HOST
/// decides what an edict DOES; this only answers which one was hit. Press-frame
/// only (cold path).
pub fn edict_at<'a>(panel: &'a LoadedKit, px: i64, py: i64) -> Option<&'a str> {
    panel.doc.baked.iter().find_map(|b| {
        let id = b.attrs.on_click_edict.as_deref()?;
        let hit = panel
            .ui
            .layout
            .iter()
            .any(|bx| bx.stable_key.0 == b.stable_key && bx.rect.contains(px, py));
        hit.then_some(id)
    })
}

/// The key twin of [`edict_at`]: the first baked `on_key=<chord>:edict:<id>` whose
/// chord matches. Geometry-free — keys don't aim. Press-frame only.
pub fn edict_key_at(panel: &LoadedKit, chord: u32) -> Option<&str> {
    panel.doc.baked.iter().find_map(|b| {
        let (c, id) = b.attrs.on_key_edict.as_ref()?;
        (*c == chord).then_some(id.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: IrRect = IrRect { min_x: 0, min_y: 0, max_x: 640_000, max_y: 480_000 };

    /// The panel this whole lane exists for: shell/panels/raycast_brush_panel.kit.vixi,
    /// ported from v2 this same session. Its dial is the one main.rs:1049 names.
    const BRUSH_PANEL: &str = "#vixi:kit v2\n\
        surface: raycast_brush_panel\n\
        profile: studio_3d\n\
        slot root kind=region layout=stack_h padding=mu(8) gap=mu(8) material=bone align=center\n\
        slot root.title kind=text ramp=type.ramp[0] color=palette.fg_text\n\
        slot root.size kind=widget name=dial size=mu(44) material=bone bind=brush.raycast_size\n";

    /// The seam this module's doc called "the open work". Two kits identical
    /// but for their `profile:` line must NOT wear the same palette — that
    /// sameness was the defect.
    #[test]
    fn the_profile_header_actually_picks_the_sheet() {
        let kit = |p: &str| {
            format!("#vixi:kit v1\nprofile: {p}\nslot root kind=region layout=stack_v\n")
        };
        let (_, molten) = load_kit_profiled(&kit("molten"), VP, 1).expect("molten loads");
        let (_, frost) = load_kit_profiled(&kit("permafrost"), VP, 1).expect("permafrost loads");
        assert_ne!(molten.bg_far, frost.bg_far, "two profiles wore one ground");
        assert_eq!(molten.bg_far, crate::tokens::BaseProfile::molten().palette.bg_far);
        assert_eq!(frost.bg_far, crate::tokens::BaseProfile::permafrost().palette.bg_far);
    }

    /// An unknown or absent profile falls to the floor rather than inventing a
    /// theme — v2's `ctx_for` cascade (`live.rs:75-83`), same shape.
    #[test]
    fn an_unknown_profile_falls_to_the_floor_it_never_guesses() {
        let floor = crate::tokens::BaseProfile::studio_dark().palette;
        for src in [
            "#vixi:kit v1\nprofile: no_such_sheet\nslot root kind=region layout=stack_v\n",
            "#vixi:kit v1\nslot root kind=region layout=stack_v\n",
        ] {
            let (_, p) = load_kit_profiled(src, VP, 1).expect("still loads");
            assert_eq!(p.bg_far, floor.bg_far);
        }
    }

    /// The launcher authors `profile: molten`; it must actually get molten.
    #[test]
    fn the_authored_launcher_wears_the_sheet_it_asked_for() {
        const LAUNCHER: &str = include_str!("../../../shell/panels/launcher.kit.vixi");
        let (kit, palette) = load_kit_profiled(LAUNCHER, VP, 1).expect("the front door loads");
        assert_eq!(kit.profile(), Some("molten"));
        assert_eq!(palette.bg_far, crate::tokens::BaseProfile::molten().palette.bg_far);
    }

    #[test]
    fn the_brush_panel_loads_and_lowers() {
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).expect("the authored panel parses");
        assert!(!p.ui.layout.is_empty(), "it lowered to real boxes");
        assert_eq!(p.profile(), Some("studio_3d"), "the profile header survives the load");
    }

    #[test]
    fn a_kit_that_declares_no_profile_still_loads() {
        let src = "#vixi:kit v1\nslot root kind=region layout=stack_v\n";
        let p = load_kit_comfy(src, VP, 1).expect("parses");
        assert_eq!(p.profile(), None);
        assert!(!p.ui.layout.is_empty());
    }

    #[test]
    fn a_refusal_is_returned_not_panicked() {
        let err = load_kit_comfy("#vixi:kit v1\nslot root layout=stack_v\n", VP, 1)
            .expect_err("a slot with no kind= must refuse");
        assert_eq!(err.line, 2, "the refusal points at the offending source line");
        assert!(err.message.contains("kind"), "and says what was missing: {}", err.message);
    }

    #[test]
    fn an_explicit_ctx_and_the_comfy_floor_agree_when_the_ctx_is_comfy() {
        let a = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        let b = load_kit(BRUSH_PANEL, &TokenCtx::comfy(), VP, 1).unwrap();
        assert_eq!(a.ui.layout.len(), b.ui.layout.len());
        assert_eq!(a.ui.source_binds, b.ui.source_binds);
    }

    #[test]
    fn the_dial_reports_the_bind_the_kit_authored() {
        // This is the whole point of the lane: main.rs:1049 names
        // `bind=brush.raycast_size` as the contract, and until 2026-08-24 BOTH
        // parsers dropped it on the floor.
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        assert_eq!(p.bind("root.size"), Some("brush.raycast_size"));
        assert_eq!(p.binds(), vec![("root.size", "brush.raycast_size")]);
    }

    #[test]
    fn a_slot_with_no_bind_reports_none() {
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        assert_eq!(p.bind("root.title"), None, "the title binds to nothing");
        assert_eq!(p.bind("root.nope"), None, "an unknown key is None, not a panic");
    }

    #[test]
    fn a_region_bind_is_captured_verbatim_and_not_confused_with_source() {
        // Authored `bind=` is used two ways in the corpus — a paint token on a
        // region and a value path on a widget. Both ride through untouched, and
        // NEITHER lands in source_binds (which is `source=`'s home).
        let src = "#vixi:kit v1\n\
            slot root kind=region layout=stack_v bind=palette.bg_near\n\
            slot root.label kind=text source=place\n";
        let p = load_kit_comfy(src, VP, 1).expect("parses");
        assert_eq!(p.bind("root"), Some("palette.bg_near"));
        assert_eq!(p.bind("root.label"), None, "source= is not a bind");
        assert!(
            p.ui.source_binds.iter().any(|(k, v)| k == "root.label" && v == "place"),
            "source= still lands in source_binds"
        );
        assert!(
            !p.ui.source_binds.iter().any(|(k, _)| k == "root"),
            "a bind= must NOT leak into the text-source map"
        );
    }

    #[test]
    fn an_unknown_text_key_is_none_never_a_panic() {
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        assert_eq!(p.text("root.nope"), None);
        assert_eq!(p.text(""), None);
    }

    #[test]
    fn a_press_outside_every_box_hits_no_edict() {
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        // Far outside the viewport: nothing can contain it.
        assert_eq!(edict_at(&p, -9_000_000, -9_000_000), None);
    }

    #[test]
    fn an_unbound_chord_hits_no_edict() {
        let p = load_kit_comfy(BRUSH_PANEL, VP, 1).unwrap();
        assert_eq!(edict_key_at(&p, 0xDEAD_BEEF), None);
    }

    #[test]
    fn a_click_edict_is_found_by_the_box_that_carries_it() {
        let src = "#vixi:kit v1\n\
            slot root kind=region layout=stack_v\n\
            slot root.go kind=widget name=button on_click=edict:fire size=mu(44)\n";
        let p = load_kit_comfy(src, VP, 1).expect("parses");
        let target = p
            .ui
            .layout
            .iter()
            .find(|bx| bx.stable_key.0 == "root.go")
            .expect("the button lowered to a box");
        let (px, py) = (target.rect.min_x + 1, target.rect.min_y + 1);
        assert_eq!(edict_at(&p, px, py), Some("fire"), "a press inside the button fires its edict");
        assert_eq!(edict_at(&p, target.rect.max_x + 1_000, py), None, "just outside does not");
    }
}
