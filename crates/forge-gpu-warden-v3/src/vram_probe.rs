//! Driver-reported VRAM residency (total/used/free) for the demo's ACTUAL bar.
//! Dep-free `nvidia-smi` path by default; NVML behind the optional `nvml` feature.
//! Cold path — call on a telemetry tick, never per frame.

/// Which mechanism produced a [`VramReading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VramSource {
    /// `nvidia-smi --query-gpu=memory.total,memory.used`, dep-free subprocess.
    NvidiaSmi,
    /// `nvml_wrapper` device memory info, requires the `nvml` feature.
    Nvml,
}

/// A driver-reported VRAM residency sample, whole-card across all processes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VramReading {
    /// Card capacity in MB as the driver reports it.
    pub total_mb: u32,
    /// Currently resident MB across every process on the card.
    pub used_mb: u32,
    /// Mechanism that produced this sample.
    pub source: VramSource,
}

impl VramReading {
    /// Free MB, saturating so a driver that reports `used > total` never wraps.
    #[inline]
    pub fn free_mb(&self) -> u32 {
        self.total_mb.saturating_sub(self.used_mb)
    }

    /// Residency as a percentage 0..=100, for a bar's fill width.
    #[inline]
    pub fn used_pct(&self) -> u8 {
        if self.total_mb == 0 {
            return 0;
        }
        ((self.used_mb as u64 * 100) / self.total_mb as u64).min(100) as u8
    }
}

/// Sample the driver. Returns `None` when no mechanism answers — an honest
/// absence, never a silent zero that would read as "nothing is resident".
pub fn probe() -> Option<VramReading> {
    #[cfg(feature = "nvml")]
    {
        if let Some(r) = probe_nvml() {
            return Some(r);
        }
    }
    probe_nvidia_smi()
}

/// Dep-free path: one subprocess, both fields in a single query.
pub fn probe_nvidia_smi() -> Option<VramReading> {
    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args([
        "--query-gpu=memory.total,memory.used",
        "--format=csv,noheader,nounits",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?;
    let mut fields = line.split(',');
    let total_mb: u32 = fields.next()?.trim().parse().ok()?;
    let used_mb: u32 = fields.next()?.trim().parse().ok()?;
    if total_mb == 0 {
        return None;
    }
    Some(VramReading { total_mb, used_mb, source: VramSource::NvidiaSmi })
}

/// Richer path behind the `nvml` feature: no subprocess, bytes straight from
/// the driver library.
#[cfg(feature = "nvml")]
pub fn probe_nvml() -> Option<VramReading> {
    const BYTES_PER_MB: u64 = 1024 * 1024;
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;
    let mem = device.memory_info().ok()?;
    if mem.total == 0 {
        return None;
    }
    Some(VramReading {
        total_mb: (mem.total / BYTES_PER_MB) as u32,
        used_mb: (mem.used / BYTES_PER_MB) as u32,
        source: VramSource::Nvml,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_saturates_when_used_exceeds_total() {
        let r = VramReading { total_mb: 100, used_mb: 250, source: VramSource::NvidiaSmi };
        assert_eq!(r.free_mb(), 0);
    }

    #[test]
    fn used_pct_clamps_to_100_and_never_divides_by_zero() {
        let r = VramReading { total_mb: 0, used_mb: 42, source: VramSource::NvidiaSmi };
        assert_eq!(r.used_pct(), 0);
        let r = VramReading { total_mb: 8192, used_mb: 99_999, source: VramSource::NvidiaSmi };
        assert_eq!(r.used_pct(), 100);
    }

    #[test]
    fn used_pct_is_the_bar_fill_width() {
        let r = VramReading { total_mb: 8192, used_mb: 4096, source: VramSource::NvidiaSmi };
        assert_eq!(r.used_pct(), 50);
        assert_eq!(r.free_mb(), 4096);
    }

    #[test]
    fn a_probe_that_answers_reports_a_sane_card() {
        // Hermetic: a box with no NVIDIA driver returns None, which is a pass.
        if let Some(r) = probe() {
            assert!(r.total_mb >= 256, "implausible card total: {} MB", r.total_mb);
            assert!(r.used_mb <= r.total_mb.saturating_mul(2));
            assert!(r.used_pct() <= 100);
        }
    }
}
