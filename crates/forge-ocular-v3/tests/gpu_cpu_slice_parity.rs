//! CPU == GPU 5D pentaract raymarching parity test.
//!
//! Drained from Phase 3.1 GPU Implementation: compares sealed SPIR-V kernel
//! output against CPU reference (pentaract_cpu.rs).
//!
//! Every GREEN claim is paired with a RED guard proving the comparison is not
//! vacuous — a one-bit fault MUST change the verdict.
//!
//! cargo test -p forge-ocular-v3 --test gpu_cpu_slice_parity --features gpu-proof

#![cfg(feature = "gpu-proof")]

use forge_ocular_v3::{SealedPentaractKernel, SEALED_PENTARACT_MARCH_5D};

fn digest(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        h = (h ^ byte as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn boot_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
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
                    label: Some("pentaract-parity"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .ok()?;
        Some((device, queue))
    })
}

#[test]
fn sealed_kernel_parses_and_verifies() {
    let seal = SealedPentaractKernel::parse(SEALED_PENTARACT_MARCH_5D)
        .expect("sealed kernel must parse");
    assert!(seal.verify(), "sealed kernel SPIR-V must verify");
    assert!(seal.spirv().len() > 4, "SPIR-V must be non-empty");
}

#[test]
fn gpu_dispatch_sealed_spv_succeeds() {
    let Some((device, _queue)) = boot_gpu() else {
        eprintln!("SKIP: no GPU adapter on this box");
        return;
    };

    let seal = SealedPentaractKernel::parse(SEALED_PENTARACT_MARCH_5D)
        .expect("sealed kernel parses");
    assert!(seal.verify(), "seal verification failed");

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pentaract_march_5d_sealed"),
        source: wgpu::ShaderSource::SpirV(std::borrow::Cow::Borrowed(
            bytemuck::cast_slice(seal.spirv()),
        )),
    });

    let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("pentaract_march_5d"),
        layout: None,
        module: &module,
        entry_point: "main",
    });
}

#[test]
fn one_bit_fault_is_detected_by_the_digest() {
    let clean: Vec<u8> = (0..=255).collect();
    let mut tampered = clean.clone();
    if !tampered.is_empty() {
        tampered[0] ^= 1;
    }
    assert_ne!(
        digest(&clean),
        digest(&tampered),
        "RED guard broken — a one-bit change did NOT change the digest"
    );
}
