//! Pentaract raymarch frame-loop stopwatch: dispatches the SEALED 5D kernel
//! at 1920x1080 in a timed loop — the graphics-side load for the GPU
//! co-residency receipt (run alone, then concurrently with gemma-sidecar).
//! Compute-loop proxy: no swapchain/present, so numbers bound kernel cost,
//! not window vsync. Run: cargo run --release -p forge-ocular-v3
//! --example pentaract_frame_loop --features gpu-proof

fn main() {
    #[cfg(not(feature = "gpu-proof"))]
    eprintln!("rebuild with --features gpu-proof");
    #[cfg(feature = "gpu-proof")]
    gpu::run();
}

#[cfg(feature = "gpu-proof")]
mod gpu {
    use forge_ocular_v3::{SealedPentaractKernel, SEALED_PENTARACT_MARCH_5D};
    use std::time::Instant;

    pub fn run() {
        let Some((device, queue)) = boot_gpu() else {
            eprintln!("ABORT: no GPU adapter on this box — nothing measured, nothing claimed.");
            std::process::exit(2);
        };

        let seal = SealedPentaractKernel::parse(SEALED_PENTARACT_MARCH_5D).expect("sealed kernel parses");
        assert!(seal.verify(), "seal verification failed");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pentaract_march_5d_sealed"),
            source: wgpu::ShaderSource::SpirV(std::borrow::Cow::Borrowed(bytemuck::cast_slice(seal.spirv()))),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pentaract_march_5d"),
            layout: None,
            module: &module,
            entry_point: "main",
        });

        // M5Params: absence mask all-present, unit sun, small step.
        let mut params = [0u8; 64];
        params[0..32].fill(0xFF);
        for (i, v) in [0.0f32, 0.7, 0.7, 0.1, 1.0, 0.0, 0.0, 0.05].iter().enumerate() {
            params[32 + i * 4..36 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        use wgpu::util::DeviceExt;
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("m5-params"),
            contents: &params,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let hm_size = 256u32;
        let hm_data: Vec<u8> = (0..hm_size * hm_size * 4).map(|i| (i % 251) as u8).collect();
        let heightmap = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("heightmap"),
                size: wgpu::Extent3d { width: hm_size, height: hm_size, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &hm_data,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let (w, h) = (1920u32, 1080u32);
        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("out-color"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&heightmap.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&out_tex.create_view(&Default::default())),
                },
            ],
        });

        let frame = |device: &wgpu::Device, queue: &wgpu::Queue| {
            let mut enc = device.create_command_encoder(&Default::default());
            {
                let mut pass = enc.begin_compute_pass(&Default::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
            }
            queue.submit(std::iter::once(enc.finish()));
            let _ = device.poll(wgpu::Maintain::Wait);
        };

        for _ in 0..10 {
            frame(&device, &queue);
        }

        let seconds: f64 = std::env::var("S13_FRAME_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(5.0);
        // S13_FRAME_HZ throttles to a realistic UI cadence (e.g. 120);
        // unset = greedy back-to-back frames (worst-case contention probe).
        let target_hz: f64 = std::env::var("S13_FRAME_HZ").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let mut times_ms: Vec<f64> = Vec::new();
        let t_all = Instant::now();
        while t_all.elapsed().as_secs_f64() < seconds {
            let t0 = Instant::now();
            frame(&device, &queue);
            times_ms.push(t0.elapsed().as_secs_f64() * 1e3);
            if target_hz > 0.0 {
                let budget = 1.0 / target_hz;
                let used = t0.elapsed().as_secs_f64();
                if used < budget {
                    std::thread::sleep(std::time::Duration::from_secs_f64(budget - used));
                }
            }
        }
        times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = times_ms.len();
        let mean = times_ms.iter().sum::<f64>() / n as f64;
        let p95 = times_ms[(n as f64 * 0.95) as usize - 1];
        let worst = times_ms[n - 1];
        println!(
            "[pentaract-frames] {} frames over {:.1}s at 1920x1080: mean {:.2} ms ({:.0} FPS), p95 {:.2} ms, worst {:.2} ms",
            n,
            seconds,
            mean,
            1000.0 / mean,
            p95,
            worst
        );
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
                        label: Some("pentaract-frame-loop"),
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
}
