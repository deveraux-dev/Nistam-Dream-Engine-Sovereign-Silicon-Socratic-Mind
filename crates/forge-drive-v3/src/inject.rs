//! Win32 keyboard injection + window focus — the "hands" half of T6, landed
//! as functions over [`crate::InputFrame64`] per this crate's own Cargo.toml
//! note ("window driving and CDP arrive in later tranches as functions over
//! this type, never as a second home").
//!
//! ARCH000 nod (Sean, 2026-08-14, "this is dev tooling"): raw `user32` FFI is
//! unavoidably `unsafe`, so this is the one locally-scoped exception to the
//! main workspace's `unsafe_code = "deny"` — the same shape `shell/src/
//! pen_input.rs:107,164` already uses for its own Win32 subclassing, not a
//! wider license. Quarried from `F:\NewRepo\crates\forge-vision\src\
//! window_driver.rs`'s `inject` module — keyboard + focus only; the capture
//! side (xcap/PrintWindow) and CDP are not part of this stroke.

/// Standard Win32 virtual-key codes for the four movement keys — ASCII
/// letter codes double as VK codes for A-Z, per the Win32 keyboard API.
pub mod vk {
    /// `W`.
    pub const W: u16 = 0x57;
    /// `A`.
    pub const A: u16 = 0x41;
    /// `S`.
    pub const S: u16 = 0x53;
    /// `D`.
    pub const D: u16 = 0x44;
}

#[cfg(windows)]
mod win32 {
    use std::ffi::c_void;
    use std::time::Duration;

    type Hwnd = *mut c_void;
    type Bool = i32;
    type Lparam = isize;

    #[repr(C)]
    struct KeybdInput {
        w_vk: u16,
        w_scan: u16,
        dw_flags: u32,
        time: u32,
        dw_extra_info: usize,
    }

    /// Mirrors Win32 `INPUT` (x64 = 40 bytes); `_tail` pads the union to the
    /// size of its largest member (`MOUSEINPUT`) so `SendInput`'s `cbSize`
    /// matches.
    #[repr(C)]
    struct Input {
        r#type: u32,
        ki: KeybdInput,
        _tail: [u8; 8],
    }

    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const SW_RESTORE: i32 = 9;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;

    #[allow(unsafe_code)]
    #[link(name = "user32")]
    extern "system" {
        fn SendInput(c_inputs: u32, p_inputs: *const Input, cb_size: i32) -> u32;
        fn SetForegroundWindow(hwnd: Hwnd) -> Bool;
        fn PostMessageW(hwnd: Hwnd, msg: u32, w_param: usize, l_param: isize) -> Bool;
        fn EnumWindows(callback: extern "system" fn(Hwnd, Lparam) -> Bool, lparam: Lparam) -> Bool;
        fn GetWindowTextW(hwnd: Hwnd, lp_string: *mut u16, n_max_count: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> Bool;
        fn ShowWindow(hwnd: Hwnd, n_cmd_show: i32) -> Bool;
    }

    fn send_key(vk: u16, up: bool) {
        let input = Input {
            r#type: INPUT_KEYBOARD,
            ki: KeybdInput {
                w_vk: vk,
                w_scan: 0,
                dw_flags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dw_extra_info: 0,
            },
            _tail: [0u8; 8],
        };
        #[allow(unsafe_code)]
        unsafe {
            SendInput(1, &input, std::mem::size_of::<Input>() as i32);
        }
    }

    /// `SendInput` key-down. Requires the target window hold real OS
    /// foreground (Win32's foreground-lock restriction) — [`focus`] first.
    pub fn key_down(vk: u16) {
        send_key(vk, false);
    }

    /// `SendInput` key-up.
    pub fn key_up(vk: u16) {
        send_key(vk, true);
    }

    /// A full press: down, brief hold, up.
    pub fn key_tap(vk: u16) {
        send_key(vk, false);
        std::thread::sleep(Duration::from_millis(15));
        send_key(vk, true);
    }

    /// Post a bare `WM_KEYDOWN` with no matching `WM_KEYUP` — the caller owns
    /// the hold duration and must pair this with [`post_key_up`].
    pub fn post_key_down(hwnd_raw: usize, vk: u16) {
        let hwnd = hwnd_raw as Hwnd;
        #[allow(unsafe_code)]
        unsafe {
            PostMessageW(hwnd, WM_KEYDOWN, vk as usize, 0);
        }
    }

    /// Post a bare `WM_KEYUP` — pairs with [`post_key_down`].
    pub fn post_key_up(hwnd_raw: usize, vk: u16) {
        let hwnd = hwnd_raw as Hwnd;
        #[allow(unsafe_code)]
        unsafe {
            PostMessageW(hwnd, WM_KEYUP, vk as usize, 0xC000_0001isize);
        }
    }

    struct FindCtx {
        needle: String,
        found: Hwnd,
    }

    extern "system" fn enum_cb(hwnd: Hwnd, lparam: Lparam) -> Bool {
        #[allow(unsafe_code)]
        unsafe {
            let ctx = &mut *(lparam as *mut FindCtx);
            if IsWindowVisible(hwnd) == 0 {
                return 1; // continue
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
                if title.contains(&ctx.needle) {
                    ctx.found = hwnd;
                    return 0; // stop enumeration
                }
            }
            1
        }
    }

    /// Find a visible window by title substring and bring it to the
    /// foreground. `Some(hwnd_as_usize)` when found (even if
    /// `SetForegroundWindow` is rejected by the OS foreground lock — the
    /// caller should prefer [`post_key`] in that case); `None` if no window
    /// matches.
    pub fn focus(title_substr: &str) -> Option<usize> {
        let mut ctx = FindCtx { needle: title_substr.to_lowercase(), found: std::ptr::null_mut() };
        #[allow(unsafe_code)]
        unsafe {
            EnumWindows(enum_cb, &mut ctx as *mut _ as Lparam);
        }
        if ctx.found.is_null() {
            return None;
        }
        #[allow(unsafe_code)]
        unsafe {
            ShowWindow(ctx.found, SW_RESTORE);
            let _ = SetForegroundWindow(ctx.found);
        }
        Some(ctx.found as usize)
    }
}

#[cfg(not(windows))]
mod win32 {
    pub fn key_down(_vk: u16) {}
    pub fn key_up(_vk: u16) {}
    pub fn key_tap(_vk: u16) {}
    pub fn post_key_down(_hwnd_raw: usize, _vk: u16) {}
    pub fn post_key_up(_hwnd_raw: usize, _vk: u16) {}
    pub fn focus(_title_substr: &str) -> Option<usize> {
        None
    }
}

pub use win32::{focus, key_down, key_tap, key_up, post_key_down, post_key_up};

/// Hold W/A/S/D (bit0=W, bit1=A, bit2=S, bit3=D) against `hwnd` for
/// `hold_ms`: posts `WM_KEYDOWN` for every set bit, sleeps `hold_ms`, then
/// posts `WM_KEYUP` for the same bits — a real sustained hold (unlike
/// [`post_key`]'s own down+up-together tap), bypassing the `SendInput`
/// foreground-lock restriction the same way `shell/src/main.rs`'s own
/// `Backquote` arm's doc comment already names.
pub fn hold_wasd_bits(hwnd: usize, bits: u8, hold_ms: u64) {
    let keys: [(u8, u16); 4] = [(1, vk::W), (2, vk::A), (4, vk::S), (8, vk::D)];
    for &(bit, code) in &keys {
        if bits & bit != 0 {
            post_key_down(hwnd, code);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(hold_ms));
    for &(bit, code) in &keys {
        if bits & bit != 0 {
            post_key_up(hwnd, code);
        }
    }
}
