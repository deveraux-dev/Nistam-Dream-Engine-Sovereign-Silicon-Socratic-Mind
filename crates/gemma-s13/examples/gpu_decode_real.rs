// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! MEASURED GPU decode on REAL quantized Gemma weights (.s13m files from
//! `quantize-s13 pack-gemma`, geometry auto-detected from S13M headers): loads
//! every blk_N layer, realigns rows via base-243 digit shifts, fuses qkv and
//! gate+up, asserts host-simulator parity on real rows, then times full decode
//! steps (layers x 4 dispatches). Dir: S13_GEMMA_DIR env (or ./s13_gemma).

use gemma_s13::gpu_warden::{
    pack_s13_bytes_to_words_slice, simulate_s13_gemv_wgsl, trit_unpack_lut, GemmParams,
    S13_WGSL_COMPUTE_SHADER,
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
                    label: Some("s13-gpu-decode-real"),
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

/// One `.s13m` matrix: `S13M` magic, out/in features, per-tensor f32 scale,
/// then `(out*in+4)/5` continuously packed base-243 trit bytes.
struct S13m {
    out_f: usize,
    in_f: usize,
    scale: f32,
    packed: Vec<u8>,
}

fn load_s13m(path: &std::path::Path) -> S13m {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(bytes.len() >= 16 && &bytes[0..4] == b"S13M", "{}: bad S13M header", path.display());
    let out_f = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let in_f = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let scale = f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let expected = (out_f * in_f + 4) / 5;
    assert_eq!(bytes.len() - 16, expected, "{}: payload length mismatch", path.display());
    S13m { out_f, in_f, scale, packed: bytes[16..].to_vec() }
}

const POW3: [u32; 5] = [1, 3, 9, 27, 81];

/// Continuous flat packing -> row-aligned `bytes_per_row` layout via base-243
/// digit shifts: new byte j of row r merges the high digits of source byte
/// b0+j with the low digits of b0+j+1 (b0 = row start / 5, k = row start % 5).
/// Rows already byte-aligned (`in_f % 5 == 0`, k always 0) copy straight through.
fn realign_rows(m: &S13m, bytes_per_row: usize) -> Vec<u8> {
    let mut out = vec![0u8; m.out_f * bytes_per_row];
    for r in 0..m.out_f {
        let t0 = r * m.in_f;
        let b0 = t0 / 5;
        let k = t0 % 5;
        let row = &mut out[r * bytes_per_row..(r + 1) * bytes_per_row];
        if k == 0 {
            let end = (b0 + bytes_per_row).min(m.packed.len());
            row[..end - b0].copy_from_slice(&m.packed[b0..end]);
        } else {
            let lo_div = POW3[k];
            let hi_mul = POW3[5 - k];
            for (j, slot) in row.iter_mut().enumerate() {
                let lo = *m.packed.get(b0 + j).unwrap_or(&0) as u32;
                let hi = *m.packed.get(b0 + j + 1).unwrap_or(&0) as u32;
                *slot = ((lo / lo_div) + (hi % lo_div) * hi_mul) as u8;
            }
        }
    }
    // spot-check the shift: 97 strided sites must decode identically pre/post
    for s in 0..97u64 {
        let r = ((s * 1009) % m.out_f as u64) as usize;
        let c = ((s * 2003) % m.in_f as u64) as usize;
        let flat = r * m.in_f + c;
        let src_d = (m.packed[flat / 5] as u32 / POW3[flat % 5]) % 3;
        let dst_d = (out[r * bytes_per_row + c / 5] as u32 / POW3[c % 5]) % 3;
        assert_eq!(src_d, dst_d, "realign digit mismatch at row {r} col {c}");
    }
    out
}

/// Word-packs a row-aligned byte matrix for the GPU storage buffer.
fn to_words(bytes: &[u8], params: &GemmParams, rows: usize) -> Vec<u32> {
    let mut words = vec![0u32; rows * params.words_per_row as usize];
    pack_s13_bytes_to_words_slice(
        bytes,
        params.bytes_per_row as usize,
        params.words_per_row as usize,
        rows,
        &mut words,
    )
    .expect("packing");
    words
}

fn main() {
    println!("===============================================================================");
    println!("   S13 TERNARY GEMV — MEASURED GPU DECODE, REAL QUANTIZED GEMMA WEIGHTS");
    println!("===============================================================================");

    let dir = std::env::var("S13_GEMMA_DIR").unwrap_or_else(|_| "s13_gemma".to_string());
    let dir = std::path::PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("ABORT: weight dir {} not found — set S13_GEMMA_DIR to the", dir.display());
        eprintln!("       output of `quantize-s13 pack-gemma` (blk_N_*.s13m per layer).");
        std::process::exit(2);
    }

    // geometry from the S13M headers themselves: count blk_N layers, read dims
    let mut n_layers = 0usize;
    while dir.join(format!("blk_{n_layers}_attn_q_weight.s13m")).is_file() {
        n_layers += 1;
    }
    if n_layers == 0 {
        eprintln!("ABORT: no blk_0_attn_q_weight.s13m in {}", dir.display());
        std::process::exit(2);
    }
    let q0 = load_s13m(&dir.join("blk_0_attn_q_weight.s13m"));
    let k0 = load_s13m(&dir.join("blk_0_attn_k_weight.s13m"));
    let up0 = load_s13m(&dir.join("blk_0_ffn_up_weight.s13m"));
    let (q_dim, d_model, kv_dim, d_ff) = (q0.out_f, q0.in_f, k0.out_f, up0.out_f);
    assert_eq!(k0.in_f, d_model, "attn_k in_features disagrees with attn_q");
    assert_eq!(up0.in_f, d_model, "ffn_up in_features disagrees with attn_q");

    let Some(gpu) = boot_gpu() else {
        eprintln!("ABORT: no GPU adapter on this box — nothing measured, nothing claimed.");
        std::process::exit(2);
    };
    println!("  Adapter: {}", gpu.adapter_name);
    println!("  Geometry (S13M headers)      : {n_layers} layers, d_model {d_model}, q {q_dim}, kv {kv_dim}, d_ff {d_ff}");
    let stage_params: [GemmParams; 4] = [
        GemmParams::new_gemv((q_dim + 2 * kv_dim) as u32, d_model as u32, 10_000),
        GemmParams::new_gemv(d_model as u32, q_dim as u32, 10_000),
        GemmParams::new_gemv((d_ff * 2) as u32, d_model as u32, 10_000),
        GemmParams::new_gemv(d_model as u32, d_ff as u32, 10_000),
    ];

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
    let lut = trit_unpack_lut();
    let lut_buf = storage(&gpu, "trit-lut", (lut.len() * 4) as u64);
    gpu.queue
        .write_buffer(&lut_buf, 0, &lut.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());

    // ── [1] Load, verify, fuse, upload all 34 layers ──
    let t_up = Instant::now();
    let mut total_disk_bytes = 0usize;
    let mut sentinel_bytes = 0usize;
    let mut scale_min = f32::MAX;
    let mut scale_max = f32::MIN;
    let mut parity_words: Option<Vec<u32>> = None;
    let mut layer_wbufs: Vec<[wgpu::Buffer; 4]> = Vec::with_capacity(n_layers);

    for layer in 0..n_layers {
        let load = |tensor: &str, want_out: usize, want_in: usize| -> S13m {
            let p = dir.join(format!("blk_{layer}_{tensor}_weight.s13m"));
            let m = load_s13m(&p);
            assert_eq!(
                (m.out_f, m.in_f),
                (want_out, want_in),
                "{}: header dims [{}x{}] != expected [{want_out}x{want_in}]",
                p.display(), m.out_f, m.in_f
            );
            m
        };
        let q = load("attn_q", q_dim, d_model);
        let k = load("attn_k", kv_dim, d_model);
        let v = load("attn_v", kv_dim, d_model);
        let o = load("attn_output", d_model, q_dim);
        let gate = load("ffn_gate", d_ff, d_model);
        let up = load("ffn_up", d_ff, d_model);
        let down = load("ffn_down", d_model, d_ff);

        for m in [&q, &k, &v, &o, &gate, &up, &down] {
            total_disk_bytes += m.packed.len();
            sentinel_bytes += m.packed.iter().filter(|&&b| b >= 243).count();
            scale_min = scale_min.min(m.scale);
            scale_max = scale_max.max(m.scale);
        }

        // every tensor is realigned to kernel rows (identity copy when in_f is
        // 5-divisible), then qkv and gate+up fuse by row concatenation.
        let mut qkv_bytes = realign_rows(&q, stage_params[0].bytes_per_row as usize);
        qkv_bytes.extend_from_slice(&realign_rows(&k, stage_params[0].bytes_per_row as usize));
        qkv_bytes.extend_from_slice(&realign_rows(&v, stage_params[0].bytes_per_row as usize));
        let o_bytes = realign_rows(&o, stage_params[1].bytes_per_row as usize);
        let mut gu_bytes = realign_rows(&gate, stage_params[2].bytes_per_row as usize);
        gu_bytes.extend_from_slice(&realign_rows(&up, stage_params[2].bytes_per_row as usize));
        let down_bytes = realign_rows(&down, stage_params[3].bytes_per_row as usize);

        let stage_bytes: [(&[u8], usize); 4] = [
            (&qkv_bytes, q_dim + 2 * kv_dim),
            (&o_bytes, d_model),
            (&gu_bytes, d_ff * 2),
            (&down_bytes, d_model),
        ];
        let bufs: [wgpu::Buffer; 4] = std::array::from_fn(|si| {
            let (bytes, rows) = stage_bytes[si];
            let words = to_words(bytes, &stage_params[si], rows);
            if layer == 0 && si == 0 {
                parity_words = Some(words.clone());
            }
            let buf = storage(&gpu, &format!("L{layer}-s{si}"), (words.len() * 4) as u64);
            gpu.queue
                .write_buffer(&buf, 0, &words.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());
            buf
        });
        layer_wbufs.push(bufs);
    }
    let _ = gpu.device.poll(wgpu::Maintain::Wait);
    println!(
        "  [1] REAL weights resident    : {n_layers} layers, {:.1} MB packed on disk, {:.1}s load+upload",
        total_disk_bytes as f64 / 1e6,
        t_up.elapsed().as_secs_f64()
    );
    println!(
        "      sentinel bytes (>=243)   : {sentinel_bytes} (must be 0); per-tensor scale range {scale_min:.6}..{scale_max:.6}"
    );
    assert_eq!(sentinel_bytes, 0, "sentinel byte found in real quantized weights");

    // ── [2] Parity on REAL rows: first 64 rows of blk_0 fused qkv ──
    {
        let words_per_row = stage_params[0].words_per_row as usize;
        let words = &parity_words.as_ref().expect("blk_0 qkv words")[..64 * words_per_row];
        let params = GemmParams::new_gemv(64, d_model as u32, 10_000);
        let acts: Vec<i32> = (0..d_model).map(|i| ((i * 37) % 4001) as i32 - 2000).collect();

        let wbuf = storage(&gpu, "parity-w", (words.len() * 4) as u64);
        let abuf = storage(&gpu, "parity-a", (acts.len() * 4) as u64);
        let obuf = storage(&gpu, "parity-o", 64 * 4);
        let pbuf = uniform(&gpu, "parity-p", &params);
        gpu.queue
            .write_buffer(&wbuf, 0, &words.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());
        gpu.queue
            .write_buffer(&abuf, 0, &acts.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());
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
            pass.dispatch_workgroups(64, 1, 1);
        }
        gpu.queue.submit(std::iter::once(enc.finish()));
        let gpu_out = read_back_i32(&gpu, &obuf, 64);
        let mut cpu_out = vec![0i32; 64];
        simulate_s13_gemv_wgsl(&params, words, &acts, &mut cpu_out).expect("simulator");
        assert_eq!(gpu_out, cpu_out, "GPU vs host-simulator parity FAILED on real weights");
        println!("  [2] Parity on REAL rows      : BIT-IDENTICAL (64 rows of blk_0 qkv vs host simulator)");
    }

    // ── [3] Timed decode: 136 GEMV dispatches/token (34 layers x 4 stages) ──
    let act_bytes = (d_ff as u64) * 2 * 4;
    let layer_in = storage(&gpu, "layer_in", act_bytes);
    let qkv_out = storage(&gpu, "qkv_out", act_bytes);
    let o_out = storage(&gpu, "o_out", act_bytes);
    let gateup_out = storage(&gpu, "gateup_out", act_bytes);
    let init_acts: Vec<u8> = (0..(d_ff * 2))
        .flat_map(|i| (((i * 13) % 2001) as i32 - 1000).to_le_bytes())
        .collect();
    gpu.queue.write_buffer(&layer_in, 0, &init_acts);
    let chain: [(&wgpu::Buffer, &wgpu::Buffer); 4] = [
        (&layer_in, &qkv_out),
        (&qkv_out, &o_out),
        (&o_out, &gateup_out),
        (&gateup_out, &layer_in),
    ];
    let ubufs: Vec<wgpu::Buffer> = stage_params
        .iter()
        .enumerate()
        .map(|(si, p)| uniform(&gpu, &format!("u{si}"), p))
        .collect();
    let mut binds: Vec<(wgpu::BindGroup, u32)> = Vec::with_capacity(n_layers * 4);
    for bufs in &layer_wbufs {
        for si in 0..4 {
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipe.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: bufs[si].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: chain[si].0.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: chain[si].1.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: ubufs[si].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: lut_buf.as_entire_binding() },
                ],
            });
            binds.push((bind, stage_params[si].m_rows));
        }
    }
    let decode_step = |gpu: &Gpu| {
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipe);
            for (bind, groups) in &binds {
                pass.set_bind_group(0, bind, &[]);
                pass.dispatch_workgroups(*groups, 1, 1);
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
    let weights_per_token: u64 =
        (n_layers as u64) * ((q_dim + 2 * kv_dim + d_ff * 2) as u64 * d_model as u64
            + (d_model * q_dim) as u64 + (d_model * d_ff) as u64);
    let gweights_s = (weights_per_token as f64 * tok_per_s) / 1e9;
    let roofline = 448.0e9 / total_disk_bytes as f64;

    println!("  [3] MEASURED GEMV-pass timing : {n_tokens} passes, {:.2}s wall", total_s);
    println!("      • {:.2} ms/GEMV-pass  =>  {:.1} GEMV passes/sec", ms_per_token, tok_per_s);
    println!("      • {:.1} Gweights/s effective ({} GEMV dispatches/pass)", gweights_s, n_layers * 4);
    println!("      SCOPE — read this before quoting the number above:");
    println!("      This is NOT tokens/sec. One 'pass' is every per-token GEMV on-device over");
    println!("      the REAL quantized weights, VRAM-resident, activations chained on-GPU.");
    println!("      EXCLUDED: host norm/attention softmax, per-tensor scale application.");
    println!("      A full-pipeline engine (vLLM, llama.cpp) measures strictly more work,");
    println!("      so comparing this figure against theirs overstates this engine.");
    println!("      MEASUREMENT VALIDITY: lock clocks before quoting —");
    println!("        nvidia-smi -pm 1 -lgc 2100 -lmc 7001   (restore: -rgc -rmc)");
    println!("      An unlocked card idles in P8 (~10% core, ~6% mem) through a burst this");
    println!("      short and the driver never ramps; run-to-run spread of 2x is then pure");
    println!("      power-state hysteresis, not throughput. Record pstate/clocks WITH the run.");
    println!("      BANDWIDTH ROOFLINE ({:.1} MB/pass at 448 GB/s = {:.0}/s) IS NOT THE BINDING", total_disk_bytes as f64 / 1e6, roofline);
    println!("      CONSTRAINT — measured 2026-08-27: locking memory 405->7001 MHz (17x) moved");
    println!("      throughput DOWN, so this kernel is launch/reduction-bound, not bandwidth-");
    println!("      bound. The tail at s13_gemv_1d is a serial 128-step reduce on one thread.");
    println!("===============================================================================");
}
