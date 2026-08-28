// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Dispatch-floor probe for the S13 GEMV decode structure: times the SAME 168
//! dispatches (42 x qkv/o/gateup/down, one layer's weights reused) three ways —
//! (A) hazard-chained as in real decode, (B) hazard-free with private outputs,
//! (C) hazard-chained but near-empty — to split serialization cost from work cost.

use gemma_s13::gpu_warden::{
    pack_s13_bytes_to_words_slice, GemmParams, S13_WGSL_COMPUTE_SHADER,
};
use std::time::Instant;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
}

fn boot_gpu() -> Option<Gpu> {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;
        let adapter_name = adapter.get_info().name.clone();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("s13-dispatch-floor"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;
        Some(Gpu { device, queue, adapter_name })
    })
}

fn storage(gpu: &Gpu, label: &str, bytes: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn uniform(gpu: &Gpu, label: &str, params: &GemmParams) -> wgpu::Buffer {
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: 32,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut raw = [0u8; 24];
    raw[0..4].copy_from_slice(&params.m_rows.to_le_bytes());
    raw[4..8].copy_from_slice(&params.k_cols.to_le_bytes());
    raw[8..12].copy_from_slice(&params.n_cols.to_le_bytes());
    raw[12..16].copy_from_slice(&params.bytes_per_row.to_le_bytes());
    raw[16..20].copy_from_slice(&params.words_per_row.to_le_bytes());
    raw[20..24].copy_from_slice(&params.scale_permyriad.to_le_bytes());
    gpu.queue.write_buffer(&buf, 0, &raw);
    buf
}

fn synth_packed_words(m_rows: usize, params: &GemmParams, seed: u32) -> Vec<u32> {
    let bytes_per_row = params.bytes_per_row as usize;
    let words_per_row = params.words_per_row as usize;
    let mut bytes = vec![0u8; m_rows * bytes_per_row];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (((i as u32).wrapping_mul(31).wrapping_add(seed.wrapping_mul(17))) % 243) as u8;
    }
    let mut words = vec![0u32; m_rows * words_per_row];
    pack_s13_bytes_to_words_slice(&bytes, bytes_per_row, words_per_row, m_rows, &mut words)
        .expect("packing");
    words
}

const N_CYCLES: usize = 42;
const REPS: usize = 5;

fn time_rounds<F: Fn(&mut wgpu::CommandEncoder)>(gpu: &Gpu, encode: F) -> f64 {
    // warmup
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    encode(&mut enc);
    gpu.queue.submit(std::iter::once(enc.finish()));
    let _ = gpu.device.poll(wgpu::Maintain::Wait);
    // best-of-REPS
    let mut best = f64::MAX;
    for _ in 0..REPS {
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        encode(&mut enc);
        let t0 = Instant::now();
        gpu.queue.submit(std::iter::once(enc.finish()));
        let _ = gpu.device.poll(wgpu::Maintain::Wait);
        let dt = t0.elapsed().as_secs_f64() * 1000.0;
        if dt < best {
            best = dt;
        }
    }
    best
}

fn main() {
    println!("===============================================================================");
    println!("   S13 GEMV DISPATCH-FLOOR PROBE (168 dispatches/round, one layer reused)");
    println!("===============================================================================");

    let Some(gpu) = boot_gpu() else {
        eprintln!("ABORT: no GPU adapter on this box — nothing measured, nothing claimed.");
        std::process::exit(2);
    };
    println!("  Adapter: {}", gpu.adapter_name);

    let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("s13_wgsl"),
        source: wgpu::ShaderSource::Wgsl(S13_WGSL_COMPUTE_SHADER.into()),
    });
    let pipe = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("s13_gemv_1d"),
        layout: None,
        module: &module,
        entry_point: Some("s13_gemv_1d"),
        compilation_options: Default::default(),
        cache: None,
    });

    let lut = gemma_s13::gpu_warden::trit_unpack_lut();
    let lut_buf = storage(&gpu, "trit-lut", (lut.len() * 4) as u64);
    gpu.queue
        .write_buffer(&lut_buf, 0, &lut.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());

    let d_ff: u32 = 14_336;
    // (label, m_rows, k_cols) — the 4 fused decode-stage shapes of one 9B layer
    let shapes: [(&str, u32, u32); 4] = [
        ("qkv", 8_192, 3_584),
        ("o", 3_584, 4_096),
        ("gateup", d_ff * 2, 3_584),
        ("down", 3_584, d_ff),
    ];

    // one layer's weights, reused every cycle (same DRAM traffic per round as
    // real decode only for the fraction not caught by L2 — stated, not hidden)
    let mut wbufs = Vec::new();
    let mut ubufs = Vec::new();
    let mut layer_mb = 0.0f64;
    for (si, (label, m, k)) in shapes.iter().enumerate() {
        let params = GemmParams::new_gemv(*m, *k, 10_000);
        let words = synth_packed_words(*m as usize, &params, si as u32);
        let bytes: Vec<u8> = words.iter().flat_map(|x| x.to_le_bytes()).collect();
        layer_mb += bytes.len() as f64 / 1e6;
        let wbuf = storage(&gpu, label, bytes.len() as u64);
        gpu.queue.write_buffer(&wbuf, 0, &bytes);
        wbufs.push(wbuf);
        ubufs.push(uniform(&gpu, label, &params));
    }
    println!("  One-layer weights resident : {:.1} MB x {} cycles = {:.2} GB nominal read/round",
        layer_mb, N_CYCLES, layer_mb * N_CYCLES as f64 / 1e3);

    let act_bytes = (d_ff as u64) * 2 * 4;
    let init_acts: Vec<u8> = (0..(d_ff as usize) * 2)
        .flat_map(|i| (((i * 13) % 2001) as i32 - 1000).to_le_bytes())
        .collect();

    let bind = |w: &wgpu::Buffer, src: &wgpu::Buffer, dst: &wgpu::Buffer, u: &wgpu::Buffer| {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: w.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: src.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dst.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: u.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: lut_buf.as_entire_binding() },
            ],
        })
    };

    // ── (A) hazard-chained, real decode wiring ──
    let layer_in = storage(&gpu, "a-layer_in", act_bytes);
    let qkv_out = storage(&gpu, "a-qkv_out", act_bytes);
    let o_out = storage(&gpu, "a-o_out", act_bytes);
    let gateup_out = storage(&gpu, "a-gateup_out", act_bytes);
    gpu.queue.write_buffer(&layer_in, 0, &init_acts);
    let chain: [(&wgpu::Buffer, &wgpu::Buffer); 4] = [
        (&layer_in, &qkv_out),
        (&qkv_out, &o_out),
        (&o_out, &gateup_out),
        (&gateup_out, &layer_in),
    ];
    let a_binds: Vec<wgpu::BindGroup> = (0..4)
        .map(|si| bind(&wbufs[si], chain[si].0, chain[si].1, &ubufs[si]))
        .collect();
    let a_ms = time_rounds(&gpu, |enc| {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipe);
        for _ in 0..N_CYCLES {
            for (si, (_, m, _)) in shapes.iter().enumerate() {
                pass.set_bind_group(0, &a_binds[si], &[]);
                pass.dispatch_workgroups(*m, 1, 1);
            }
        }
    });

    // ── (B) same 168 dispatches, hazard-free: constant input, private outputs ──
    let in_b = storage(&gpu, "b-in", act_bytes);
    gpu.queue.write_buffer(&in_b, 0, &init_acts);
    let mut b_binds = Vec::with_capacity(N_CYCLES * 4);
    for c in 0..N_CYCLES {
        for (si, (label, m, _)) in shapes.iter().enumerate() {
            let out = storage(&gpu, &format!("b-out-{c}-{label}"), (*m as u64) * 4);
            b_binds.push(bind(&wbufs[si], &in_b, &out, &ubufs[si]));
            std::mem::forget(out);
        }
    }
    let b_ms = time_rounds(&gpu, |enc| {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipe);
        let mut i = 0;
        for _ in 0..N_CYCLES {
            for (_, m, _) in shapes.iter() {
                pass.set_bind_group(0, &b_binds[i], &[]);
                pass.dispatch_workgroups(*m, 1, 1);
                i += 1;
            }
        }
    });

    // ── (C) hazard-chained near-empty: m=8 rows ping-pong, work ~0 ──
    let tiny = GemmParams::new_gemv(8, 3_584, 10_000);
    let tiny_words = synth_packed_words(8, &tiny, 99);
    let tiny_w = storage(&gpu, "c-w", (tiny_words.len() * 4) as u64);
    gpu.queue
        .write_buffer(&tiny_w, 0, &tiny_words.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());
    let tiny_u = uniform(&gpu, "c-u", &tiny);
    let ping = storage(&gpu, "c-ping", 3_584 * 4);
    let pong = storage(&gpu, "c-pong", 3_584 * 4);
    gpu.queue.write_buffer(&ping, 0, &init_acts[..3_584 * 4]);
    let c_binds = [bind(&tiny_w, &ping, &pong, &tiny_u), bind(&tiny_w, &pong, &ping, &tiny_u)];
    let c_ms = time_rounds(&gpu, |enc| {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipe);
        for i in 0..(N_CYCLES * 4) {
            pass.set_bind_group(0, &c_binds[i % 2], &[]);
            pass.dispatch_workgroups(8, 1, 1);
        }
    });

    let n = (N_CYCLES * 4) as f64;
    println!("  (A) chained, real wiring    : {a_ms:8.2} ms/round  ({:6.1} us/dispatch)", a_ms * 1000.0 / n);
    println!("  (B) hazard-free, same work  : {b_ms:8.2} ms/round  ({:6.1} us/dispatch)", b_ms * 1000.0 / n);
    println!("  (C) chained, near-empty     : {c_ms:8.2} ms/round  ({:6.1} us/dispatch)", c_ms * 1000.0 / n);
    println!("  A-B (serialization cost)    : {:8.2} ms/round", a_ms - b_ms);
    println!("  A-B-C interpretation: if C is large, the dispatch boundary itself is the");
    println!("  floor; if C is small but A>>B, hazard drains around real work are; if");
    println!("  A~=B, the work itself (waves x latency) is, and backend choice is moot.");
    println!("===============================================================================");
}
