//! Dev-tooling smoke test (ARCH000 nod, Sean 2026-08-14): find `13FORGE
//! STUDIO` by title, focus it, and hold W for two seconds via
//! [`forge_drive_v3::inject::hold_wasd_bits`] — the real end-to-end check
//! for whether `shell/src/main.rs`'s WASD wiring actually moves the
//! `world5d` walker. Run with `cargo run -p forge-drive-v3 --example
//! tap_wasd`.
#![allow(missing_docs)]

fn main() {
    match forge_drive_v3::inject::focus("sovereign window") {
        Some(hwnd) => {
            println!("tap_wasd: focused hwnd={hwnd:#x}, holding W for 2000ms");
            forge_drive_v3::inject::hold_wasd_bits(hwnd, 0b0001, 2000);
            println!("tap_wasd: done");
        }
        None => println!("tap_wasd: no window matching 'sovereign window' found"),
    }
}
