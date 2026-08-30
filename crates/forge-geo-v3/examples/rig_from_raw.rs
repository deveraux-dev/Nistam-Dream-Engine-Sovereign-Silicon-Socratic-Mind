//! Auto-rig a raw 8-bit alpha sheet and report where the joints landed.
//! `cargo run -p forge-geo-v3 --example rig_from_raw -- <alpha.bin> <w> <h>`
//!
//! The crate has no image decoder, so the caller supplies one byte of alpha per
//! texel, row-major. Prints the per-region share of the subject plus any
//! proportion warnings — the fast way to see whether a sheet is actually in the
//! pose `DEFAULT_REGIONS` assumes.

use forge_geo_v3::auto_rig::{auto_rig, validate_rig, WarnSeverity, DEFAULT_REGIONS};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!("usage: rig_from_raw <alpha.bin> <width> <height>");
        std::process::exit(2);
    }
    let (w, h) = match (args[1].parse::<u32>(), args[2].parse::<u32>()) {
        (Ok(w), Ok(h)) => (w, h),
        _ => {
            eprintln!("width/height must be u32");
            std::process::exit(2);
        }
    };
    let alpha = match std::fs::read(&args[0]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {}: {e}", args[0]);
            std::process::exit(1);
        }
    };
    let want = w as usize * h as usize;
    if alpha.len() != want {
        eprintln!("alpha len {} != {w}x{h} = {want}", alpha.len());
        std::process::exit(1);
    }

    // One byte of alpha -> packed 0xAARRGGBB (colour is irrelevant to the rig).
    let pixels: Vec<u32> = alpha.iter().map(|&a| (a as u32) << 24).collect();

    let r = auto_rig(&pixels, w, h, &DEFAULT_REGIONS);
    println!("SHEET {w}x{h} · assigned {} of {want} texels", r.total_assigned);
    if r.total_assigned == 0 {
        println!("(nothing opaque — nothing to rig)");
        return;
    }

    let mut share: Vec<(usize, u32, i32)> = DEFAULT_REGIONS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let c = r.counts[i];
            (i, c, (c as i64 * 1000 / r.total_assigned as i64) as i32)
        })
        .collect();
    share.sort_by(|a, b| b.1.cmp(&a.1));
    for (i, count, permille) in &share {
        let bar = "#".repeat((*permille / 10).max(0) as usize);
        println!("  {:<12} {:>7}  {:>4}.{}%  {bar}", DEFAULT_REGIONS[*i].id_str(), count, permille / 10, permille % 10);
    }

    let (warns, n) = validate_rig(&r, &DEFAULT_REGIONS);
    if n == 0 {
        println!("RIG  clean — no proportion warnings");
    } else {
        println!("RIG  {n} warning(s):");
        for wn in &warns[..n] {
            let tag = match wn.severity {
                WarnSeverity::Error => "ERROR",
                WarnSeverity::Warn => "warn ",
            };
            println!("  {tag} {}", DEFAULT_REGIONS[wn.joint_idx].id_str());
        }
    }
}
