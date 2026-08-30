//! Native NDK event loop: `android_main` drains the [`ByteRouter`], gates
//! each packet through [`RestGate::evaluate_coherence`], and dispatches
//! AWAKE packets to an AAudio sink (REST: zero execution, zero registers
//! written -- the whole point). The `ALooper`-driven blocking poll (via
//! `android-activity`'s `poll_events`, itself a wrapper over
//! `ALooper_pollOnce`) is what actually drops the CPU thread into a
//! deep-sleep C-state between anchor beats instead of busy-polling.
//!
//! `router`/`governor`/`link_bridge`/`link_tls` are platform-agnostic
//! (`std`, no NDK deps) and unit-tested on any host. The Android entry point
//! below is compiled only under `target_os = "android"` --
//! `android-activity`/`ndk`/`ndk-sys` are target-gated in `Cargo.toml` so
//! `cargo test` runs clean on any dev box without the NDK installed.
//!
//! Ported verbatim from `F:\NewRepo\crates\link-android` (2026-08-19, the
//! `we-got-sdk-the-fancy-rainbow` plan, Wave 2). This is the one crate in
//! the workspace with a deliberate, scoped `#![allow(unsafe_code)]`
//! override of the `unsafe_code = "deny"` workspace lint -- same precedent
//! as `forge-index-v3`'s `unsafe-fast-scan` feature (ARCH000-scoped raw FFI,
//! not a general exemption). Two real needs, both SAFETY-commented at the
//! call site: [`router::ByteRouter`]'s lock-free SPSC ring (the whole reason
//! this crate exists -- zero-alloc, no mutex, on the Android link's hot
//! path) and `ndk_entry`'s raw AAudio C API calls (no safe wrapper crate
//! covers the low-latency stream builder this needs).
#![allow(unsafe_code)]

pub mod router;
pub mod governor;
pub mod link_bridge;
pub mod link_tls;

pub use governor::{RestGate, RestVerdict, MIN_ALIGN_Q};
pub use link_bridge::{IngestReport, LinkBridge, TAG_CTRL, TAG_RAW, TAG_TEXT};
pub use router::{ByteRouter, UmpPacket64};

/// Ring capacity: packets in flight between the link-ingest thread and the
/// drain loop. `ByteRouter` doesn't require a power of two, but it keeps the
/// `% N` in the hot path a cheap bitmask on real hardware.
pub const RING_CAPACITY: usize = 256;

/// Governor phase-alignment period, ticks -- shares `LinkBridge`'s 16-bit
/// wrapping local-delta-tick space.
pub const GOVERNOR_LOOP_TICKS: u64 = 600;

/// `ALooper` blocking timeout between ritual anchor beats, milliseconds.
/// This is the actual deep-sleep mechanism: a blocking
/// `ALooper_pollOnce(timeout, ...)` parks the thread instead of spinning, so
/// the scheduler idles the core (WFI/cpuidle) for the full duration whenever
/// no link fd becomes readable.
pub const ANCHOR_INTERVAL_MS: u64 = 250;

/// Loopback link endpoint the ingest thread connects out to. A real
/// deployment resolves this from the paired desktop's advertised address
/// (the desktop bridge's pairing/discovery is desktop-side, out of this
/// module's scope); this constant is the override point.
#[cfg(target_os = "android")]
pub const LINK_HOST: &str = "127.0.0.1";
#[cfg(target_os = "android")]
/// Loopback link endpoint port.
pub const LINK_PORT: u16 = 13_017;

#[cfg(target_os = "android")]
mod ndk_entry {
    use super::*;
    use android_activity::{AndroidApp, MainEvent, PollEvent};
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::sync::Arc;
    use std::time::Duration;

    /// One AAudio PCM16 mono output stream, opened once and reused -- no
    /// per-dispatch allocation or stream churn.
    struct AAudioSink {
        stream: *mut ndk_sys::AAudioStream,
    }

    impl AAudioSink {
        /// Opens a low-latency mono 48kHz PCM16 output stream via the raw
        /// AAudio C API (builder -> open -> start). `None` on any failure --
        /// the drain loop then runs gate-only (no dispatch) rather than panic.
        fn open() -> Option<Self> {
            unsafe {
                let mut builder: *mut ndk_sys::AAudioStreamBuilder = core::ptr::null_mut();
                if ndk_sys::AAudio_createStreamBuilder(&mut builder) != 0 || builder.is_null() {
                    return None;
                }
                ndk_sys::AAudioStreamBuilder_setDirection(builder, ndk_sys::AAUDIO_DIRECTION_OUTPUT as i32);
                ndk_sys::AAudioStreamBuilder_setFormat(builder, ndk_sys::AAUDIO_FORMAT_PCM_I16);
                ndk_sys::AAudioStreamBuilder_setChannelCount(builder, 1);
                ndk_sys::AAudioStreamBuilder_setSampleRate(builder, 48_000);
                ndk_sys::AAudioStreamBuilder_setPerformanceMode(
                    builder,
                    ndk_sys::AAUDIO_PERFORMANCE_MODE_LOW_LATENCY as i32,
                );
                let mut stream: *mut ndk_sys::AAudioStream = core::ptr::null_mut();
                let opened = ndk_sys::AAudioStreamBuilder_openStream(builder, &mut stream);
                ndk_sys::AAudioStreamBuilder_delete(builder);
                if opened != 0 || stream.is_null() {
                    return None;
                }
                if ndk_sys::AAudioStream_requestStart(stream) != 0 {
                    ndk_sys::AAudioStream_close(stream);
                    return None;
                }
                Some(Self { stream })
            }
        }

        /// Dispatches one AWAKE packet's payload as a single PCM16 frame
        /// (low 16 bits of `payload`). Non-blocking (0ns timeout) -- a full
        /// AAudio buffer drops the frame rather than stalling the drain loop.
        fn dispatch(&self, packet: &UmpPacket64) {
            let sample = packet.payload as i16;
            let frame = [sample];
            unsafe {
                ndk_sys::AAudioStream_write(self.stream, frame.as_ptr().cast(), 1, 0);
            }
        }
    }

    impl Drop for AAudioSink {
        fn drop(&mut self) {
            unsafe {
                ndk_sys::AAudioStream_requestStop(self.stream);
                ndk_sys::AAudioStream_close(self.stream);
            }
        }
    }

    /// Owns the actual packet source: reconnects to [`LINK_HOST`]:[`LINK_PORT`]
    /// over pinned-cert TLS and unpacks every [`link_wire::Packet`] into
    /// `ring` via [`LinkBridge::ingest_packet`]. Runs on its own thread so
    /// the ALooper-driven drain loop below never blocks on I/O.
    fn spawn_link_reader(ring: Arc<ByteRouter<RING_CAPACITY>>, quit: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            let mut bridge = LinkBridge::new();
            while !quit.load(AtomicOrdering::Relaxed) {
                let Ok(tls) = crate::link_tls::connect(LINK_HOST, LINK_PORT, crate::link_tls::PINNED_DER_FINGERPRINT) else {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                };
                let _ = crate::link_tls::read_packets(tls, |packet| {
                    bridge.ingest_packet(&packet, &ring);
                });
                if quit.load(AtomicOrdering::Relaxed) {
                    return;
                }
            }
        });
    }

    /// Native entry point -- `android-activity`'s generated
    /// `ANativeActivity_onCreate` is this function's live caller, the same
    /// shape as any NativeActivity app's `android_main`.
    #[no_mangle]
    fn android_main(app: AndroidApp) {
        let ring = Arc::new(ByteRouter::<RING_CAPACITY>::new());
        let quit = Arc::new(AtomicBool::new(false));
        spawn_link_reader(ring.clone(), quit.clone());

        let mut governor = RestGate::new(GOVERNOR_LOOP_TICKS);
        let sink = AAudioSink::open();
        let mut anchor_tick: u64 = 0;

        loop {
            let mut destroyed = false;
            app.poll_events(Some(Duration::from_millis(ANCHOR_INTERVAL_MS)), |event| {
                if let PollEvent::Main(MainEvent::Destroy) = event {
                    destroyed = true;
                }
            });
            if destroyed {
                quit.store(true, AtomicOrdering::Relaxed);
                break;
            }

            // Ritual anchor: re-seat phase every beat so both sides of the
            // link converge even through idle stretches with no traffic.
            anchor_tick = anchor_tick.wrapping_add(1);
            governor.hard_sync_at_anchor(anchor_tick);

            while let Some(packet) = ring.try_pop() {
                if governor.evaluate_coherence(&packet) {
                    if let Some(sink) = sink.as_ref() {
                        sink.dispatch(&packet);
                    }
                }
                // Rest: zero execution, zero registers written -- dropped.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_ingest_gate_dispatch_without_the_ndk_loop() {
        // Exercises the exact pipeline android_main drives (ingest -> drain
        // -> evaluate_coherence -> dispatch-or-rest) without needing the
        // Android target: router/governor/link_bridge are platform-agnostic.
        let ring: ByteRouter<RING_CAPACITY> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        bridge.ingest_text("anchor beat text pulse", &ring); // 22 bytes -> 6 packets

        let mut governor = RestGate::new(GOVERNOR_LOOP_TICKS);
        governor.hard_sync_at_anchor(0);

        let mut awake = 0u32;
        let mut rested = 0u32;
        while let Some(packet) = ring.try_pop() {
            if governor.evaluate_coherence(&packet) {
                awake += 1;
            } else {
                rested += 1;
            }
        }
        assert_eq!(awake + rested, 6);
        assert!(awake > 0, "packets at/near the anchor tick must wake the gate");
    }
}
