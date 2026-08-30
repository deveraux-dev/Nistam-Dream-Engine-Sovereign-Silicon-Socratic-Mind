//! `cargo run -p forge-reel-v3 --features gif-window --example gif_window_proof`
//! — encode a real scrubbable GIF off the landed ReelClock and print its
//! column<->frame join. Writes `.forge/photons/reel_window.gif`.

use forge_reel_v3::clock::ReelClock;
use forge_reel_v3::edl::{column_of, EdlReceipt, EdlRow};
use forge_reel_v3::gif_window::GifWindow;

const W: u16 = 128;
const H: u16 = 96;
/// One column per Drop Law dwell; 8 columns is long enough to read the hold.
const COLUMNS: u32 = 8;

/// A molten ramp: index 0 is soot, 255 is ember. Indexed palette, 3 bytes/entry.
fn molten_palette() -> Vec<u8> {
    let mut p = Vec::with_capacity(256 * 3);
    for i in 0..256u32 {
        // Integer ramp: red leads, green trails, blue barely rises.
        p.push((i * 255 / 255) as u8);
        p.push((i * 106 / 255) as u8);
        p.push((i * 26 / 255) as u8);
    }
    p
}

/// Column `col` as indexed pixels — a rising band, so each frame differs and
/// the hold is legible when the GIF plays.
fn column_pixels(col: u32) -> Vec<u8> {
    let mut px = vec![0u8; W as usize * H as usize];
    let band = (H as u32 * (col + 1) / COLUMNS).min(H as u32);
    for y in (H as u32 - band)..H as u32 {
        for x in 0..W as u32 {
            let heat = 40 + (x * 215 / W as u32);
            px[(y * W as u32 + x) as usize] = heat as u8;
        }
    }
    px
}

fn main() {
    let clock = ReelClock::kept();
    println!(
        "clock: dwell {}ms · {} carrier frames/column · {}cs GIF delay",
        clock.dwell_ms(),
        clock.frames_per_column(),
        clock.dwell_ms() / 10
    );

    let mut win = GifWindow::new(clock, W, H, molten_palette()).expect("KEPT_MS clears the floor");
    for col in 0..COLUMNS {
        win.add_frame(col, &column_pixels(col)).expect("column encodes");
    }

    // The manifest side of the same tape: one EDL row per column, stamped with
    // the carrier frame that column starts on.
    let rows: Vec<EdlRow> = (0..COLUMNS)
        .map(|col| EdlRow {
            frame: col as usize,
            file: format!("{col:04}.gif"),
            tick: u64::from(col * clock.frames_per_column()),
            scene: 0,
            truth: "the ember holds".to_string(),
            palette: "molten".to_string(),
            cam: "hold".to_string(),
            flash: false,
            scar: false,
            voice_note: None,
            wav_offset_ms: u64::from(col * clock.dwell_ms()),
        })
        .collect();

    for (i, r) in rows.iter().enumerate() {
        let edl_col = column_of(r, &clock).expect("row stamps");
        let gif_col = win.column_of_frame(i as u32).expect("frame exists");
        println!("frame {i} · edl column {edl_col} · gif column {gif_col} · tick {}", r.tick);
        assert_eq!(edl_col, gif_col, "the manifest and the reel must agree on the column");
    }

    let out_dir = std::path::Path::new(".forge/photons");
    std::fs::create_dir_all(out_dir).expect("photon dir");
    let manifest = out_dir.join("reel_window.jsonl");
    forge_reel_v3::edl::write_edl(
        &manifest,
        &rows,
        &EdlReceipt { wav: "none".to_string(), width: u32::from(W), height: u32::from(H) },
    )
    .expect("manifest writes");

    let bytes = win.finalize().expect("gif encodes");
    let gif_path = out_dir.join("reel_window.gif");
    std::fs::write(&gif_path, &bytes).expect("gif writes");
    println!("gif: {} ({} bytes)", gif_path.display(), bytes.len());
    println!("edl: {}", manifest.display());
}
