//! Integration test: SplitShader GPU Warden, Timeline Semaphore Hotswaps, and BQ Routing Pipeline.

use forge_gpu_warden_v3::fence::{timeline_fence_pair, TimelineError, TimelineSemaphore};
use forge_gpu_warden_v3::vram_staging::{DoubleBufferedStagingBuffers, STAGING_SLOT_SIZE};
use forge_gpu_warden_v3::workgroup::WorkgroupTileContract;
use forge_ml_bqrouter::{BqCentroid, BqRouter, BQ_BYTES, NUM_SPECIALISTS};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_end_to_end_timeline_staging_hotswap_pipeline() {
    // 1. Initialize shared monotonic timeline semaphore starting at 0
    let semaphore = Arc::new(TimelineSemaphore::new(0));
    assert_eq!(semaphore.current_value(), 0);

    // 2. Initialize double-buffered 2 x 64 KB VRAM staging manager
    let mut staging = DoubleBufferedStagingBuffers::new(semaphore.clone());
    assert_eq!(staging.active_index(), 0);
    assert_eq!(staging.staging_index(), 1);

    // 3. Construct and train/configure BQ MetaRouter for 7 micro-experts
    let mut router = BqRouter::new(512);
    for sid in 0..NUM_SPECIALISTS {
        let mut bits = [0u8; BQ_BYTES];
        // Populate deterministic pattern for each specialist
        for (i, b) in bits.iter_mut().enumerate() {
            *b = ((sid * 37 + i * 13) % 256) as u8;
        }
        router.centroids[sid] = BqCentroid {
            bits,
            record_count: 100,
            positive_count: 95,
            active: true,
        };
    }

    // 4. Pack router centroids into a staging slot payload (483 bytes)
    let mut staging_payload = vec![0u8; 1024];
    let payload_len = router.pack_into_staging_slot(&mut staging_payload).unwrap();
    assert_eq!(payload_len, 483);

    // 5. Asynchronously stage Expert #1 (BQ Router Centroids) into Inactive Slot 1
    // Target completion timeline point = 100
    let fence1 = staging
        .stage_weights(1, &staging_payload[..payload_len], 100)
        .expect("staging into slot 1 should succeed");
    assert_eq!(fence1.target_point, 100);
    assert!(!fence1.is_ready());

    // Zero-stall assertion: try_swap returns false while DMA is in-flight
    assert_eq!(staging.try_swap().unwrap(), false);
    assert_eq!(staging.active_index(), 0); // Active remains slot 0

    // 6. Simulate asynchronous GPU DMA completion in background thread
    let sem_async = semaphore.clone();
    let dma_worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(15));
        // Signal timeline point 100
        sem_async.signal(100).expect("monotonic signal to 100");
    });

    // Wait for fence on consumer side
    assert!(fence1.wait(Duration::from_millis(500)));
    dma_worker.join().expect("dma thread completed cleanly");

    // 7. Perform zero-stall active buffer flip
    let swapped = staging.try_swap().expect("swap should succeed");
    assert!(swapped);
    assert_eq!(staging.active_index(), 1);
    assert_eq!(staging.staging_index(), 0);
    assert_eq!(staging.active_expert_id(), Some(1));
    assert_eq!(staging.active_payload().len(), 483);

    // Verify unpack from active VRAM payload
    let unpacked_router =
        BqRouter::unpack_from_staging_slot(staging.active_payload(), 512).unwrap();
    assert_eq!(unpacked_router.active_count(), NUM_SPECIALISTS);
    assert_eq!(
        unpacked_router.centroids[0].bits,
        router.centroids[0].bits
    );

    // 8. Stage Expert #2 weights (e.g. 768-dim S13 weights = 1,078 bytes) into Slot 0
    let mut s13_mock_weights = vec![0u8; 1078];
    for (i, b) in s13_mock_weights.iter_mut().enumerate() {
        *b = ((i * 7 + 3) % 256) as u8;
    }
    let fence2 = staging
        .stage_weights(2, &s13_mock_weights, 200)
        .expect("staging into slot 0 should succeed");
    assert_eq!(fence2.target_point, 200);

    // Swap blocked before point 200
    assert_eq!(staging.try_swap().unwrap(), false);

    // Signal point 200
    semaphore.signal(200).unwrap();
    assert!(fence2.is_ready());

    // Swap active back to Slot 0
    assert!(staging.try_swap().unwrap());
    assert_eq!(staging.active_index(), 0);
    assert_eq!(staging.active_expert_id(), Some(2));
    assert_eq!(staging.active_payload().len(), 1078);
    assert_eq!(staging.active_payload(), &s13_mock_weights[..]);

    // 9. Verify Monotonicity Law: Retrograde signals rejected
    let err = semaphore.signal(150).unwrap_err();
    assert!(matches!(
        err,
        TimelineError::RetrogradeSignal {
            attempted: 150,
            current: 200
        }
    ));

    // 10. Verify 32x32 SPIR-V Workgroup Tile Alignment & Ampere Warp Dispatch Contract
    let contract = WorkgroupTileContract::ampere_32x32();
    assert_eq!(contract.total_threads, 1024);
    assert_eq!(contract.warps_per_workgroup, 32);
    assert_eq!(contract.warp_size, 32);

    // Validate 768 x 768 matrix dispatch plan (24 x 24 = 576 workgroups, 589,824 threads)
    let plan = contract.plan_dispatch(768, 768);
    assert_eq!(plan.grid_dim, (24, 24, 1));
    assert_eq!(plan.total_tiles, 576);
    assert_eq!(plan.total_threads, 576 * 1024);
    // Shared memory: 32 rows * 132 bytes (33 elements * 4 bytes) = 4,224 bytes per workgroup
    assert_eq!(plan.shared_memory_bytes, 32 * 132);

    // Verify 128-byte L1 cache-line coalescing
    assert!(contract.is_coalesced_aligned(0));
    assert!(contract.is_coalesced_aligned(STAGING_SLOT_SIZE));
    assert!(contract.is_coalesced_aligned(512)); // 512-byte vector is 128-byte cache-line aligned
    assert_eq!(448 % 64, 0); // 7 * 64 aligned centroid matrix is 64-byte warp aligned
}

#[test]
fn test_timeline_fence_pair_creation_and_channel() {
    let sem = Arc::new(TimelineSemaphore::new(10));
    let (fence, sink) = timeline_fence_pair(999, 50, sem.clone());

    assert_eq!(fence.ticket_id, 999);
    assert_eq!(fence.target_point, 50);
    assert!(!fence.is_ready());

    sink.signal_completion(50).unwrap();
    assert!(fence.is_ready());
    assert_eq!(sem.current_value(), 50);
}
