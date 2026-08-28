//! NTFS volume enumeration — the fast path `walker.rs` deliberately left
//! unported, now landed behind an opt-in feature (Sean 2026-08-18 ARCH000
//! rescoping: "the hot path is the user's attention during an active
//! shell/game/creative process, not a one-shot xtask build tool"). Verbatim
//! port of `F:\NewRepo\crates\outland\src\mft.rs`, unsafe blocks and their
//! `SAFETY:` comments unchanged — this is a citation, not a rewrite.
//!
//! `cargo build` never sees this file unless `--features unsafe-fast-scan`
//! is passed; only `xtask` requests that feature, and only for its own
//! offline scans (`tractor-beam`, `ramusprime`) — nothing under `shell/` or
//! any runtime crate touches it. Every call into the fast path also prints a
//! visible one-line notice before it runs (`fast_scan_notice`) — the tool
//! switching on is never silent.
#![allow(unsafe_code)]

#[cfg(windows)]
mod win {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Ioctl::{FSCTL_ENUM_USN_DATA, MFT_ENUM_DATA_V0};
    use windows_sys::Win32::System::IO::DeviceIoControl;

    /// One file record as the volume holds it: identity, parent, name. The path is NOT
    /// stored — it is rebuilt from the parent chain, which is the whole point of reading
    /// records instead of walking directories.
    #[derive(Clone, Debug)]
    pub struct MftEntry {
        /// This record's own file reference number (volume-unique identity).
        pub frn: u64,
        /// The parent directory's file reference number.
        pub parent_frn: u64,
        /// The entry's bare name (no path — rebuilt separately via [`super::resolve_paths`]).
        pub name: String,
        /// Whether this record is a directory.
        pub is_dir: bool,
    }

    /// Bulk-read every file record on a volume. `letter` is the drive letter alone (`'C'`).
    ///
    /// Requires a raw volume handle, which is an ADMIN right — a non-elevated caller gets
    /// `PermissionDenied` here rather than a silently empty index, because an empty index
    /// reads as "the drive holds nothing" and that is the false-absent this crate exists to
    /// prevent.
    pub fn enumerate_volume(letter: char) -> io::Result<Vec<MftEntry>> {
        let path: Vec<u16> = std::ffi::OsString::from(format!(r"\\.\{letter}:"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `path` is a NUL-terminated wide string that outlives the call, and every
        // other argument is a plain constant. The returned handle is checked before use.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                windows_sys::Win32::Foundation::GENERIC_READ as u32,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let out = enumerate_handle(handle);
        // SAFETY: `handle` is a live handle from CreateFileW and is not used after this.
        unsafe { CloseHandle(handle) };
        out
    }

    /// The pump: ask for records from `next_id` onward until the volume stops giving them.
    /// Split out so the handle lifetime stays with the caller and every early return above
    /// still closes it.
    fn enumerate_handle(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<Vec<MftEntry>> {
        // 64 KiB is the size the FS driver fills efficiently; smaller buffers multiply the
        // ioctl count back toward the per-entry cost this exists to escape.
        const BUF_BYTES: usize = 64 * 1024;
        let mut med = MFT_ENUM_DATA_V0 { StartFileReferenceNumber: 0, LowUsn: 0, HighUsn: i64::MAX };
        let mut buf = vec![0u8; BUF_BYTES];
        let mut entries: Vec<MftEntry> = Vec::new();
        loop {
            let mut returned: u32 = 0;
            // SAFETY: both buffers are live and their byte lengths are passed exactly; the
            // driver writes at most `BUF_BYTES` into `buf` and reports how much in `returned`.
            let ok = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    std::ptr::addr_of!(med).cast(),
                    std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                    buf.as_mut_ptr().cast(),
                    BUF_BYTES as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                let e = io::Error::last_os_error();
                match e.raw_os_error() {
                    // ERROR_HANDLE_EOF — the volume saying "that was the last record".
                    Some(38) if !entries.is_empty() => break,
                    // ERROR_INVALID_FUNCTION — the handle opened but the filesystem has no
                    // record table to enumerate. ReFS answers exactly this way, and the miss
                    // is a FILESYSTEM fact and must never be reported as a rights problem.
                    Some(1) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "volume has no MFT to enumerate (ReFS/exFAT/FAT) — NTFS only",
                        ))
                    }
                    _ => return Err(e),
                }
            }
            // The first 8 bytes are the next start FRN; records follow it.
            if (returned as usize) <= 8 {
                break;
            }
            med.StartFileReferenceNumber = u64::from_le_bytes(buf[..8].try_into().unwrap());
            let mut off = 8usize;
            while off + 60 <= returned as usize {
                let rec = &buf[off..];
                let rec_len = u32::from_le_bytes(rec[0..4].try_into().unwrap()) as usize;
                if rec_len < 60 || off + rec_len > returned as usize {
                    break;
                }
                let frn = u64::from_le_bytes(rec[8..16].try_into().unwrap());
                let parent_frn = u64::from_le_bytes(rec[16..24].try_into().unwrap());
                let attrs = u32::from_le_bytes(rec[52..56].try_into().unwrap());
                let name_len = u16::from_le_bytes(rec[56..58].try_into().unwrap()) as usize;
                let name_off = u16::from_le_bytes(rec[58..60].try_into().unwrap()) as usize;
                if name_off + name_len <= rec_len {
                    let raw = &rec[name_off..name_off + name_len];
                    let wide: Vec<u16> = raw
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    entries.push(MftEntry {
                        frn,
                        parent_frn,
                        name: String::from_utf16_lossy(&wide),
                        is_dir: attrs & FILE_ATTRIBUTE_DIRECTORY != 0,
                    });
                }
                off += rec_len;
            }
        }
        Ok(entries)
    }
}

#[cfg(windows)]
pub use win::{enumerate_volume, MftEntry};

/// Rebuild `dir/dir/name` for every record from the flat set.
///
/// Records arrive in volume order with no paths, so the chain is walked per entry against a
/// parent map. A record whose parent is not in the set (outside the volume, or a root) stops
/// the walk — the partial path is returned rather than dropped, because a name without its
/// full ancestry is still a real file and dropping it would under-report the volume.
#[cfg(windows)]
pub fn resolve_paths(entries: &[MftEntry]) -> Vec<String> {
    use std::collections::HashMap;
    let by_frn: HashMap<u64, &MftEntry> = entries.iter().map(|e| (e.frn, e)).collect();
    entries
        .iter()
        .map(|e| {
            let mut parts = vec![e.name.as_str()];
            // The volume root is its own parent — that, not a frn comparison against the key
            // we just looked up by, is the stop. The hop cap independently bounds any cycle
            // a damaged record set could otherwise spin on forever.
            let mut cur = e.parent_frn;
            let mut hops = 0;
            while hops < 64 && cur != e.frn {
                let Some(p) = by_frn.get(&cur) else { break };
                parts.push(p.name.as_str());
                if p.parent_frn == p.frn {
                    break;
                }
                cur = p.parent_frn;
                hops += 1;
            }
            parts.reverse();
            parts.join("/")
        })
        .collect()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    // [BOARD: W1-C1] The parent chain IS the path: records arrive flat and in volume order,
    // so a wrong reconstruction silently mislabels every file on the drive. Measured live
    // (donor, 08-02): 1,871,924 records off C: in 12,069ms (6.4us each) against a walk floor
    // of 18us.
    #[test]
    fn a_record_set_rebuilds_its_paths_from_the_parent_chain_alone() {
        let entries = vec![
            MftEntry { frn: 5, parent_frn: 5, name: ".".into(), is_dir: true },
            MftEntry { frn: 10, parent_frn: 5, name: "crates".into(), is_dir: true },
            MftEntry { frn: 20, parent_frn: 10, name: "outland".into(), is_dir: true },
            MftEntry { frn: 30, parent_frn: 20, name: "mft.rs".into(), is_dir: false },
            // Parent outside the set: keep the name, never drop the record — a dropped
            // record under-reports the volume, which reads as a file that does not exist.
            MftEntry { frn: 40, parent_frn: 999, name: "orphan.rs".into(), is_dir: false },
        ];
        let paths = resolve_paths(&entries);
        assert_eq!(paths[3], "./crates/outland/mft.rs", "the chain walks to the root");
        assert_eq!(paths[1], "./crates");
        assert_eq!(paths[4], "orphan.rs", "an unrooted record survives as its bare name");
        assert_eq!(paths[0], ".", "the root is its own parent and must not loop");
    }

    // A cycle in a damaged record set must not spin the resolver forever.
    #[test]
    fn a_cyclic_parent_chain_stops_instead_of_hanging() {
        let entries = vec![
            MftEntry { frn: 1, parent_frn: 2, name: "a".into(), is_dir: true },
            MftEntry { frn: 2, parent_frn: 1, name: "b".into(), is_dir: true },
        ];
        let paths = resolve_paths(&entries);
        assert_eq!(paths.len(), 2, "both records still resolve to something");
        assert!(paths[0].len() < 4096, "the hop cap bounded the walk");
    }
}

#[cfg(not(windows))]
pub fn enumerate_volume(_letter: char) -> std::io::Result<Vec<()>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "MFT enumeration is an NTFS primitive — this platform has no volume record table",
    ))
}

/// Whole-volume scan as a [`crate::walker::WalkReport`], the same shape
/// `walk_bounded_skipping` returns — a caller can try this first and fall
/// back to the safe walker on any error (no admin rights, non-NTFS volume)
/// without branching on two different result types.
///
/// Prints one visible line before touching the volume — the fast path never
/// fires silently. Paths matching `crate::walker::DEFAULT_SKIP_DIRS` (or
/// `extra_skip`) are dropped by name, same filter the safe walker applies,
/// so the two produce comparable entry sets.
#[cfg(windows)]
pub fn walk_volume_fast(letter: char, extra_skip: &[String]) -> std::io::Result<crate::walker::WalkReport> {
    use crate::walker::{WalkEntry, WalkReport, WalkStop};
    use std::path::PathBuf;

    eprintln!("[unsafe-fast-scan] enumerating {letter}: via raw NTFS MFT/USN read (admin required) ...");

    let entries = enumerate_volume(letter)?;
    let paths = resolve_paths(&entries);

    let mut out = Vec::with_capacity(entries.len());
    'entries: for (e, p) in entries.iter().zip(paths.iter()) {
        for seg in p.split('/') {
            if crate::walker::skips_dir(seg, extra_skip) {
                continue 'entries;
            }
        }
        out.push(WalkEntry { path: PathBuf::from(format!(r"{letter}:\{p}")), is_dir: e.is_dir });
    }

    eprintln!("[unsafe-fast-scan] {letter}: {} record(s), {} after skip-dir filtering", entries.len(), out.len());
    Ok(WalkReport { entries: out, stop: WalkStop::Exhausted })
}
