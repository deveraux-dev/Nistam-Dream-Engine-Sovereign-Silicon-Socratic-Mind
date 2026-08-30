// Emulated i64 GPU kernel — i64 represented as vec2<u32> (.x = low 32, .y = high 32).
// Uses ONLY core WGSL u32 arithmetic (no SHADER_INT64), so it runs and is
// bit-deterministic on ANY GPU that the proven u32 kernel runs on. This is the
// cross-vendor cornerstone: it proves wide-integer determinism without native i64.
//
// Two's-complement throughout: the 64-bit pattern IS the value; sign is bit 63.
// Buffer bytes are identical to the native kernel's i64 arrays (little-endian:
// element[0]=low u32 at offset 0, element[1]=high u32 at offset 4).

// ---- 64-bit primitives over vec2<u32> ----------------------------------------

// a + b (mod 2^64), with carry from the low word into the high word.
fn add64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    let carry = select(0u, 1u, lo < a.x); // unsigned wrap detected => carry out
    let hi = a.y + b.y + carry;
    return vec2<u32>(lo, hi);
}

// a - b (mod 2^64), with borrow.
fn sub64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x - b.x;
    let borrow = select(0u, 1u, a.x < b.x);
    let hi = a.y - b.y - borrow;
    return vec2<u32>(lo, hi);
}

// two's-complement negation: ~a + 1.
fn neg64(a: vec2<u32>) -> vec2<u32> {
    return add64(vec2<u32>(~a.x, ~a.y), vec2<u32>(1u, 0u));
}

fn is_neg64(a: vec2<u32>) -> bool {
    return (a.y >> 31u) != 0u;
}

// unsigned 32x32 -> 64, via 16-bit limbs (each partial product fits in u32).
fn mul32x32(x: u32, y: u32) -> vec2<u32> {
    let x0 = x & 0xFFFFu; let x1 = x >> 16u;
    let y0 = y & 0xFFFFu; let y1 = y >> 16u;
    let p00 = x0 * y0;
    let p01 = x0 * y1;
    let p10 = x1 * y0;
    let p11 = x1 * y1;
    // carry = high half of p00 plus the low halves of the two middle products.
    let carry = (p00 >> 16u) + (p01 & 0xFFFFu) + (p10 & 0xFFFFu);
    let lo = (p00 & 0xFFFFu) | ((carry & 0xFFFFu) << 16u);
    let hi = p11 + (p01 >> 16u) + (p10 >> 16u) + (carry >> 16u);
    return vec2<u32>(lo, hi);
}

// i64 * i64 -> low 64 bits. Sign-agnostic: low bits of a*b are identical whether
// operands are read signed or unsigned (two's-complement property). The a.hi*b.hi
// term lands entirely at >= bit 64 and is discarded mod 2^64.
fn mul64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let ll = mul32x32(a.x, b.x);
    let cross = a.x * b.y + a.y * b.x; // u32-wrapping; only its low 32 bits survive
    return vec2<u32>(ll.x, ll.y + cross);
}

// unsigned compare a >= b.
fn ge64(a: vec2<u32>, b: vec2<u32>) -> bool {
    if (a.y != b.y) { return a.y > b.y; }
    return a.x >= b.x;
}

// logical shift-left by 1.
fn shl1(a: vec2<u32>) -> vec2<u32> {
    let hi = (a.y << 1u) | (a.x >> 31u);
    let lo = a.x << 1u;
    return vec2<u32>(lo, hi);
}

fn bit64(a: vec2<u32>, i: u32) -> u32 {
    if (i < 32u) { return (a.x >> i) & 1u; }
    return (a.y >> (i - 32u)) & 1u;
}

// unsigned 64/64 division (floor). Binary long division, MSB-first, 64 steps.
// Divisor must be non-zero (the divide-back constant 10000 is non-zero).
fn divu64(n: vec2<u32>, d: vec2<u32>) -> vec2<u32> {
    var q = vec2<u32>(0u, 0u);
    var r = vec2<u32>(0u, 0u);
    for (var i: i32 = 63; i >= 0; i = i - 1) {
        r = shl1(r);
        r.x = r.x | bit64(n, u32(i)); // bring down the next dividend bit
        if (ge64(r, d)) {
            r = sub64(r, d);
            let bi = u32(i);
            if (bi < 32u) { q.x = q.x | (1u << bi); }
            else          { q.y = q.y | (1u << (bi - 32u)); }
        }
    }
    return q;
}

// signed i64 / i64, truncating toward zero (matches Rust's `/` and wrapping_div
// for every input here; |MIN| is representable as a u64 magnitude so MIN/-1 is
// handled too, though the corpus divides only by the positive constant 10000).
fn div64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let an = is_neg64(a);
    let bn = is_neg64(b);
    let ua = select(a, neg64(a), an); // magnitudes
    let ub = select(b, neg64(b), bn);
    let uq = divu64(ua, ub);
    if (an != bn) { return neg64(uq); } // result sign = XOR of operand signs
    return uq;
}

// ---- kernel ------------------------------------------------------------------

@group(0) @binding(0) var<storage, read>       a:    array<vec2<u32>>;
@group(0) @binding(1) var<storage, read>       b:    array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> prod: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> quot: array<vec2<u32>>;

@compute @workgroup_size(64)
fn main_cs(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&a)) {
        let p = mul64(a[i], b[i]);
        prod[i] = p;
        quot[i] = div64(p, vec2<u32>(10000u, 0u)); // divide-back by Permyriad denominator
    }
}
