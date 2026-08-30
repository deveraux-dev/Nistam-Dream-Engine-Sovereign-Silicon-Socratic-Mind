//! The uploaded KV copy, attested device-side.
//!
//! `sealed_pipeline` verifies the SPIR-V sha256, so the CODE is sealed — but the
//! bytes the shader reads arrived by `queue.write_buffer`, and nothing verified
//! that copy. Tolerance-0.0 stopped at the upload
//! (debt row KV-WINDOW-UNATTESTED-ACROSS-UPLOAD).
//!
//! HMAC-SHA256 seals the window CPU-side and stays there — it is not portable to
//! WGSL at a price worth paying. What IS already proven bit-identical on both
//! sides is `prismatic_hash` (cpu_gpu_integer_parity.rs), so the copy is attested
//! with that: the GPU folds the buffer it actually reads, the CPU folds the bytes
//! it meant to send, and the two digests must agree. A single changed word in the
//! upload breaks the agreement, which the RED guard proves.
//!
//! Run at BOTH 32 and 64 wide: an attestation that only holds at one workgroup
//! width is a property of the dispatch, not of the data.
//!
//! cargo test -p forge-kv-math-v3 --test kv_upload_attestation --features gpu-proof

#![cfg(feature = "gpu-proof")]

/// Multiple of both candidate widths, so neither lane needs a bounds-only tail.
const N_WORDS: usize = 4096;
const WIDTHS: [u32; 2] = [32, 64];

// ── CPU references — MUST stay byte-equivalent to the WGSL below ────────────

/// Byte-for-byte the kernel in `cpu_gpu_integer_parity.rs:30`, already proven
/// bit-identical CPU vs GPU. Duplicated rather than shared because the parity
/// test owns it as a *claim*; this file consumes it as a *tool*.
fn prismatic_hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1); // 2^32 / golden ratio
    h ^= y.wrapping_mul(0x85EB_CA77);
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 13;
    h
}

/// A permyriad-lattice KV window, packed the way the cache packs one: interleaved
/// key/value lanes, negatives included, spanning the sign boundary.
fn kv_window() -> Vec<u32> {
    (0..N_WORDS as i32)
        .map(|i| {
            let v = if i % 2 == 0 { (i - 2048).wrapping_mul(10_001) } else { (2048 - i).wrapping_mul(7) };
            v as u32
        })
        .collect()
}

/// Position-bound fold: the index rides into the hash, so moving a word is as
/// visible as changing one. Order-independent reduction would not catch a swap.
fn attest_cpu(words: &[u32]) -> Vec<u32> {
    words.iter().enumerate().map(|(i, &w)| prismatic_hash(w, i as u32)).collect()
}

/// FNV-1a over the raw bytes: one number stands in for the whole attestation.
fn digest(words: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in words.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>().iter() {
        h = (h ^ byte as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The device-side fold. `{W}` is the only thing that varies between the two
/// lanes — same arithmetic, same bindings, different dispatch width.
fn attest_wgsl(width: u32) -> String {
    format!(
        r#"
@group(0) @binding(0) var<storage, read> uploaded: array<u32>;
@group(0) @binding(1) var<storage, read_write> attest: array<u32>;

fn prismatic_hash(x: u32, y: u32) -> u32 {{
    var h: u32 = x * 0x9E3779B1u;
    h = h ^ (y * 0x85EBCA77u);
    h = h * 0xC2B2AE3Du;
    h = h ^ (h >> 15u);
    h = h * 0x27D4EB2Fu;
    h = h ^ (h >> 13u);
    return h;
}}

@compute @workgroup_size({width})
fn main_cs(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= arrayLength(&uploaded)) {{ return; }}
    attest[i] = prismatic_hash(uploaded[i], i);
}}
"#
    )
}

// ── GPU plumbing (headless, no surface) ─────────────────────────────────────

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// `None` when the box has no adapter — the test then SKIPS loudly rather than
/// passing vacuously.
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
                    label: Some("kv-upload-attestation"),
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

fn storage(gpu: &Gpu, label: &str, bytes: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn read_back(gpu: &Gpu, src: &wgpu::Buffer, bytes: u64) -> Vec<u32> {
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
    let out: Vec<u32> = bytemuck::cast_slice(&view).to_vec();
    drop(view);
    staging.unmap();
    out
}

/// Upload `words` exactly the way the KV lane does — `queue.write_buffer` — then
/// fold the uploaded copy on-device and read the attestation back.
fn attest_gpu(gpu: &Gpu, words: &[u32], width: u32) -> Vec<u32> {
    let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("kv-attest"),
        source: wgpu::ShaderSource::Wgsl(attest_wgsl(width).into()),
    });
    let pipe = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("kv-attest"),
        layout: None,
        module: &module,
        entry_point: Some("main_cs"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bytes = (words.len() * 4) as u64;
    let uploaded = storage(gpu, "uploaded-kv", bytes);
    let attest = storage(gpu, "attestation", bytes);
    gpu.queue.write_buffer(&uploaded, 0, bytemuck::cast_slice(words));

    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: uploaded.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: attest.as_entire_binding() },
        ],
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(words.len() as u32 / width, 1, 1);
    }
    gpu.queue.submit(std::iter::once(enc.finish()));
    read_back(gpu, &attest, bytes)
}

// ── GREEN: the uploaded copy attests bit-for-bit against the CPU digest ─────
// [BOARD: KV-UPLOAD-ATTESTED]
#[test]
fn uploaded_kv_window_attests_bit_for_bit_against_the_cpu_digest() {
    let Some(gpu) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };
    let words = kv_window();
    let want = digest(&attest_cpu(&words));
    for width in WIDTHS {
        assert_eq!(
            want,
            digest(&attest_gpu(&gpu, &words, width)),
            "upload attestation FAILED at workgroup_size({width}) — the copy the shader reads is not the buffer that was sent"
        );
    }
}

// ── GREEN: the attestation is a property of the data, not the dispatch ──────
// [BOARD: KV-UPLOAD-ATTESTED]
#[test]
fn attestation_is_invariant_across_workgroup_width() {
    let Some(gpu) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };
    let words = kv_window();
    assert_eq!(
        digest(&attest_gpu(&gpu, &words, 32)),
        digest(&attest_gpu(&gpu, &words, 64)),
        "attestation drifted between 32 and 64 wide — the fold depends on the dispatch, so it attests nothing"
    );
}

// ── RED: a one-word change in the UPLOAD must break the attestation ─────────
// [BOARD: KV-UPLOAD-ATTESTED]
#[test]
fn one_word_changed_in_the_upload_is_caught_on_device() {
    let Some(gpu) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };
    let clean = kv_window();
    let want = digest(&attest_cpu(&clean));

    let mut tampered = clean.clone();
    tampered[N_WORDS / 2] ^= 1; // the smallest fault the lattice has
    for width in WIDTHS {
        assert_ne!(
            want,
            digest(&attest_gpu(&gpu, &tampered, width)),
            "RED guard broken at workgroup_size({width}) — a tampered upload still attested, so GREEN proves nothing"
        );
    }
}

// ── RED: a MOVED word must break it too, or the fold is order-blind ─────────
// [BOARD: KV-UPLOAD-ATTESTED]
#[test]
fn two_swapped_words_are_caught_because_position_rides_the_hash() {
    let clean = kv_window();
    let mut swapped = clean.clone();
    swapped.swap(7, 4093);
    assert_ne!(
        digest(&attest_cpu(&clean)),
        digest(&attest_cpu(&swapped)),
        "RED guard broken — a reordered window attested clean, which an order-independent reduce would allow"
    );
}
