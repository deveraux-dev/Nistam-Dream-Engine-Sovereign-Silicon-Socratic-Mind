//! CPU == GPU integer parity, in `cargo test` form.
//!
//! Drained 2026-07-31 from E:/.airgap/13forge/2026-06-02-1920-v9full/forge-kv-math/
//! tests/cpu_gpu_integer_parity.rs — the ONE artifact the July-26 snapshot lacks
//! (live tree and that snapshot hash SAME across lib/registry/seal/kernels/example,
//! so the harness form is the whole delta). `examples/determinism_proof.rs` already
//! proves these claims with fail-seed negative controls, but an example is a thing
//! you remember to run; this is a gate.
//!
//! Extended past the drained original by one claim: the EMULATED i64 kernel
//! (vec2<u32>, no SHADER_INT64) — the cross-vendor cornerstone, and the reason
//! integer-exact KV can be claimed on hardware we do not own. Same corpus the
//! example judges, same bytes to both sides.
//!
//! Every GREEN claim is paired with a RED guard proving the comparison is not
//! vacuous — a one-bit fault MUST change the verdict.
//!
//! cargo test -p forge-kv-math-v3 --test cpu_gpu_integer_parity --features gpu-proof

#![cfg(feature = "gpu-proof")]

use forge_kv_math_v3::registry::{entry, SemanticPrimitive};

const N_U32: usize = 4096; // multiple of workgroup_size (64)
const N_I64: usize = 256;
const DIVISOR: i64 = 10000; // Permyriad denominator — the canonical divide-back

// ── CPU references — MUST stay byte-equivalent to the WGSL ──────────────────

fn prismatic_hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1); // 2^32 / golden ratio
    h ^= y.wrapping_mul(0x85EB_CA77);
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 13;
    h
}

fn cpu_u32() -> Vec<u32> {
    (0..N_U32 as u32).map(|i| prismatic_hash(i, i ^ 0xABCD_1234)).collect()
}

/// Adversarial corpus: negatives, > i32::MAX, multiply-overflow (wrap), and
/// truncate-toward-zero divides. A kernel that is right only on small positives
/// passes nothing here.
fn i64_corpus() -> (Vec<i64>, Vec<i64>) {
    let mut a = Vec::with_capacity(N_I64);
    let mut b = Vec::with_capacity(N_I64);
    for i in 0..N_I64 as i64 {
        a.push((i - 128).wrapping_mul(1_000_003));
        b.push(match i % 8 {
            0 => 10_000,
            1 => -3,
            2 => i.wrapping_mul(987_654_321),
            3 => -10_001,
            4 => 7,
            5 => -1,
            6 => 2_147_483_648,
            _ => i - 200,
        });
    }
    (a, b)
}

fn cpu_i64(a: &[i64], b: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let prod: Vec<i64> = a.iter().zip(b).map(|(x, y)| x.wrapping_mul(*y)).collect();
    let quot: Vec<i64> = prod.iter().map(|p| p.wrapping_div(DIVISOR)).collect();
    (prod, quot)
}

/// FNV-1a over the raw bytes: one number stands in for the whole buffer, and a
/// single flipped bit anywhere changes it (which `*_red_guard` proves).
fn digest(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        h = (h ^ byte as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

// ── GPU plumbing (headless, no surface) ─────────────────────────────────────

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// `None` when the box has no adapter — the test then SKIPS loudly rather than
/// passing vacuously (a green that proves nothing is worse than a red).
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
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("cpu-gpu-integer-parity"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;
        Some(Gpu { device, queue })
    })
}

fn pipeline(gpu: &Gpu, label: &str, src: &str) -> wgpu::ComputePipeline {
    let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    });
    gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main_cs"),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn read_back(gpu: &Gpu, src: &wgpu::Buffer, bytes: u64) -> Vec<u8> {
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
    let out = view.to_vec();
    drop(view);
    staging.unmap();
    out
}

fn storage(gpu: &Gpu, label: &str, bytes: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn run(gpu: &Gpu, pipe: &wgpu::ComputePipeline, bufs: &[&wgpu::Buffer], groups: u32) {
    let entries: Vec<wgpu::BindGroupEntry> = bufs
        .iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() })
        .collect();
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipe);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    gpu.queue.submit(std::iter::once(enc.finish()));
}

// ── GREEN: u32 prismatic_hash is bit-identical CPU vs GPU ───────────────────
// [BOARD: KV-MATH-INTEGER-PARITY]
#[test]
fn u32_hash_is_bit_identical_on_cpu_and_gpu() {
    let Some(gpu) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };
    let src = entry(SemanticPrimitive::PrismaticHashU32).kernel_src.as_single_src();
    let pipe = pipeline(&gpu, "prismatic_hash_u32", src);
    let bytes = (N_U32 * 4) as u64;
    let out = storage(&gpu, "u32-storage", bytes);
    run(&gpu, &pipe, &[&out], (N_U32 / 64) as u32);

    let gpu_bytes = read_back(&gpu, &out, bytes);
    let cpu_bytes: Vec<u8> = cpu_u32().iter().flat_map(|w| w.to_le_bytes()).collect();
    assert_eq!(
        digest(&cpu_bytes),
        digest(&gpu_bytes),
        "u32 integer parity FAILED — output is not bit-identical"
    );
}

// ── GREEN: emulated i64 (vec2<u32>) — no SHADER_INT64, runs anywhere ────────
// [BOARD: KV-MATH-INTEGER-PARITY]
#[test]
fn emulated_i64_mul_div_is_bit_identical_on_cpu_and_gpu() {
    let Some(gpu) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };
    let src = entry(SemanticPrimitive::PermyriadMulDivI64).kernel_src.as_emulated_src();
    let pipe = pipeline(&gpu, "permyriad_mul_div_i64_emu", src);

    let (a, b) = i64_corpus();
    let bytes = (N_I64 * 8) as u64;
    let (ba, bb) = (storage(&gpu, "a", bytes), storage(&gpu, "b", bytes));
    let (bp, bq) = (storage(&gpu, "prod", bytes), storage(&gpu, "quot", bytes));
    let to_bytes = |v: &[i64]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    gpu.queue.write_buffer(&ba, 0, &to_bytes(&a));
    gpu.queue.write_buffer(&bb, 0, &to_bytes(&b));
    run(&gpu, &pipe, &[&ba, &bb, &bp, &bq], (N_I64 / 64) as u32);

    let (cpu_prod, cpu_quot) = cpu_i64(&a, &b);
    assert_eq!(
        digest(&to_bytes(&cpu_prod)),
        digest(&read_back(&gpu, &bp, bytes)),
        "emulated i64 MULTIPLY diverged — wide-integer wrap is not bit-identical"
    );
    assert_eq!(
        digest(&to_bytes(&cpu_quot)),
        digest(&read_back(&gpu, &bq, bytes)),
        "emulated i64 DIVIDE-BACK diverged — truncation toward zero is not bit-identical"
    );
}

// ── RED: the comparison must catch a one-bit fault ──────────────────────────
// [BOARD: KV-MATH-INTEGER-PARITY]
#[test]
fn one_bit_fault_is_detected_by_the_digest() {
    let clean = cpu_u32();
    let mut tampered = clean.clone();
    tampered[N_U32 / 2] ^= 1;
    let bytes = |v: &[u32]| -> Vec<u8> { v.iter().flat_map(|w| w.to_le_bytes()).collect() };
    assert_ne!(
        digest(&bytes(&clean)),
        digest(&bytes(&tampered)),
        "RED guard broken — a one-bit change did NOT change the digest, so GREEN proves nothing"
    );

    let (a, b) = i64_corpus();
    let (prod, _) = cpu_i64(&a, &b);
    let mut bad = prod.clone();
    bad[0] = bad[0].wrapping_add(1); // the smallest possible i64 fault
    let i64_bytes = |v: &[i64]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    assert_ne!(digest(&i64_bytes(&prod)), digest(&i64_bytes(&bad)), "RED guard broken on the i64 lane");
}
