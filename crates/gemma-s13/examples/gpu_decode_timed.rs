// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! MEASURED GPU decode timing for the S13 ternary GEMV kernel — the stopwatch
//! the projection never had. Uploads all 42 layers of synthetic packed 9B
//! weights (~1.55 GB, distinct VRAM buffers) and times full decode steps: all
//! 294 GEMV dispatches per token (q/k/v/o/gate/up/down × 42), chained on-device.
//! Host-side norm/attention-softmax are NOT included — this measures the
//! GEMV-dominant cost, stated as such. Parity vs the host WGSL simulator is
//! asserted on a small case before timing so the kernel is proven live first.

use gemma_s13::gpu_warden::{
    pack_s13_bytes_to_words_slice, simulate_s13_gemv_wgsl, trit_unpack_lut, GemmParams,
    S13_WGSL_COMPUTE_SHADER,
};
use gemma_s13::model_9b::Gemma9bConfig;
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
        // A3 subgroup diagnostic: names whether a subgroup-reduce rewrite is
        // even expressible on this box's wgpu backend before anyone chases it.
        let f = adapter.features();
        println!(
            "  Subgroups: adapter {} SUBGROUP (min..max size {}..{})",
            if f.contains(wgpu::Features::SUBGROUP) { "SUPPORTS" } else { "LACKS" },
            adapter.limits().min_subgroup_size,
            adapter.limits().max_subgroup_size
        );
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("s13-gpu-decode-timed"),
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

fn read_back_i32(gpu: &Gpu, src: &wgpu::Buffer, count: usize) -> Vec<i32> {
    let bytes = (count * 4) as u64;
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(src, 0, &staging, 0, bytes);
    gpu.queue.submit(std::iter::once(enc.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = gpu.device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel closed").expect("buffer map failed");
    let view = slice.get_mapped_range();
    let out: Vec<i32> = view
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    drop(view);
    staging.unmap();
    out
}

/// Synthetic S13 packed rows: every byte in 0..=242 (no sentinels), varied per row.
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

fn main() {
    println!("===============================================================================");
    println!("   S13 TERNARY GEMV — MEASURED GPU DECODE TIMING (not a projection)");
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
    // A1 A/B toggle: S13_KERNEL=4row selects the 4-rows-per-workgroup kernel
    // (ceil(m/4) dispatch); default stays the 1-row baseline.
    let four_row = std::env::var("S13_KERNEL").map(|v| v == "4row").unwrap_or(false);
    let entry = if four_row { "s13_gemv_4row" } else { "s13_gemv_1d" };
    let groups_of = |m: u32| if four_row { m.div_ceil(4) } else { m };
    println!("  Kernel: {entry}");
    let pipe = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    });

    // 243-entry trit unpack LUT (binding 4)
    let lut = trit_unpack_lut();
    let lut_buf = storage(&gpu, "trit-lut", (lut.len() * 4) as u64);
    gpu.queue
        .write_buffer(&lut_buf, 0, &lut.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());

    // ── [1] Live parity: GPU output must be bit-identical to the host WGSL simulator ──
    {
        let params = GemmParams::new_gemv(64, 320, 10_000);
        let words = synth_packed_words(64, &params, 7);
        let acts: Vec<i32> = (0..320).map(|i| ((i * 37) % 4001) - 2000).collect();

        let wbuf = storage(&gpu, "parity-w", (words.len() * 4) as u64);
        let abuf = storage(&gpu, "parity-a", (acts.len() * 4) as u64);
        let obuf = storage(&gpu, "parity-o", 64 * 4);
        let pbuf = uniform(&gpu, "parity-p", &params);
        let to_bytes_u32 = |v: &[u32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
        let to_bytes_i32 = |v: &[i32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
        gpu.queue.write_buffer(&wbuf, 0, &to_bytes_u32(&words));
        gpu.queue.write_buffer(&abuf, 0, &to_bytes_i32(&acts));

        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wbuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: abuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: obuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: pbuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: lut_buf.as_entire_binding() },
            ],
        });
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipe);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(groups_of(64), 1, 1); // per-kernel row grouping
        }
        gpu.queue.submit(std::iter::once(enc.finish()));

        let gpu_out = read_back_i32(&gpu, &obuf, 64);
        let mut cpu_out = vec![0i32; 64];
        simulate_s13_gemv_wgsl(&params, &words, &acts, &mut cpu_out).expect("simulator");
        assert_eq!(gpu_out, cpu_out, "GPU vs host-simulator parity FAILED");
        println!("  [1] Kernel parity on device : BIT-IDENTICAL (64 rows vs host simulator)");
    }

    // ── [2] Upload all 42 layers of synthetic packed 9B weights ──
    let cfg = Gemma9bConfig::default();
    let q_dim = (cfg.n_heads * cfg.d_head) as u32; // 4096
    let kv_dim = (cfg.n_kv_heads * cfg.d_head) as u32; // 2048
    let d_model = cfg.d_model as u32; // 3584
    let d_ff = cfg.d_ff as u32; // 14336

    // (label, m_rows, k_cols) for the 4 fused GEMV dispatches of one decode
    // layer: q/k/v share one input -> one concatenated matrix; gate/up likewise.
    let shapes: [(&str, u32, u32); 4] = [
        ("qkv", q_dim + kv_dim + kv_dim, d_model),
        ("o", d_model, q_dim),
        ("gateup", d_ff * 2, d_model),
        ("down", d_model, d_ff),
    ];

    let t_up = Instant::now();
    let mut total_bytes: u64 = 0;
    let mut layer_binds: Vec<Vec<(wgpu::BindGroup, u32)>> = Vec::with_capacity(cfg.n_layers);

    // dedicated per-stage buffers: q/k/v share only their INPUT, so they carry no
    // write-hazard between each other and can overlap; same for gate/up. The
    // serial dependency chain per layer is layer_in -> {q,k,v} -> o -> {gate,up}
    // -> down -> layer_in: 4 barriers instead of 7.
    let buf_of = |label: &str| storage(&gpu, label, (d_ff as u64) * 2 * 4);
    let layer_in = buf_of("layer_in");
    let qkv_out = buf_of("qkv_out");
    let o_out = buf_of("o_out");
    let gateup_out = buf_of("gateup_out");
    let init_acts: Vec<i32> = (0..d_ff as usize).map(|i| ((i * 13) % 2001) as i32 - 1000).collect();
    gpu.queue
        .write_buffer(&layer_in, 0, &init_acts.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());

    let params_bufs: Vec<wgpu::Buffer> = shapes
        .iter()
        .map(|(label, m, k)| uniform(&gpu, label, &GemmParams::new_gemv(*m, *k, 10_000)))
        .collect();

    for layer in 0..cfg.n_layers {
        let mut binds = Vec::with_capacity(7);
        for (si, (label, m, k)) in shapes.iter().enumerate() {
            let params = GemmParams::new_gemv(*m, *k, 10_000);
            let words = synth_packed_words(*m as usize, &params, (layer * 7 + si) as u32);
            let wbytes: Vec<u8> = words.iter().flat_map(|x| x.to_le_bytes()).collect();
            total_bytes += wbytes.len() as u64;
            let wbuf = storage(&gpu, &format!("L{layer}-{label}"), wbytes.len() as u64);
            gpu.queue.write_buffer(&wbuf, 0, &wbytes);
            let (src, dst): (&wgpu::Buffer, &wgpu::Buffer) = match si {
                0 => (&layer_in, &qkv_out),  // qkv reads layer input
                1 => (&qkv_out, &o_out),     // o reads q rows (first 4096 of qkv_out)
                2 => (&o_out, &gateup_out),  // gate+up read o output
                _ => (&gateup_out, &layer_in), // down reads gate rows (first 14336)
            };
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipe.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wbuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: src.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: dst.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: params_bufs[si].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: lut_buf.as_entire_binding() },
                ],
            });
            // buffer must outlive the bind group: bind group holds refs internally
            std::mem::forget(wbuf);
            binds.push((bind, groups_of(*m))); // workgroups per kernel's row grouping
        }
        layer_binds.push(binds);
    }
    let _ = gpu.device.poll(wgpu::Maintain::Wait);
    println!(
        "  [2] Weights resident in VRAM : {} layers, {:.3} GB uploaded in {:.1}s",
        cfg.n_layers,
        total_bytes as f64 / 1e9,
        t_up.elapsed().as_secs_f64()
    );

    // ── [3] Timed decode steps: 294 GEMV dispatches per token ──
    let decode_step = |gpu: &Gpu| {
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipe);
            for binds in &layer_binds {
                for (bind, groups) in binds {
                    pass.set_bind_group(0, bind, &[]);
                    pass.dispatch_workgroups(*groups, 1, 1);
                }
            }
        }
        gpu.queue.submit(std::iter::once(enc.finish()));
        let _ = gpu.device.poll(wgpu::Maintain::Wait);
    };

    decode_step(&gpu); // warmup + shader compile settle
    let n_tokens = 10;
    let t0 = Instant::now();
    for _ in 0..n_tokens {
        decode_step(&gpu);
    }
    let total_s = t0.elapsed().as_secs_f64();
    let ms_per_token = total_s * 1000.0 / n_tokens as f64;
    let tok_per_s = n_tokens as f64 / total_s;
    let weights_per_token: u64 = shapes.iter().map(|(_, m, k)| (*m as u64) * (*k as u64)).sum::<u64>()
        * cfg.n_layers as u64;
    let gweights_s = (weights_per_token as f64 * tok_per_s) / 1e9;

    println!("  [3] MEASURED decode timing  : {n_tokens} tokens, {:.2}s wall", total_s);
    println!("      • {:.2} ms/token  =>  {:.1} tokens/sec", ms_per_token, tok_per_s);
    println!("      • {:.1} Gweights/s effective ({} GEMV dispatches/token)", gweights_s, cfg.n_layers * shapes.len());
    println!("      SCOPE: all 294 per-token GEMVs on-device, weights VRAM-resident,");
    println!("      synthetic weights/activations chained on-GPU; host norm/attention");
    println!("      softmax excluded. This is the GEMV-dominant decode cost, measured.");
    println!("===============================================================================");
}
