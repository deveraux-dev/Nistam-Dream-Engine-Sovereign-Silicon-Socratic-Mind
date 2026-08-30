#![allow(missing_docs)]
//! Fixed-point filtering and quantization helpers.

/// Clamp a fixed-point quantity to the engine's standard 0..10000 lane.
#[inline]
pub const fn clamp_q(v: i32) -> i16 {
    if v < 0 {
        0
    } else if v > 10000 {
        10000
    } else {
        v as i16
    }
}

/// Exponential moving average in Q10000 integer space.
///
/// `alpha_q` is clamped to 0..10000.
#[inline]
pub fn ema_q(prev_q: i32, sample_q: i32, alpha_q: i32) -> i32 {
    let alpha = alpha_q.clamp(0, 10000);
    let inv = 10000 - alpha;
    ((prev_q * inv) + (sample_q * alpha)) / 10000
}

/// Low-cost variance proxy for small fixed-size channels.
#[inline]
pub fn variance_q(channels: &[i32]) -> i16 {
    if channels.is_empty() {
        return 0;
    }
    let mut sum: i64 = 0;
    for v in channels {
        sum += *v as i64;
    }
    let mean = sum / channels.len() as i64;
    let mut acc: i64 = 0;
    for v in channels {
        let d = *v as i64 - mean;
        acc += d * d;
    }
    clamp_q((acc / channels.len() as i64).min(10000) as i32)
}

/// Quantize 0..10000 into four deterministic bands.
#[inline]
pub const fn band_0_to_3(v_q: i16) -> u8 {
    if v_q < 2500 {
        0
    } else if v_q < 5000 {
        1
    } else if v_q < 7500 {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ema_is_deterministic() {
        assert_eq!(ema_q(1000, 9000, 2500), 3000);
        assert_eq!(ema_q(1000, 9000, 2500), 3000);
    }

    #[test]
    fn bands_are_stable() {
        assert_eq!(band_0_to_3(0), 0);
        assert_eq!(band_0_to_3(2499), 0);
        assert_eq!(band_0_to_3(2500), 1);
        assert_eq!(band_0_to_3(5000), 2);
        assert_eq!(band_0_to_3(7500), 3);
    }
}
