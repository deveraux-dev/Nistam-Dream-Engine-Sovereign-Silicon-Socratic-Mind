//! Run-length — RLE encode/decode a tilemap's u16 tiles as (count, value) runs.
//! A compact encoding for large flat regions.

/// Encode `tiles` into `(count, value)` runs.
pub fn encode(tiles: &[u16]) -> Vec<(u32, u16)> {
    let mut runs = Vec::new();
    let mut iter = tiles.iter().copied();
    let Some(mut cur) = iter.next() else {
        return runs;
    };
    let mut count = 1u32;
    for t in iter {
        if t == cur {
            count += 1;
        } else {
            runs.push((count, cur));
            cur = t;
            count = 1;
        }
    }
    runs.push((count, cur));
    runs
}

/// Decode `(count, value)` runs back into tiles.
pub fn decode(runs: &[(u32, u16)]) -> Vec<u16> {
    let mut out = Vec::new();
    for &(count, value) in runs {
        out.extend(std::iter::repeat_n(value, count as usize));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let tiles = vec![5, 5, 5, 2, 2, 7, 7, 7, 7];
        let runs = encode(&tiles);
        assert_eq!(runs, vec![(3, 5), (2, 2), (4, 7)]);
        assert_eq!(decode(&runs), tiles);
    }

    #[test]
    fn empty_and_singleton() {
        assert!(encode(&[]).is_empty());
        assert_eq!(encode(&[9]), vec![(1, 9)]);
        assert_eq!(decode(&[(1, 9)]), vec![9]);
    }
}
