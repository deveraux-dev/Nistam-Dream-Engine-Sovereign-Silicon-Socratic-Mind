//! Double-buffered VRAM staging slots for zero-stall micro-expert hot-swapping.
//!
//! Provides 2 × 64 KB double-buffered staging buffers synchronized via
//! [`TimelineSemaphore`] fences. While the GPU dispatches or real-time
//! playback reads from the active slot, the host asynchronously stages the
//! next micro-expert weights into the alternate slot. Once the DMA timeline
//! point completes, the active slot flips with zero-stall overhead.

use crate::fence::{TimelineError, TimelineFence, TimelineSemaphore};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Size of each staging buffer slot in bytes: 64 KB (65,536 bytes).
pub const STAGING_SLOT_SIZE: usize = 64 * 1024;

/// Total number of double-buffered staging slots.
pub const NUM_STAGING_SLOTS: usize = 2;

/// Errors arising from VRAM staging buffer operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StagingError {
    /// Payload exceeds the 64 KB slot capacity.
    #[error("payload size {length} B exceeds staging slot capacity of {max} B")]
    PayloadTooLarge {
        /// Attempted payload length in bytes.
        length: usize,
        /// Maximum capacity (65,536 bytes).
        max: usize,
    },
    /// Staging slot cannot be activated because its timeline point has not been signaled.
    #[error("staging slot {slot_index} not ready: required point {required_point}, current {current_point}")]
    SlotNotReady {
        /// Index of the staging slot (0 or 1).
        slot_index: usize,
        /// Timeline point required for completion.
        required_point: u64,
        /// Current timeline point on the semaphore.
        current_point: u64,
    },
    /// Underlying timeline synchronization error.
    #[error(transparent)]
    Timeline(#[from] TimelineError),
}

/// A single 64 KB staging buffer slot and its associated metadata.
pub struct StagingSlot {
    /// Pinned 64 KB heap buffer.
    pub buffer: Box<[u8; STAGING_SLOT_SIZE]>,
    /// Valid payload length in bytes within the buffer.
    pub payload_len: usize,
    /// Expert / specialist ID currently resident in this slot.
    pub expert_id: u32,
    /// Monotonic timeline point corresponding to DMA completion for this slot.
    pub staged_point: u64,
    /// Whether this slot contains a staged, valid payload.
    pub is_valid: bool,
}

impl StagingSlot {
    fn new() -> Self {
        Self {
            buffer: Box::new([0u8; STAGING_SLOT_SIZE]),
            payload_len: 0,
            expert_id: 0,
            staged_point: 0,
            is_valid: false,
        }
    }
}

/// Double-buffered 2 × 64 KB **host-side** staging buffer pair.
///
/// Both slots are ordinary heap allocations (`Box<[u8; STAGING_SLOT_SIZE]>`,
/// see [`StagingSlot::new`]) — this type holds no device memory, performs no
/// DMA, and never touches PCIe, despite the historical name. Staging is a
/// `copy_from_slice` into the inactive slot; the swap is an atomic index flip
/// gated on a [`TimelineSemaphore`] point. It is a real synchronisation
/// primitive modelling the CPU half of a hot-swap pipeline — but bandwidth
/// measured through it is host memcpy bandwidth, not a device transfer rate.
///
/// Prefer the accurate alias [`DoubleBufferedStagingBuffers`].
pub struct DoubleBufferedVramStaging {
    slots: [StagingSlot; NUM_STAGING_SLOTS],
    active_index: AtomicUsize,
    semaphore: Arc<TimelineSemaphore>,
}

/// Accurate name for [`DoubleBufferedVramStaging`]: both slots are heap
/// allocations, so this type never touches device memory, DMA, or PCIe.
/// Bandwidth measured through it is host memcpy bandwidth.
pub type DoubleBufferedStagingBuffers = DoubleBufferedVramStaging;

impl DoubleBufferedVramStaging {
    /// Initialize double-buffered staging with a shared [`TimelineSemaphore`].
    pub fn new(semaphore: Arc<TimelineSemaphore>) -> Self {
        Self {
            slots: [StagingSlot::new(), StagingSlot::new()],
            active_index: AtomicUsize::new(0),
            semaphore,
        }
    }

    /// Index of the currently active (readable/playback) slot (0 or 1).
    #[inline]
    pub fn active_index(&self) -> usize {
        self.active_index.load(Ordering::Acquire)
    }

    /// Index of the staging slot available for asynchronous host writes (`1 - active_index`).
    #[inline]
    pub fn staging_index(&self) -> usize {
        1 - self.active_index()
    }

    /// Reference to the active payload slice currently bound for real-time playback/compute.
    pub fn active_payload(&self) -> &[u8] {
        let idx = self.active_index();
        let slot = &self.slots[idx];
        &slot.buffer[..slot.payload_len]
    }

    /// Reference to the full 64 KB buffer of the active slot.
    pub fn active_slot_raw(&self) -> &[u8; STAGING_SLOT_SIZE] {
        let idx = self.active_index();
        &self.slots[idx].buffer
    }

    /// Expert ID resident in the active slot, if valid.
    pub fn active_expert_id(&self) -> Option<u32> {
        let idx = self.active_index();
        let slot = &self.slots[idx];
        if slot.is_valid {
            Some(slot.expert_id)
        } else {
            None
        }
    }

    /// Timeline point associated with the active slot.
    pub fn active_timeline_point(&self) -> u64 {
        let idx = self.active_index();
        self.slots[idx].staged_point
    }

    /// Check whether the inactive staging slot has completed DMA transfer and is ready to swap.
    pub fn is_staging_ready(&self) -> bool {
        let s_idx = self.staging_index();
        let s_slot = &self.slots[s_idx];
        if !s_slot.is_valid {
            return false;
        }
        self.semaphore.poll_value(s_slot.staged_point)
    }

    /// Asynchronously stage micro-expert weights into the inactive staging slot.
    ///
    /// Copies `weights` into the inactive buffer and registers `target_point` on the timeline.
    /// Returns a [`TimelineFence`] tracking DMA completion.
    pub fn stage_weights(
        &mut self,
        expert_id: u32,
        weights: &[u8],
        target_point: u64,
    ) -> Result<TimelineFence, StagingError> {
        if weights.len() > STAGING_SLOT_SIZE {
            return Err(StagingError::PayloadTooLarge {
                length: weights.len(),
                max: STAGING_SLOT_SIZE,
            });
        }

        let s_idx = self.staging_index();
        let slot = &mut self.slots[s_idx];
        slot.buffer[..weights.len()].copy_from_slice(weights);
        slot.payload_len = weights.len();
        slot.expert_id = expert_id;
        slot.staged_point = target_point;
        slot.is_valid = true;

        Ok(TimelineFence::new(
            expert_id as u64,
            target_point,
            self.semaphore.clone(),
        ))
    }

    /// Non-blocking zero-stall swap: if the staging slot's timeline point has completed,
    /// flips the active slot index in $O(1)$ time.
    ///
    /// Returns `Ok(true)` if swapped, `Ok(false)` if staging DMA is still in-flight.
    pub fn try_swap(&mut self) -> Result<bool, StagingError> {
        let s_idx = self.staging_index();
        let s_slot = &self.slots[s_idx];

        if !s_slot.is_valid {
            return Ok(false);
        }

        if self.semaphore.poll_value(s_slot.staged_point) {
            // Monotonic flip
            self.active_index.store(s_idx, Ordering::Release);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Force an immediate active slot flip without checking timeline status (e.g. initial warm-start).
    pub fn force_swap(&mut self) -> usize {
        let new_idx = self.staging_index();
        self.active_index.store(new_idx, Ordering::Release);
        new_idx
    }

    /// Reset staging slots to an empty state.
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.payload_len = 0;
            slot.expert_id = 0;
            slot.staged_point = 0;
            slot.is_valid = false;
        }
        self.active_index.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_buffering_ping_pong_hotswap() {
        let sem = Arc::new(TimelineSemaphore::new(0));
        let mut staging = DoubleBufferedVramStaging::new(sem.clone());

        assert_eq!(staging.active_index(), 0);
        assert_eq!(staging.staging_index(), 1);
        assert_eq!(staging.active_payload().len(), 0);

        // Stage expert #1 weights (32 bytes) into Slot 1
        let expert1_weights = [0xABu8; 32];
        let fence1 = staging.stage_weights(1, &expert1_weights, 10).unwrap();
        assert_eq!(fence1.target_point, 10);
        assert_eq!(staging.active_index(), 0); // Still 0

        // Attempt swap before DMA complete -> fails zero-stall
        assert_eq!(staging.try_swap().unwrap(), false);
        assert_eq!(staging.active_index(), 0);

        // Signal DMA complete at point 10
        sem.signal(10).unwrap();
        assert!(staging.is_staging_ready());

        // Now swap succeeds
        assert_eq!(staging.try_swap().unwrap(), true);
        assert_eq!(staging.active_index(), 1);
        assert_eq!(staging.staging_index(), 0);
        assert_eq!(staging.active_expert_id(), Some(1));
        assert_eq!(staging.active_payload(), &[0xABu8; 32]);

        // Stage expert #2 weights (64 bytes) into Slot 0
        let expert2_weights = [0xCDu8; 64];
        let fence2 = staging.stage_weights(2, &expert2_weights, 20).unwrap();
        assert_eq!(fence2.target_point, 20);

        // Cannot swap yet
        assert_eq!(staging.try_swap().unwrap(), false);

        // Signal DMA complete at point 20
        sem.signal(20).unwrap();
        assert_eq!(staging.try_swap().unwrap(), true);
        assert_eq!(staging.active_index(), 0);
        assert_eq!(staging.active_expert_id(), Some(2));
        assert_eq!(staging.active_payload(), &[0xCDu8; 64]);
    }

    #[test]
    fn oversized_payload_rejected() {
        let sem = Arc::new(TimelineSemaphore::new(0));
        let mut staging = DoubleBufferedVramStaging::new(sem);

        let big_payload = vec![0u8; STAGING_SLOT_SIZE + 1];
        let err = staging.stage_weights(99, &big_payload, 1).unwrap_err();
        assert!(matches!(
            err,
            StagingError::PayloadTooLarge {
                length,
                max: STAGING_SLOT_SIZE,
            } if length == STAGING_SLOT_SIZE + 1
        ));
    }
}
