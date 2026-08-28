//! Audio bus — shared mixer bus for panel system continuity.
//! Absorbed from standalone `forge-audio::bus` crate (2026-04-26).

pub mod audio_context;
// Intentional `bus/bus.rs` — preserves the canonical handle/error layout from
// the absorbed crate (re-exports flowed into existing call sites without a
// rename). `clippy::module_inception` is the documented pattern for this.
#[allow(clippy::module_inception)]
pub mod bus;
pub mod command;
pub mod command_tx;
pub mod dead_drop;
pub mod effect;
// Un-orphaned 2026-06-20: the crate→bus bridge + real cpal feeder. Now that the
// low-level engine (`crate::mixer` et al.) is ported, this compiles and the
// silent stub can be retired in favour of `real_feeder_loop`.
pub mod engine_adapter;
pub mod kit_bridge;
pub mod mixer;
pub mod panel_manager;
pub mod panel;
pub mod panels;
pub mod recorder;
pub mod sequencer;
pub mod snapshot;
pub mod track;
pub mod ump_codec;
pub mod uniforms;

#[cfg(feature = "vision")]
pub mod vision_map;

pub use command::MixerCommand;
pub use snapshot::{LiveMixerState, DeckSnapshot, DeckState, BeatGridInfo};
pub use track::{TrackInfo, AudioFormat, ValidationError};
pub use effect::{EffectType, ActiveEffect};
pub use bus::{AudioBusHandle, AudioBusError, stub_feeder_loop};
pub use mixer::MixerCommandHub;
pub use kit_bridge::resolve_kit_binding;
pub use uniforms::{AudioUniforms, build_audio_uniforms_from_ctx, fill_constellation_from_spectrum};
#[allow(deprecated)]
pub use uniforms::build_audio_uniforms;
pub use audio_context::{AudioContext, NowPlaying};
pub use panel::{Panel, PanelEntry, TickContext, RenderContext, HubTapeBar};
pub use panel_manager::PanelManager;
pub use panels::{DawPanel, HudPanel, StudioPanel, BroskiPanel, NdePanel, AdminPanel, create_unified_panels, run_frame};
pub use recorder::WavRecorder;
pub use ump_codec::{decode as decode_hub_ump, encode_ump, HubEvent, HubTag};
pub use bus::{HubTapeStat, HubTapeGlobal, HUB_TAPE};
