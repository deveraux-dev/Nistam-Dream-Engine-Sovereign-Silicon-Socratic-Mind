//! The lockstep chain, as pixels.
//!
//! Each released tick has a 64-bit chain hash. Here every bit becomes a block, so a tick is a
//! 64-block strip: ALICE on top, BOB directly beneath. Two machines that agree paint two
//! identical strips. The corrupted tick paints two visibly different ones, in red.
//!
//! This is the L09/witness face of `forge_core_v3::lockstep` — not a test asserting equality,
//! but a surface where disagreement is something you can see.
//!
//! Run: `cargo run -p forge-canvas-v3 --example lockstep_photon`
//! Writes: `.forge/lockstep_photon.bmp`

use forge_canvas_v3::rasterizer::{write_bmp, PixelBuffer};
use forge_core_v3::lockstep::LockstepBarrier;

/// Pixels per hash bit.
const SCALE: u32 = 9;
/// Bits in a chain hash.
const BITS: u32 = 64;
/// Height of one machine's strip.
const STRIP_H: u32 = SCALE;
/// Vertical gap between one tick's pair and the next.
const GAP: u32 = 5;
/// Left/top margin.
const MARGIN: u32 = 12;

const BG: [u8; 4] = [10, 12, 16, 255];
const AGREE_ON: [u8; 4] = [53, 214, 160, 255];
const AGREE_OFF: [u8; 4] = [20, 35, 30, 255];
const DESYNC_ON: [u8; 4] = [255, 77, 77, 255];
const DESYNC_OFF: [u8; 4] = [42, 20, 22, 255];

fn fill_rect(buf: &mut PixelBuffer, x: u32, y: u32, w: u32, h: u32, rgba: [u8; 4]) {
    for py in y..(y + h).min(buf.height) {
        for px in x..(x + w).min(buf.width) {
            let at = ((py * buf.width + px) * 4) as usize;
            buf.data[at..at + 4].copy_from_slice(&rgba);
        }
    }
}

fn get_pixel(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * buf.width + x) * 4) as usize;
    [buf.data[at], buf.data[at + 1], buf.data[at + 2], buf.data[at + 3]]
}

/// Paint one 64-bit hash as a strip of blocks, MSB at the left.
fn paint_hash(buf: &mut PixelBuffer, y: u32, hash: u64, agree: bool) {
    let (on, off) = if agree { (AGREE_ON, AGREE_OFF) } else { (DESYNC_ON, DESYNC_OFF) };
    for bit in 0..BITS {
        let set = (hash >> (BITS - 1 - bit)) & 1 == 1;
        let x = MARGIN + bit * SCALE;
        // One texel of breathing room between blocks, so bits stay countable by eye.
        fill_rect(buf, x, y, SCALE - 1, STRIP_H - 1, if set { on } else { off });
    }
}

fn play(m: &mut LockstepBarrier, tick: u64, order: &[usize], script: &[(u8, u32); 3]) -> u64 {
    for &i in order {
        let (peer, input) = script[i];
        m.submit(tick, peer, input).expect("input inside window");
    }
    m.try_advance().expect("all peers in").chain_hash
}

fn main() {
    let script: [[(u8, u32); 3]; 5] = [
        [(0, 0x01), (1, 0x04), (2, 0x00)],
        [(0, 0x01), (1, 0x04), (2, 0x10)],
        [(0, 0x09), (1, 0x00), (2, 0x10)],
        [(0, 0x08), (1, 0x02), (2, 0x14)],
        [(0, 0x00), (1, 0x02), (2, 0x04)],
    ];

    let mut alice = LockstepBarrier::new(3);
    let mut bob = LockstepBarrier::new(3);

    // (tick, alice, bob) — bob always receives the same inputs in shuffled arrival order.
    let mut chain: Vec<(u64, u64, u64)> = Vec::new();

    for (t, s) in script.iter().enumerate() {
        let tick = t as u64;
        let a = play(&mut alice, tick, &[0, 1, 2], s);
        let b = play(&mut bob, tick, &[2, 0, 1], s);
        chain.push((tick, a, b));
    }

    // The corrupted tick: one extra bit on peer 1, bob only.
    let t = alice.tick();
    let truth = [(0u8, 0x01u32), (1, 0x08), (2, 0x02)];
    let mut corrupt = truth;
    corrupt[1].1 = 0x0c;
    for &(p, i) in truth.iter() {
        alice.submit(t, p, i).unwrap();
    }
    for &(p, i) in corrupt.iter() {
        bob.submit(t, p, i).unwrap();
    }
    let a = alice.try_advance().unwrap().chain_hash;
    let b = bob.try_advance().unwrap().chain_hash;
    chain.push((t, a, b));

    // Rollback bob onto alice's authoritative point, then replay one clean tick.
    bob.rollback_to(alice.tick(), alice.chain_hash());
    let t = alice.tick();
    let resume = [(0u8, 0x10u32), (1, 0x01), (2, 0x08)];
    for &(p, i) in resume.iter() {
        alice.submit(t, p, i).unwrap();
        bob.submit(t, p, i).unwrap();
    }
    let a = alice.try_advance().unwrap().chain_hash;
    let b = bob.try_advance().unwrap().chain_hash;
    chain.push((t, a, b));

    // ---- paint ----
    let pair_h = STRIP_H * 2 + GAP;
    let w = MARGIN * 2 + BITS * SCALE;
    let h = MARGIN * 2 + pair_h * chain.len() as u32;

    let mut buf = PixelBuffer::new(w, h);
    fill_rect(&mut buf, 0, 0, w, h, BG);

    for (row, &(_tick, a, b)) in chain.iter().enumerate() {
        let y = MARGIN + row as u32 * pair_h;
        let agree = a == b;
        paint_hash(&mut buf, y, a, agree);
        paint_hash(&mut buf, y + STRIP_H, b, agree);
    }

    // ---- readback receipt (L09: the surface is proven by reading it back) ----
    let mut lit = 0usize;
    let mut red = 0usize;
    for y in 0..buf.height {
        for x in 0..buf.width {
            let p = get_pixel(&buf, x, y);
            if p == AGREE_ON {
                lit += 1;
            } else if p == DESYNC_ON {
                red += 1;
            }
        }
    }

    // Prove at the PIXEL level — not from the hashes that drew them — that each agreeing tick's
    // two strips are identical and the desyncing one is not. Reading the surface back is the
    // only thing that proves the surface; the hash comparison above proves only the math.
    let mut pixel_diffs: Vec<usize> = Vec::new();
    for row in 0..chain.len() {
        let y_a = MARGIN + row as u32 * pair_h;
        let y_b = y_a + STRIP_H;
        let mut differing = 0;
        for bit in 0..BITS {
            let x = MARGIN + bit * SCALE;
            if get_pixel(&buf, x, y_a) != get_pixel(&buf, x, y_b) {
                differing += 1;
            }
        }
        pixel_diffs.push(differing);
    }
    for (row, &(_, a, b)) in chain.iter().enumerate() {
        let painted_same = pixel_diffs[row] == 0;
        assert_eq!(
            painted_same,
            a == b,
            "row {row}: pixels and hashes disagree — the surface is lying about the math"
        );
    }
    let differing_blocks = pixel_diffs.iter().copied().max().unwrap_or(0);

    let out = std::path::Path::new(".forge/lockstep_photon.bmp");
    write_bmp(&buf, out).expect("write .forge/lockstep_photon.bmp");

    println!("LOCKSTEP PHOTON");
    println!("  surface      : {w} x {h} px, RGBA8, no filter, no sRGB, no compression");
    println!("  ticks painted: {} (each = 2 strips of {BITS} bits)", chain.len());
    for (row, &(tick, a, b)) in chain.iter().enumerate() {
        println!(
            "    tick {:>2}  alice {:016x}  bob {:016x}  {:<6}  pixel-diff {:>2}/{BITS}",
            tick,
            a,
            b,
            if a == b { "agree" } else { "DESYNC" },
            pixel_diffs[row]
        );
    }
    println!("  readback     : {lit} teal texels (agreeing bits), {red} red texels (desync bits)");
    println!("  desync row   : {differing_blocks}/{BITS} hash bits differ between alice and bob");
    println!("  written      : {}", out.display());
}
