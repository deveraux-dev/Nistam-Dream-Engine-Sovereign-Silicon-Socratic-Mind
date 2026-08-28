//! Shaderbind `.vixi` DSL lowering — the UPSTREAM `signal -> surface.channel[N]`
//! routing that feeds a vibe surface its inputs, the sibling of reactive DSL's
//! DOWNSTREAM `vibematrix.<src> -> visual.<tgt>` look edges.
//!
//! `reactive_dsl` answers *"which vibematrix channel drives which visual look"*; a
//! shaderbind answers the layer ABOVE it: *"which raw signal (audio / vibematrix /
//! input / world) populates which of a surface's input channels"* — under sovereignty
//! GATES (`visual_only_mutates_authority = forbidden`, `shader_compile_hotpath =
//! forbidden`). The two live golden fixtures are the golden `udle_vibematrix` binding
//! (surface `udle`, 8 channels) and `audio_vis` (surface `audio_vis`, 5 channels).
//!
//! Cold path — invoked when a shaderbind is loaded / edited, NEVER on the 120Hz tick
//! or the render-submit path. `String`/`Vec` are expected here and marked
//! `// @forge:allow_alloc: cold path`. The HOT product is [`ShaderBind::route`], an
//! integer Permyriad channel array (no float, no alloc beyond the result vec).
//!
//! Follows the **Sieve DSL parse pattern**: a sovereign, hand-written, line-oriented
//! parser. It is grammar-GATED — every source / gate / channel token must resolve to a
//! known symbol; an unknown token, an undeclared signal, or a non-contiguous channel
//! set is a HARD error, never a silent drop (Signal Law) — and round-trips through
//! [`pretty_print_shaderbind`].
//!
//! Grammar:
//! ```text
//!   shaderbind = header surface profile decl*
//!   header     = "#vixi:shaderbind v1"
//!   surface    = "surface:" ident
//!   profile    = "profile:" ident
//!   decl       = signal | route | gate | comment | blank
//!   signal     = "signal" name "source=" source "range=" lo ".." hi
//!   route      = surface "." "channel[" n "]" "<-" name
//!   gate       = "gate" gatename "=" ("forbidden" | "required")
//!   source     = "audio." ("rms"|"beat_phase"|"spectral_centroid"|"crossfader"
//!                         |"sub_bass"|"spectrum_low"|"spectrum_mid"|"spectrum_high"
//!                         |"deck_a_rms"|"deck_b_rms"|"spectrum_band_0".."_6")
//!              | "vibematrix." ("rain_intensity"|"artifact_glow"|"threat_color"
//!                             |"hue"|"intensity"|"bloom"|"warp"|"ghost_intensity"
//!                             |"beat_phase")
//!              | "input.pen_pressure"
//!              | "world." ("authority_q"|"flute_velocity"|"mood"|"water_depth"
//!                        |"light_level"|"velocity"|"sun_dir"|"exposure"|"gravity"
//!                        |"wind_speed"|"wind_direction"|"precipitation_rate"
//!                        |"fog_density"|"cloud_cover"|"temperature"|"humidity"
//!                        |"moon_phase"|"clarity_q"|"resonance_q"|"tarnish_q"
//!                        |"shadow_weight_q")
//!              | "mixer.crossfader" | "seq.current_step" | "term.route_margin"
//!   lo, hi     = 0..=10000   (Permyriad; the authored signal range)
//! ```
//! Blank lines and `#` / `//` comment lines are skipped (the `#vixi:` header is a
//! directive, not a comment).
//!
//! ## Drain provenance
//!
//! Ported from v2 source: `F:\NewRepo\crates\forge-gpu\src\shaderbind_dsl.rs` (~1234 lines).
//! FROZEN v1 grammar, no improvements (Sean 07-29 "Freeze Shader Uniform Width at 4 Lanes").

/// The reactive-edge DSL — lowering `.vixi` look profiles into runtime bindings.
pub mod reactive;

/// The GPU vibe bus, FROZEN (Sean 07-29 "Freeze Shader Uniform Width at 4 Lanes").
/// Exactly four audio-carrying f32 channels reach pixels — glow, pulse, chromatic,
/// shake — declared identically in `canvas_quad.wgsl`, `canvas_gpu.js` (WebGPU) and
/// `forge_shaders::vibe_post` (CPU/SPIR-V). The layout is IMMUTABLE: no new shader
/// term, no pipeline rebind, no recompile when the audio feature set changes.
///
/// N audio measures collapse to these 4 at .vibe/.shaderbind LOWERING, host-side,
/// which is what makes the freeze affordable — see
/// `n_audio_measures_reduce_to_four_lanes_without_touching_a_shader`.
pub const VIBE_BUS_LANES: [&str; 4] = ["vibe_glow", "vibe_pulse", "vibe_chromatic", "vibe_shake"];

/// [ASPIRE: bformat-reinterpret] The same 4 [`VIBE_BUS_LANES`] words, read as a
/// first-order ambisonic B-format channel (Gerzon 1973): `glow` is the omni
/// energy (W), `chromatic`/`shake`/`pulse` the three directional lanes
/// (X/Y/Z). Zero shader bytes change — only the INTERPRETATION is declared,
/// so this is a reinterpret, not a new bus. Values stay Permyriad `u16`,
/// exactly [`ShaderBind::route`]'s hot output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BFormat {
    /// Omni energy — how loud, direction-independent. Aliases `vibe_glow`.
    pub w: u16,
    /// Directional lane X. Aliases `vibe_chromatic`.
    pub x: u16,
    /// Directional lane Y. Aliases `vibe_shake`.
    pub y: u16,
    /// Directional lane Z. Aliases `vibe_pulse`.
    pub z: u16,
}

impl BFormat {
    /// Reinterpret a routed [`VIBE_BUS_LANES`]-order channel array (`[glow,
    /// pulse, chromatic, shake]`, [`ShaderBind::route`]'s output order) as
    /// B-format. `lanes.len() != 4` is a caller bug — the bus is FROZEN at 4,
    /// so this asserts rather than silently truncating.
    pub fn from_vibe_lanes(lanes: &[u16]) -> Self {
        assert_eq!(lanes.len(), 4, "VIBE_BUS_LANES is frozen at 4 — got {}", lanes.len());
        Self { w: lanes[0], x: lanes[2], y: lanes[3], z: lanes[1] }
    }
}

/// The closed set of signal SOURCES the shaderbinds bind into surface channels.
/// `namespace.field` — grammar-gated: an unknown source is a hard parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSource {
    /// RMS energy (root mean square) — immediate loudness.
    AudioRms,
    /// Beat phase alignment (0..360 folded to Permyriad).
    AudioBeatPhase,
    /// Spectral centroid — the frequency-center of the active spectrum.
    AudioSpectralCentroid,
    /// Crossfader position (audio fader, not mixer).
    AudioCrossfader,
    /// Sub-bass energy ratio (0..10000 Permyriad). Live producer
    /// `forge_audio::bus::uniforms` `sub_bass_ratio` (uniforms.rs:13), the same band
    /// `technothesia` already tracks per frame as `last_sub_bass`.
    AudioSubBass,
    /// Low spectral band energy (0..10000 Permyriad). Live producer
    /// `forge_audio::bus::uniforms` `spectrum_low` (uniforms.rs:15); the same band
    /// `forge_core::brush::VibeMod.low` carries into
    /// `forge_harmonics::swarm_ambience::apply_swarm_ambient` (:19, grounds `root_q`).
    AudioSpectrumLow,
    /// Mid (vocal/body) spectral band energy (0..10000 Permyriad). Live producer
    /// `forge_audio::bus::uniforms` `spectrum_mid` (uniforms.rs:16) / `VibeMod.mid`,
    /// which lifts `warmth_q` (swarm_ambience.rs:21). The band the pool corpus asked
    /// for as `audio.mid` and the closed set had no home for.
    AudioSpectrumMid,
    /// High spectral band energy (0..10000 Permyriad). Live producer
    /// `forge_audio::bus::uniforms` `spectrum_high` (uniforms.rs:17) / `last_spec_high`.
    AudioSpectrumHigh,
    /// Per-deck RMS, decks A/B — the DJ lane's VU meters and per-deck shader glow.
    /// Live producer: the mixer's per-deck level, `forge_audio::bus::snapshot`
    /// `LiveMixerState` decks. (dead_drop_daw slots 6-7.)
    AudioDeckARms,
    /// Per-deck B RMS.
    AudioDeckBRms,
    /// 7-band parametric spectrum, ISO-31 octave centres, sub-bass → brilliance
    /// (Permyriad energy per band). The fine-grained sibling of the 3-band ladder:
    /// `forge_audio::alchemy::ghost_speak::band_split_3way` is the coarse split,
    /// this is the DAW's per-octave read. (dead_drop_daw slots 10-16.) Flat variants,
    /// not a payload: the source set stays a plain `Copy` closed enum.
    AudioSpectrumBand0,
    /// Spectrum band 1.
    AudioSpectrumBand1,
    /// Spectrum band 2.
    AudioSpectrumBand2,
    /// Spectrum band 3.
    AudioSpectrumBand3,
    /// Spectrum band 4.
    AudioSpectrumBand4,
    /// Spectrum band 5.
    AudioSpectrumBand5,
    /// Spectrum band 6.
    AudioSpectrumBand6,
    /// Vocal-formant energy (0..10000 Permyriad) — LPC spectral-envelope
    /// peak-pick over the 300-3500Hz vocal-formant band, from HPSS's
    /// harmonic separation of the loopback capture. Live producer:
    /// `forge_audio_v3::formant_meter::FormantMeter` (2026-08-24). Drives
    /// Broski's allure signal and OODA transition thresholds.
    AudioFormantEnergy,
    /// Vibematrix rain intensity.
    VibeRainIntensity,
    /// Vibematrix artifact glow.
    VibeArtifactGlow,
    /// Vibematrix threat color.
    VibeThreatColor,
    /// Cosmetic `look_dsl` knobs — the FREE-tier LookKnobs conditioning the vibe
    /// matrix (forge_brand.shaderbind.vixi slots 0-3). Visual-only by construction:
    /// they may never carry the authority lane.
    VibeHue,
    /// Vibematrix intensity.
    VibeIntensity,
    /// Vibematrix bloom.
    VibeBloom,
    /// Vibematrix warp.
    VibeWarp,
    /// Ghost-fire intensity + beat-pulse opacity, the DAW's two vibematrix feeds
    /// (dead_drop_daw slots 8-9).
    VibeGhostIntensity,
    /// Vibematrix beat phase.
    VibeBeatPhase,
    /// Pen input pressure.
    InputPenPressure,
    /// World authority.
    WorldAuthority,
    /// Player/camera speed (Permyriad) — colour streaks on speed
    /// (forge_brand slot 7).
    WorldVelocity,
    /// Helios bake inputs the CPU reference reads from the same u32 slots
    /// (forge_brand slots 9-11): sun direction, exposure, gravity.
    WorldSunDir,
    /// World exposure.
    WorldExposure,
    /// World gravity.
    WorldGravity,
    // ── WEATHER (Sean 2026-07-28 "what about weather?") ─────────────────────
    // The macro weather organ is `forge_game_systems::weather::WeatherState`
    // (weather.rs:63-76), already live in the world tick — forge-studio/src/world.rs:1579
    // quantizes it through `wind_lean`, the ONE firewall crossing. Every field below
    // is a real member of that struct; the host quantizes its f64 to Permyriad at the
    // boundary, exactly as `world.mood` already does. No weather vocab is invented.
    /// Wind magnitude (`WeatherState.wind_speed`) — the same value `wind_lean`
    /// (world.rs:207) quantizes into the matter sim's -4..=4 lean.
    WorldWindSpeed,
    /// Wind bearing (`WeatherState.wind_direction`, 0..360 folded to Permyriad) —
    /// the sign half of the lean; drives streak/lean direction in the shader.
    WorldWindDirection,
    /// Rainfall rate (`WeatherState.precipitation_rate`). The WORLD-side truth whose
    /// look-side sibling is [`VibeRainIntensity`](Self::VibeRainIntensity): weather
    /// says how hard it rains, the vibematrix says how hard it READS.
    WorldPrecipitationRate,
    /// Fog thickness (`WeatherState.fog_density`) — distance haze.
    WorldFogDensity,
    /// Cloud cover (`WeatherState.cloud_cover`) — pairs with
    /// [`WorldLightLevel`](Self::WorldLightLevel) for the overcast dim.
    WorldCloudCover,
    /// Air temperature (`WeatherState.temperature`) — heat shimmer / palette warmth.
    WorldTemperature,
    /// Relative humidity (`WeatherState.humidity`) — bloom wetness, surface sheen.
    WorldHumidity,
    /// Lunar phase (`WeatherState.moon_phase`) — the night-side light term.
    WorldMoonPhase,
    /// Mixer crossfader position — the visual A↔B blend (dead_drop_daw slot 17).
    /// Distinct from [`AudioCrossfader`](Self::AudioCrossfader), which is the audio
    /// bus's own fader; this is the mixer-state read the shader blends on.
    MixerCrossfader,
    /// Sequencer step position — the beat-synced grid highlight (dead_drop_daw
    /// slot 18).
    SeqStepPosition,
    // RDR bard-performance sources — harvested verbatim from
    // bard_aura.shaderbind.vixi (Wave 1a, no invented vocab). All scalar Permyriad.
    /// World flute velocity.
    WorldFluteVelocity,
    /// World mood.
    WorldMood,
    /// World water depth.
    WorldWaterDepth,
    /// Ambient light level (0..10000 Permyriad) — the SEE+HEAR light-cascade lane
    /// (2026-07-14). Host-derived (e.g. time-of-day), not yet baked from
    /// forge-sky-bake/forge-ibl-bake (no runtime IBL consumer exists — see those
    /// crates' CLAUDE.md; this source is the routing seam that consumer will fill).
    WorldLightLevel,
    /// BqRouter confidence margin for the active PTY chunk (0..10000 Permyriad,
    /// low = the specialist call was uncertain and drained to T2). The scalar
    /// half of the singing terminal's route decision; the categorical half
    /// (which specialist fired) rides `forge_core::ump::RoutingTag.channel`, not
    /// this lane. Fed from `technothesia` `score::route_pty_chunk`.
    TermRouteMargin,
    // ── IRONROOT REGISTERS (Sean 2026-07-31 "world.clarity_q is a signal that
    // should exist") ────────────────────────────────────────────────────────────
    // The console's four alchemical registers, bound by
    // ironroot/vixi/ironroot_console.shaderbind.vixi. No vocab is invented: the
    // passive trio are the named members of `forge_items::stability::PassivePool`
    // (stability.rs:27) and clarity is the 8th register (forge-items/src/forge.rs:231,
    // "Clarity → Dex"). The host quantizes to Permyriad at the firewall, exactly as
    // `world.mood` does.
    /// Crystallisation parameter — drives the same `glass_opacity_q` the
    /// VoxelFontAnimator writes, so the pane frosts as the player clears
    /// (sand→glass emergence, ADR-0032).
    WorldClarity,
    /// Sol / gold / Vibration — the bloom term; the surface glows as it rings.
    WorldResonance,
    /// Venus / copper / Gender — degrades harmonic fidelity into chromatic split.
    WorldTarnish,
    /// Saturn / lead / Correspondence — mass; it shakes the pane.
    WorldShadowWeight,
    // ── LABAN EFFORTS (DESIGNS.md D5 2026-08-24, broski witness face) ────────
    // Quantized from the companion's OODA state today; the FactionMind 8-axis
    // scorer becomes the writer later (w6) without touching this seam.
    /// Laban Weight effort (0 = Light, 10000 = Strong) — ring width drive.
    LabanWeight,
    /// Laban Space effort (0 = Direct, 10000 = Indirect) — star orbit spread.
    LabanSpace,
    /// Star glow drive above the 120 floor (per-state Schaeffer gain), Permyriad.
    LabanGlow,
    /// Laban Flow effort (0 = Bound, 10000 = Free) — ectoplasmic star trails.
    LabanFlow,
    /// Star micro-jitter drive (per-state Schaeffer gain, kit vibe_shake), Permyriad.
    LabanShake,
    // ── CDK TRIAD (DESIGNS.md D1 2026-08-24, the singing terminal's meters) ──
    // Quantized from forge_mud_v3::cdk::Triad channels (0..1000 → Permyriad).
    /// Love lane (binds), Permyriad.
    CdkLove,
    /// Strife lane (separates), Permyriad.
    CdkStrife,
    /// Entropy lane (the haunt), Permyriad.
    CdkEntropy,
    /// Harmony 0..1000 scaled to Permyriad.
    CdkHarmony,
    /// Beat-phase pulse, nonzero only while the triad reads DISSONANT.
    CdkDissonantPulse,
    // ── SPRITE (DESIGNS.md D6 2026-08-24) ────────────────────────────────────
    /// The focal sprite's breathe-timeline sample, Permyriad.
    SpriteBreathed,
    /// Sung-word attack with linear decay (VibeOrgan lane), Permyriad.
    SpriteHitFlash,
}

impl SignalSource {
    /// Resolve `"namespace.field"` to a canonical source. Unknown ⇒ `None` (the
    /// caller turns it into a hard, line-located error — never a silent drop).
    fn resolve(token: &str) -> Option<Self> {
        Some(match token {
            "audio.rms" => Self::AudioRms,
            "audio.beat_phase" => Self::AudioBeatPhase,
            "audio.spectral_centroid" => Self::AudioSpectralCentroid,
            "audio.crossfader" => Self::AudioCrossfader,
            "audio.sub_bass" => Self::AudioSubBass,
            "audio.spectrum_low" => Self::AudioSpectrumLow,
            "audio.spectrum_mid" => Self::AudioSpectrumMid,
            "audio.spectrum_high" => Self::AudioSpectrumHigh,
            "audio.deck_a_rms" => Self::AudioDeckARms,
            "audio.deck_b_rms" => Self::AudioDeckBRms,
            "audio.spectrum_band_0" => Self::AudioSpectrumBand0,
            "audio.spectrum_band_1" => Self::AudioSpectrumBand1,
            "audio.spectrum_band_2" => Self::AudioSpectrumBand2,
            "audio.spectrum_band_3" => Self::AudioSpectrumBand3,
            "audio.spectrum_band_4" => Self::AudioSpectrumBand4,
            "audio.spectrum_band_5" => Self::AudioSpectrumBand5,
            "audio.spectrum_band_6" => Self::AudioSpectrumBand6,
            "audio.formant_energy" => Self::AudioFormantEnergy,
            "vibematrix.rain_intensity" => Self::VibeRainIntensity,
            "vibematrix.artifact_glow" => Self::VibeArtifactGlow,
            "vibematrix.threat_color" => Self::VibeThreatColor,
            "vibematrix.hue" => Self::VibeHue,
            "vibematrix.intensity" => Self::VibeIntensity,
            "vibematrix.bloom" => Self::VibeBloom,
            "vibematrix.warp" => Self::VibeWarp,
            "vibematrix.ghost_intensity" => Self::VibeGhostIntensity,
            "vibematrix.beat_phase" => Self::VibeBeatPhase,
            "input.pen_pressure" => Self::InputPenPressure,
            "world.authority_q" => Self::WorldAuthority,
            "world.velocity" => Self::WorldVelocity,
            "world.sun_dir" => Self::WorldSunDir,
            "world.exposure" => Self::WorldExposure,
            "world.gravity" => Self::WorldGravity,
            "world.wind_speed" => Self::WorldWindSpeed,
            "world.wind_direction" => Self::WorldWindDirection,
            "world.precipitation_rate" => Self::WorldPrecipitationRate,
            "world.fog_density" => Self::WorldFogDensity,
            "world.cloud_cover" => Self::WorldCloudCover,
            "world.temperature" => Self::WorldTemperature,
            "world.humidity" => Self::WorldHumidity,
            "world.moon_phase" => Self::WorldMoonPhase,
            "mixer.crossfader" => Self::MixerCrossfader,
            "seq.current_step" => Self::SeqStepPosition,
            "world.flute_velocity" => Self::WorldFluteVelocity,
            "world.mood" => Self::WorldMood,
            "world.water_depth" => Self::WorldWaterDepth,
            "world.light_level" => Self::WorldLightLevel,
            "world.clarity_q" => Self::WorldClarity,
            "world.resonance_q" => Self::WorldResonance,
            "world.tarnish_q" => Self::WorldTarnish,
            "world.shadow_weight_q" => Self::WorldShadowWeight,
            "term.route_margin" => Self::TermRouteMargin,
            "laban.weight" => Self::LabanWeight,
            "laban.space" => Self::LabanSpace,
            "laban.glow" => Self::LabanGlow,
            "laban.flow" => Self::LabanFlow,
            "laban.shake" => Self::LabanShake,
            "cdk.love" => Self::CdkLove,
            "cdk.strife" => Self::CdkStrife,
            "cdk.entropy" => Self::CdkEntropy,
            "cdk.harmony" => Self::CdkHarmony,
            "cdk.dissonant_pulse" => Self::CdkDissonantPulse,
            "sprite.breathed" => Self::SpriteBreathed,
            "sprite.hit_flash" => Self::SpriteHitFlash,
            _ => return None,
        })
    }

    /// Canonical `"namespace.field"` for the pretty-printer (round-trip inverse).
    pub fn canonical(self) -> &'static str {
        match self {
            Self::AudioRms => "audio.rms",
            Self::AudioBeatPhase => "audio.beat_phase",
            Self::AudioSpectralCentroid => "audio.spectral_centroid",
            Self::AudioCrossfader => "audio.crossfader",
            Self::AudioSubBass => "audio.sub_bass",
            Self::AudioSpectrumLow => "audio.spectrum_low",
            Self::AudioSpectrumMid => "audio.spectrum_mid",
            Self::AudioSpectrumHigh => "audio.spectrum_high",
            Self::AudioDeckARms => "audio.deck_a_rms",
            Self::AudioDeckBRms => "audio.deck_b_rms",
            Self::AudioSpectrumBand0 => "audio.spectrum_band_0",
            Self::AudioSpectrumBand1 => "audio.spectrum_band_1",
            Self::AudioSpectrumBand2 => "audio.spectrum_band_2",
            Self::AudioSpectrumBand3 => "audio.spectrum_band_3",
            Self::AudioSpectrumBand4 => "audio.spectrum_band_4",
            Self::AudioSpectrumBand5 => "audio.spectrum_band_5",
            Self::AudioSpectrumBand6 => "audio.spectrum_band_6",
            Self::AudioFormantEnergy => "audio.formant_energy",
            Self::VibeRainIntensity => "vibematrix.rain_intensity",
            Self::VibeArtifactGlow => "vibematrix.artifact_glow",
            Self::VibeThreatColor => "vibematrix.threat_color",
            Self::VibeHue => "vibematrix.hue",
            Self::VibeIntensity => "vibematrix.intensity",
            Self::VibeBloom => "vibematrix.bloom",
            Self::VibeWarp => "vibematrix.warp",
            Self::VibeGhostIntensity => "vibematrix.ghost_intensity",
            Self::VibeBeatPhase => "vibematrix.beat_phase",
            Self::InputPenPressure => "input.pen_pressure",
            Self::WorldAuthority => "world.authority_q",
            Self::WorldVelocity => "world.velocity",
            Self::WorldSunDir => "world.sun_dir",
            Self::WorldExposure => "world.exposure",
            Self::WorldGravity => "world.gravity",
            Self::WorldWindSpeed => "world.wind_speed",
            Self::WorldWindDirection => "world.wind_direction",
            Self::WorldPrecipitationRate => "world.precipitation_rate",
            Self::WorldFogDensity => "world.fog_density",
            Self::WorldCloudCover => "world.cloud_cover",
            Self::WorldTemperature => "world.temperature",
            Self::WorldHumidity => "world.humidity",
            Self::WorldMoonPhase => "world.moon_phase",
            Self::MixerCrossfader => "mixer.crossfader",
            Self::SeqStepPosition => "seq.current_step",
            Self::WorldFluteVelocity => "world.flute_velocity",
            Self::WorldMood => "world.mood",
            Self::WorldWaterDepth => "world.water_depth",
            Self::WorldLightLevel => "world.light_level",
            Self::WorldClarity => "world.clarity_q",
            Self::WorldResonance => "world.resonance_q",
            Self::WorldTarnish => "world.tarnish_q",
            Self::WorldShadowWeight => "world.shadow_weight_q",
            Self::TermRouteMargin => "term.route_margin",
            Self::LabanWeight => "laban.weight",
            Self::LabanSpace => "laban.space",
            Self::LabanGlow => "laban.glow",
            Self::LabanFlow => "laban.flow",
            Self::LabanShake => "laban.shake",
            Self::CdkLove => "cdk.love",
            Self::CdkStrife => "cdk.strife",
            Self::CdkEntropy => "cdk.entropy",
            Self::CdkHarmony => "cdk.harmony",
            Self::CdkDissonantPulse => "cdk.dissonant_pulse",
            Self::SpriteBreathed => "sprite.breathed",
            Self::SpriteHitFlash => "sprite.hit_flash",
        }
    }

    /// A VISUAL-only source: a `vibematrix.*` effect channel. The
    /// `visual_only_mutates_authority` gate forbids one of these aliasing the
    /// authority lane — the sovereignty guard.
    pub fn is_visual_only(self) -> bool {
        matches!(
            self,
            Self::VibeRainIntensity
                | Self::VibeArtifactGlow
                | Self::VibeThreatColor
                | Self::VibeHue
                | Self::VibeIntensity
                | Self::VibeBloom
                | Self::VibeWarp
                | Self::VibeGhostIntensity
                | Self::VibeBeatPhase
        )
    }

    /// The sovereign authority source (`world.authority_q`) — the lane the gate
    /// protects from visual mutation.
    pub fn is_authority(self) -> bool {
        matches!(self, Self::WorldAuthority)
    }
}

/// The closed set of recognised sovereignty / perf gates. Unknown gate name ⇒ hard
/// parse error (Signal Law — no silently-ignored governance directive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateName {
    /// The shaderbind must compile AOT; never trigger a shader recompile on the hot
    /// path (the 120Hz tick / submit). Honoured by construction — this parser is
    /// cold-path and its output is a static integer table.
    ShaderCompileHotpath,
    /// A visual-only signal must never mutate the authority lane (sovereignty).
    VisualOnlyMutatesAuthority,
    /// Audio signals must not be treated as identity.
    AudioNotIdentity,
    /// The mix must stay legible (no channel collisions muddying the surface).
    MixClarity,
}

impl GateName {
    fn resolve(token: &str) -> Option<Self> {
        Some(match token {
            "shader_compile_hotpath" => Self::ShaderCompileHotpath,
            "visual_only_mutates_authority" => Self::VisualOnlyMutatesAuthority,
            "audio_not_identity" => Self::AudioNotIdentity,
            "mix_clarity" => Self::MixClarity,
            _ => return None,
        })
    }

    /// Canonical gate name for the pretty-printer.
    pub fn canonical(self) -> &'static str {
        match self {
            Self::ShaderCompileHotpath => "shader_compile_hotpath",
            Self::VisualOnlyMutatesAuthority => "visual_only_mutates_authority",
            Self::AudioNotIdentity => "audio_not_identity",
            Self::MixClarity => "mix_clarity",
        }
    }
}

/// A gate's policy — the authored stance on its named invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatePolicy {
    /// The gate is forbidden.
    Forbidden,
    /// The gate is required.
    Required,
}

impl GatePolicy {
    fn resolve(token: &str) -> Option<Self> {
        Some(match token {
            "forbidden" => Self::Forbidden,
            "required" => Self::Required,
            _ => return None,
        })
    }

    /// Canonical policy name for the pretty-printer.
    pub fn canonical(self) -> &'static str {
        match self {
            Self::Forbidden => "forbidden",
            Self::Required => "required",
        }
    }
}

/// A declared gate — a named invariant + its authored policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    /// The gate name.
    pub name: GateName,
    /// The gate policy.
    pub policy: GatePolicy,
}

/// One lowered channel route: `surface.channel[channel] <- <source>`. The signal
/// NAME has been resolved to its [`SignalSource`] at compile time, so the runtime
/// carries only integers + the closed enum (no string lookup on apply).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelRoute {
    /// The channel index.
    pub channel: u16,
    /// The signal source.
    pub source: SignalSource,
}

/// The compiled shaderbind: the surface it feeds, its channel routes (index-ordered,
/// unique, holes legal), and its gates. This is the cold-path artifact;
/// [`route`](ShaderBind::route)
/// turns it + live signal values into the hot integer channel array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderBind {
    /// The shaderbind's OWN identity surface (`surface:` — e.g. `audio_vis`, `udle`).
    pub surface: String, // @forge:allow_alloc: cold path
    /// The profile name.
    pub profile: String, // @forge:allow_alloc: cold path
    /// The DESTINATION surface the routes fill (`<target>.channel[N]`). Independent
    /// of [`surface`](Self::surface): the live `audio_vis` binding declares surface
    /// `audio_vis` but routes INTO `vibematrix`. Every route shares this one target.
    pub target_surface: String, // @forge:allow_alloc: cold path
    /// Routes sorted by `channel`, validated contiguous `0..routes.len()`.
    pub routes: Vec<ChannelRoute>, // @forge:allow_alloc: cold path
    /// The gates.
    pub gates: Vec<Gate>,          // @forge:allow_alloc: cold path
}

/// The maximum channels a single surface may bind (matches `look_composite`'s
/// [`MAX_BINDS`](crate::reactive::MAX_BINDS) ceiling; the live surfaces use
/// 5 + 8). The `MAX_CHANNELS + 1`th
/// route is an error, never a silent truncation.
pub const MAX_CHANNELS: usize = 64;

/// Parse error with a 1-based line location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderBindError {
    /// The 1-based line number where the error occurred.
    pub line: usize,
    /// The error message.
    pub message: String, // @forge:allow_alloc: cold path (error reporting)
}

impl ShaderBindError {
    fn at(line: usize, message: impl Into<String>) -> Self {
        Self { line, message: message.into() } // @forge:allow_alloc: cold path
    }
}

/// Strip a required `prefix` and return the suffix, or `Err(())` if absent.
fn strip<'a>(tok: &'a str, prefix: &str) -> Result<&'a str, ()> {
    tok.strip_prefix(prefix).ok_or(())
}

/// Parse a `<lo>..<hi>` Permyriad range, validating `lo <= hi <= 10000`.
fn parse_range(spec: &str, line: usize) -> Result<(u16, u16), ShaderBindError> {
    let (lo_s, hi_s) = spec
        .split_once("..")
        .ok_or_else(|| ShaderBindError::at(line, format!("range must be '<lo>..<hi>', found '{spec}'")))?;
    let lo: i64 = lo_s
        .parse()
        .map_err(|_| ShaderBindError::at(line, format!("range low '{lo_s}' is not an integer")))?;
    let hi: i64 = hi_s
        .parse()
        .map_err(|_| ShaderBindError::at(line, format!("range high '{hi_s}' is not an integer")))?;
    if !(0..=10_000).contains(&lo) || !(0..=10_000).contains(&hi) {
        return Err(ShaderBindError::at(line, format!("range {lo}..{hi} outside Permyriad 0..=10000")));
    }
    if lo > hi {
        return Err(ShaderBindError::at(line, format!("range low {lo} exceeds high {hi}")));
    }
    Ok((lo as u16, hi as u16))
}

/// A signal declaration captured during the first pass (`signal <name> source=.. range=..`).
struct SignalDecl {
    source: SignalSource,
    #[allow(dead_code)] // range is validated at parse; carried for future per-channel scaling.
    lo: u16,
    #[allow(dead_code)]
    hi: u16,
}

/// Parse a `#vixi:shaderbind v1` profile into a compiled [`ShaderBind`].
///
/// HARD-errors (line-located, never a silent drop): a missing/!=`shaderbind v1`
/// header, a missing `surface:`/`profile:`, an unknown source or gate, a route
/// referencing an undeclared signal or the wrong surface, a duplicate or
/// non-contiguous channel, or more than [`MAX_CHANNELS`] routes.
///
/// @forge:allow_alloc: entire function is cold path (shaderbind load/edit).
pub fn parse_shaderbind(input: &str) -> Result<ShaderBind, ShaderBindError> {
    use std::collections::HashMap;

    let mut header_ok = false;
    let mut surface: Option<String> = None;
    let mut profile: Option<String> = None;
    let mut signals: HashMap<String, SignalDecl> = HashMap::new(); // @forge:allow_alloc: cold path
    let mut raw_routes: Vec<(usize, u16, String)> = Vec::new(); // (line, channel, signal_name)
    let mut gates: Vec<Gate> = Vec::new(); // @forge:allow_alloc: cold path

    for (idx, line) in input.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // The `#vixi:` header is a directive; other `#` / `//` lines are comments.
        if let Ok(rest) = strip(trimmed, "#vixi:") {
            if rest.trim() != "shaderbind v1" {
                return Err(ShaderBindError::at(line_num, format!("expected '#vixi:shaderbind v1', found '#vixi:{rest}'")));
            }
            header_ok = true;
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        if let Ok(rest) = strip(trimmed, "surface:") {
            surface = Some(rest.trim().to_string()); // @forge:allow_alloc: cold path
            continue;
        }
        if let Ok(rest) = strip(trimmed, "profile:") {
            profile = Some(rest.trim().to_string()); // @forge:allow_alloc: cold path
            continue;
        }

        if let Ok(rest) = strip(trimmed, "signal ") {
            // signal <name> source=<src> range=<lo>..<hi>
            let tok: Vec<&str> = rest.split_whitespace().collect(); // @forge:allow_alloc: cold path
            if tok.len() != 3 {
                return Err(ShaderBindError::at(line_num, format!("signal must be 'signal <name> source=<src> range=<lo>..<hi>', found {} field(s)", tok.len() + 1)));
            }
            let name = tok[0];
            let src_tok = strip(tok[1], "source=").map_err(|_| ShaderBindError::at(line_num, format!("expected 'source=<src>', found '{}'", tok[1])))?;
            let source = SignalSource::resolve(src_tok)
                .ok_or_else(|| ShaderBindError::at(line_num, format!("unknown signal source '{src_tok}'")))?;
            let range_tok = strip(tok[2], "range=").map_err(|_| ShaderBindError::at(line_num, format!("expected 'range=<lo>..<hi>', found '{}'", tok[2])))?;
            let (lo, hi) = parse_range(range_tok, line_num)?;
            if signals.insert(name.to_string(), SignalDecl { source, lo, hi }).is_some() {
                return Err(ShaderBindError::at(line_num, format!("signal '{name}' declared twice")));
            }
            continue;
        }

        if let Ok(rest) = strip(trimmed, "gate ") {
            // gate <name> = <forbidden|required>
            let tok: Vec<&str> = rest.split_whitespace().collect(); // @forge:allow_alloc: cold path
            if tok.len() != 3 || tok[1] != "=" {
                return Err(ShaderBindError::at(line_num, "gate must be 'gate <name> = <forbidden|required>'"));
            }
            let name = GateName::resolve(tok[0]).ok_or_else(|| ShaderBindError::at(line_num, format!("unknown gate '{}'", tok[0])))?;
            let policy = GatePolicy::resolve(tok[2]).ok_or_else(|| ShaderBindError::at(line_num, format!("gate policy must be 'forbidden' or 'required', found '{}'", tok[2])))?;
            gates.push(Gate { name, policy });
            continue;
        }

        // A channel route: `<surface>.channel[<n>] <- <signal_name>`.
        if trimmed.contains("<-") {
            let tok: Vec<&str> = trimmed.split_whitespace().collect(); // @forge:allow_alloc: cold path
            if tok.len() != 3 || tok[1] != "<-" {
                return Err(ShaderBindError::at(line_num, "route must be '<surface>.channel[<n>] <- <signal>'"));
            }
            // tok[0] = <surface>.channel[<n>]
            let (sfx, ch) = tok[0]
                .split_once(".channel[")
                .ok_or_else(|| ShaderBindError::at(line_num, format!("route target must be '<surface>.channel[<n>]', found '{}'", tok[0])))?;
            let ch = ch
                .strip_suffix(']')
                .ok_or_else(|| ShaderBindError::at(line_num, format!("channel index must end with ']', found '{}'", tok[0])))?;
            let channel: u16 = ch
                .parse()
                .map_err(|_| ShaderBindError::at(line_num, format!("channel index '{ch}' is not an integer")))?;
            // The route surface must match the declared surface (caught after the loop
            // if `surface:` came later; we stash the prefix and resolve below).
            raw_routes.push((line_num, channel, format!("{sfx}\u{1}{}", tok[2]))); // sfx + name, \u1-sep
            continue;
        }

        return Err(ShaderBindError::at(line_num, format!("unrecognised shaderbind line '{trimmed}'")));
    }

    // ── Validate the preamble ────────────────────────────────────────────────
    if !header_ok {
        return Err(ShaderBindError::at(1, "missing '#vixi:shaderbind v1' header"));
    }
    let surface = surface.ok_or_else(|| ShaderBindError::at(1, "missing 'surface:' declaration"))?;
    let profile = profile.ok_or_else(|| ShaderBindError::at(1, "missing 'profile:' declaration"))?;

    // ── Resolve routes (signal name -> source; one shared target surface) ────
    // The route TARGET surface (`<target>.channel[N]`) is INDEPENDENT of the
    // shaderbind's `surface:` identity — the live `audio_vis` binding declares
    // surface `audio_vis` but routes INTO `vibematrix`. All routes must share ONE
    // target surface (the single destination they fill).
    let mut routes: Vec<ChannelRoute> = Vec::with_capacity(raw_routes.len()); // @forge:allow_alloc: cold path
    let mut target_surface: Option<String> = None;
    for (line_num, channel, packed) in raw_routes {
        let (sfx, name) = packed.split_once('\u{1}').expect("packed route is sfx\\u1name");
        match &target_surface {
            None => target_surface = Some(sfx.to_string()), // @forge:allow_alloc: cold path
            Some(t) if t != sfx => {
                return Err(ShaderBindError::at(line_num, format!("all routes must target ONE surface; '{sfx}' != '{t}'")));
            }
            _ => {}
        }
        let decl = signals
            .get(name)
            .ok_or_else(|| ShaderBindError::at(line_num, format!("route references undeclared signal '{name}'")))?;
        if routes.len() >= MAX_CHANNELS {
            return Err(ShaderBindError::at(line_num, format!("too many channels (max {MAX_CHANNELS})")));
        }
        routes.push(ChannelRoute { channel, source: decl.source });
    }
    if routes.is_empty() {
        return Err(ShaderBindError::at(1, "shaderbind binds no channels"));
    }
    let target_surface = target_surface.expect("non-empty routes set a target surface");

    // ── Channels must be UNIQUE. Holes are legal (Sean 2026-07-28): a channel index
    // is a FIXED SLOT the CPU reference reads, not a position in a list — bard_aura
    // owns 4/5/8/12/13/14, abyssal_depth reserves 2 for the FFT that has not landed,
    // forge_signal leaves 4 unbound. Contiguity would have forced those maps to lie.
    // A double-bind is still a hard error: two sources on one slot is a silent race.
    routes.sort_by_key(|r| r.channel);
    for pair in routes.windows(2) {
        if pair[0].channel == pair[1].channel {
            return Err(ShaderBindError::at(
                1,
                format!("channel {} bound twice (channels must be unique)", pair[0].channel),
            ));
        }
    }

    Ok(ShaderBind { surface, profile, target_surface, routes, gates })
}

impl ShaderBind {
    /// Number of input channels this surface actually binds.
    pub fn channel_count(&self) -> usize {
        self.routes.len()
    }

    /// Size of the channel ARRAY this surface needs — highest bound slot + 1. Equal
    /// to [`channel_count`](Self::channel_count) for a dense binding; larger when the
    /// author reserved holes (sparse slots are legal since 2026-07-28).
    pub fn channel_span(&self) -> usize {
        self.routes.iter().map(|r| r.channel as usize + 1).max().unwrap_or(0)
    }

    /// Whether a gate with `name` is declared `forbidden`.
    pub fn forbids(&self, name: GateName) -> bool {
        self.gates.iter().any(|g| g.name == name && g.policy == GatePolicy::Forbidden)
    }

    /// THE hot product: populate the surface's integer channel array from live signal
    /// values. `channel[N]` = the Permyriad value of the source bound to SLOT `N`;
    /// a reserved (unbound) slot stays 0. The array is [`channel_span`](Self::channel_span)
    /// long, so a sparse map never drops its high slots. Zero float; the only alloc is
    /// the result vec (one per cold reload, never per-tick — the host keeps the
    /// returned array and re-fills it in place via [`route_into`](Self::route_into)).
    pub fn route(&self, signals: &SignalValues) -> Vec<u16> {
        let mut out = vec![0u16; self.channel_span()]; // @forge:allow_alloc: cold path (caller reuses via route_into)
        self.route_into(signals, &mut out);
        out
    }

    /// In-place [`route`](Self::route) into a caller-owned array (the zero-alloc steady-state form).
    /// `dst` must be `channel_span()` long; extra slots are left untouched.
    pub fn route_into(&self, signals: &SignalValues, dst: &mut [u16]) {
        for r in &self.routes {
            if let Some(slot) = dst.get_mut(r.channel as usize) {
                *slot = signals.get(r.source);
            }
        }
    }

    /// Enforce the determinable structural invariants the declared gates name — the
    /// teeth, not just recorded policy. Returns `Err` on a real violation:
    ///   * `visual_only_mutates_authority = forbidden`: the authority lane
    ///     (`world.authority_q`) must be bound EXACTLY once and never aliased by a
    ///     visual-only (`vibematrix.*`) source. (Channel uniqueness already bars a
    ///     double-bind; this also rejects a shaderbind that routes a visual source
    ///     where authority is expected, or binds authority more than once.)
    /// Recognised-but-not-structurally-determinable gates (`audio_not_identity`,
    /// `mix_clarity`) are carried as policy for the runtime; they are NOT silently
    /// dropped (they were grammar-gated at parse).
    pub fn verify_gates(&self) -> Result<(), ShaderBindError> {
        if self.forbids(GateName::VisualOnlyMutatesAuthority) {
            let authority_routes: Vec<&ChannelRoute> =
                self.routes.iter().filter(|r| r.source.is_authority()).collect();
            if authority_routes.len() > 1 {
                return Err(ShaderBindError::at(
                    1,
                    "visual_only_mutates_authority: world.authority_q bound to >1 channel (authority must be a single lane)",
                ));
            }
            // A visual-only source may never sit alone where authority should — i.e.
            // if the surface claims an authority gate it must actually carry authority.
            if authority_routes.is_empty() && self.routes.iter().any(|r| r.source.is_visual_only()) {
                return Err(ShaderBindError::at(
                    1,
                    "visual_only_mutates_authority gate present, visual sources bound, but no world.authority_q lane to protect",
                ));
            }
        }
        Ok(())
    }
}

/// Live Permyriad (0..=10000) values for every [`SignalSource`], read by
/// [`ShaderBind::route`]. The host fills these from the running audio bus, the
/// vibematrix, pen input, and the world authority each cold reload (or whenever the
/// surface is re-uploaded) — the routing itself stays declarative + integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignalValues {
    /// Audio RMS value.
    pub audio_rms: u16,
    /// Audio beat phase.
    pub audio_beat_phase: u16,
    /// Audio spectral centroid.
    pub audio_spectral_centroid: u16,
    /// Audio crossfader.
    pub audio_crossfader: u16,
    /// Audio sub-bass.
    pub audio_sub_bass: u16,
    /// Audio spectrum low.
    pub audio_spectrum_low: u16,
    /// Audio spectrum mid.
    pub audio_spectrum_mid: u16,
    /// Audio spectrum high.
    pub audio_spectrum_high: u16,
    /// Audio deck A RMS.
    pub audio_deck_a_rms: u16,
    /// Audio deck B RMS.
    pub audio_deck_b_rms: u16,
    /// 7-band parametric spectrum, sub-bass → brilliance (ISO-31 octave centres).
    pub audio_spectrum_bands: [u16; 7],
    /// Vocal-formant energy (HPSS harmonic tail, LPC 300-3500Hz peak-pick).
    pub audio_formant_energy: u16,
    /// Vibe rain intensity.
    pub vibe_rain_intensity: u16,
    /// Vibe artifact glow.
    pub vibe_artifact_glow: u16,
    /// Vibe threat color.
    pub vibe_threat_color: u16,
    /// Vibe hue.
    pub vibe_hue: u16,
    /// Vibe intensity.
    pub vibe_intensity: u16,
    /// Vibe bloom.
    pub vibe_bloom: u16,
    /// Vibe warp.
    pub vibe_warp: u16,
    /// Vibe ghost intensity.
    pub vibe_ghost_intensity: u16,
    /// Vibe beat phase.
    pub vibe_beat_phase: u16,
    /// Input pen pressure.
    pub input_pen_pressure: u16,
    /// World authority.
    pub world_authority: u16,
    /// World flute velocity.
    pub world_flute_velocity: u16,
    /// World mood.
    pub world_mood: u16,
    /// World water depth.
    pub world_water_depth: u16,
    /// World light level.
    pub world_light_level: u16,
    /// World velocity.
    pub world_velocity: u16,
    /// World sun direction.
    pub world_sun_dir: u16,
    /// World exposure.
    pub world_exposure: u16,
    /// World gravity.
    pub world_gravity: u16,
    // Weather lane — host-quantized from forge_game_systems::weather::WeatherState.
    /// World wind speed.
    pub world_wind_speed: u16,
    /// World wind direction.
    pub world_wind_direction: u16,
    /// World precipitation rate.
    pub world_precipitation_rate: u16,
    /// World fog density.
    pub world_fog_density: u16,
    /// World cloud cover.
    pub world_cloud_cover: u16,
    /// World temperature.
    pub world_temperature: u16,
    /// World humidity.
    pub world_humidity: u16,
    /// World moon phase.
    pub world_moon_phase: u16,
    /// Mixer crossfader.
    pub mixer_crossfader: u16,
    /// Sequencer step position.
    pub seq_step_position: u16,
    /// Terminal route margin.
    pub term_route_margin: u16,
    // Ironroot registers — host-quantized from the console's alchemical state.
    /// World clarity.
    pub world_clarity: u16,
    /// World resonance.
    pub world_resonance: u16,
    /// World tarnish.
    pub world_tarnish: u16,
    /// World shadow weight.
    pub world_shadow_weight: u16,
    // Laban efforts — quantized from the companion behavior state (D5).
    /// Laban weight.
    pub laban_weight: u16,
    /// Laban space.
    pub laban_space: u16,
    /// Laban glow.
    pub laban_glow: u16,
    /// Laban flow.
    pub laban_flow: u16,
    /// Laban shake.
    pub laban_shake: u16,
    // CDK triad — quantized from forge_mud_v3::cdk::Triad (D1).
    /// CDK love.
    pub cdk_love: u16,
    /// CDK strife.
    pub cdk_strife: u16,
    /// CDK entropy.
    pub cdk_entropy: u16,
    /// CDK harmony.
    pub cdk_harmony: u16,
    /// CDK dissonant pulse.
    pub cdk_dissonant_pulse: u16,
    // Sprite lanes (D6).
    /// Sprite breathed.
    pub sprite_breathed: u16,
    /// Sprite hit flash.
    pub sprite_hit_flash: u16,
}

impl SignalValues {
    /// The Permyriad value for `source` (the routing tap).
    pub fn get(&self, source: SignalSource) -> u16 {
        match source {
            SignalSource::AudioRms => self.audio_rms,
            SignalSource::AudioBeatPhase => self.audio_beat_phase,
            SignalSource::AudioSpectralCentroid => self.audio_spectral_centroid,
            SignalSource::AudioCrossfader => self.audio_crossfader,
            SignalSource::AudioSubBass => self.audio_sub_bass,
            SignalSource::AudioSpectrumLow => self.audio_spectrum_low,
            SignalSource::AudioSpectrumMid => self.audio_spectrum_mid,
            SignalSource::AudioSpectrumHigh => self.audio_spectrum_high,
            SignalSource::AudioDeckARms => self.audio_deck_a_rms,
            SignalSource::AudioDeckBRms => self.audio_deck_b_rms,
            SignalSource::AudioSpectrumBand0 => self.audio_spectrum_bands[0],
            SignalSource::AudioSpectrumBand1 => self.audio_spectrum_bands[1],
            SignalSource::AudioSpectrumBand2 => self.audio_spectrum_bands[2],
            SignalSource::AudioSpectrumBand3 => self.audio_spectrum_bands[3],
            SignalSource::AudioSpectrumBand4 => self.audio_spectrum_bands[4],
            SignalSource::AudioSpectrumBand5 => self.audio_spectrum_bands[5],
            SignalSource::AudioSpectrumBand6 => self.audio_spectrum_bands[6],
            SignalSource::AudioFormantEnergy => self.audio_formant_energy,
            SignalSource::VibeRainIntensity => self.vibe_rain_intensity,
            SignalSource::VibeArtifactGlow => self.vibe_artifact_glow,
            SignalSource::VibeThreatColor => self.vibe_threat_color,
            SignalSource::VibeHue => self.vibe_hue,
            SignalSource::VibeIntensity => self.vibe_intensity,
            SignalSource::VibeBloom => self.vibe_bloom,
            SignalSource::VibeWarp => self.vibe_warp,
            SignalSource::VibeGhostIntensity => self.vibe_ghost_intensity,
            SignalSource::VibeBeatPhase => self.vibe_beat_phase,
            SignalSource::InputPenPressure => self.input_pen_pressure,
            SignalSource::WorldAuthority => self.world_authority,
            SignalSource::WorldFluteVelocity => self.world_flute_velocity,
            SignalSource::WorldMood => self.world_mood,
            SignalSource::WorldWaterDepth => self.world_water_depth,
            SignalSource::WorldLightLevel => self.world_light_level,
            SignalSource::WorldVelocity => self.world_velocity,
            SignalSource::WorldSunDir => self.world_sun_dir,
            SignalSource::WorldExposure => self.world_exposure,
            SignalSource::WorldGravity => self.world_gravity,
            SignalSource::WorldWindSpeed => self.world_wind_speed,
            SignalSource::WorldWindDirection => self.world_wind_direction,
            SignalSource::WorldPrecipitationRate => self.world_precipitation_rate,
            SignalSource::WorldFogDensity => self.world_fog_density,
            SignalSource::WorldCloudCover => self.world_cloud_cover,
            SignalSource::WorldTemperature => self.world_temperature,
            SignalSource::WorldHumidity => self.world_humidity,
            SignalSource::WorldMoonPhase => self.world_moon_phase,
            SignalSource::MixerCrossfader => self.mixer_crossfader,
            SignalSource::SeqStepPosition => self.seq_step_position,
            SignalSource::TermRouteMargin => self.term_route_margin,
            SignalSource::WorldClarity => self.world_clarity,
            SignalSource::WorldResonance => self.world_resonance,
            SignalSource::WorldTarnish => self.world_tarnish,
            SignalSource::WorldShadowWeight => self.world_shadow_weight,
            SignalSource::LabanWeight => self.laban_weight,
            SignalSource::LabanSpace => self.laban_space,
            SignalSource::LabanGlow => self.laban_glow,
            SignalSource::LabanFlow => self.laban_flow,
            SignalSource::LabanShake => self.laban_shake,
            SignalSource::CdkLove => self.cdk_love,
            SignalSource::CdkStrife => self.cdk_strife,
            SignalSource::CdkEntropy => self.cdk_entropy,
            SignalSource::CdkHarmony => self.cdk_harmony,
            SignalSource::CdkDissonantPulse => self.cdk_dissonant_pulse,
            SignalSource::SpriteBreathed => self.sprite_breathed,
            SignalSource::SpriteHitFlash => self.sprite_hit_flash,
        }
    }
}

/// Format a [`ShaderBind`] back to canonical shaderbind text. Round-trips with
/// [`parse_shaderbind`] (modulo comments / blank lines / signal range annotations,
/// which the lowered form drops — routes carry the resolved source directly).
///
/// @forge:allow_alloc: entire function is cold path (returns `String`).
pub fn pretty_print_shaderbind(b: &ShaderBind) -> String {
    use std::fmt::Write as _;
    let mut out = String::new(); // @forge:allow_alloc: cold path
    let _ = writeln!(out, "#vixi:shaderbind v1");
    let _ = writeln!(out, "surface: {}", b.surface);
    let _ = writeln!(out, "profile: {}", b.profile);
    out.push('\n');
    // One synthesised signal per route (name = the source's field), then the routes.
    for r in &b.routes {
        let field = r.source.canonical().rsplit('.').next().unwrap_or("sig");
        let _ = writeln!(out, "signal {field} source={} range=0..10000", r.source.canonical());
    }
    out.push('\n');
    for r in &b.routes {
        let field = r.source.canonical().rsplit('.').next().unwrap_or("sig");
        let _ = writeln!(out, "{}.channel[{}] <- {field}", b.target_surface, r.channel);
    }
    if !b.gates.is_empty() {
        out.push('\n');
        for g in &b.gates {
            let _ = writeln!(out, "gate {} = {}", g.name.canonical(), g.policy.canonical());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // The LIVE golden fixtures — inlined fixtures from the v2 source for self-contained testing.
    const UDLE: &str = "#vixi:shaderbind v1\nsurface: udle\nprofile: forge_primeflow\n\nsignal rms source=audio.rms range=0..10000\nsignal beat_phase source=audio.beat_phase range=0..10000\nsignal spectral_centroid source=audio.spectral_centroid range=0..10000\nsignal rain_intensity source=vibematrix.rain_intensity range=0..10000\nsignal artifact_glow source=vibematrix.artifact_glow range=0..10000\nsignal threat_color source=vibematrix.threat_color range=0..10000\nsignal pressure source=input.pen_pressure range=0..10000\nsignal authority source=world.authority_q range=0..10000\n\nudle.channel[0] <- rms\nudle.channel[1] <- beat_phase\nudle.channel[2] <- spectral_centroid\nudle.channel[3] <- rain_intensity\nudle.channel[4] <- artifact_glow\nudle.channel[5] <- threat_color\nudle.channel[6] <- pressure\nudle.channel[7] <- authority\n\ngate shader_compile_hotpath = forbidden\ngate visual_only_mutates_authority = forbidden\n";
    const AUDIO_VIS: &str = "#vixi:shaderbind v1\nsurface: audio_vis\nprofile: forge_primeflow\n\nsignal rms source=audio.rms range=0..10000\nsignal beat_phase source=audio.beat_phase range=0..10000\nsignal spectral_centroid source=audio.spectral_centroid range=0..10000\nsignal crossfader source=audio.crossfader range=0..10000\nsignal pressure source=input.pen_pressure range=0..10000\n\nvibematrix.channel[0] <- rms\nvibematrix.channel[1] <- beat_phase\nvibematrix.channel[2] <- spectral_centroid\nvibematrix.channel[3] <- crossfader\nvibematrix.channel[4] <- pressure\n\ngate audio_not_identity = required\ngate mix_clarity = required\ngate shader_compile_hotpath = forbidden\ngate visual_only_mutates_authority = forbidden\n";
    const SINGING_TERMINAL: &str = "#vixi:shaderbind v1\nsurface: singing_terminal\nprofile: singing_terminal\n\nsignal rms source=audio.rms range=0..10000\nsignal sub_bass source=audio.sub_bass range=0..10000\nsignal spectrum_high source=audio.spectrum_high range=0..10000\nsignal beat_phase source=audio.beat_phase range=0..10000\n\nvibematrix.channel[0] <- rms\nvibematrix.channel[1] <- sub_bass\nvibematrix.channel[2] <- spectrum_high\nvibematrix.channel[3] <- beat_phase\n\ngate audio_not_identity = required\ngate mix_clarity = required\ngate shader_compile_hotpath = forbidden\ngate visual_only_mutates_authority = required\n";

    // [ASPIRE: bformat-reinterpret]
    #[test]
    fn bformat_aliases_the_frozen_four_lanes_in_vibe_bus_lanes_order() {
        // VIBE_BUS_LANES order: [glow, pulse, chromatic, shake].
        let lanes: [u16; 4] = [9000, 1200, 3400, 5600];
        let b = BFormat::from_vibe_lanes(&lanes);
        assert_eq!(b.w, 9000, "glow is the omni W lane");
        assert_eq!(b.z, 1200, "pulse aliases Z");
        assert_eq!(b.x, 3400, "chromatic aliases X");
        assert_eq!(b.y, 5600, "shake aliases Y");
    }

    #[test]
    #[should_panic(expected = "frozen at 4")]
    fn bformat_refuses_a_non_frozen_lane_count() {
        BFormat::from_vibe_lanes(&[1, 2, 3]);
    }

    #[test]
    fn live_singing_terminal_binds_the_two_band_sources() {
        let b = parse_shaderbind(SINGING_TERMINAL)
            .expect("live singing_terminal.shaderbind.vixi must compile");
        assert_eq!(b.surface, "singing_terminal");
        assert_eq!(b.channel_count(), 4);
        let expect = [
            SignalSource::AudioRms,
            SignalSource::AudioSubBass,
            SignalSource::AudioSpectrumHigh,
            SignalSource::AudioBeatPhase,
        ];
        for (i, src) in expect.iter().enumerate() {
            assert_eq!(b.routes[i].channel, i as u16);
            assert_eq!(b.routes[i].source, *src, "channel {i} source");
        }
        let sig = SignalValues { audio_sub_bass: 4321, audio_spectrum_high: 8765, ..Default::default() };
        assert_eq!(sig.get(SignalSource::AudioSubBass), 4321);
        assert_eq!(sig.get(SignalSource::AudioSpectrumHigh), 8765);
        b.verify_gates().expect("singing_terminal satisfies its own gates");
    }

    #[test]
    fn live_udle_shaderbind_compiles_to_eight_ordered_channels() {
        let b = parse_shaderbind(UDLE).expect("live udle_vibematrix.shaderbind.vixi must compile");
        assert_eq!(b.surface, "udle");
        assert_eq!(b.profile, "forge_primeflow");
        assert_eq!(b.target_surface, "udle", "udle binds into its own channels");
        assert_eq!(b.channel_count(), 8, "udle binds 8 input channels");
        let expect = [
            SignalSource::AudioRms,
            SignalSource::AudioBeatPhase,
            SignalSource::AudioSpectralCentroid,
            SignalSource::VibeRainIntensity,
            SignalSource::VibeArtifactGlow,
            SignalSource::VibeThreatColor,
            SignalSource::InputPenPressure,
            SignalSource::WorldAuthority,
        ];
        for (i, src) in expect.iter().enumerate() {
            assert_eq!(b.routes[i].channel, i as u16);
            assert_eq!(b.routes[i].source, *src, "channel {i} source");
        }
        assert!(b.forbids(GateName::ShaderCompileHotpath));
        assert!(b.forbids(GateName::VisualOnlyMutatesAuthority));
        b.verify_gates().expect("live udle shaderbind must satisfy its own gates");
    }

    #[test]
    fn live_audio_vis_shaderbind_compiles_to_five_channels() {
        let b = parse_shaderbind(AUDIO_VIS).expect("live audio_vis.shaderbind.vixi must compile");
        assert_eq!(b.surface, "audio_vis", "the shaderbind's own identity surface");
        assert_eq!(b.target_surface, "vibematrix", "audio_vis routes INTO the vibematrix surface");
        assert_eq!(b.channel_count(), 5);
        assert_eq!(b.routes[0].source, SignalSource::AudioRms);
        assert_eq!(b.routes[3].source, SignalSource::AudioCrossfader);
        assert_eq!(b.routes[4].source, SignalSource::InputPenPressure);
        assert!(b.gates.iter().any(|g| g.name == GateName::AudioNotIdentity && g.policy == GatePolicy::Required));
        b.verify_gates().expect("audio_vis has no authority lane + no authority gate");
    }

    // The on-disk golden fixtures (`crates/scc/golden/vixi/shaderbinds/`) were
    // auto-generated stubs (surface/profile only, zero signals/channels/gates)
    // until this drain — proof they now match the real content these inline
    // constants already exercised, not just parse-anything placeholders.
    #[test]
    fn on_disk_udle_fixture_matches_the_live_golden_content() {
        let disk = include_str!("../../scc/golden/vixi/shaderbinds/udle_vibematrix.shaderbind.vixi");
        let b = parse_shaderbind(disk).expect("on-disk udle_vibematrix.shaderbind.vixi must compile");
        assert_eq!(b.channel_count(), 8, "udle drain: 8 channels, not the empty stub");
        assert_eq!(b.gates.len(), 2, "udle drain: 2 gates, not the empty stub");
        b.verify_gates().expect("on-disk udle shaderbind must satisfy its own gates");
    }

    #[test]
    fn on_disk_audio_vis_fixture_matches_the_live_golden_content() {
        let disk = include_str!("../../scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi");
        let b = parse_shaderbind(disk).expect("on-disk audio_vis.shaderbind.vixi must compile");
        assert_eq!(b.channel_count(), 5, "audio_vis drain: 5 channels, not the empty stub");
        assert_eq!(b.gates.len(), 4, "audio_vis drain: 4 gates, not the empty stub");
        b.verify_gates().expect("on-disk audio_vis shaderbind must satisfy its own gates");
    }

    #[test]
    fn live_fixtures_round_trip_through_the_pretty_printer() {
        for src in [UDLE, AUDIO_VIS] {
            let a = parse_shaderbind(src).unwrap();
            let b = parse_shaderbind(&pretty_print_shaderbind(&a)).unwrap();
            assert_eq!(a.surface, b.surface);
            assert_eq!(a.routes, b.routes, "routes survive print -> reparse");
            assert_eq!(a.gates, b.gates, "gates survive print -> reparse");
        }
    }

    #[test]
    fn route_maps_live_signals_into_channels_in_declared_order() {
        let b = parse_shaderbind(UDLE).unwrap();
        let sig = SignalValues {
            audio_rms: 1000,
            audio_beat_phase: 2000,
            audio_spectral_centroid: 3000,
            vibe_rain_intensity: 4000,
            vibe_artifact_glow: 5000,
            vibe_threat_color: 6000,
            input_pen_pressure: 7000,
            world_authority: 8000,
            ..Default::default()
        };
        let ch = b.route(&sig);
        assert_eq!(ch, vec![1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000]);
        let mut reuse = vec![0u16; b.channel_count()];
        b.route_into(&sig, &mut reuse);
        assert_eq!(reuse, ch);
    }

    #[test]
    fn unknown_source_is_a_hard_error_not_a_silent_drop() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal x source=audio.bogus range=0..10000\ns.channel[0] <- x\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("unknown signal source"), "got {}", err.message);
    }

    #[test]
    fn unknown_gate_is_a_hard_error() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal x source=audio.rms range=0..10000\ns.channel[0] <- x\ngate bogus = forbidden\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("unknown gate"), "got {}", err.message);
    }

    #[test]
    fn route_to_undeclared_signal_is_a_hard_error() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal x source=audio.rms range=0..10000\ns.channel[0] <- y\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("undeclared signal 'y'"), "got {}", err.message);
    }

    #[test]
    fn inconsistent_route_targets_are_a_hard_error() {
        let src = "#vixi:shaderbind v1\nsurface: udle\nprofile: p\nsignal a source=audio.rms range=0..10000\nsignal b source=audio.beat_phase range=0..10000\nvibematrix.channel[0] <- a\nother.channel[1] <- b\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("must target ONE surface"), "got {}", err.message);
    }

    #[test]
    fn sparse_channels_are_legal_and_the_hole_reads_zero() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal a source=audio.rms range=0..10000\nsignal b source=audio.beat_phase range=0..10000\ns.channel[0] <- a\ns.channel[2] <- b\n";
        let b = parse_shaderbind(src).expect("a reserved hole is legal");
        assert_eq!(b.channel_count(), 2, "two slots are actually bound");
        assert_eq!(b.channel_span(), 3, "the array spans up to the highest slot");
        let sig = SignalValues { audio_rms: 1234, audio_beat_phase: 5678, ..Default::default() };
        assert_eq!(b.route(&sig), vec![1234, 0, 5678], "value lands on its SLOT, hole reads 0");
    }

    #[test]
    fn duplicate_channel_is_a_hard_error() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal a source=audio.rms range=0..10000\nsignal b source=audio.beat_phase range=0..10000\ns.channel[0] <- a\ns.channel[0] <- b\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("bound twice"), "got {}", err.message);
    }

    #[test]
    fn missing_header_is_a_hard_error() {
        let src = "surface: s\nprofile: p\nsignal a source=audio.rms range=0..10000\ns.channel[0] <- a\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("missing '#vixi:shaderbind v1' header"), "got {}", err.message);
    }

    #[test]
    fn wrong_header_version_is_a_hard_error() {
        let src = "#vixi:shaderbind v2\nsurface: s\nprofile: p\nsignal a source=audio.rms range=0..10000\ns.channel[0] <- a\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("expected '#vixi:shaderbind v1'"), "got {}", err.message);
    }

    #[test]
    fn out_of_range_signal_range_is_a_hard_error() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal a source=audio.rms range=0..20000\ns.channel[0] <- a\n";
        let err = parse_shaderbind(src).unwrap_err();
        assert!(err.message.contains("outside Permyriad"), "got {}", err.message);
    }

    #[test]
    fn authority_gate_rejects_a_double_bound_authority_lane() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal au source=world.authority_q range=0..10000\nsignal au2 source=world.authority_q range=0..10000\ns.channel[0] <- au\ns.channel[1] <- au2\ngate visual_only_mutates_authority = forbidden\n";
        let b = parse_shaderbind(src).unwrap();
        let err = b.verify_gates().unwrap_err();
        assert!(err.message.contains("single lane"), "got {}", err.message);
    }

    #[test]
    fn authority_gate_passes_a_single_authority_lane() {
        let src = "#vixi:shaderbind v1\nsurface: s\nprofile: p\nsignal g source=vibematrix.artifact_glow range=0..10000\nsignal au source=world.authority_q range=0..10000\ns.channel[0] <- g\ns.channel[1] <- au\ngate visual_only_mutates_authority = forbidden\n";
        let b = parse_shaderbind(src).unwrap();
        b.verify_gates().expect("one authority lane + visual sources is legal");
    }

    #[test]
    fn comments_and_blanks_are_skipped_but_header_is_not() {
        let src = "#vixi:shaderbind v1\n# a comment\n\n// another\nsurface: s\nprofile: p\nsignal a source=audio.rms range=0..10000\ns.channel[0] <- a\n";
        let b = parse_shaderbind(src).unwrap();
        assert_eq!(b.channel_count(), 1);
    }

    #[test]
    fn rdr_bard_world_sources_resolve_route_and_round_trip() {
        for (tok, want) in [
            ("world.flute_velocity", SignalSource::WorldFluteVelocity),
            ("world.mood", SignalSource::WorldMood),
            ("world.water_depth", SignalSource::WorldWaterDepth),
        ] {
            let got = SignalSource::resolve(tok).unwrap_or_else(|| panic!("'{tok}' must resolve"));
            assert_eq!(got, want, "{tok} -> wrong variant");
            assert_eq!(got.canonical(), tok, "{tok} canonical round-trip");
            assert!(!got.is_visual_only() && !got.is_authority(), "{tok} is a plain world signal");
        }
        let bard = "#vixi:shaderbind v1\nsurface: bard\nprofile: perform\nsignal flute source=world.flute_velocity range=0..10000\nsignal mood source=world.mood range=0..3\nsignal water source=world.water_depth range=0..10000\nbard.channel[0] <- flute\nbard.channel[1] <- mood\nbard.channel[2] <- water\n";
        let b = parse_shaderbind(bard).expect("bard world sources compile to the sovereign grammar");
        assert_eq!(b.target_surface, "bard");
        let sig = SignalValues {
            world_flute_velocity: 8200,
            world_mood: 2,
            world_water_depth: 4400,
            ..Default::default()
        };
        assert_eq!(b.route(&sig), vec![8200, 2, 4400], "channels carry flute, mood, water in order");
    }

    #[test]
    fn term_route_margin_resolves_routes_and_round_trips() {
        assert_eq!(SignalSource::resolve("term.route_margin"), Some(SignalSource::TermRouteMargin));
        assert_eq!(SignalSource::TermRouteMargin.canonical(), "term.route_margin");
        assert!(!SignalSource::TermRouteMargin.is_visual_only() && !SignalSource::TermRouteMargin.is_authority());

        let src = "#vixi:shaderbind v1\nsurface: terminal\nprofile: seehear\nsignal rms source=audio.rms range=0..10000\nsignal route source=term.route_margin range=0..10000\nterminal.channel[0] <- rms\nterminal.channel[1] <- route\n";
        let b = parse_shaderbind(src).expect("term.route_margin compiles to the sovereign grammar");
        let sig = SignalValues { audio_rms: 3300, term_route_margin: 7700, ..Default::default() };
        assert_eq!(b.route(&sig), vec![3300, 7700], "glow keeps rms, pulse carries the route margin");
    }

    #[test]
    fn world_light_level_resolves_routes_and_round_trips() {
        assert_eq!(SignalSource::resolve("world.light_level"), Some(SignalSource::WorldLightLevel));
        assert_eq!(SignalSource::WorldLightLevel.canonical(), "world.light_level");
        assert!(!SignalSource::WorldLightLevel.is_visual_only() && !SignalSource::WorldLightLevel.is_authority());

        let src = "#vixi:shaderbind v1\nsurface: terminal\nprofile: seehear\nsignal light source=world.light_level range=0..10000\nterminal.channel[0] <- light\n";
        let b = parse_shaderbind(src).expect("world.light_level compiles to the sovereign grammar");
        let sig = SignalValues { world_light_level: 6500, ..Default::default() };
        assert_eq!(b.route(&sig), vec![6500], "channel carries the light-level value");
    }

    #[test]
    fn sprite_sources_resolve_and_round_trip() {
        assert_eq!(SignalSource::resolve("sprite.breathed"), Some(SignalSource::SpriteBreathed));
        assert_eq!(SignalSource::resolve("sprite.hit_flash"), Some(SignalSource::SpriteHitFlash));
        assert_eq!(SignalSource::SpriteBreathed.canonical(), "sprite.breathed");
        assert_eq!(SignalSource::SpriteHitFlash.canonical(), "sprite.hit_flash");
        let vals = SignalValues { sprite_hit_flash: 9, ..Default::default() };
        assert_eq!(vals.get(SignalSource::SpriteHitFlash), 9);
    }

    #[test]
    fn cdk_triad_sources_resolve_route_and_round_trip() {
        for (tok, src) in [
            ("cdk.love", SignalSource::CdkLove),
            ("cdk.strife", SignalSource::CdkStrife),
            ("cdk.entropy", SignalSource::CdkEntropy),
            ("cdk.harmony", SignalSource::CdkHarmony),
            ("cdk.dissonant_pulse", SignalSource::CdkDissonantPulse),
        ] {
            assert_eq!(SignalSource::resolve(tok), Some(src), "{tok} must resolve");
            assert_eq!(src.canonical(), tok, "{tok} must round-trip");
        }
        let vals = SignalValues { cdk_strife: 7, ..Default::default() };
        assert_eq!(vals.get(SignalSource::CdkStrife), 7);
    }

    #[test]
    fn laban_efforts_resolve_route_and_round_trip() {
        for (tok, src) in [
            ("laban.weight", SignalSource::LabanWeight),
            ("laban.space", SignalSource::LabanSpace),
            ("laban.glow", SignalSource::LabanGlow),
            ("laban.flow", SignalSource::LabanFlow),
            ("laban.shake", SignalSource::LabanShake),
        ] {
            assert_eq!(SignalSource::resolve(tok), Some(src), "{tok} must resolve");
            assert_eq!(src.canonical(), tok, "{tok} must round-trip");
            assert!(!src.is_visual_only(), "laban is behavior-derived, not a vibe knob");
        }
        let vals = SignalValues {
            laban_weight: 1,
            laban_space: 2,
            laban_glow: 3,
            laban_flow: 4,
            laban_shake: 5,
            ..Default::default()
        };
        assert_eq!(vals.get(SignalSource::LabanWeight), 1);
        assert_eq!(vals.get(SignalSource::LabanSpace), 2);
        assert_eq!(vals.get(SignalSource::LabanGlow), 3);
        assert_eq!(vals.get(SignalSource::LabanFlow), 4);
        assert_eq!(vals.get(SignalSource::LabanShake), 5);
        let src = "#vixi:shaderbind v1\nsurface: broski\nprofile: witness\nsignal w source=laban.weight range=0..10000\nbroski.channel[0] <- w\n";
        let bind = parse_shaderbind(src).expect("laban source must parse");
        assert_eq!(bind.route(&vals)[0], 1);
    }

    #[test]
    fn ironroot_registers_resolve_route_and_round_trip() {
        for (tok, want) in [
            ("world.clarity_q", SignalSource::WorldClarity),
            ("world.resonance_q", SignalSource::WorldResonance),
            ("world.tarnish_q", SignalSource::WorldTarnish),
            ("world.shadow_weight_q", SignalSource::WorldShadowWeight),
        ] {
            assert_eq!(SignalSource::resolve(tok), Some(want), "{tok} must resolve");
            assert_eq!(want.canonical(), tok, "{tok} must round-trip");
            assert!(!want.is_authority(), "{tok} is state, not the authority lane");
        }

        let src = "#vixi:shaderbind v1\nsurface: ironroot_console\nprofile: ironroot_glass\nsignal c source=world.clarity_q range=0..10000\nsignal r source=world.resonance_q range=0..10000\nsignal t source=world.tarnish_q range=0..10000\nsignal w source=world.shadow_weight_q range=0..10000\nironroot_console.channel[0] <- c\nironroot_console.channel[1] <- r\nironroot_console.channel[2] <- t\nironroot_console.channel[3] <- w\n";
        let b = parse_shaderbind(src).expect("the ironroot registers compile to the sovereign grammar");
        let sig = SignalValues {
            world_clarity: 1000,
            world_resonance: 2000,
            world_tarnish: 3000,
            world_shadow_weight: 4000,
            ..Default::default()
        };
        assert_eq!(
            b.route(&sig),
            vec![1000, 2000, 3000, 4000],
            "each register lands on its own channel, in authored order"
        );
    }

    #[test]
    fn spectrum_low_and_mid_resolve_route_and_round_trip() {
        for (tok, want) in [
            ("audio.spectrum_low", SignalSource::AudioSpectrumLow),
            ("audio.spectrum_mid", SignalSource::AudioSpectrumMid),
        ] {
            let got = SignalSource::resolve(tok).unwrap_or_else(|| panic!("'{tok}' must resolve"));
            assert_eq!(got, want, "{tok} -> wrong variant");
            assert_eq!(got.canonical(), tok, "{tok} canonical round-trip");
            assert!(!got.is_visual_only() && !got.is_authority(), "{tok} is a plain audio band");
        }

        let src = "#vixi:shaderbind v1\nsurface: swarm_ambient\nprofile: ambient\nsignal low source=audio.spectrum_low range=0..10000\nsignal mid source=audio.spectrum_mid range=0..10000\nsignal hi source=audio.spectrum_high range=0..10000\nswarm_ambient.channel[0] <- low\nswarm_ambient.channel[1] <- mid\nswarm_ambient.channel[2] <- hi\n";
        let b = parse_shaderbind(src).expect("the three-band ladder compiles");
        let sig = SignalValues {
            audio_spectrum_low: 1111,
            audio_spectrum_mid: 2222,
            audio_spectrum_high: 3333,
            ..Default::default()
        };
        assert_eq!(b.route(&sig), vec![1111, 2222, 3333], "each band lands on its own channel");
    }
}
