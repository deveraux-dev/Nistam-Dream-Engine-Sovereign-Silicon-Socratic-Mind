//! F3 — Proof harness RED→GREEN demonstration in v3.
//!
//! This test demonstrates the reusable proof scaffold that every future task calls.
//! It exercises the FULL harness path.

use forge_ocular_v3::{colour_check, confirm_pixels, CheckState};

/// RED case: a vivid green authored colour observed as grey — chroma collapse.
/// The harness MUST reject this (Fail). This is the RED proof: a genuine defect
/// (grey-collapse in the compositor) gets caught by the ColourID gate.
#[test]
fn red_grey_collapse_detected() {
    let authored = [40u8, 200, 80]; // vivid green
    let observed = [128, 128, 128]; // grey (chroma collapsed)
    let check = colour_check(authored, observed);
    assert_eq!(
        check.state,
        CheckState::Fail,
        "RED: grey-collapse MUST fail the ColourID gate — the harness catches compositor defects"
    );
    println!(
        "[F3 RED] ✓ grey-collapse correctly rejected: hue_delta={}, chroma_delta={}",
        check.hue_delta, check.chroma_delta_pmy
    );
}

/// RED case: a blank/trivial frame. The harness MUST detect non-triviality failure.
#[test]
fn red_blank_frame_detected() {
    // All-black frame (common failure: GPU never presented, readback is zeroed)
    let black = vec![0u8; 128 * 128 * 4];
    let (_phash, content_tiles, total_tiles) = confirm_pixels(&black, 128, 128);
    assert_eq!(
        content_tiles, 1,
        "RED: uniform frame must collapse to 1 distinct tile hash (trivial)"
    );
    assert!(total_tiles > 1);
    println!(
        "[F3 RED] ✓ blank frame detected as trivial: content={}, total={}",
        content_tiles, total_tiles
    );
}

/// GREEN case: a correctly composed frame with rich colour preserved.
#[test]
fn green_rich_colour_preserved() {
    let authored = [200u8, 60, 40]; // warm red-orange
    let observed = [198, 62, 41]; // tiny GPU quantise jitter (normal)
    let check = colour_check(authored, observed);
    assert_eq!(
        check.state,
        CheckState::Pass,
        "GREEN: small jitter within tolerance MUST pass"
    );
    println!(
        "[F3 GREEN] ✓ rich colour preserved: hue_delta={}, value_delta={}, chroma_delta={}",
        check.hue_delta, check.value_delta_pmy, check.chroma_delta_pmy
    );
}

/// GREEN case: a structured (non-trivial) frame passes the content gate.
#[test]
fn green_structured_frame_passes() {
    // Top half = vivid green, bottom half = deep blue — two distinct regions
    let (w, h) = (128usize, 128usize);
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            if y < h / 2 {
                px[i] = 40;
                px[i + 1] = 200;
                px[i + 2] = 80;
            } else {
                px[i] = 30;
                px[i + 1] = 50;
                px[i + 2] = 180;
            }
            px[i + 3] = 255;
        }
    }
    let (_phash, content_tiles, total_tiles) = confirm_pixels(&px, w, h);
    assert!(
        content_tiles > 1,
        "GREEN: structured frame must have multiple distinct tile hashes"
    );
    println!(
        "[F3 GREEN] ✓ structured frame: content={} distinct tiles out of {} total",
        content_tiles, total_tiles
    );
}

/// Integration: the full RED→GREEN cycle in one test.
/// First fail (RED proves the gate catches defects), then pass (GREEN proves correctness).
#[test]
fn full_red_green_cycle() {
    // RED: hue flip (authored green, observed red) must FAIL
    let red_check = colour_check([40, 210, 60], [210, 40, 40]);
    assert_eq!(red_check.state, CheckState::Fail, "RED: hue flip must fail");

    // GREEN: same colour with tiny jitter must PASS
    let green_check = colour_check([40, 210, 60], [42, 208, 62]);
    assert_eq!(green_check.state, CheckState::Pass, "GREEN: jitter must pass");

    println!("[F3] ✓ Full RED→GREEN cycle proven in one test");
    println!("  RED:   hue_delta={} → Fail (correct rejection)", red_check.hue_delta);
    println!("  GREEN: hue_delta={} → Pass (correct acceptance)", green_check.hue_delta);
}
