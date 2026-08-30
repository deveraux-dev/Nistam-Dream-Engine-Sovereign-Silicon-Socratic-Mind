// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Runs the whole seam once: notes -> harmonics -> SignalValues -> ShaderBind
//! route -> WGSL -> SPIR-V, printing what each stage actually produced.

use forge_harmonics::scc_bridge::emit_shaderbind_wgsl;
use forge_harmonics::shaderbind_bridge::signals_with_colour;
use forge_harmonics::synthxml::ScheduledNote;

fn main() {
    let src = include_str!("../../scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi");
    let bind = forge_shaderbind::parse_shaderbind(src).expect("audio_vis parses");

    // C major triad, held.
    let plan: Vec<ScheduledNote> = [60u8, 64, 67]
        .iter()
        .map(|&note| ScheduledNote { fire_tick: 0, note, vel: 110, dur_ms: 2000 })
        .collect();

    let signals = signals_with_colour(&plan, 30);
    println!("surface        {}", bind.surface);
    println!("channels       {} (span {})", bind.channel_count(), bind.channel_span());
    println!();
    println!("rms            {}", signals.audio_rms);
    println!("beat_phase     {}", signals.audio_beat_phase);
    println!("centroid       {}", signals.audio_spectral_centroid);
    println!("bands          {:?}", signals.audio_spectrum_bands);
    println!("low/mid/high   {} / {} / {}",
        signals.audio_spectrum_low, signals.audio_spectrum_mid, signals.audio_spectrum_high);
    println!("hue / sat      {} / {}", signals.vibe_hue, signals.vibe_intensity);
    println!();

    let routed = bind.route(&signals);
    println!("routed         {routed:?}");

    let wgsl = emit_shaderbind_wgsl(&bind).expect("audio_vis emits");
    let spv = forge_shader_build_v3::compile_spv(&wgsl).expect("naga accepts");
    println!("spirv          {} bytes", spv.len());
    println!();
    println!("{wgsl}");
}
