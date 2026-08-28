//! `AudioDeviceInfo` — one home (L05) for both the output (`realtime.rs`,
//! currently excluded from this crate: real `unsafe` blocks, thread priority
//! + `ptr::read`) and input (`input_capture.rs`) device-enumeration paths.
//! Split out 2026-08-19 so `input_capture.rs` doesn't need to depend on
//! `realtime`'s excluded, unsafe-laden module for one plain data struct.

/// Information about an available audio device (input or output).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub api: String,
    pub sample_rates: Vec<u32>,
    pub channels: u16,
}
