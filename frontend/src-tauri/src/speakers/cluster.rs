//! Agglomerative (average-linkage) clustering of speaker embeddings by cosine similarity.

// ponytail: calibration knob. Measured on Italian TTS voices: same speaker ≈ 0.9, distinct but similar
// male voices average ≈ 0.55 (max 0.61); 0.6 keeps those apart. Retune on real meetings.
pub const SAME_SPEAKER_THRESHOLD: f32 = 0.6;

fn normalized(v: &[f32]) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    v.iter().map(|x| x / n).collect()
}

/// Labels per embedding, numbered by first appearance; merges while best average similarity ≥ threshold.
// ponytail: dense k×k matrix, fine for the ~1–2k segments of a long meeting.
pub fn cluster(embs: &[Vec<f32>], threshold: f32) -> Vec<u32> {
    let n = embs.len();
    if n == 0 {
        return Vec::new();
    }
    let unit: Vec<Vec<f32>> = embs.iter().map(|e| normalized(e)).collect();
    let mut sim = vec![vec![0f32; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let s: f32 = unit[i].iter().zip(&unit[j]).map(|(a, b)| a * b).sum();
            sim[i][j] = s;
            sim[j][i] = s;
        }
    }
    let mut active = vec![true; n];
    let mut size = vec![1usize; n];
    let mut members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let best_of = |i: usize, sim: &Vec<Vec<f32>>, active: &Vec<bool>| {
        (0..n)
            .filter(|&j| j != i && active[j])
            .max_by(|&a, &b| sim[i][a].total_cmp(&sim[i][b]))
    };
    let mut best: Vec<Option<usize>> = (0..n).map(|i| best_of(i, &sim, &active)).collect();

    loop {
        let pick = (0..n)
            .filter(|&i| active[i])
            .filter_map(|i| best[i].map(|j| (i, j, sim[i][j])))
            .max_by(|a, b| a.2.total_cmp(&b.2));
        let Some((i, j, s)) = pick else { break };
        if s < threshold {
            break;
        }
        // Merge j into i (Lance–Williams update for average linkage).
        let (si, sj) = (size[i] as f32, size[j] as f32);
        for l in 0..n {
            if active[l] && l != i && l != j {
                let v = (si * sim[i][l] + sj * sim[j][l]) / (si + sj);
                sim[i][l] = v;
                sim[l][i] = v;
            }
        }
        active[j] = false;
        size[i] += size[j];
        let moved = std::mem::take(&mut members[j]);
        members[i].extend(moved);
        for l in 0..n {
            if !active[l] {
                continue;
            }
            if l == i || best[l] == Some(i) || best[l] == Some(j) {
                best[l] = best_of(l, &sim, &active);
            } else if l != i && best[l].map_or(true, |b| sim[l][i] > sim[l][b]) {
                best[l] = Some(i);
            }
        }
    }

    let mut roots: Vec<usize> = (0..n).filter(|&i| active[i]).collect();
    roots.sort_by_key(|&r| members[r].iter().min().copied().unwrap_or(r));
    let mut labels = vec![0u32; n];
    for (label, r) in roots.iter().enumerate() {
        for &m in &members[*r] {
            labels[m] = label as u32;
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec<f32> {
        vec![x, y, z]
    }

    #[test]
    fn cluster_separates_two_groups() {
        let embs = vec![v(1.0, 0.1, 0.0), v(0.9, 0.0, 0.1), v(0.0, 1.0, 0.1), v(0.1, 0.9, 0.0), v(1.0, 0.0, 0.05)];
        assert_eq!(cluster(&embs, SAME_SPEAKER_THRESHOLD), vec![0, 0, 1, 1, 0]);
    }

    #[test]
    fn cluster_first_appearance_order() {
        let embs = vec![v(0.0, 1.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.95, 0.1)];
        assert_eq!(cluster(&embs, SAME_SPEAKER_THRESHOLD), vec![0, 1, 0]);
        assert!(cluster(&[], SAME_SPEAKER_THRESHOLD).is_empty());
    }
}
