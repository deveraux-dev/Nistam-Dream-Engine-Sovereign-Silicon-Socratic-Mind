//! Minimal Win32 helpers — PID liveness, working-set bytes, process enumeration,
//! and targeted process termination. Isolated here; rest of daemon is safe Rust.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};

/// `GetExitCodeProcess` returns this sentinel while a process is still running.
const STILL_ACTIVE: u32 = 259;

fn open(pid: u32) -> HANDLE {
    // SAFETY: OpenProcess is null-returning on failure; we never deref the handle,
    // only pass it back to Win32 and CloseHandle it.
    unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) }
}

/// True iff `pid` exists in the Toolhelp process snapshot (no OpenProcess rights needed).
/// Conservative: snapshot failure → assume alive.
fn pid_in_snapshot(pid: u32) -> bool {
    // SAFETY: TH32CS_SNAPPROCESS snapshot; handle is valid or INVALID_HANDLE_VALUE.
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return true; // can't snapshot → assume alive to avoid false-positive reap
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut found = false;
    // SAFETY: `snap` is a valid snapshot; `entry` is correctly sized.
    if unsafe { Process32FirstW(snap, &mut entry) } != 0 {
        loop {
            if entry.th32ProcessID == pid {
                found = true;
                break;
            }
            entry = unsafe { std::mem::zeroed() };
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if unsafe { Process32NextW(snap, &mut entry) } == 0 {
                break;
            }
        }
    }
    // SAFETY: `snap` is a valid handle we own.
    unsafe { CloseHandle(snap) };
    found
}

/// True iff `pid` names a process that is alive right now.
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let h = open(pid);
    if h.is_null() {
        // OpenProcess failed: gone OR we lack rights to query it.
        // Fall back to snapshot — no rights needed — to distinguish the two.
        // If it's in the snapshot it's alive (conservatively skip the reap).
        return pid_in_snapshot(pid);
    }
    let mut code: u32 = 0;
    // SAFETY: `h` is a valid handle from OpenProcess; `code` is a live u32.
    let ok = unsafe { GetExitCodeProcess(h, &mut code) };
    unsafe { CloseHandle(h) };
    ok != 0 && code == STILL_ACTIVE
}

/// Enumerate all running processes whose exe name matches `target_name`
/// (case-insensitive, without path). Returns `(pid, parent_pid)` pairs.
pub fn enumerate_by_name(target_name: &str) -> Vec<(u32, u32)> {
    let target = target_name.to_ascii_lowercase();
    let mut results = Vec::new();

    // SAFETY: TH32CS_SNAPPROCESS snapshot; handle is valid or INVALID_HANDLE_VALUE.
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return results;
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

    // SAFETY: `snap` is a valid snapshot handle; `entry` is correctly sized.
    if unsafe { Process32FirstW(snap, &mut entry) } != 0 {
        loop {
            let name = String::from_utf16_lossy(
                entry.szExeFile.iter().copied().take_while(|&c| c != 0).collect::<Vec<_>>().as_slice(),
            ).to_ascii_lowercase();
            // Match with or without .exe suffix.
            if name == target || name == format!("{target}.exe") {
                results.push((entry.th32ProcessID, entry.th32ParentProcessID));
            }
            entry = unsafe { std::mem::zeroed() };
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if unsafe { Process32NextW(snap, &mut entry) } == 0 {
                break;
            }
        }
    }

    // SAFETY: `snap` is a valid handle we own.
    unsafe { CloseHandle(snap) };
    results
}

/// Forcibly terminate `pid`. Returns `true` if the kill was issued successfully.
/// A `false` return means the process was already gone or we lacked rights.
pub fn kill_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: OpenProcess returns null on failure; we close the handle in all paths.
    let h = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if h.is_null() {
        return false;
    }
    // SAFETY: `h` is a valid handle; exit code 1 is the conventional forced-exit code.
    let ok = unsafe { TerminateProcess(h, 1) };
    unsafe { CloseHandle(h) };
    ok != 0
}

/// Working-set (resident) bytes for `pid`, or `None` if it can't be read
/// (process gone / access denied).
pub fn working_set_bytes(pid: u32) -> Option<u64> {
    if pid == 0 {
        return None;
    }
    let h = open(pid);
    if h.is_null() {
        return None;
    }
    // SAFETY: PROCESS_MEMORY_COUNTERS is a plain-old-data struct; zeroing then
    // stamping `cb` is exactly the documented call contract.
    let mut c: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let ok = unsafe { GetProcessMemoryInfo(h, &mut c, c.cb) };
    unsafe { CloseHandle(h) };
    if ok != 0 {
        Some(c.WorkingSetSize as u64)
    } else {
        None
    }
}
