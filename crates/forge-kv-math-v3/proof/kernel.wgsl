// GPU side of the determinism proof — hand-mirrored from the Rust reference.
// WGSL u32 arithmetic wraps (mod 2^32), matching Rust's wrapping_mul/_add.
// Shifts are < 32 (logical, defined). No float. => bit-identical to the CPU.

fn prismatic_hash(x: u32, y: u32) -> u32 {
    var h: u32 = x * 0x9E3779B1u;        // 2^32 / golden ratio
    h = h ^ (y * 0x85EBCA77u);
    h = h * 0xC2B2AE3Du;
    h = h ^ (h >> 15u);
    h = h * 0x27D4EB2Fu;
    h = h ^ (h >> 13u);
    return h;
}

@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64)
fn main_cs(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&data)) {
        data[i] = prismatic_hash(i, i ^ 0xABCD1234u);
    }
}
