// Native i64 GPU kernel — requires Features::SHADER_INT64 (naga Capabilities::SHADER_INT64).
// Mirrors the Rust reference: prod = a*b (low 64 bits, two's-complement wrap),
// quot = prod / 10000 (signed, truncates toward zero). No float. => bit-identical to CPU.
//
// Inputs a,b and outputs prod,quot are 8-byte little-endian i64; identical buffer
// bytes are also consumed by kernel_i64_emu.wgsl as vec2<u32>.

@group(0) @binding(0) var<storage, read>       a:    array<i64>;
@group(0) @binding(1) var<storage, read>       b:    array<i64>;
@group(0) @binding(2) var<storage, read_write> prod: array<i64>;
@group(0) @binding(3) var<storage, read_write> quot: array<i64>;

@compute @workgroup_size(64)
fn main_cs(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&a)) {
        let p: i64 = a[i] * b[i];
        prod[i] = p;
        quot[i] = p / i64(10000);   // divide-back by the Permyriad denominator
    }
}
