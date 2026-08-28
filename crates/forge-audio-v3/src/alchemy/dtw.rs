//! DTW — Dynamic Time Warping for tempo alignment.

/// Compute the DTW alignment path between two feature sequences.
/// Uses Sakoe-Chiba band constraint with `radius` to limit computation.
/// Returns Vec of (index_a, index_b) pairs representing the optimal warping path.
pub fn dtw_align(a: &[f32], b: &[f32], radius: usize) -> Vec<(usize, usize)> {
    let n = a.len();
    let m = b.len();

    if n == 0 || m == 0 {
        return Vec::new();
    }

    let mut cost = vec![vec![f32::INFINITY; m]; n];

    for i in 0..n {
        let j_center = (i as f64 * m as f64 / n as f64) as usize;
        let j_start = j_center.saturating_sub(radius);
        let j_end = (j_center + radius + 1).min(m);
        for j in j_start..j_end {
            let local = (a[i] - b[j]).powi(2);
            let prev = match (i, j) {
                (0, 0) => 0.0,
                (0, _) => cost[0][j - 1],
                (_, 0) => cost[i - 1][0],
                _ => cost[i - 1][j - 1]
                    .min(cost[i - 1][j])
                    .min(cost[i][j - 1]),
            };
            cost[i][j] = local + prev;
        }
    }

    let mut path = Vec::new();
    let mut i = n - 1;
    let mut j = m - 1;
    path.push((i, j));

    while i > 0 || j > 0 {
        if i == 0 {
            j -= 1;
        } else if j == 0 {
            i -= 1;
        } else {
            let diag = cost[i - 1][j - 1];
            let up = cost[i - 1][j];
            let left = cost[i][j - 1];
            if diag <= up && diag <= left {
                i -= 1;
                j -= 1;
            } else if up <= left {
                i -= 1;
            } else {
                j -= 1;
            }
        }
        path.push((i, j));
    }

    path.reverse();
    path
}

/// Convert a DTW warping path into a per-frame time-stretch ratio sequence.
pub fn path_to_stretch_map(path: &[(usize, usize)], n_frames_a: usize) -> Vec<f32> {
    let mut stretch = vec![1.0f32; n_frames_a];
    let mut i = 0;
    while i < path.len() {
        let a_idx = path[i].0;
        let mut count = 0;
        while i < path.len() && path[i].0 == a_idx {
            count += 1;
            i += 1;
        }
        if a_idx < n_frames_a {
            stretch[a_idx] = count as f32;
        }
    }
    stretch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtw_identical_sequences() {
        let a = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let path = dtw_align(&a, &b, 2);
        assert_eq!(path.len(), 5);
        for (i, &(ai, bi)) in path.iter().enumerate() {
            assert_eq!(ai, i);
            assert_eq!(bi, i);
        }
    }

    #[test]
    fn test_dtw_shifted_sequence() {
        let a = vec![1.0f32, 2.0, 3.0, 4.0];
        let b = vec![1.0f32, 1.0, 2.0, 3.0, 3.0, 4.0];
        let path = dtw_align(&a, &b, 3);
        assert_eq!(path.first().unwrap(), &(0, 0));
        assert_eq!(path.last().unwrap(), &(3, 5));
    }

    #[test]
    fn test_dtw_sakoe_chiba_constrains() {
        let n = 100;
        let a: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..n).map(|i| i as f32 + 0.5).collect();
        let path = dtw_align(&a, &b, 5);
        for &(i, j) in &path {
            assert!(
                (i as i32 - j as i32).abs() <= 6,
                "({},{}) exceeds Sakoe-Chiba band of 5",
                i,
                j
            );
        }
    }
}
