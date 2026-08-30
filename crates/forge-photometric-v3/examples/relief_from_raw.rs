//! Run the relief solver over a raw 8-bit coverage buffer and report integer
//! receipts. `cargo run -p forge-photometric-v3 --example relief_from_raw -- <path> <w> <h>`
//!
//! The crate has no image decoder by design (glam + rustfft only), so the caller
//! supplies coverage already flattened to one byte per texel, row-major. The
//! point of this example is the CONTRACT: floats live inside the solver, and
//! everything that comes back out — depth in permyriad, octahedral normals — is
//! integer and bit-checkable.

use forge_photometric_v3::{solver::solve_relief, NormalAlbedo8, PMY_MAX};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("usage: relief_from_raw <coverage.bin> <width> <height> [depth_out.bin]");
        std::process::exit(2);
    }
    let (path, w, h) = (&args[0], args[1].parse::<u16>(), args[2].parse::<u16>());
    let (w, h) = match (w, h) {
        (Ok(w), Ok(h)) => (w, h),
        _ => {
            eprintln!("width/height must be u16");
            std::process::exit(2);
        }
    };

    let cov = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {path}: {e}");
            std::process::exit(1);
        }
    };
    let want = w as usize * h as usize;
    if cov.len() != want {
        eprintln!("coverage len {} != {w}x{h} = {want}", cov.len());
        std::process::exit(1);
    }

    let inked = cov.iter().filter(|&&c| c > 16).count();
    println!("IN   {w}x{h} = {want} texels · inked(>16) {inked} ({}%)", 100 * inked / want);

    let t0 = std::time::Instant::now();
    let relief = solve_relief(&cov, w, h);
    let ms = t0.elapsed().as_millis();

    assert_eq!(relief.depth_pmy.len(), want, "depth is one word per texel");
    assert_eq!(relief.normals.len(), want, "normals are one word per texel");

    // Depth: permyriad, so every value must sit inside the declared ring.
    let (dmin, dmax) = relief
        .depth_pmy
        .iter()
        .fold((u16::MAX, 0u16), |(lo, hi), &d| (lo.min(d), hi.max(d)));
    let dsum: u64 = relief.depth_pmy.iter().map(|&d| d as u64).sum();
    let over = relief.depth_pmy.iter().filter(|&&d| d > PMY_MAX).count();

    // Normals: how many are non-flat, and do they all decode?
    let flat = relief
        .normals
        .iter()
        .filter(|n| n.oct_u == NormalAlbedo8::FLAT.oct_u && n.oct_v == NormalAlbedo8::FLAT.oct_v)
        .count();
    let invalid = relief.normals.iter().filter(|n| !n.is_valid()).count();

    println!("OUT  solve_relief in {ms} ms");
    println!("     depth_pmy  min {dmin} · max {dmax} · mean {} · over PMY_MAX {over}", dsum / want as u64);
    println!("     normals    non-flat {} · flat {flat} · invalid {invalid}", want - flat);

    // The contract this example exists to demonstrate.
    assert_eq!(over, 0, "depth must stay inside the permyriad ring");
    assert_eq!(invalid, 0, "every normal word must decode");

    // Determinism: the same coverage must give the same words, every time.
    let again = solve_relief(&cov, w, h);
    assert_eq!(again.depth_pmy, relief.depth_pmy, "depth is deterministic");
    let same_normals = again
        .normals
        .iter()
        .zip(relief.normals.iter())
        .all(|(a, b)| a.encode() == b.encode());
    assert!(same_normals, "normal words are deterministic");
    println!("OK   depth in-ring · all normals decode · re-run bit-identical");

    // ── Round-trip oracle ────────────────────────────────────────────────
    // solve_relief runs FORWARD (coverage -> height). forge-geo-v3's
    // reverse_poisson runs the other way (height -> gradients). Decomposing the
    // reconstruction and comparing it against the gradients the SOURCE demanded
    // scores fidelity — the two halves check each other instead of me judging a
    // grey image by eye. match_score is permyriad: 10000 = the reconstruction
    // carries exactly the gradient field its input asked for.
    let as_rgba = |grey: &[u8]| -> Vec<u8> {
        let mut v = Vec::with_capacity(grey.len() * 4);
        for &g in grey {
            v.extend_from_slice(&[g, g, g, 255]);
        }
        v
    };
    let depth_grey: Vec<u8> =
        relief.depth_pmy.iter().map(|&d| (d as u32 * 255 / PMY_MAX as u32) as u8).collect();

    let (want_gx, want_gy) =
        forge_geo_v3::reverse_poisson::extract_observed_gradients(&as_rgba(&cov), w as u32, h as u32);
    let (got_gx, got_gy) = forge_geo_v3::reverse_poisson::extract_observed_gradients(
        &as_rgba(&depth_grey),
        w as u32,
        h as u32,
    );
    let diff = forge_geo_v3::reverse_poisson::compute_residual(
        &want_gx,
        &want_gy,
        &got_gx,
        &got_gy,
        w as u32,
        h as u32,
        &forge_geo_v3::reverse_poisson::DifferentialConfig::default(),
    );
    println!(
        "ORACLE reverse-poisson match_score {} pmy · defect regions {}",
        diff.match_score,
        diff.defect_regions.len()
    );

    // Optional: dump the recovered depth as one byte per texel so a caller can
    // look at it. Integer scale-down only — permyriad 0..10000 -> 0..255 by
    // floor division, the same rounding discipline as everything else here.
    if let Some(dst) = args.get(3) {
        let grey: Vec<u8> = relief.depth_pmy.iter().map(|&d| (d as u32 * 255 / PMY_MAX as u32) as u8).collect();
        match std::fs::write(dst, &grey) {
            Ok(()) => println!("DUMP depth -> {dst} ({} bytes, {w}x{h} grey)", grey.len()),
            Err(e) => eprintln!("write {dst}: {e}"),
        }
    }
}
