//! Aspire (gauges 2026-07-20, widened 07-29): fold backlog — 17 OFF-skills, 9 ᐫ verbs
//! + 8 ᐬ hybrid-halves — plus ᐭ frontend-CI QA/QC loop, ᐯ wormholes, ᑫ CDK.
//! NOW/NEXT/LATER/HORIZON/EDGE, roi-tagged, each mapped to its fold target and to
//! the lateral reach that produced it (Sean 07-29: the reach IS the generator).

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::latent_synthesis::{Synthesis, SYNTHESES};

/// The lateral carrier — the far domain a row was projected FROM, and what the
/// crossing costs and buys. A row without a `mechanism` is aimed, not synthesised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reach {
    /// The orthogonal field reached into (paper, crate, discipline). "" = unrecorded.
    pub domain: &'static str,
    /// HOW — algorithm or coordinates, never a metaphor. "" = unrecorded.
    pub mechanism: &'static str,
    /// WHAT — the capability that exists only once it lands. "" = unrecorded.
    pub impact: &'static str,
}

/// Rows generated before the reach was carried in the type (07-29). Not a verdict:
/// the row stands, its crossing was simply never written down. Gauged, never hidden.
pub const UNSOURCED: Reach = Reach { domain: "", mechanism: "", impact: "" };

impl Reach {
    /// Aimed at a domain but with no body written.
    pub const fn aimed(domain: &'static str) -> Self {
        Reach { domain, mechanism: "", impact: "" }
    }

    /// A wormhole row borrows its body from its SoT — pointer, never a copy.
    pub const fn from_synthesis(s: &'static Synthesis) -> Self {
        Reach { domain: s.from, mechanism: s.mechanism, impact: s.impact }
    }

    /// Full crossing recorded: domain reached, mechanism stated, impact named.
    pub const fn is_sourced(&self) -> bool {
        !self.mechanism.is_empty() && !self.impact.is_empty() && !self.domain.is_empty()
    }
}

/// Triage tier — what a row is waiting on, which is not the same question as roi.
/// `'1'` wire-in-place (the parts are already compiled) · `'2'` priced buy (an
/// absent crate/paper, pull_gate receipt owed) · `'3'` spec owed (no body yet).
/// `'·'` = ungauged, rows generated before the gauge was carried in the type.
pub const UNGAUGED_TRIAGE: char = '·';

/// One look-ahead candidate: organ glyph, bucket, roi, skill, fold target, reach,
/// plus the two gauges every run must print — estimated LoC and triage tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aspirant {
    /// The organ category glyph (e.g., ᐫ verb, ᐬ hybrid, ᐭ QA organ).
    pub glyph: &'static str,
    /// Time horizon: NOW, NEXT, LATER, HORIZON, EDGE.
    pub bucket: &'static str,
    /// Return-on-investment tier: 'H' high, 'M' medium, 'L' low, 'E' experimental.
    pub roi: char,
    /// Skill name from the forge tooling or domain.
    pub skill: &'static str,
    /// Concrete deliverable target when landed.
    pub target: &'static str,
    /// Lateral domain this row was synthesised from, with mechanism and impact stated.
    pub reach: Reach,
    /// Estimated lines to land the row. `0` = ungauged, never "free".
    pub loc: u32,
    /// Triage tier, see [`UNGAUGED_TRIAGE`].
    pub triage: char,
}

impl Aspirant {
    /// A row carries its full gauge only with both an LoC estimate and a tier.
    pub const fn is_gauged(&self) -> bool {
        self.loc > 0 && self.triage != UNGAUGED_TRIAGE
    }
}

/// Row constructor — keeps the table one line per candidate. Ungauged by
/// construction: pre-gauge rows stay honest rather than inheriting a guessed LoC.
const fn a(
    glyph: &'static str,
    bucket: &'static str,
    roi: char,
    skill: &'static str,
    target: &'static str,
    reach: Reach,
) -> Aspirant {
    Aspirant { glyph, bucket, roi, skill, target, reach, loc: 0, triage: UNGAUGED_TRIAGE }
}

/// Gauged row constructor — every row from a run carrying the ROI/LoC/Triage
/// contract lands through this one.
#[allow(clippy::too_many_arguments)]
const fn ag(
    glyph: &'static str,
    bucket: &'static str,
    roi: char,
    loc: u32,
    triage: char,
    skill: &'static str,
    target: &'static str,
    reach: Reach,
) -> Aspirant {
    Aspirant { glyph, bucket, roi, skill, target, reach, loc, triage }
}

/// ᐫ = clean forge.exe verb. ᐬ = hybrid (fold determ. half, keep thin LLM front).
/// ᐭ = frontend-CI QA/QC organ (slapp+forgeVision+vix+xtask), aspired 2026-07-20.
pub const ASPIRE: &[Aspirant] = &[
    a("ᐫ", "NOW", 'H', "audio-engineer", "forge audio doctor (forge-audio + arena_test)", UNSOURCED), // LANDED 07-21: forge-audio/src/doctor.rs (LLM review half stays skill-side)
    a("ᐫ", "NOW", 'H', "signal-condense", "forge condense (pure md max-density)", UNSOURCED), // LANDED 07-21: cargo xtask condense (xtask/src/condense.rs)
    a("ᐫ", "NOW", 'H', "stink-worker", "cargo xtask lint (Gates A-D crate crawl)", UNSOURCED), // LANDED 07-21: xtask/src/lint.rs
    a("ᐫ", "NOW", 'H', "handoff", "forge pkm (forge-pkm amortize/archive)", UNSOURCED), // LANDED 07-21: forge-cli pkm arm; twin standalone bin residue=FLAG
    a("ᐫ", "NEXT", 'M', "build-door", "cargo xtask build-door (cargo + audit-gate)", UNSOURCED),
    a("ᐫ", "NEXT", 'M', "corpse-assimilator", "forge vixi assimilate (forge-vix harvest+gate)", UNSOURCED),
    a("ᐫ", "NEXT", 'M', "youtube-forge", "forge devlog (yt-dlp/whisper/ffmpeg_mux)", Reach::aimed("yt-dlp / whisper / ffmpeg")),
    a("ᐫ", "LATER", 'M', "cloudflare", "studio publish + forge site deploy [ext:wrangler]", Reach::aimed("Cloudflare wrangler")),
    a("ᐫ", "LATER", 'M', "stripe-projects", "forge stripe provision [ext:stripe-cli]", Reach::aimed("stripe-cli")),
    a("ᐬ", "NOW", 'H', "realign", "MEASURE/HALT gate (slapp read_gauge) | LLM re-anchor", UNSOURCED),
    a("ᐬ", "NOW", 'H', "invention_pricing_oracle", "price data-table | LLM judgment", UNSOURCED), // LANDED 07-21 (determ. half): src/pricing.rs -> seed::full_atlas
    a("ᐬ", "NEXT", 'M', "white-paper", "weasyprint render | LLM research", Reach::aimed("weasyprint")),
    a("ᐬ", "NEXT", 'M', "chapter-editor", "wkhtmltopdf+voice_lint+slapp recording | LLM editorial", Reach::aimed("wkhtmltopdf")),
    a("ᐬ", "NEXT", 'M', "press-triage", "triage+ledger | LLM reply", UNSOURCED),
    a("ᐬ", "LATER", 'L', "ccg-architect", "economy/rarity tables | LLM card design", UNSOURCED),
    a("ᐬ", "LATER", 'L', "stripe-directory", "lookup | LLM purchase decision", UNSOURCED),
    a("ᐬ", "LATER", 'L', "rundevrun", "SKILL→API→MCP growing edge", UNSOURCED),
    a("ᐬ", "NEXT", 'H', "fae-collider", "triage FAE moral/consequence mappings | LLM lateral weights", UNSOURCED),
    a("ᐭ", "NOW", 'H', "machine-eyes-gate", "forge qa eyes (vision_capture→PNG→verdict loop)", UNSOURCED), // LANDED pre-07-21 (forge-cli qa eyes/diff); verified 07-21
    a("ᐭ", "NOW", 'H', "pixel-diff-golden", "NEW: odiff gate (xtask qa-diff + _qa/golden)", Reach::aimed("odiff (perceptual image diff)")),
    a("ᐭ", "NOW", 'H', "board-ci-stage", "cargo xtask board --emit-hooks as the CI gate", UNSOURCED), // LANDED 07-21: xtask/src/main.rs gate_json + .forge/hooks/board-gate.json
    a("ᐭ", "NEXT", 'H', "headless-drive", "NEW: tauri-driver e2e over det-clock tick replay", Reach::aimed("tauri-driver / WebDriver")),
    a("ᐭ", "NEXT", 'M', "cst-snapshots", "NEW: cargo-insta over forge vixi check CST", Reach::aimed("cargo-insta snapshot testing")),
    a("ᐭ", "NEXT", 'M', "a11y-audit", "NEW: axe-core in-WebView JSON gate (WCAG)", Reach::aimed("axe-core / WCAG")),
    a("ᐭ", "NEXT", 'M', "terminal-readback", "ghost_words_pixels pattern → slapp scan_terminal", UNSOURCED),
    a("ᐭ", "LATER", 'M', "perf-gate", "NEW: criterion bench vs history (xtask perf-gate)", Reach::aimed("criterion")),
    a("ᐭ", "LATER", 'M', "mutation-audit", "NEW: cargo-mutants over board-harvested tests", Reach::aimed("cargo-mutants (mutation testing)")),
    a("ᐭ", "LATER", 'M', "grammar-fuzz", "NEW: cargo-fuzz on forge-vix parser", Reach::aimed("cargo-fuzz / libFuzzer")),
    a("ᐭ", "HORIZON", 'M', "vrt-bless", "NEW: golden bless verb, Sean-gated baselines", Reach::aimed("visual regression testing")),
    a("ᐭ", "HORIZON", 'L', "vlm-critic", "NEW: local VLM capture-vs-DESIGN rubric score", Reach::aimed("local VLM rubric scoring")),
    a("ᐭ", "EDGE", 'E', "design-quality-lane", "membrane: critic score→u8 board gate column", UNSOURCED),
    // ᐯ LATENT WORMHOLES (Sean 2026-07-27) — each a measured 5D trajectory between
    // two organs that already exist; the fold target is the layer BETWEEN them.
    // Bodies: latent_synthesis::SYNTHESES (borrowed below, never copied).
    a("ᐯ", "NOW", 'H', "acoustic-calligraphy", "z212992→147456: .kit.vixi strokes → dream_channel SDF (kill the DOM/callback layer)", Reach::from_synthesis(&SYNTHESES[0])),
    a("ᐯ", "NOW", 'H', "replay-phase-lock", "z98304→229376: dream_journal_query θ-scrub → dream_wire PLL over TripleBuffer", Reach::from_synthesis(&SYNTHESES[1])),
    a("ᐯ", "NEXT", 'H', "self-heal-wormhole", "z114688→180224→1: spec lexicon → airgap 5D match → tractor-pull → cargo gate", Reach::from_synthesis(&SYNTHESES[2])),
    // ᑫ COSMIC DISSONANCE KERNEL (Sean 2026-07-28, /aspire run n=1): canon =
    // _book/17-cosmic-dissonance-kernel.md (superset 07-28) + forge_core::dissonance_sieve.
    // 14/15 survived; confab dropped+recorded: web_rtb (claimed lock-free ring buffer,
    // canon §4) — nearest real: rtrb. Split 29% interior / 71% exterior.
    a("ᑫ", "NOW", 'H', "cdk-resolver", "resolve_cosmic_conflict port (canon §6, authority_q 750..2000) → forge_core::dissonance_sieve", Reach::aimed("_book/17 canon §6")),
    a("ᑫ", "NOW", 'H', "cdk-euclid", "NEW: forge-harmonics::euclid (Toussaint 2005, Bjorklund) — Yod/Quincunx pulse spacing", Reach::aimed("Toussaint 2005 / Bjorklund")),
    a("ᑫ", "NOW", 'M', "cdk-harmonic-body", "HarmonicBody tier table (40/432/inv/800Hz) → ironroot::resonance_constants (its never-hardcode law)", Reach::aimed("_book/17 canon HarmonicBody")),
    a("ᑫ", "NOW", 'M', "cdk-hitstop", "NEW: forge-game-systems::hitstop (Swink 2009) — Nigredo mass→frames inside the 120Hz metronome", Reach::aimed("Swink 2009, Game Feel")),
    a("ᑫ", "NEXT", 'H', "cdk-sethares", "NEW: forge-audio::sethares (Sethares 1993) — dissonance_pressure from spectral roughness", Reach::aimed("Sethares 1993")),
    a("ᑫ", "NEXT", 'H', "cdk-faction-stimulus", "DissonanceVerdict → forge_game_systems::faction_mind stimulus axis delta", UNSOURCED),
    a("ᑫ", "NEXT", 'M', "cdk-soulword-pack", "base-243 5-trit/byte ElementQualities → outland::SoulWord + repo_query::trit_hamming_sheet", Reach::aimed("balanced-ternary packing")),
    a("ᑫ", "NEXT", 'M', "cdk-balanced-ternary", "NEW: pp-math::balanced_ternary (Knuth TAOCP v2 §4.1)", Reach::aimed("Knuth TAOCP v2 §4.1")),
    a("ᑫ", "NEXT", 'M', "cdk-monzo", "NEW: forge-harmonics::monzo (Tenney harmonic distance) — canon 5D axis-1", Reach::aimed("Tenney harmonic distance")),
    a("ᑫ", "NEXT", 'M', "cdk-stego-witness", "NEW: forge-stego::spread_spectrum (Cox et al. 1997) — RoutedUmp payload into PCM", Reach::aimed("Cox et al. 1997")),
    a("ᑫ", "LATER", 'M', "cdk-phase-lock", "NEW: forge-audio::phase_lock (Laroche-Dolson 1999) — Albedo reflection alignment", Reach::aimed("Laroche-Dolson 1999")),
    a("ᑫ", "LATER", 'L', "cdk-roughness-lut", "NEW: forge-audio::roughness_lut (Plomp-Levelt 1965) — permyriad integer LUT", Reach::aimed("Plomp-Levelt 1965")),
    a("ᑫ", "LATER", 'L', "cdk-echo-hiding", "NEW: forge-stego::echo_hiding (Gruhl/Bender 1996) — low-bitrate second stego lane", Reach::aimed("Gruhl/Bender 1996")),
    a("ᑫ", "HORIZON", 'L', "cdk-ambisonic", "NEW: forge-audio::ambisonic_b (Gerzon 1973 B-format) — resonance injection as a field", Reach::aimed("Gerzon 1973, B-format")),
    // ᓭ EMIT / PORTRAYAL (Sean 2026-07-29, /aspire run n=1): the vixi→HTML5 emitter
    // seed + the site's no-JS front + the three orphan permyriad channels. Reach =
    // cartographic portrayal (ISO 19117 feature-vs-portrayal, classed intervals,
    // Töpfer's radical law). 15/15 survived; 5 confabs recorded from the specs that
    // provoked the run (CosmicVibeUniforms→vibe_uber_pass.rs:42 VibeUniforms ·
    // calcify_glow→calcify_q · density_pmy-as-uniform→unified.rs:1436 · XorInteractionGate
    // and vibe-v2.js→none). Split 27% interior / 73% exterior. First gauged run.
    ag("ᓭ", "NOW", 'H', 40, '1', "portrayal-slot-resolve", "emit_html.rs:48 token_shade hash → HtmlPalette 8-slot resolve (html_lower.rs:18)", Reach::aimed("ISO 19117 portrayal catalogue")),
    ag("ᓭ", "NOW", 'H', 90, '1', "emit-text-runs", "emit_html.rs:26: DrawCmd text runs → <span> + FACE_TABLE faces", Reach::aimed("ISO 19117 portrayal catalogue")),
    ag("ᓭ", "NOW", 'H', 25, '1', "trit-validity-gate", "dar.rs:117 balance_trit → checked Option accessor, honours the :120 flag", Reach::aimed("SQL three-valued logic / NULL validity")),
    ag("ᓭ", "NOW", 'H', 60, '1', "html5ever-parity-oracle", "WIRE Cargo.lock:5576 — parse emit_html output, assert DOM order == draws order", Reach::aimed("multiple-representation DB consistency")),
    ag("ᓭ", "NOW", 'M', 70, '1', "form-urlencoded-receiver", "WIRE Cargo.lock:4587 — serve the no-JS contact form offline (public/contact.html)", Reach::aimed("RFC 1866 §8.2.1 form-urlencoded")),
    ag("ᓭ", "NEXT", 'M', 80, '3', "calcify-q-channel-wire", "ARCH-004-creation-surface.md:125 calcify_q permyriad → TokenStatus, both emitters", Reach::aimed("dasymetric attribute overlay")),
    ag("ᓭ", "NEXT", 'H', 120, '2', "fisher-jenks-classing", "NEW: integer natural-breaks over a permyriad histogram — calcify_q / density_pmy", Reach::aimed("Fisher-Jenks classed choropleth")),
    ag("ᓭ", "NEXT", 'M', 150, '1', "selectors-cssparser-cascade", "WIRE Cargo.lock:9588 + :1934 — real specificity in html_lower's cascade", Reach::aimed("CSS Cascade Level 5 specificity")),
    ag("ᓭ", "NEXT", 'M', 55, '2', "topfer-graphic-load", "NEW: element budget n_target = n_source·sqrt(scale) as the ≤4±1 assert", Reach::aimed("Töpfer's radical law")),
    ag("ᓭ", "LATER", 'M', 200, '2', "portrayal-catalogue", "NEW: forge-vix portrayal registry — feature code held apart from symbol", Reach::aimed("ISO 19117 portrayal catalogue")),
    ag("ᓭ", "LATER", 'M', 110, '2', "smallest-visible-object", "NEW: raster-mode generalization filter — one kit, phone + desktop, no media queries", Reach::aimed("Li-Openshaw 1992")),
    ag("ᓭ", "LATER", 'L', 30, '2', "prepress-trap-hairline", "NEW: expand adjacent slot edges 1 MilliUnit before px truncation", Reach::aimed("offset-press trap rule")),
    ag("ᓭ", "LATER", 'L', 45, '2', "minify-byte-floor", "NEW: deterministic post-emit token minify — the 20k-page Pages cap", Reach::aimed("HTML5 §13 tokenizer whitespace elision")),
    ag("ᓭ", "HORIZON", 'M', 260, '2', "braille-portrayal-target", "NEW: emit_braille — same LoweredUi → BRF cells, tactile catalogue", Reach::aimed("tactile cartography / BRF")),
    ag("ᓭ", "HORIZON", 'L', 180, '2', "postscript-path-emitter", "NEW: emit_ps — draws → PostScript path ops, integer coords only", Reach::aimed("PostScript imaging model")),
    // ᔨ HEAR / 5D SPATIAL AUDIO (Sean 2026-07-29 "this is 5D Spatial Audio", /aspire n=1).
    // REACH = array signal processing + ambisonics. The repo calls shaderbind_dsl an
    // N→4 reducer, which is the language of throwing information away. Interferometry
    // calls the same operation a BEAMFORMER: N sensor measures projected onto a steering
    // basis, where the reduction is a change of coordinates and the cross-terms between
    // measures — never computed here — carry direction of arrival. Ambisonics closes it:
    // first-order B-format is EXACTLY four channels, W omni + XYZ directional, so the
    // frozen bus is not four unrelated scalars, it is dimensionally a field with an
    // orientation. That is the 5D: three space axes + time + spectrum riding four f32
    // the shaders already read. Rotation, steering and DOA then live host-side as linear
    // algebra on the reducer's output, so the freeze is not a ceiling at all — it is a
    // coordinate system nobody had named. rustfft/realfft/num-complex/glam are already
    // compiled (Cargo.lock:9279/8906/7305/5102), so most of the crossing is t1.
    ag("ᔨ", "NOW", 'H', 70, '1', "bformat-reinterpret", "shaderbind_dsl.rs VIBE_BUS_LANES → W/X/Y/Z first-order B-format alias, same 4 f32", Reach { domain: "Gerzon 1973 first-order ambisonics", mechanism: "alias glow=W (omni energy), chromatic/shake/pulse=X/Y/Z; values stay permyriad, only the interpretation is declared", impact: "the bus carries WHERE a sound is, not just how loud — orientation for free, zero shader bytes changed" }),
    ag("ᔨ", "NOW", 'H', 55, '1', "bformat-rotate-host", "WIRE Cargo.lock:5102 glam Mat3 — rotate XYZ lanes before upload, listener-relative field", Reach { domain: "ambisonic B-format rotation", mechanism: "3x3 rotation on the three directional lanes at reduce time; W is rotation-invariant so the omni lane is untouched", impact: "turning the camera turns the sound field with it, one Mat3 multiply, no pipeline rebind" }),
    ag("ᔨ", "NOW", 'H', 90, '1', "gerzon-energy-vector", "forge-audio: spectrum_bands[7] → Gerzon energy vector, the honest XYZ producer", Reach { domain: "Gerzon energy/velocity localisation vectors", mechanism: "per-band energy weighted by band azimuth, summed as a 3-vector, magnitude = directional confidence", impact: "XYZ lanes get a real physical source instead of a hand-tuned mapping row" }),
    ag("ᔨ", "NOW", 'M', 45, '1', "covariance-channel", "shaderbind_dsl ChannelRoute over a PAIR of sources, not one", Reach { domain: "array covariance matrix / interferometric visibility", mechanism: "route value = integer product of two normalised sources, the off-diagonal term the reducer never forms today", impact: "correlation between measures becomes routable — the cross-terms stop being invisible" }),
    ag("ᔨ", "NEXT", 'H', 160, '1', "music-doa-steer", "WIRE Cargo.lock:9279 rustfft — MUSIC pseudo-spectrum → XYZ steering", Reach { domain: "Schmidt 1986 MUSIC algorithm", mechanism: "eigendecompose the covariance of the band vector, project onto the noise subspace, peak of 1/|a*En| gives azimuth", impact: "direction of arrival from the existing 7-band spectrum, no extra microphone and no new uniform" }),
    ag("ᔨ", "NEXT", 'H', 130, '2', "srp-phat-whiten", "NEW: steered-response power with phase transform over the band vector", Reach { domain: "DiBiase 2000 SRP-PHAT", mechanism: "whiten each band to unit magnitude before steering so loud bands cannot dominate the direction estimate", impact: "direction stays stable under a bass drop — the failure mode every naive RMS mapping has" }),
    ag("ᔨ", "NEXT", 'H', 120, '2', "gammatone-cochlea", "NEW: forge-audio gammatone filterbank replacing the linear 7-band split", Reach { domain: "Patterson 1992 auditory filterbank / ERB scale", mechanism: "cascade of 4th-order gammatone IIR sections on ERB-spaced centres, integer biquad state", impact: "bands land where hearing actually resolves, so the same 4 lanes read musical instead of arithmetic" }),
    ag("ᔨ", "NEXT", 'M', 85, '2', "zwicker-mask-gate", "NEW: psychoacoustic masking as a GatePolicy on a channel route", Reach { domain: "Zwicker spreading function / ISO 226", mechanism: "per-band mask threshold from neighbouring band energy; a route below its mask emits zero rather than noise", impact: "channels stop reacting to sound a listener cannot hear — silence that is actually silent" }),
    ag("ᔨ", "NEXT", 'M', 100, '1', "itd-precedence-lane", "WIRE Cargo.lock:8906 realfft — inter-channel phase → precedence-weighted azimuth", Reach { domain: "Wallach 1949 precedence effect / ITD", mechanism: "cross-correlate deck A/B via FFT, first-arrival peak within 40ms wins the azimuth, later reflections attenuated", impact: "a reflection-heavy room stops smearing the direction lanes" }),
    ag("ᔨ", "LATER", 'H', 190, '2', "second-order-collapse", "NEW: 9-channel 2nd-order ambisonic decode collapsed to 4 by energy projection", Reach { domain: "Daniel 2001 higher-order ambisonics", mechanism: "compute order-2 spherical harmonic coefficients host-side, project onto the order-1 basis by max-energy before upload", impact: "sharper directional resolution reaching pixels through an unchanged 4-lane bus — the freeze scales" }),
    ag("ᔨ", "LATER", 'M', 140, '2', "rir-schroeder-tail", "NEW: room impulse response tail as a decaying channel envelope", Reach { domain: "Schroeder 1962 artificial reverberation", mechanism: "comb+allpass cascade on the reduced channel value, T60 taken from the zone's own geometry", impact: "the vibe field inherits the room, so a cavern reads different from a corridor with no new signal" }),
    ag("ᔨ", "LATER", 'M', 170, '2', "nmf-stem-split", "NEW: non-negative matrix factorisation over band history → per-stem routes", Reach { domain: "Lee-Seung 1999 NMF source separation", mechanism: "factor the band-by-time magnitude matrix into k basis spectra and activations, multiplicative integer updates", impact: "drums and vocals can drive different lanes from one mixed input, no stems required" }),
    ag("ᔨ", "LATER", 'M', 75, '2', "tonnetz-hue-route", "NEW: chroma vector → Tonnetz position → hue channel", Reach { domain: "Euler/Riemann Tonnetz, chroma circle", mechanism: "fold spectrum to 12 pitch classes, place on the torus, angle drives hue and radius drives saturation", impact: "harmony becomes colour rather than brightness — the one musical axis the 4 lanes cannot express today" }),
    ag("ᔨ", "HORIZON", 'M', 220, '2', "wfs-rayleigh-field", "NEW: wave field synthesis driving a per-vixel phase field", Reach { domain: "Berkhout 1993 wave field synthesis / Rayleigh integral", mechanism: "treat each vixel as a secondary source, per-vixel delay from the Rayleigh I integral, phase carried in the existing matrix lane", impact: "a wavefront that moves ACROSS the canvas instead of the whole surface pulsing together" }),
    ag("ᔨ", "HORIZON", 'L', 110, '2', "doppler-velocity-lane", "NEW: source velocity from inter-frame phase drift", Reach { domain: "Doppler shift estimation", mechanism: "unwrap per-band phase across frames, df/dt over centre frequency gives radial velocity, sign gives approach or recede", impact: "approaching and receding sources look different — the fifth dimension the bus has no producer for yet" }),

    // ── ᒥ SEE — the launcher as INSTRUMENT (2026-07-30, ceiling 2500 LOC) ────────
    // REACH: the planispheric astrolabe, 11th-13th century Andalusian and Mamluk.
    // The field this organ has no vocabulary for is INSTRUMENT-MAKING: an astrolabe
    // is not a picture of the sky, it is three pierced brass plates that rotate
    // against each other, and the reading is the ALIGNMENT BETWEEN LAYERS rather
    // than any mark on one layer. The rete (pierced star map) turns over a tympan
    // (the stereographic projection of one latitude) seated in the mater. What the
    // instrument-makers knew that this compositor does not: depth and state can be
    // carried by REGISTRATION — two sparse layers offset by a known angle say more
    // than one dense layer ever can, and the eye reads the offset preattentively.
    // The isomorphism is already compiled: `triple_loop::{WorldBridge, OverlayBridge}`
    // publish independent planes to one composite, and `LayerPlane` is literally a
    // tympan slot. The launcher today paints ONE flat layer and throws that away.
    // Louis XIV's Romain du Roi (1692) closes the loop from the other side: it was
    // drawn on a 16x16 module grid before being cut, which is exactly `emit_layout5d`'s
    // tile lattice — the first grid-derived typeface and this engine's layout share
    // a construction, three centuries apart.
    ag("ᒥ", "NOW", 'H', 120, '1', "rete-tympan-split", "launcher.kit.vixi → two planes: sparse rete (cards+wordmark) over a tympan ground", Reach { domain: "planispheric astrolabe rete/tympan registration", mechanism: "publish the nav rail on OverlayBridge and the ground on WorldBridge, composite with a fixed parallax offset in tile units", impact: "the front door gains real depth from layers already allocated, instead of one flat DrawList" }),
    ag("ᒥ", "NOW", 'H', 60, '1', "vibe-mask-unhardcode", "canvas_quad.wgsl:511 apply_vibe_matrix(base,0x0Fu) → the instance's own vibe_mask", Reach { domain: "measured pixel readback vs authored token", mechanism: "the untextured branch passes a hardcoded full mask, lifting every chrome quad ~+0x2E; read packed_flags instead", impact: "authored colour finally survives to glass — ground #382E26 returns to its token #0A0705" }),
    ag("ᒥ", "NOW", 'H', 140, '1', "composer-multiatlas", "ComposeFrame atlas: FontAtlas → MultiAtlas, the whole ramp on one live surface", Reach { domain: "Romain du Roi 1692 grid-cut type ladder", mechanism: "upload the 6 ramp atlases as an array, index by DrawList::text_face ordinal already recorded per Text cmd", impact: "the boot door reaches Display+Body at once — the wordmark gets Cormorant while labels stay readable" }),
    ag("ᒥ", "NOW", 'M', 80, '1', "radial-ambient-ground", "WIRE Cargo.lock:11162 tiny-skia — one radial gradient behind the focal, not a full-height ramp", Reach { domain: "Baroque copperplate plate-tone / chiaroscuro", mechanism: "bake a single radial alpha ramp to a small texture once, blit scaled behind the rail as an Image quad", impact: "ambient depth without the linear GradientRect that washed the door mud on its first capture" }),
    // REJECTED at price (Sean 2026-07-30 "the Rust Text Rendering Iceberg"): the
    // first cut of this row wired swash+rustybuzz for shaping. In-lock is not the
    // whole cost — that stack is ~50k transitive LOC of alloc-heavy, f32 Bezier and
    // a HarfBuzz port, against `alloc_steady = forbidden` + `float_in_ir = forbidden`.
    // And it would have bought nothing: the space collapse was a FONT-CHOICE error
    // (an Arabic-first face doing Latin chrome), not a shaping gap — Reem Kufi's
    // U+0020 advance is correct in its own hmtx. Shaping the wrong font correctly is
    // still the wrong font. What survives is the cold-clock half of the same idea.
    ag("ᒥ", "NEXT", 'H', 120, '1', "cold-clock-metrics-bake", "ttf-parser hmtx → integer u16 metrics table baked at atlas build, never at tick", Reach { domain: "MSDF/bitmap atlas pipelines; ttf-parser is no_std zero-alloc", mechanism: "two clocks already split this: parse+rasterise on the COLD lane beside MultiAtlas::from_ramp, publish an integer advance/UV table, T3 blit is an array lookup", impact: "correct per-face metrics with 0 steady alloc and 0 CPU float — the gates hold because the float boundary stays at the atlas, where text.rs already put it" }),
    ag("ᒥ", "NEXT", 'H', 150, '2', "square-kufic-sigil", "NEW: square-Kufic lattice generator for the corner sigil slot", Reach { domain: "12th-c foliated/square Kufic architectural frieze", mechanism: "rasterise a word onto an NxN binary lattice under stroke-width-1 rules, emit as a stencil bit-plane", impact: "the sigil_corner slot gets a real generated mark tied to the surface's name, not an imported glyph" }),
    ag("ᒥ", "NEXT", 'M', 110, '2', "stereographic-nav", "NEW: stereographic projection as the nav layout, doors on an azimuth ring", Reach { domain: "al-Sufi Book of Fixed Stars / astrolabe plate projection", mechanism: "place the four doors by azimuth on a projected circle, integer polar-to-tile mapping in the lowerer", impact: "adding a fifth door re-seats the ring instead of squeezing a row — the layout scales by construction" }),
    ag("ᒥ", "NEXT", 'M', 95, '1', "glow-sdf-single-pass", "push_bloom 6-ring stack → one SDF falloff quad", Reach { domain: "signed-distance-field antialiasing", mechanism: "one oversized quad, alpha = smoothstep over the rounded-box SDF already in canvas_quad.wgsl:439", impact: "smooth bloom at 1 draw instead of 6, and it matches the reference gaussian rather than banding" }),
    ag("ᒥ", "NEXT", 'M', 70, '1', "shadow-ground-derived", "ShadowRect colour = ground darkened, never the body fill", Reach { domain: "cast-shadow photometry", mechanism: "sample BgVoid, multiply luminance by ~0.4, use that as the penumbra colour with the body's radius", impact: "restores the card lift that was reverted for painting light slabs under every matte card" }),
    ag("ᒥ", "LATER", 'H', 200, '2', "brass-engrave-material", "NEW: engraved-brass PanelMaterial — anisotropic ring highlight", Reach { domain: "Mamluk astrolabe brass turning marks", mechanism: "anisotropic specular along concentric UV rings plus a fine burin-line noise octave, one apply_* fn beside apply_gunmetal", impact: "the instrument reads as a made object rather than a coloured rectangle" }),
    ag("ᒥ", "LATER", 'M', 130, '1', "launcher-shaderbind", "NEW: launcher.shaderbind.vixi — the front door breathes on the audio bus", Reach { domain: "the repo's own shaderbind DSL, unused by this surface", mechanism: "signal audio.rms → surface channel[0], route to the focal card's vibe lane through ShaderBind::route", impact: "the door's heat answers sound instead of sitting static — tier-1/2/3 finally reaches this surface" }),
    ag("ᒥ", "LATER", 'M', 90, '2', "tracking-allcaps-title", "NEW: authored letter-spacing for the Display stop", Reach { domain: "17th-c French copperplate title tracking", mechanism: "carry a tracking permyriad on the text slot, add it to each advance at shaping time", impact: "Cormorant all-caps gets its royal-chart title bar; today every face draws at native tracking" }),
    ag("ᒥ", "LATER", 'L', 60, '2', "fleur-corner-rule", "NEW: Baroque corner rule generator for panel frames", Reach { domain: "Jannon/Sedan printers' fleurons and corner ornament", mechanism: "parametric quadratic-curve ornament emitted as Line cmds at the four corners of a chrome region", impact: "frames gain period ornament from geometry rather than an imported image asset" }),
    ag("ᒥ", "HORIZON", 'M', 240, '2', "rotating-rete-state", "NEW: layer ROTATION as the state readout — the rete turns with session state", Reach { domain: "astrolabe rete rotation = time-of-night reading", mechanism: "map board frontier depth to a rete angle, rotate the overlay plane by that angle at composite", impact: "session state becomes readable as an alignment between layers, the astrolabe's whole idea" }),
    ag("ᒥ", "HORIZON", 'L', 180, '2', "latitude-tympan-swap", "NEW: profile swap as a tympan change — one instrument, interchangeable plates", Reach { domain: "astrolabe interchangeable latitude plates", mechanism: "each .profile.sheet.vixi becomes a tympan; swapping re-seats the ground plane without touching the rete", impact: "theme swap stops being a token recolour and becomes a physical plate change with its own motion" }),

    // ── ᓇ SEE — the launcher COMPOSITION (Sean 07-30 "about 45% of where it needs
    // to be"). ᒥ aspired the instrument; this run aspires the PAGE.
    // REACH: medieval mise-en-page — the illuminated manuscript's layout discipline.
    // The problem the wireframe has is not missing features, it is that two thirds of
    // the surface is empty and the emptiness reads as unfinished rather than composed.
    // Manuscript scribes solved exactly this: a codex page is mostly blank vellum and
    // still reads as authoritative, because the void is RULED — margins set by ratio
    // (the Van de Graaf canon puts the text block on ninths, so the outer margin is
    // twice the inner), entry points rubricated in a second colour, secondary channels
    // pushed to the margin as gloss instead of a footer, and a catchword in the corner
    // pointing at what comes next. What scribes knew that this layout engine does not:
    // emptiness is a POSITIONED element with a ratio, never leftover space — which is
    // the same claim the repo's own Ma law makes and the launcher does not yet honour.
    // The lattice is already there: `emit_layout5d` rules the page in tiles exactly as
    // a scribe pricked and ruled vellum before writing a single letter.
    ag("ᓇ", "NOW", 'H', 40, '1', "van-de-graaf-margins", "launcher.kit.vixi root padding → ninths canon, outer margin 2x inner", Reach { domain: "Van de Graaf / Tschichold canon of page construction", mechanism: "derive padding from root w/h ninths in MilliUnit rather than a flat mu(12) on all four sides", impact: "the void becomes a ratio the eye can resolve instead of an even border that reads as default" }),
    ag("ᓇ", "NOW", 'H', 35, '1', "optical-center-rail", "rail spacers weighted 45/55 — optical centre, not geometric", Reach { domain: "typographic optical centring", mechanism: "trailing Fill spacer gets a larger flex weight so the rail sits slightly above/left of true centre", impact: "the rail stops looking low on the page, the artefact of centring a block by arithmetic" }),
    ag("ᓇ", "NOW", 'H', 55, '1', "rubricated-entry", "the focal door rubricated — second colour reserved to ONE mark", Reach { domain: "rubrication: red ink reserved for entry points", mechanism: "accent is spent ONLY on the focal stroke; every other chrome edge drops to border token", impact: "one preattentive mark on the whole page, which is the aperture law stated in ink" }),
    ag("ᓇ", "NOW", 'M', 45, '1', "wordmark-initial-ratio", "wordmark:tagline size bound to the ramp, not two free numbers", Reach { domain: "the historiated initial's fixed ratio to its text block", mechanism: "wordmark = ramp[4], tagline = ramp[1], gap = one ramp step — sizes derive, never authored loose", impact: "the hero holds proportion at any window size instead of drifting apart on a wide monitor" }),
    ag("ᓇ", "NEXT", 'H', 110, '1', "ruled-baseline-grid", "every text slot snapped to one shared baseline lattice", Reach { domain: "scribal ruling — prick, rule, then write", mechanism: "quantise text slot y to a baseline multiple derived from ramp[1] leading, in tile units", impact: "crumb, hero, labels and status finally align across the page instead of floating per-band" }),
    ag("ᓇ", "NEXT", 'H', 95, '1', "door-hover-alidade", "hover = a sight-rule laid across the focal, not a colour swap", Reach { domain: "astrolabe alidade sighting rule", mechanism: "on hover, draw a thin full-width rule through the door's centre plus a tick at each edge", impact: "hover reads as measurement rather than a web button, and survives colour-blind viewing" }),
    ag("ᓇ", "NEXT", 'M', 80, '1', "marginalia-status", "status + tape move to the OUTER margin as gloss, not footer bars", Reach { domain: "manuscript marginalia / gloss column", mechanism: "rotate the status band into a narrow left margin column, set at ramp[0] in the muted token", impact: "two full-width bars stop bracketing the page, and the content block gets its ratio back" }),
    ag("ᓇ", "NEXT", 'M', 90, '2', "throne-kursi-anchor", "NEW: the crumb sits in a throne, the way an astrolabe hangs from its kursi", Reach { domain: "the kursi (throne) of a planispheric astrolabe", mechanism: "an authored chrome slot above the plate with its own ornament, carrying the suspension mark", impact: "the title anchors the instrument instead of floating as a label in the corner" }),
    ag("ᓇ", "LATER", 'H', 160, '2', "historiated-door-marks", "NEW: each door gets a generated mark, word demoted to caption", Reach { domain: "historiated initials — the letter that contains its subject", mechanism: "one procedural glyph per door derived from its edict id, drawn as stencil bit-plane at rail scale", impact: "the rail reads at a glance instead of requiring four words to be read in sequence" }),
    ag("ᓇ", "LATER", 'M', 130, '2', "gold-leaf-tooling", "NEW: burnished-leaf accent — tooled highlight, not a flat fill", Reach { domain: "gold leaf laid on gesso, tooled and burnished", mechanism: "accent stroke gains a one-pixel lighter inner bevel and a darker outer, both derived from the accent token", impact: "the focal mark reads as raised metal rather than a coloured outline" }),
    ag("ᓇ", "LATER", 'M', 60, '2', "catchword-continuity", "NEW: bottom-corner catchword naming the surface a door opens onto", Reach { domain: "the catchword — first word of the next leaf, written on this one", mechanism: "on hover, the bottom outer corner prints the target surface's first heading in the muted token", impact: "the door tells you where it goes before you commit, with no panel and no tooltip chrome" }),
    ag("ᓇ", "LATER", 'L', 50, '2', "ruling-prick-marks", "NEW: the tile lattice faintly visible at the page edges", Reach { domain: "prick marks left in the vellum margin after ruling", mechanism: "emit 1px ticks at tile intervals along the outer margin at very low alpha", impact: "the underlying 5D lattice becomes legible as craft — the grid stops being invisible scaffolding" }),
    ag("ᓇ", "LATER", 'L', 40, '2', "quire-signature-build", "NEW: build stamp set as a quire signature, not a version string", Reach { domain: "quire signatures marking gathering order in a codex", mechanism: "render BUILD id at ramp[0] in the bottom inner corner, the position a signature always occupies", impact: "provenance sits where a reader expects it instead of competing with the wordmark" }),
    ag("ᓇ", "HORIZON", 'M', 190, '2', "bifolium-spread", "NEW: wide windows open as a two-page spread, not one stretched page", Reach { domain: "the bifolium — a codex opening is TWO pages with a gutter", mechanism: "past an aspect threshold, split the root into recto/verso with a gutter, doors on recto and state on verso", impact: "an ultrawide monitor gains a second column instead of stretching one block of empty ground" }),
    // CDK — Cosmic Dissonance Kernel (Sean 07-31). Moved OFF forge-gpu's domain line,
    // which had carried it as a target: a domain line says what a crate IS, a target
    // is a row. Supersedes "turn shaders into VIXIBLOBS", which predates the
    // shaderbind DSL and splitshaders and so aimed at a lane that no longer exists.
    ag("ᒥ", "HORIZON", 'H', 260, '3', "cdk-converge", "NEW: shaderbind DSL + splitshaders + matmul32/64 + spriteblob + WGSL + SPIR-V as ONE lane", Reach { domain: "the repo's own shader surfaces, currently six unjoined dialects", mechanism: "one authored source lowering to every backend the engine already emits, so a shader is written once and sealed per target instead of hand-kept per lane", impact: "the shader lane gets a single home; today a change must be repeated across dialects and nothing proves they agree" }),
    ag("ᓇ", "HORIZON", 'M', 140, '2', "palimpsest-recent", "NEW: recently-used surfaces ghosted beneath the current page", Reach { domain: "the palimpsest — scraped vellum still showing its under-text", mechanism: "render the last surface's lowered outline at very low alpha on the tympan plane beneath the rete", impact: "history is visible as depth rather than a list, and costs one extra sparse plane" }),
    // ᐮ SPINE / RAY / INDEX (Sean 07-31, /aspire run n=1). Reach = 5D embedding-space indexing
    // (vector similarity search, nearest-neighbor queries, coordinate-native ranking). When the
    // river.idx moved to 5D-native (TAG\t#coord\t@hash, 148/153 rows), text-tag routing became
    // geometric: keywords → vector space, and we gain R-tree / spatial-indexing theory. The
    // crossing costs: point queries vs top-N, cell-block access, grain-backed bodies cascade,
    // coordless-row policy, audit-trail (tape.idx) coherence. Isomorphism: nearest_neighbor::
    // nearest() is the kernel; repo_query::dispatch consumes; triple_loop::{WorldBridge} carries
    // published coordinates. 15/15 survived, 27% interior / 73% exterior.
    ag("ᐮ", "NOW", 'H', 40, '1', "coord-rank-nth", "expose top-N nearest neighbors in ranked order over 5D space", Reach { domain: "vector similarity search / nearest-neighbor indexing", mechanism: "5D squared-euclidean nearest-of-N over embed_river_line(tag+body_len), return sorted by perp_sq", impact: "downstream query_ray asks 'what's near me' with single coordinate lookup, no linear scan" }),
    ag("ᐮ", "NOW", 'H', 25, '1', "coordless-row-pass", "filter PROSE_TAGS (HEAD/APERTURE/BUILD) out of 5D space", Reach { domain: "coordinate space purity / feature engineering", mechanism: "check tag against const PROSE_TAGS in repo_query::spine_coord_row before embedding", impact: "coordinate space stays pure, prose rows never pollute the 5D grid or cause embedding drift" }),
    ag("ᐮ", "NOW", 'H', 35, '1', "grain-body-alias-route", "read spill bodies via @hash pointer, no second index scan", Reach { domain: "index body dereferencing / grain pointers", mechanism: "RiverRow carries grain pointer, resolve via grain_path(), memoize first hit in threadlocal lru", impact: "query results return full bodies without re-reading river.idx or walking .forge/spill/" }),
    ag("ᐮ", "NOW", 'M', 50, '1', "ray-origin-anchor", "decode raycast aim, embed it, use as search center not origin", Reach { domain: "context-relative search / query expansion", mechanism: "repo_query::dispatch splits 'from' field, embed_river_line(from) yields 5D seed for nearest", impact: "'what's near me' becomes genuinely contextual instead of always measuring from 0,0,0,0,0" }),
    ag("ᐮ", "NOW", 'M', 45, '1', "vocab-projection-lane", "map syntactic tags (HEAD/MAP/TOOL) to a distinct semantic lane", Reach { domain: "categorical feature encoding / faceted search", mechanism: "RiverRow::tag → u8 enum projection, add to x-axis embedding as tag_type identity", impact: "tag-type queries isolate rows via geometric range (e.g., 'all TOOL rows' = x ∈ [0,5])" }),
    ag("ᐮ", "NEXT", 'H', 60, '2', "index-compaction-delta", "track deltas since last spine read, emit incremental index", Reach::aimed("incremental index update protocol")),
    ag("ᐮ", "NEXT", 'H', 55, '1', "nearest-hamming-fallback", "try 5D nearest first, fall back to FNV1a hamming on cache miss", Reach { domain: "hybrid distance metrics / metric space search", mechanism: "embed both syntactic (FNV1a, embed_river_line lane 0) + semantic (5D meaning), try semantic → syntactic", impact: "novel/malformed rows still surface instead of returning empty, no customer sees 'not found'" }),
    ag("ᐮ", "NEXT", 'M', 70, '2', "dual-channel-spine", "read-only .forge/river.idx.probe, write-only .forge/river.idx.live", Reach::aimed("lock-free read-write index patterns")),
    ag("ᐮ", "NEXT", 'M', 80, '2', "spill-lazy-load", "populate grain bodies on first access, not upfront", Reach { domain: "lazy evaluation / on-demand I/O", mechanism: "RiverRow carries body: Option<&str>, populate on grain_body_alias_route first hit", impact: "index loads instantly (no upfront I/O spike), bodies stream in as consumed" }),
    ag("ᐮ", "NEXT", 'M', 65, '1', "coordinate-validation-gate", "reject rows whose embedding is NaN/inf, LOUD on first find", Reach { domain: "data quality gates / embedding validation", mechanism: "check embed_river_line output before index write, log to .forge/river.evt if bad", impact: "embedding gaps surface immediately instead of silently returning junk neighbors" }),
    ag("ᐮ", "LATER", 'M', 100, '3', "triage-clustering", "pre-group ASPIRE rows by tier (t1/t2/t3), expose as binary search", Reach::aimed("range query optimization / clustered indices")),
    ag("ᐮ", "LATER", 'M', 110, '3', "spine-recompact-timer", "track delta size, recompact when > 1% of spine size", Reach::aimed("adaptive index maintenance")),
    ag("ᐮ", "LATER", 'L', 30, '1', "ray-confidence-score", "compute squared-distance percentile rank, expose as u8", Reach { domain: "result ranking / confidence scoring", mechanism: "sort results by perp_sq, compute percentile bin (0-255), carry in result metadata", impact: "clients know which results are certain vs noisy, can threshold on confidence" }),
    ag("ᐮ", "HORIZON", 'M', 120, '3', "5d-native-tape", "record every coord+hash emitted to river.idx as append-only tape", Reach::aimed("audit-trail logging / point-in-time recovery")),
    ag("ᐮ", "HORIZON", 'L', 90, '2', "coordless-row-manifest", "explicit const PROSE_TAGS list, never guessed at runtime", Reach { domain: "semantic versioning / policy as code", mechanism: "const PROSE_TAGS in repo_query.rs, parallel const in forge-book for chronicle", impact: "the 'coordless policy' is declared once, portable to every consumer, no silent assumptions" }),
    // ᒐ ROUTE — the router/collapse substrate (Sean 07-31, /aspire run n=1). Reach =
    // sparse-MoE routing literature + metric-space indexing. The organ: forge-book::routers
    // (30+ routers → 7 axes, must NOT collapse), forge-ml::bq_router, forge-audio::
    // dimensional_collapse, outland::soulword, forge-daemon::repo_query (5D raycast),
    // forge-ml::master_decode (NDE ladder). 15/15 survived, 27% interior / 73% exterior.
    ag("ᒐ", "NOW", 'H', 70, '1', "routers-axis-gate", "routers.rs census → compiled test that the 7-expert ladder cannot collapse to fewer axes", Reach { domain: "sparse-MoE expert collapse (all tokens routed to one expert)", mechanism: "assert every axis holds >=1 router and that no two axes share a router id, run off the existing census table", impact: "the crate's own domain line becomes enforceable instead of a comment" }),
    ag("ᒐ", "NOW", 'H', 60, '1', "bq-router-topk", "bq_router.rs 64B XOR+POPCNT → top-k with a tie-break, not argmax", Reach::aimed("Shazeer 2017 sparsely-gated MoE top-k")),
    ag("ᒐ", "NOW", 'H', 80, '1', "collapse-safety-clamp", "forge-audio::dimensional_collapse → energy-preserving clamp on the down-mix", Reach::aimed("Gerzon energy-preserving downmix")),
    ag("ᒐ", "NOW", 'M', 90, '1', "soulword-trit-index", "outland::soulword base-243 packing → repo_query trit_hamming lane as a routing key", Reach::aimed("balanced-ternary Hamming search")),
    ag("ᒐ", "NEXT", 'H', 130, '2', "switch-capacity", "NEW: forge-ml::switch_capacity — capacity factor + token drop instead of unbounded expert queues", Reach::aimed("Fedus 2021, Switch Transformer")),
    ag("ᒐ", "NEXT", 'H', 120, '2', "expert-choice", "NEW: forge-ml::expert_choice — experts pick tokens, so load balance holds by construction", Reach::aimed("Zhou 2022, Expert-Choice routing")),
    ag("ᒐ", "NEXT", 'H', 70, '2', "router-zloss", "NEW: forge-ml::router_zloss — logit-magnitude penalty that keeps routing stable under drift", Reach::aimed("Zoph 2022, ST-MoE router z-loss")),
    ag("ᒐ", "NEXT", 'H', 150, '2', "hilbert-key", "NEW: forge-daemon::hilbert_key — linearise the 5D index so a raycast is a range scan", Reach::aimed("Lawder/King, Hilbert-curve multidimensional indexing")),
    ag("ᒐ", "NEXT", 'M', 140, '2', "vp-tree-prune", "NEW: forge-daemon::vp_tree — triangle-inequality pruning for the 5D raycast", Reach::aimed("Yianilos 1993, vantage-point trees")),
    ag("ᒐ", "LATER", 'H', 90, '2', "seqlock-reader", "NEW: forge-hal::seqlock — version-counter proof that TripleBuffer readers never tear", Reach::aimed("Lamport seqlock")),
    ag("ᒐ", "LATER", 'H', 200, '2', "wave-digital-dsp", "NEW: forge-audio::wdf — passivity-guaranteed DSP so the collapse cannot inject energy", Reach::aimed("Fettweis 1986, wave digital filters")),
    ag("ᒐ", "LATER", 'M', 160, '2', "polyphase-decimate", "NEW: forge-audio::polyphase_kaiser — polyphase decimation for the GPU DSP lane", Reach::aimed("Crochiere-Rabiner polyphase decimation")),
    ag("ᒐ", "LATER", 'M', 180, '2', "product-quantize", "NEW: forge-ml::product_quantization — codebook tier between student and teacher .nde", Reach::aimed("Jegou 2011, product quantization")),
    ag("ᒐ", "HORIZON", 'H', 220, '3', "speculative-tier", "NEW: forge-ml::speculative_tier — student drafts, teacher verifies, across the 3-tier flywheel", Reach::aimed("Leviathan 2023, speculative decoding")),
    ag("ᒐ", "HORIZON", 'M', 170, '3', "wang-tile-zplane", "NEW: forge-core::wang_tiles — edge-matched aperiodic tiling for material_canvas z-planes", Reach::aimed("Cohen 2003, Wang tiles")),

    // ── ᓄ SENSE — what reaches a body (2026-08-01, /aspire run n=1) ─────────────
    // Reach: field biology has spent forty years on the exact question this organ
    // answers with a hard edge — how far does a thing carry, and what does a
    // listener miss. Distance sampling made detectability a curve, occupancy
    // modelling made absence a probability instead of a fact, and sensory ecology
    // made the answer species-specific. All of it is integer-collapsible.
    ag("ᓄ", "NOW", 'H', 60, '1', "almost-hear-the-well", "mud_sieve::felt boolean edge → a carry that thins with distance instead of stopping", Reach { domain: "Buckland 2001, distance sampling detection function", mechanism: "half-normal g(r)=exp(-r²/2σ²) as an integer permyriad lookup on the ellipsoid sum already computed; a reading below the draw threshold degrades the tell instead of deleting it", impact: "the edge of hearing stops being a wall you can pace out — a fading well reads as fading, which is what a player calls atmosphere" }),
    ag("ᓄ", "NOW", 'H', 45, '1', "your-noise-hides-you", "perception::Senses muted_q is one-way — make the same roar cover you from what hunts", Reach { domain: "acoustic crypsis / masked predator detection", mechanism: "expose muted_q as an EMISSION as well as a suppression: an actor's detection check against the player subtracts the player's own noise floor", impact: "Sulfur stops being pure penalty — the loud build is loud in a room that is also listening, which is a trade instead of a tax" }),
    ag("ᓄ", "NOW", 'H', 70, '2', "the-room-goes-quiet", "NEW: sieve.bank.social room_goes_quiet_q driven by the player's own arrival, not ambient state", Reach { domain: "sentinel and alarm-call ethology (Zuberbühler)", mechanism: "entering a room raises a decaying disturbance on the social channel proportional to presence and noise; the tell fires on the transient, not the level", impact: "the bank that already says conversation stops when you enter finally reacts to you entering" }),
    ag("ᓄ", "NOW", 'H', 90, '2', "how-far-before-you-lose-it", "NEW: perception::detect_q — a detection curve per lane, replacing the single reach scalar", Reach { domain: "Buckland 2001, hazard-rate vs half-normal shoulder", mechanism: "each lane carries a shoulder width and a fall-off exponent; reach becomes the distance where the curve crosses 5000 permyriad", impact: "Lane0 can carry far with a hard shoulder while ambience trails off soft — one lattice, two honest shapes" }),
    ag("ᓄ", "NOW", 'M', 55, '2', "the-weight-you-carry-deafens", "NEW: burden accrual feeds SHA over ticks rather than only at the moment of action", Reach { domain: "McEwen 1998, allostatic load", mechanism: "a slow integrator on burden_q that only unwinds on rest verbs, so the suppressor tracks accumulated wear and not the last swing", impact: "the shadow suppressor becomes a thing you manage across a session instead of a number that jumps once" }),
    ag("ᓄ", "NEXT", 'H', 120, '2', "every-beast-hears-different", "NEW: per-creature Senses profile — the sieve read through something that is not the player", Reach { domain: "von Uexküll 1934, Umwelt", mechanism: "Senses is already a plain struct of grants and suppressors; give companions and mobs their own and run the same tells_at against it", impact: "a companion notices what you cannot, which is the entire reason to have one walking with you" }),
    ag("ᓄ", "NEXT", 'H', 110, '2', "nothing-sings-over-another", "NEW: bank channels claim disjoint frequency niches before they reach the mixer", Reach { domain: "Krause 1993, acoustic niche hypothesis", mechanism: "assign each of the 26 banks a band on the drone/bell axes; a bank whose niche is occupied this tick defers rather than sums", impact: "a busy parish stays legible — twenty banks firing reads as a place, not as mud" }),
    ag("ᓄ", "NEXT", 'H', 95, '2', "casting-for-the-scent", "NEW: a faint tell drives a search cue instead of a flat report", Reach { domain: "Kennedy 1983, moth anemotaxis and plume casting", mechanism: "when a reading sits between the noticing and naming thresholds, emit a direction-only cue derived from the gradient across sampled cells", impact: "the player gets something to DO with a half-heard thing — cast across it until the plume firms up" }),
    ag("ᓄ", "NEXT", 'M', 80, '2', "the-track-goes-cold", "NEW: t-axis staleness as an aging curve a tracker could read, not a cutoff", Reach { domain: "San and Plains tracker sign-aging", mechanism: "t_tolerance becomes an age band table; a reading past its band still surfaces, worded as old rather than suppressed", impact: "the world keeps a memory you can work backwards through instead of one that blinks out" }),
    ag("ᓄ", "NEXT", 'H', 85, '2', "you-cannot-swear-it-was-nothing", "NEW: quiet ground reports a confidence, not an absence", Reach { domain: "MacKenzie 2002, occupancy modelling and detection probability", mechanism: "carry p(detect) per lane; repeated quiet reads compound into a confidence the status face can state, absence never asserted from one look", impact: "the engine stops claiming an unmeasured zone is a calm one — the governor's own rule, applied to the player" }),
    ag("ᓄ", "LATER", 'M', 100, '2', "twice-as-loud-is-not-twice", "NEW: perceived magnitude curve between permyriad field value and reported intensity", Reach { domain: "Stevens 1957, psychophysical power law", mechanism: "integer power-law lookup per channel exponent, applied where a reading becomes a tell threshold comparison", impact: "a field at 9000 stops reading as merely slightly worse than 8000 — the numbers finally match how they land" }),
    ag("ᓄ", "LATER", 'M', 130, '2', "hearing-what-is-not-there", "NEW: false tells for a strained listener, under a criterion the player can shift", Reach { domain: "Green & Swets 1966, signal detection theory d' and criterion", mechanism: "a dulled or blocked listener draws from a noise distribution; sensitivity sets d', and a caution stat sets the criterion between miss and false alarm", impact: "being half-blind gets its own texture — you hear things, and some of them are not there" }),
    ag("ᓄ", "LATER", 'M', 140, '1', "the-shape-of-the-water", "WIRE Cargo.lock:9341 rustfft — spectral read of a ScalarField instead of a point sample", Reach { domain: "spatial spectral analysis of scalar fields", mechanism: "FFT a row of the field around the player; dominant spatial frequency says whether the taint is a patch, a seam or a front", impact: "a reading gains SHAPE — the difference between a spill and a spreading edge, off data already installed" }),
    ag("ᓄ", "LATER", 'L', 90, '1', "the-line-you-walk-each-day", "WIRE Cargo.lock:8218 petgraph — remembered routes between rooms a player has actually worked", Reach { domain: "bumblebee trapline foraging", mechanism: "accumulate an edge weight per traversal on a room graph; the sieve raises sightline grants along worked lines", impact: "the parish learns your habits and pays you for them, which is what makes a place yours" }),
    ag("ᓄ", "HORIZON", 'M', 200, '2', "the-fish-feels-the-room", "NEW: gradient-field sensing — read the field's slope, not its value at your feet", Reach { domain: "teleost lateral line hydrodynamic imaging", mechanism: "central-difference gradient across the sampled neighbourhood becomes its own reading channel with its own banks and thresholds", impact: "a body can feel a wall it cannot see and a current it is standing in — sensing without a line of sight" }),
    // ᕒ REEL (2026-08-01) — the 1000-drop generative reel pipeline. Reach: the
    // tracker/demoscene lineage, which authors a whole audiovisual piece as a small
    // declarative pattern table replayed by a deterministic engine.
    ag("ᕒ", "NOW", 'H', 120, '1', "reel-verb-fold", "13forge-studio reel <script> — the drop leaves the test harness for the one bin", Reach { domain: "demoscene intro executable", mechanism: "lift render_frames/soundtrack/edl out of midi_pipe_poc.rs into forge-studio::reel, the test becomes a caller not the home", impact: "a reel is something Sean runs, not something only cargo test can produce" }),
    ag("ᕒ", "NOW", 'H', 90, '2', "pattern-table-script", "NEW: reel.pattern — the script as a MOD/XM-style row table, not Rust literals", Reach { domain: "Amiga MOD / FastTracker XM pattern tables", mechanism: "one row per column: note, palette, cam, flash, scar, truth text; parsed to ScriptColumn, engine unchanged", impact: "a new reel is a new table — no recompile, and the author never touches Rust" }),
    ag("ᕒ", "NOW", 'H', 60, '1', "gif-teaser-emit", "WIRE Cargo.lock:5076 gif — a 3-second looping teaser off the same frames", Reach { domain: "demoscene loader loop", mechanism: "sample every Nth frame into an animated gif with a global palette from albedo_means", impact: "the post has a moving thumbnail without a video host" }),
    ag("ᕒ", "NOW", 'M', 70, '2', "ass-karaoke-export", "NEW: karaoke.jsonl → .ass subtitle with \\k timing tags", Reach { domain: "Advanced SubStation Alpha karaoke tags", mechanism: "each word window becomes {\\kNN} centiseconds in a Dialogue line; players and YouTube both read it", impact: "the 12,089 word timings become real captions instead of a private jsonl" }),
    ag("ᕒ", "NEXT", 'H', 110, '2', "schillinger-interference", "NEW: rhythm generator from two coprime periods, not a hand-authored SMF", Reach { domain: "Schillinger System of Musical Composition, book I", mechanism: "interference of pulses a and b over lcm(a,b) yields the attack table; coprime pairs give the non-repeating groove", impact: "an endless supply of drum fixtures that are structured, not random" }),
    ag("ᕒ", "NEXT", 'H', 80, '2', "bs1770-loudness-normalise", "NEW: integrated loudness gate on the master before write_soundtrack", Reach { domain: "ITU-R BS.1770-4 / EBU R128", mechanism: "K-weighted mean square over 400ms blocks, gated at -70 and -10 LU, single gain to hit -14 LUFS", impact: "every reel lands at platform loudness instead of being quietly turned down on upload" }),
    ag("ᕒ", "NEXT", 'H', 100, '2', "karplus-strong-voice", "NEW: plucked-string carrier beside ghost_voice's additive stack", Reach { domain: "Karplus-Strong 1983 digital waveguide", mechanism: "noise burst into a delay line of length sr/f with a 2-tap averaging filter; integer state, no float core", impact: "a second instrument family for free — the score stops being one timbre in two registers" }),
    ag("ᕒ", "NEXT", 'M', 75, '2', "cinemetric-pacing-gauge", "NEW: average-shot-length gauge over the edl, asserted per reel", Reach { domain: "Barry Salt cinemetrics / ASL distribution", mechanism: "shot lengths from edl rows, assert median and dispersion inside an authored band per section", impact: "pacing becomes a test, so a boring reel fails before it renders" }),
    ag("ᕒ", "NEXT", 'M', 65, '2', "cmx3600-edl-export", "NEW: edl.jsonl → CMX3600 EDL with SMPTE timecode", Reach { domain: "CMX 3600 edit decision list, the broadcast interchange", mechanism: "frame index → hh:mm:ss:ff at 20fps, one event per cut, standard record/source columns", impact: "the reel opens in Resolve or Premiere as a conformed timeline, not a folder of pngs" }),
    ag("ᕒ", "NEXT", 'M', 55, '1', "cut-gauge-full-drop", "vision_cycle gauges all 1000 frames, not the 40 sheet samples", Reach { domain: "full-population sampling vs convenience sample", mechanism: "feed every composed frame to CycleGauge while it is still in hand, before it is written and dropped", impact: "the QA receipt describes the reel that shipped rather than a 4% slice of it" }),
    ag("ᕒ", "LATER", 'M', 140, '2', "bayer-ordered-dither", "NEW: 4x4 ordered dither to a 16-colour palette as an authored look", Reach { domain: "Bayer ordered dithering / 1-bit demoscene look", mechanism: "threshold matrix indexed by (x&3,y&3) before palette quantise, integer only, deterministic", impact: "a second visual register — the same script renders as a plotter print or a Teletext screen" }),
    ag("ᕒ", "LATER", 'M', 120, '2', "ken-burns-kinetics", "NEW: continuous scale+translate ride instead of five discrete Cam values", Reach { domain: "Ken Burns rostrum-camera move", mechanism: "eased affine over the truth rect across a section's columns, integer MilliUnit endpoints authored per section", impact: "camera language beyond pan/tilt/hold, still one deterministic offset per frame" }),
    ag("ᕒ", "LATER", 'M', 95, '2', "smpte-burnin-slate", "NEW: head slate + timecode burn-in behind a flag", Reach { domain: "broadcast slate and burn-in convention", mechanism: "prepend a 2-second slate frame set with title/date/seal, optional per-frame timecode in the exhaust rail", impact: "a reel identifies itself on screen — provenance survives being re-uploaded" }),
    ag("ᕒ", "HORIZON", 'M', 220, '2', "one-frame-procedural-texture", "NEW: procedural ground per column instead of flat rail fills", Reach { domain: "4k intro procedural texture synthesis", mechanism: "value-noise field seeded by column index, quantised to the rail palette, evaluated per pixel at compose time", impact: "the frame stops being two rectangles — texture without a single asset byte" }),
    ag("ᕒ", "HORIZON", 'L', 180, '3', "reel-from-tape", "the forge-vcs tape becomes a script source — the repo narrates its own week", Reach { domain: "generative documentary / data-driven montage", mechanism: "commit rows map to columns: message → truth line, crate → palette, churn → cam move and flash", impact: "a weekly reel that writes itself out of work already recorded" }),
    // ᑲ CADENCE (2026-08-02) — the triadic session loop (Floor→Circuit→Surface,
    // 40/30/30, seal at close) as a compiled organ. Reach: real-time frame pacing
    // and the coatings trade both already enforce mechanically what the session
    // theory asks for in prose — fixed per-pass budgets, order barriers, a present.
    ag("ᑲ", "NOW", 'H', 110, '3', "cadence-law-module", "NEW: forge_book::session_cadence — Floor/Circuit/Surface as a compiled law", Reach { domain: "real-time frame pacing (Fiedler, Fix Your Timestep 2004)", mechanism: "three-variant Phase enum + permyriad budget [4000,3000,3000] + advance() that refuses Floor→Surface; gauged off tape lane tags", impact: "the triadic theory stops being prose — any verb can gauge a session against the compiled cadence" }),
    ag("ᑲ", "NOW", 'H', 30, '1', "applyfault-at-origin", "forge_daemon::spool::write_allowed returns ApplyFault — kill the map_err re-derive in apply_edit", Reach { domain: "holiday detection (SSPC low-voltage sponge test)", mechanism: "the Denied variant is born where the containment check fails; apply_edit stops re-deriving strings the check already dropped", impact: "refusal carries its origin — callers route on the variant instead of parsing prose" }), // LANDED 08-02: spool.rs write_allowed→ApplyFault + Traversal variant, spool 10/10 green
    ag("ᑲ", "NOW", 'H', 45, '1', "look-through-sense", "sf_wasm::mud::do_look consumes sense_here's RoomView when a world is resident", Reach { domain: "umwelt (von Uexküll 1934)", mechanism: "do_look asks sense_here() first; Some(view) renders the sensed fields, None falls back to the static room pool", impact: "look reports the world the tick lane installed, not the map the compiler shipped" }), // LANDED 08-02: RoomView.tells carries the data, do_look consumes the view, sf-wasm 228/228 green
    ag("ᑲ", "NOW", 'H', 40, '1', "depth-aware-init", "forge_ml::moe_train::init_xavier — residual segments shrink with depth, name stops lying", Reach { domain: "GPT-2 residual scaling (Radford et al. 2019 §2.3)", mechanism: "trunk+expert segments draw from ±sqrt(3/d_model)/sqrt(2·depth), depth=2+expert_layers; embed/router/heads keep 0.02", impact: "deep expert ladders start unit-variance instead of fighting the flat 0.02 the whole first epoch" }), // LANDED 08-02: per-segment bounds + init_is_depth_aware witness, moe_train 10/10 green
    ag("ᑲ", "NOW", 'H', 40, '1', "red-green-harvest-gate", "board --harvest rejects '0 passed'/'filtered out' as green (parked 08-02, in-cadence now)", Reach { domain: "TDD red-green-refactor (Beck 2002)", mechanism: "harvest parser refuses a green flip whose runner line shows zero executed tests — red must be witnessable before green counts", impact: "a filtered-out suite can never seal the board — green means witnessed, not vacuous" }),
    ag("ᑲ", "NEXT", 'M', 50, '1', "present-mode-seal", "prime-start refuses RESUME when BUILD ≠ stamped exe — the swapchain rule", Reach { domain: "Vulkan VK_PRESENT_MODE_FIFO present semantics", mechanism: "BIN.json seal compared to exe mtime at prime; mismatch prints the REDEPLOY line before any lane opens", impact: "no session runs against an unstamped binary — the frame presents before the next one starts" }),
    ag("ᑲ", "NEXT", 'M', 45, '1', "toposort-phase-dag", "WIRE Cargo.lock:8225 petgraph — phases as a 3-node DAG, no-skip = toposort assert at close", Reach { domain: "LLVM pass-manager phase ordering (Lattner & Adve, CGO 2004)", mechanism: "Floor→Circuit→Surface edges in a DiGraph; session close asserts the visited order is a topological order", impact: "phase skipping becomes a mechanical exit-1 instead of a discipline" }),
    ag("ᑲ", "NEXT", 'M', 50, '2', "phase-histogram", "NEW: hdrhistogram — per-phase token/wall-time distributions behind the board gauge", Reach { domain: "HdrHistogram (Tene) latency capture", mechanism: "one histogram per phase keyed off tape timestamps, quantiles in the board gauge; pull_gate=EXISTS-VARIANT@path (vault scan text, not compiled)", impact: "the 40/30/30 split becomes a measured distribution, not an intention" }),
    ag("ᑲ", "NEXT", 'M', 100, '3', "dft-mil-gauge", "NEW: per-phase depth gauge — churn per phase vs the 40/30/30 spec, read off tape commits", Reach { domain: "SSPC-PA 2 dry-film thickness inspection", mechanism: "tape commit churn binned by phase tag; a session fails inspection when the topcoat bin reads zero mils", impact: "phase-skipping is measurable after the fact from the record, like DFT under a gauge" }),
    ag("ᑲ", "LATER", 'M', 120, '3', "pass-invalidation", "circuit edits invalidate floor receipts unless declared preserved", Reach { domain: "LLVM analysis invalidation", mechanism: "a receipt row carries the file set it proved; a later edit intersecting that set flips it STALE until re-run", impact: "a Phase-1 green cannot silently survive a Phase-2 rewrite of the same file" }),
    ag("ᑲ", "LATER", 'M', 130, '3', "sonata-cadence-audit", "NEW: classify a session's tape into exposition/development/recap, flag the deceptive cadence", Reach { domain: "sonata form (Rosen, Sonata Forms 1980)", mechanism: "tape events labeled by lane; a session ending outside the home lane with no stamp = deceptive cadence, printed LOUD", impact: "the seal-skip gets a name and a detector instead of a habit" }),
    ag("ᑲ", "LATER", 'M', 80, '3', "heijunka-phase-level", "NEW: board queue leveled by phase, not FIFO", Reach { domain: "Toyota heijunka production leveling (Ohno 1988)", mechanism: "queued rows carry phase tags; the drain interleaves floor/circuit/surface at the spec ratio instead of draining one bin dry", impact: "a wave can no longer spend itself entirely below the surface" }),
    ag("ᑲ", "LATER", 'L', 90, '3', "fixed-timestep-accumulator", "NEW: session accumulator — phases consume fixed budgets, the remainder interpolates into PARK", Reach { domain: "Fix Your Timestep accumulator (Fiedler 2004)", mechanism: "spend accumulates against per-phase budget; leftover work is carried as the PARK alpha, never silently dropped", impact: "a session that runs long degrades by shrinking scope, not by skipping the surface" }),
    ag("ᑲ", "HORIZON", 'L', 70, '3', "brac-ultradian-pace", "NEW: wakeup cadence matched to the ~90-minute basic rest-activity cycle", Reach { domain: "Kleitman BRAC (1963)", mechanism: "loop wakeup delays derived from session-length histogram peaks instead of fixed intervals", impact: "autonomous loops breathe at the operator's real cycle, not a cron guess" }),
    ag("ᑲ", "HORIZON", 'L', 60, '3', "game-loop-interp", "NEW: update/render decoupling — PARK carries the interpolation alpha between sessions", Reach { domain: "Game Programming Patterns, Game Loop (Nystrom 2014)", mechanism: "the PARK line records how far Circuit outran Surface; the next prime renders the interpolated state first", impact: "cold starts resume mid-stride instead of re-deriving where the last frame ended" }),
    // ᕦ THORN/ROOTLESS 15 (2026-08-18) — the /aspire run over chapter 27 + the ROOTLESS
    // design doc, lateral-criticality lens (verify-first: every target anchors to a
    // path/type verified this session; JUDGE dropped none — 15/15 survived vs disk).
    // Reaches 15/15 exterior. Sean rulings carried: no-ecash; monochrome Ironroot.
    ag("ᕦ", "NOW", 'H', 250, '1', "bard-arch", "the 8th cyoa arch — School of the Bell scene pool (vocab landed, scenes absent)", Reach { domain: "Parry-Lord oral-formulaic composition", mechanism: "scenes assemble from formula slots (sung word x instrument x archetype) the way epic singers recompose set phrases per performance", impact: "the Bard campaign retells itself differently every run while staying canon" }),
    ag("ᕦ", "NOW", 'H', 40, '1', "terra0-prevhash", "creation_spine LedgerEvent gains prev_hash chaining (quarry event_ledger.rs had it; live typed ledger lacks it)", Reach { domain: "tamper-evident logging (Crosby & Wallach 2009)", mechanism: "fold the prior event's hash into event_hash so the log is one chain, verifiable from any suffix", impact: "a Terraforma relay log any client can audit without trusting the host" }),
    ag("ᕦ", "NOW", 'H', 300, '1', "thorn1-era-overlay", "port Era{Ancient,Golden,Decay,Void}+lore_terrain overlay from MYGAMEDRAIN quarry beside the landed terrain_sieve", Reach { domain: "repertory-theatre set re-dressing", mechanism: "one authored geometry; era keys swap material words, light law, prop/NPC roster, and door deltas", impact: "four games on one city — the chapter-27 production law becomes code" }),
    ag("ᕦ", "NOW", 'H', 200, '1', "yod-party-sieve", "port trig_table.rs (1024-entry permyriad LUT, AspectGeometry x7, yod_bases_for_apex, superior_dexter; dedupe celestial.rs twin) onto the landed Brand wheel", Reach { domain: "horary astrology aspect classification", mechanism: "integer milli-degree deltas classed by LUT; trio patterns {i,i+2,i+7} mod 12 over the 12 Brands", impact: "party composition becomes a sieve — a Trine trio fights as a Trine, a Yod trio gets the finger-of-god" }),
    ag("ᕦ", "NOW", 'H', 90, '1', "ironroot-monochrome-law", "Decay/Ironroot era renders GREYSCALE; colour returns per-thing as ledger facts (Sean 2026-08-18: 'Ironroot is black and white. And when the colour finally comes, its beautiful')", Reach { domain: "Sacks, The Island of the Colorblind / Pleasantville colour-as-event", mechanism: "OKLCH chroma clamped to 0 zone-wide; each healed/restored artifact lifts the clamp for its own palette entry via a LoreFact", impact: "colour stops being decoration and becomes the reward channel — the era's emotional arc is visible saturation" }),
    ag("ᕦ", "NEXT", 'H', 250, '3', "terra1-morton-ranges", "NEW: box5d_to_morton_ranges (BIGMIN/LITMAX) over MortonKey5D + the LOD ladder tags — no quarry donor exists (verified)", Reach { domain: "UB-tree range-query decomposition (Bayer 1997)", mechanism: "split a Z-order interval at BIGMIN into the minimal cover of contiguous runs; ladder-truncate for exact-match relays", impact: "any vanilla Nostr relay answers 5D box queries — the Terraforma subscription primitive" }),
    ag("ᕦ", "NEXT", 'H', 400, '1', "audio1-codebook", "port v2 SpatialCodebook + fly_source + prove_done_bar onto forge-audio-v3's landed Woodworth/Brown-Duda math", Reach { domain: "binaural psychoacoustics (ITD/head-shadow)", mechanism: "head-shadow biquad cells scattered in 5D (W axis = literal Hz), raycast-blended along the motion vector at control rate", impact: "the audible done-bar: a test that FAILS unless the sound provably swings past your head" }),
    ag("ᕦ", "NEXT", 'M', 500, '1', "thorn2-city-transpile", "thornhaven_builder.gd + thorngate_forest JSONs -> MaterialGrid zone loads (the WeaponWireframes.gd transpile precedent)", Reach { domain: "constructive solid geometry (Requicha 1980)", mechanism: "lower the bible's CSG primitive list to integer cell fills at 500mm/cell; ratify the 160x160 city grid", impact: "the 80x80m city walkable in-engine — chapter 27's plan becomes floors" }),
    ag("ᕦ", "NEXT", 'H', 600, '1', "rootless-relay", "teach forge-daemon-door the forgedaemon writer-actor skeleton: acceptor -> bounded channel -> sole writer appending per-world JSONL + subscriber fan-out + orphan consolidation", Reach { domain: "LMAX Disruptor single-writer principle", mechanism: "one thread owns the file; accept path does zero serialization; try_send drops rather than blocks; orphan sessions consolidated on disconnect", impact: "a Rootless anyone can host — the ROOTLESS server IS this binary" }),
    ag("ᕦ", "NEXT", 'H', 300, '1', "duel-on-the-glass", "port duel.kit.vixi (v2) onto the cdk wireframe — the landed 7-7-7 duel core gets its first face", Reach { domain: "MTG stack/priority as interface grammar", mechanism: "7-slot closed hand + turn-principle banner lowered to vixi kit slots on the singing terminal", impact: "Ring 1 of ROOTLESS becomes playable — the first duel photon" }),
    ag("ᕦ", "NEXT", 'M', 200, '3', "pack-seed-store", "NEW: the $1.25 pack as a committed seed; open = deterministic replay through the generator", Reach { domain: "commit-reveal provably-fair gaming", mechanism: "storefront sells a sealed seed commitment; the client replays it against the landed rarity bands; anyone re-verifies the pull", impact: "buyable packs no player has to trust — fairness is a replay, not a promise" }),
    ag("ᕦ", "LATER", 'M', 80, '1', "proof-of-vibration", "relay admission predicate: the event's word must sing in PENTATONIC_C (word_note is landed + free)", Reach { domain: "shibboleth gatekeeping (Judges 12)", mechanism: "verify word_note(event.word) is in-scale before accepting; per-npub rate caps behind it", impact: "spam costs the attacker a dictionary; the anti-abuse layer is the game's own music law" }),
    ag("ᕦ", "LATER", 'E', 400, '3', "marching-pentaracts", "NEW: 3D cross-section extraction of the 5D lattice at fixed (T,S) — mesh only as export codec", Reach { domain: "4D CT hyperplane slice reconstruction", mechanism: "fixed-axis slice over pentaract cells; sliding T animates, sliding S decay-morphs, no keyframes anywhere", impact: "terrain that morphs through time and entropy by ADDRESSING, not tweening" }),
    ag("ᕦ", "EDGE", 'H', 150, '1', "architect-forbidden-lore", "The Architect thread as Visibility::Forbidden artifacts, unlocked at the level-20 convergence (codex enum landed)", Reach { domain: "Pale Fire frame-narrative structure (Nabokov)", mechanism: "the frame story rides the artifact visibility ladder — Forbidden until the Zodiac convergence event flips it", impact: "the reveal that turns four era-games into one story, enforced by the type system" }),
    ag("ᕦ", "EDGE", 'E', 120, '1', "audible-ops", "sonify the relay stream: events through word_note into audio_bridge permyriad channels", Reach { domain: "auditory display / EEG sonification research", mechanism: "healthy gossip cannot leave the pentatonic scale by construction; dissonance_sieve flags the wrong note", impact: "an ops dashboard you can HEAR — monitoring becomes listening" }),
];

/// Compute the gauge string for an aspire pass: ROI mix, total estimated LoC, and triage split.
/// Filters by `glyph` (empty string computes for the whole table).
pub fn run_gauge(glyph: &str) -> String {
    let rows = ASPIRE.iter().filter(|r| glyph.is_empty() || r.glyph == glyph);
    let (mut h, mut m, mut l, mut loc, mut t1, mut t2, mut t3, mut n, mut ung) =
        (0, 0, 0, 0u32, 0, 0, 0, 0, 0);
    for r in rows {
        n += 1;
        match r.roi {
            'H' => h += 1,
            'M' => m += 1,
            _ => l += 1,
        }
        loc += r.loc;
        match r.triage {
            '1' => t1 += 1,
            '2' => t2 += 1,
            '3' => t3 += 1,
            _ => ung += 1,
        }
    }
    format!(
        "gauge {} n={n} · roi H{h}/M{m}/L{l} · loc~{loc} · triage wire{t1}/buy{t2}/spec{t3}/ungauged{ung}",
        if glyph.is_empty() { "ALL" } else { glyph }
    )
}

/// Construct the Capabilities chapter from the fold look-ahead table.
pub fn aspire_chapter() -> Chapter {
    let mut ch = Chapter::new("Aspire — Fold Look-Ahead", AtlasSection::Capabilities);
    ch.add_lore(
        "18 OFF-skills eligible to fold to binary (gauge 2026-07-27): 9 verbs ᐫ + \
         9 hybrid-halves ᐬ. eligible = deterministic + not-already-static, NOT worth-ranked. \
         + ᐭ frontend-CI QA/QC organ: 13 candidates (3 interior / 10 exterior NEW). \
         + ᑫ cosmic-dissonance-kernel (2026-07-28): 14 survivors, 1 confab recorded (web_rtb→rtrb).",
    );
    ch.add_lore(run_gauge(""));
    for r in ASPIRE {
        let gauge = if r.is_gauged() {
            format!("roi:{} loc~{} t{}", r.roi, r.loc, r.triage)
        } else {
            format!("roi:{}", r.roi)
        };
        let line = format!("{} {} {}  {} → {}", r.glyph, r.bucket, gauge, r.skill, r.target);
        ch.add_lore(if r.reach.domain.is_empty() {
            line
        } else {
            format!("{line}  [reach: {}]", r.reach.domain)
        });
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspire_binds_all_seventy_eight() {
        assert_eq!(ASPIRE.len(), 199); // 34 + ᑫ 14 + ᓭ 15 + ᔨ 15 + ᒥ SEE-launcher 15 + ᓇ mise-en-page 15 + CDK 1 (07-31) + ᒐ ROUTE 15 (07-31) + ᐮ SPINE 15 (07-31) + ᓄ SENSE 15 (08-01) + ᕒ REEL 15 (08-01) + ᑲ CADENCE 15 (08-02) + ᕦ THORN/ROOTLESS 15 (08-18)
        let ch = aspire_chapter();
        assert_eq!(ch.section, AtlasSection::Capabilities);
        assert_eq!(ch.lore_count(), 201); // header + run gauge + 199
    }

    /// The gauge is the contract: every row of a gauged run carries roi AND loc AND
    /// tier, and the printed gauge is computed off the rows, never hand-written.
    #[test]
    fn every_gauged_run_row_carries_roi_loc_and_triage() {
        let emit: Vec<&Aspirant> = ASPIRE.iter().filter(|r| r.glyph == "ᓭ").collect();
        assert_eq!(emit.len(), 15, "ᓭ run is 15 candidates");
        for r in &emit {
            assert!(r.is_gauged(), "{} landed ungauged", r.skill);
            assert!(matches!(r.triage, '1' | '2' | '3'), "{} bad tier {}", r.skill, r.triage);
            assert!(matches!(r.roi, 'H' | 'M' | 'L'), "{} bad roi {}", r.skill, r.roi);
        }
        let loc: u32 = emit.iter().map(|r| r.loc).sum();
        let g = run_gauge("ᓭ");
        assert!(g.contains("n=15"), "{g}");
        assert!(g.contains(&format!("loc~{loc}")), "{g}");
        assert!(g.contains("ungauged0"), "a gauged run leaves none ungauged: {g}");

        // ᔨ HEAR-5D (07-29) — same contract, plus a FULL reach: a 5D row that cannot
        // state its own crossing is recall, not a lateral, and does not belong here.
        let hear: Vec<&Aspirant> = ASPIRE.iter().filter(|r| r.glyph == "ᔨ").collect();
        assert_eq!(hear.len(), 15, "ᔨ run is 15 candidates");
        for r in &hear {
            assert!(r.is_gauged(), "{} landed ungauged", r.skill);
            assert!(matches!(r.triage, '1' | '2' | '3'), "{} bad tier {}", r.skill, r.triage);
            assert!(matches!(r.roi, 'H' | 'M' | 'L'), "{} bad roi {}", r.skill, r.roi);
            assert!(r.reach.is_sourced(), "{}: a 5D row must state its crossing", r.skill);
        }
        let hear_loc: u32 = hear.iter().map(|r| r.loc).sum();
        let gh = run_gauge("ᔨ");
        assert!(gh.contains("n=15"), "{gh}");
        assert!(gh.contains(&format!("loc~{hear_loc}")), "{gh}");
        assert!(gh.contains("ungauged0"), "{gh}");

        // ᑲ CADENCE (08-02) — same contract as ᔨ: gauged AND sourced, every row.
        let cad: Vec<&Aspirant> = ASPIRE.iter().filter(|r| r.glyph == "ᑲ").collect();
        assert_eq!(cad.len(), 15, "ᑲ run is 15 candidates");
        for r in &cad {
            assert!(r.is_gauged(), "{} landed ungauged", r.skill);
            assert!(matches!(r.triage, '1' | '2' | '3'), "{} bad tier {}", r.skill, r.triage);
            assert!(matches!(r.roi, 'H' | 'M' | 'L'), "{} bad roi {}", r.skill, r.roi);
            assert!(r.reach.is_sourced(), "{}: a cadence row must state its crossing", r.skill);
        }
        let cad_loc: u32 = cad.iter().map(|r| r.loc).sum();
        let gc = run_gauge("ᑲ");
        assert!(gc.contains("n=15"), "{gc}");
        assert!(gc.contains(&format!("loc~{cad_loc}")), "{gc}");
        assert!(gc.contains("ungauged0"), "{gc}");
    }

    #[test]
    fn every_latent_wormhole_row_has_a_body_in_latent_synthesis() {
        let holes: Vec<&str> = ASPIRE.iter().filter(|a| a.glyph == "ᐯ").map(|a| a.skill).collect();
        assert_eq!(holes.len(), 3, "three wormholes, no more no less");
        // The aspire row is the headline; the spec body is the SoT it points at.
        assert_eq!(holes.len(), crate::latent_synthesis::SYNTHESES.len());
        for (row, s) in holes.iter().zip(crate::latent_synthesis::SYNTHESES) {
            let id = s.id.to_ascii_lowercase();
            assert_eq!(*row, id, "aspire row {row} does not name its body {id}");
        }
    }

    #[test]
    fn three_organs_nine_verbs_nine_hybrid_thirteen_qa() {
        let verbs = ASPIRE.iter().filter(|a| a.glyph == "ᐫ").count();
        let hybrid = ASPIRE.iter().filter(|a| a.glyph == "ᐬ").count();
        let qa = ASPIRE.iter().filter(|a| a.glyph == "ᐭ").count();
        assert_eq!((verbs, hybrid, qa), (9, 9, 13));
    }

    /// The wormholes are the only rows whose full crossing was ever written down.
    /// This is the debt ratchet: it may only ever go UP (Sean 07-29).
    #[test]
    fn sourced_rows_only_ratchet_up() {
        let sourced = ASPIRE.iter().filter(|a| a.reach.is_sourced()).count();
        assert!(sourced >= 3, "reach coverage regressed: {sourced} sourced rows");
        assert_eq!(ASPIRE.iter().filter(|a| a.glyph == "ᐯ").count(), 3);
    }

    /// Aimed-but-bodyless rows are legal history, not a legal NEW row. A row added
    /// after 07-29 with a fully empty reach means the skill skipped its lateral pass.
    #[test]
    fn no_row_is_silently_reachless_beyond_the_recorded_debt() {
        let unsourced = ASPIRE.iter().filter(|a| a.reach == UNSOURCED).count();
        assert_eq!(unsourced, 18, "pre-07-29 reachless rows; new rows must carry a reach");
    }
}
