# 13forge Font Assets

Workspace-wide font directory. All crates reference fonts here via
`include_bytes!` relative paths. No crate should bundle its own copy.

## Bundled Fonts

| File | Family | Use | License |
|------|--------|-----|---------|
| `IBMPlexMono-Regular.ttf` | IBM Plex Mono | Code, TUI grid, monospace UI | SIL OFL 1.1 |
| `IosevkaFixed-Regular.ttf` | Iosevka Fixed | egui monospace, terminal panels | SIL OFL 1.1 |
| `CommitMono-400-Regular.ttf` | Commit Mono | Alt monospace, diff views | SIL OFL 1.1 |

## Needed (TODO)

| Role | Candidates | Notes |
|------|-----------|-------|
| UI sans-serif | Inter, Nunito, Source Sans 3 | Panel labels, buttons, menus |
| Title / display | Press Start 2P, Silkscreen, Orbitron | Title screens, splash, branding |
| Game dialogue | Merriweather, Lora, Crimson Text | In-game dialogue boxes, lore text |
| Game HUD | VT323, Share Tech Mono, Fira Code | Health bars, ammo counters, debug overlay |
| Specs / docs | Noto Serif, Literata | Spec documents, printed output |

## include_bytes! Convention

From any crate's `src/` directory, use relative paths to this directory:

```rust
// From forge-gui/src/tui/atlas.rs (3 levels up):
include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf")

// From dreadpiratedev/src/app.rs (2 levels up):
include_bytes!("../../assets/fonts/IosevkaFixed-Regular.ttf")

// From forge-render/src/some_module.rs (2 levels up):
include_bytes!("../../assets/fonts/Inter-Regular.ttf")
```
