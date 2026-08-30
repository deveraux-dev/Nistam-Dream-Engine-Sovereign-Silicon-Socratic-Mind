//! PROPRIETARY & CONFIDENTIAL / TRADE SECRET — do NOT open-source (see README).
//! First concrete evidence for Invention #156 (Integer-Exact KV Cache for
//! Deterministic Inference): wide-integer GPU math is BIT-DETERMINISTIC across
//! the CPU<->GPU boundary. The novel part is the i64 path (esp. the vec2<u32>
//! emulation); the public u32-only demonstrator lives separately and stays public.
//!
//! Determinism proof: integer-only kernels produce BIT-IDENTICAL output on
//! CPU (native Rust, x86_64) and GPU (WGSL -> naga -> SPIR-V -> Vulkan).
//!
//! Three claims, one binary:
//!   1. u32 prismatic_hash             — core integer hash (invention #7 substrate).
//!   2. i64 native   (mul + div-back)  — WGSL `i64`, behind Features::SHADER_INT64.
//!   3. i64 emulated (mul + div-back)  — i64 as vec2<u32>, core-u32 only. Portable
//!      to ANY GPU (no SHADER_INT64). The cross-vendor cornerstone.
//!
//! Exit 0 iff every ENABLED claim is bit-identical. Any divergence -> FIRST DIFF
//! line + exit 1. A claim needing absent hardware is SKIPPED (reported loudly),
//! never silently counted as a pass.
//!
//! Reproducible negative control: `--fail-seed <u32|native-mul|native-div|emu-mul|
//! emu-div>` injects one known fault into a kernel copy and exits 0 only if the
//! harness CATCHES it (targeted claim goes RED). Every claim's RED is a runnable
//! command, per op — an audit can never silently inherit from another claim.
//!
//! invention #7 = Integer-Only Deterministic Kernel (MilliUnit i64 + Permyriad).
//! The i64 multiply-then-divide-back (`pos * ratio / 10000`) is the load-bearing op.
//!
//! Ported 2026-07-20 from E:/.airgap/milestones/13forge-consolidation-2026-06-15
//! (tractor-beam S4-promote SHORTLIST diamond `fail_seed`) — verbatim, since
//! registry.rs (SEMANTIC-CODEPOINT-KERNEL-BRIDGE) already lived with a matching
//! API. Gated behind the `gpu-proof` feature Cargo.toml already stubbed for it.
//! v3 port 2026-08-16 from F:\NewRepo\crates\forge-kv-math, verbatim.
//! Run: `cargo run -p forge-kv-math-v3 --example determinism_proof --features gpu-proof`

// Bridge: semantic-codepoint-kernel registry (SEMANTIC-CODEPOINT-KERNEL-BRIDGE-001/002).
// Labels use stable SemanticPrimitive enum IDs (PUA codepoints as discriminants).
// Kernel sources come from the registry's include_str!-validated KernelSrc fields —
// no local SRC_* consts needed; the registry is the sole owner of kernel text.
use forge_kv_math_v3::{fnv1a, registry::{entry, SemanticPrimitive, REGISTRY}};

use std::process::exit;

const N_U32: usize = 4096; // multiple of workgroup_size (64)
const N_I64: usize = 256; //  multiple of workgroup_size (64)
const DIVISOR: i64 = 10000; // Permyriad denominator — the canonical divide-back

// Claim labels from stable SemanticPrimitive enum IDs, not registry array positions.
// SemanticPrimitive::X.name() is a const fn — evaluated at compile time.
const LABEL_U32: &str       = SemanticPrimitive::PrismaticHashU32.name();
const LABEL_I64_NATIVE: &str = "permyriad_mul_div_i64 [native]";    // variant label
const LABEL_I64_EMU: &str   = "permyriad_mul_div_i64 [emulated]";   // variant label
const LABEL_CODEPOINT: &str = SemanticPrimitive::StatCodepointPermyriad.name();

// ============================ CPU references =================================

/// Integer-only, wrapping ops. MUST stay byte-equivalent to kernel.wgsl.
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

/// Adversarial i64 corpus: negatives, > i32::MAX, multiply-overflow (wrap), and
/// truncate-toward-zero divide cases. CPU and GPU consume the SAME bytes.
fn i64_corpus() -> (Vec<i64>, Vec<i64>) {
    let mut a = Vec::with_capacity(N_I64);
    let mut b = Vec::with_capacity(N_I64);
    for i in 0..N_I64 as i64 {
        a.push((i - 128).wrapping_mul(1_000_003)); // straddles 0, up to ~1.3e8
        b.push(match i % 8 {
            0 => 10_000,
            1 => -3,                          // negative multiplier
            2 => i.wrapping_mul(987_654_321), // large -> i64 multiply overflow (wrap)
            3 => -10_001,                     // (a*b)/10000 -> -1, truncates toward zero
            4 => 7,
            5 => -1,
            6 => 2_147_483_648,               // > i32::MAX
            _ => i - 200,
        });
    }
    (a, b)
}

/// Corpus for the `stat_codepoint_permyriad` claim.
///
/// `a[]` values are FNV-1a codepoints of 16 canonical semantic key names, cast to
/// i64. High-bit-set u64 values land in the negative i64 range, so the corpus
/// naturally covers both positive and negative operands without separate adversarial
/// construction. `b[]` varies the ratio near the Permyriad denominator (10000 - i%100).
/// 16 names × 16 repetitions = 256 = N_I64 (multiple of workgroup_size 64).
fn codepoint_corpus() -> (Vec<i64>, Vec<i64>) {
    const KEYS: &[&[u8]] = &[
        b"hp_max",          b"mana_max",       b"gravity_mm",     b"tick_rate",
        b"era_index",       b"stamina",        b"armor_base",      b"speed_mm",
        b"strength",        b"dexterity",      b"intelligence",    b"luck",
        b"fire_resist",     b"cold_resist",    b"lightning_resist", b"void_resist",
    ];
    let mut a = Vec::with_capacity(N_I64);
    let mut b = Vec::with_capacity(N_I64);
    for i in 0..N_I64 {
        a.push(fnv1a(KEYS[i % KEYS.len()]) as i64);
        b.push(10_000i64 - (i as i64 % 100));
    }
    (a, b)
}

/// CPU reference for the i64 claim. wrapping_* so overflow WRAPS (matches WGSL)
/// and never panics under overflow-checks=true; div by +10000 truncates toward 0.
fn cpu_i64(a: &[i64], b: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let prod: Vec<i64> = a.iter().zip(b).map(|(x, y)| x.wrapping_mul(*y)).collect();
    let quot: Vec<i64> = prod.iter().map(|p| p.wrapping_div(DIVISOR)).collect();
    (prod, quot)
}

// ============================ GPU plumbing ===================================

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    has_int64: bool,
}

fn boot_gpu() -> Gpu {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("no GPU adapter found");
        let info = adapter.get_info();
        eprintln!("[gpu] adapter: {} | {:?} | backend {:?}", info.name, info.device_type, info.backend);

        let has_int64 = adapter.features().contains(wgpu::Features::SHADER_INT64);
        let required_features = if has_int64 {
            wgpu::Features::SHADER_INT64
        } else {
            wgpu::Features::empty()
        };
        eprintln!("[gpu] SHADER_INT64 available: {has_int64}");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("determinism-proof"),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("failed to create device");
        device.on_uncaptured_error(Box::new(|e| eprintln!("[gpu] UNCAPTURED ERROR: {e}")));

        Gpu { device, queue, has_int64 }
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

/// Map + copy out. Casts the mapped VIEW (8-aligned per wgpu MAP_ALIGNMENT) — do
/// NOT cast an intermediate Vec<u8>, whose pointer is not guaranteed 8-aligned.
fn read_back<T: bytemuck::Pod>(gpu: &Gpu, buf: &wgpu::Buffer) -> Vec<T> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = gpu.device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel closed").expect("buffer map failed");
    let view = slice.get_mapped_range();
    let out = bytemuck::cast_slice::<u8, T>(&view).to_vec();
    drop(view);
    buf.unmap();
    out
}

fn storage_buffer(gpu: &Gpu, name: &str, bytes: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(name),
        size: bytes,
        usage,
        mapped_at_creation: false,
    })
}

/// u32 claim: the kernel derives each value from its invocation index (no inputs).
fn run_u32(gpu: &Gpu, src: &str) -> Vec<u32> {
    let pipe = pipeline(gpu, "u32-prismatic", src);
    let bytes = (N_U32 * 4) as u64;
    let storage = storage_buffer(gpu, "u32-storage", bytes, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
    let staging = storage_buffer(gpu, "u32-staging", bytes, wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: storage.as_entire_binding() }],
    });
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((N_U32 as u32) / 64, 1, 1);
    }
    enc.copy_buffer_to_buffer(&storage, 0, &staging, 0, bytes);
    gpu.queue.submit(std::iter::once(enc.finish()));
    read_back::<u32>(gpu, &staging)
}

/// i64 claim (native or emulated kernel): 2 input + 2 output i64 buffers. The
/// input bytes are identical for both kernels (the native one reads them as i64,
/// the emulated one as vec2<u32>); CPU consumes the same `a`/`b`.
fn run_i64(gpu: &Gpu, label: &str, src: &str, a: &[i64], b: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let pipe = pipeline(gpu, label, src);
    let bytes = (a.len() * 8) as u64;
    let buf_a = storage_buffer(gpu, "a", bytes, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
    let buf_b = storage_buffer(gpu, "b", bytes, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
    let buf_prod = storage_buffer(gpu, "prod", bytes, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
    let buf_quot = storage_buffer(gpu, "quot", bytes, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
    let stg_prod = storage_buffer(gpu, "prod-stg", bytes, wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
    let stg_quot = storage_buffer(gpu, "quot-stg", bytes, wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);

    gpu.queue.write_buffer(&buf_a, 0, bytemuck::cast_slice(a));
    gpu.queue.write_buffer(&buf_b, 0, bytemuck::cast_slice(b));

    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buf_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buf_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: buf_prod.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: buf_quot.as_entire_binding() },
        ],
    });
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((a.len() as u32) / 64, 1, 1);
    }
    enc.copy_buffer_to_buffer(&buf_prod, 0, &stg_prod, 0, bytes);
    enc.copy_buffer_to_buffer(&buf_quot, 0, &stg_quot, 0, bytes);
    gpu.queue.submit(std::iter::once(enc.finish()));

    let prod = read_back::<i64>(gpu, &stg_prod);
    let quot = read_back::<i64>(gpu, &stg_quot);
    (prod, quot)
}

// ============================ comparison =====================================

enum Status {
    Pass,
    Skip(String),
    Fail(String),
}

fn i64_verdict(
    tag: &str,
    a: &[i64],
    b: &[i64],
    prod_cpu: &[i64],
    quot_cpu: &[i64],
    prod_g: &[i64],
    quot_g: &[i64],
) -> Status {
    if let Some(i) = (0..prod_cpu.len()).find(|&i| prod_cpu[i] != prod_g[i]) {
        eprintln!("[{tag}] PROD FIRST DIFF @ {i}: a={} b={} cpu={} gpu={}", a[i], b[i], prod_cpu[i], prod_g[i]);
        return Status::Fail(format!("prod diff @ {i}"));
    }
    if let Some(i) = (0..quot_cpu.len()).find(|&i| quot_cpu[i] != quot_g[i]) {
        eprintln!("[{tag}] QUOT FIRST DIFF @ {i}: a={} b={} cpu={} gpu={}", a[i], b[i], quot_cpu[i], quot_g[i]);
        return Status::Fail(format!("quot diff @ {i}"));
    }
    eprintln!("[{tag}] mul + divide-back bit-identical across {} elems", prod_cpu.len());
    Status::Pass
}

/// Run all claims against the GIVEN kernel sources. Sources are parameters
/// (not hard-coded) so the negative-control mode can pass faulted copies through
/// the identical pipeline.
fn run_claims(gpu: &Gpu, src_u32: &str, src_native: &str, src_emu: &str) -> Vec<(&'static str, Status)> {
    let mut results: Vec<(&'static str, Status)> = Vec::new();

    // Claim 1: REGISTRY[0] — prismatic_hash_u32
    {
        let cpu = cpu_u32();
        let g = run_u32(gpu, src_u32);
        let st = if let Some(i) = (0..cpu.len()).find(|&i| cpu[i] != g[i]) {
            eprintln!("[{LABEL_U32}] FIRST DIFF @ {i}: cpu={:08x} gpu={:08x}", cpu[i], g[i]);
            Status::Fail(format!("first diff @ {i}"))
        } else {
            eprintln!("[{LABEL_U32}] cpu[0..2]={:08x} {:08x} == gpu (bit-identical, {} elems)", cpu[0], cpu[1], cpu.len());
            Status::Pass
        };
        results.push((LABEL_U32, st));
    }

    let (a, b) = i64_corpus();
    let (prod_cpu, quot_cpu) = cpu_i64(&a, &b);

    // Claim 2: REGISTRY[1] native path — permyriad_mul_div_i64 [native]
    {
        let st = if !gpu.has_int64 {
            eprintln!("[{LABEL_I64_NATIVE}] SKIP — adapter lacks Features::SHADER_INT64");
            Status::Skip("adapter lacks Features::SHADER_INT64".into())
        } else {
            let (prod_g, quot_g) = run_i64(gpu, LABEL_I64_NATIVE, src_native, &a, &b);
            i64_verdict(LABEL_I64_NATIVE, &a, &b, &prod_cpu, &quot_cpu, &prod_g, &quot_g)
        };
        results.push((LABEL_I64_NATIVE, st));
    }

    // Claim 3: REGISTRY[1] emulated path — permyriad_mul_div_i64 [emulated]
    {
        let (prod_g, quot_g) = run_i64(gpu, LABEL_I64_EMU, src_emu, &a, &b);
        let st = i64_verdict(LABEL_I64_EMU, &a, &b, &prod_cpu, &quot_cpu, &prod_g, &quot_g);
        results.push((LABEL_I64_EMU, st));
    }

    // Claim 4: REGISTRY[2] — stat_codepoint_permyriad
    // FNV-1a codepoints of canonical semantic key names through the emulated i64 kernel.
    // Proves that values the semantic authority layer actually produces (FNV-1a u64
    // codepoints cast to i64) are safe operands for GPU Permyriad arithmetic.
    {
        let (cp_a, cp_b) = codepoint_corpus();
        let (cp_prod_cpu, cp_quot_cpu) = cpu_i64(&cp_a, &cp_b);
        let (cp_prod_g, cp_quot_g) = run_i64(gpu, LABEL_CODEPOINT, src_emu, &cp_a, &cp_b);
        let st = i64_verdict(LABEL_CODEPOINT, &cp_a, &cp_b, &cp_prod_cpu, &cp_quot_cpu, &cp_prod_g, &cp_quot_g);
        results.push((LABEL_CODEPOINT, st));
    }

    results
}

fn print_summary(results: &[(&'static str, Status)]) -> bool {
    eprintln!("\n──────────────────────────────────────────────────────────");
    let mut failed = false;
    for (label, st) in results {
        let tag = match st {
            Status::Pass => "PASS".to_string(),
            Status::Skip(r) => format!("SKIP ({r})"),
            Status::Fail(r) => {
                failed = true;
                format!("FAIL ({r})")
            }
        };
        eprintln!("  {label:<28} {tag}");
    }
    eprintln!("──────────────────────────────────────────────────────────");
    failed
}

// ============================ negative control ===============================

/// A reproducible negative control. Each seed injects ONE known fault into a COPY
/// of a kernel and names the claim that MUST then go RED. Run via
/// `--fail-seed <name>`: exit 0 iff the harness CAUGHT the planted fault. Anyone
/// forking this can re-prove every claim's RED, per claim — so a PASS can never
/// silently inherit another proof's audit.
struct FailSeed {
    srcs: (String, String, String), // (u32, native, emu) — exactly one mutated
    expect_red: &'static str,        // the claim label that MUST report RED
}

fn mutate(src: &str, find: &str, replace: &str) -> String {
    let out = src.replace(find, replace);
    assert_ne!(out.as_str(), src, "fail-seed: marker `{find}` not found — kernel drifted, update the seed");
    out
}

fn fail_seed(name: &str) -> Option<FailSeed> {
    // Sources come from the registry — no local SRC_* consts required.
    let u = entry(SemanticPrimitive::PrismaticHashU32).kernel_src.as_single_src().to_string();
    let n = entry(SemanticPrimitive::PermyriadMulDivI64).kernel_src.as_native_src().to_string();
    let e = entry(SemanticPrimitive::PermyriadMulDivI64).kernel_src.as_emulated_src().to_string();
    let (srcs, expect_red) = match name {
        "u32"            => ((mutate(&u, "0x9E3779B1u", "0x9E3779B2u"), n, e), LABEL_U32),
        "native-mul"     => ((u, mutate(&n, "a[i] * b[i]", "a[i] * b[i] + i64(1)"), e), LABEL_I64_NATIVE),
        "native-div"     => ((u, mutate(&n, "p / i64(10000)", "p / i64(10001)"), e), LABEL_I64_NATIVE),
        "emu-mul"        => ((u, n, mutate(&e, "ll.y + cross", "ll.y")), LABEL_I64_EMU),
        "emu-div"        => ((u, n, mutate(&e, "an != bn", "an == bn")), LABEL_I64_EMU),
        // Codepoint seeds: same emulated-kernel faults, targeting the codepoint claim.
        // The emu kernel fault causes both the emulated and codepoint claims to go RED;
        // the seed passes when LABEL_CODEPOINT is RED — proving the codepoint corpus
        // is checked, not inherited silently from the emulated claim.
        "codepoint-mul"  => ((u, n, mutate(&e, "ll.y + cross", "ll.y")), LABEL_CODEPOINT),
        "codepoint-div"  => ((u, n, mutate(&e, "an != bn", "an == bn")), LABEL_CODEPOINT),
        _ => return None,
    };
    Some(FailSeed { srcs, expect_red })
}

const SEEDS: &[&str] = &["u32", "native-mul", "native-div", "emu-mul", "emu-div", "codepoint-mul", "codepoint-div"];

/// Normal proof: every ENABLED claim must be bit-identical CPU == GPU.
fn run_normal(gpu: &Gpu) {
    // Print the semantic-codepoint-kernel bridge registry before running claims.
    // Columns: codepoint  name  domain  cpu_symbol  wgsl_symbol  wgsl_kernel  corpus_name
    eprintln!("\n── Semantic-Codepoint-Kernel Bridge (SEMANTIC-CODEPOINT-KERNEL-BRIDGE-001/002) ──");
    for reg_entry in REGISTRY.iter() {
        let inv = reg_entry.invention.map_or("—".to_string(), |i| format!("inv.#{i}"));
        eprintln!("  {}  {:<34}  {}  {}",
            reg_entry.id, reg_entry.name, reg_entry.domain, inv);
        eprintln!("      cpu:{}  wgsl:{}  kernel:{}  corpus:{}",
            reg_entry.cpu_symbol, reg_entry.wgsl_symbol,
            reg_entry.wgsl_kernel, reg_entry.corpus_name);
    }
    eprintln!("─────────────────────────────────────────────────────────────────────────────\n");

    // Sources come from the registry — no local SRC_* consts.
    let src_u32    = entry(SemanticPrimitive::PrismaticHashU32).kernel_src.as_single_src();
    let src_native = entry(SemanticPrimitive::PermyriadMulDivI64).kernel_src.as_native_src();
    let src_emu    = entry(SemanticPrimitive::PermyriadMulDivI64).kernel_src.as_emulated_src();

    let results = run_claims(gpu, src_u32, src_native, src_emu);
    let failed = print_summary(&results);
    if failed {
        println!("FAIL: at least one claim diverged. Integer-determinism NOT proven.");
        exit(1);
    }
    println!("PASS: all enabled claims bit-identical CPU == GPU. diff = 0.");
}

/// Negative-control mode: inject a planted fault and require the harness to catch it.
fn run_fail_seed(gpu: &Gpu, name: &str) {
    let fs = match fail_seed(name) {
        Some(fs) => fs,
        None => {
            eprintln!("unknown --fail-seed '{name}'. available: {}", SEEDS.join(", "));
            exit(2);
        }
    };
    eprintln!("[negative-control] injecting fault '{name}' — claim [{}] MUST go RED", fs.expect_red);
    let results = run_claims(gpu, &fs.srcs.0, &fs.srcs.1, &fs.srcs.2);
    print_summary(&results);
    let target = results.iter().find(|(l, _)| *l == fs.expect_red).map(|(_, s)| s);
    match target {
        Some(Status::Fail(_)) => println!(
            "NEGATIVE CONTROL PASSED: planted fault '{name}' was CAUGHT (RED) in [{}]. Harness detects divergence.",
            fs.expect_red
        ),
        Some(Status::Skip(_)) => println!(
            "NEGATIVE CONTROL N/A: target claim [{}] is SKIPPED on this adapter (no SHADER_INT64) — run this seed on int64-capable hardware.",
            fs.expect_red
        ),
        _ => {
            println!("NEGATIVE CONTROL FAILED: planted fault '{name}' was NOT detected — harness is BLIND; the proof cannot be trusted.");
            exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args
        .iter()
        .position(|a| a == "--fail-seed")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let gpu = boot_gpu();
    match seed {
        None => run_normal(&gpu),
        Some(name) => run_fail_seed(&gpu, &name),
    }
}
