//! Agglomerative (average-linkage) clustering of speaker embeddings by cosine similarity.

// ponytail: calibration knobs, measured on a real 80-minute Teams meeting (348 remote segments):
// 0.6 gave 167 clusters, 0.35 gives 9 substantial ones covering 87% of the speech. Synthetic TTS
// voices are cleaner (same voice ≈ 0.9) and are not the reference.
pub const SAME_SPEAKER_THRESHOLD: f32 = 0.35;
/// Clusters smaller than this are usually one speaker's badly embedded segments…
const MIN_CLUSTER_SIZE: usize = 3;
/// …and join the most similar substantial cluster when at least this similar (a genuinely
/// different voice that spoke once or twice stays on its own).
const MIN_REASSIGN_SIMILARITY: f32 = 0.2;

pub fn normalized(v: &[f32]) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    v.iter().map(|x| x / n).collect()
}

/// Unit-length mean of a set of embeddings (a voice's print).
pub fn mean_voice<'a>(embs: impl IntoIterator<Item = &'a Vec<f32>>) -> Option<Vec<f32>> {
    let mut sum: Option<Vec<f32>> = None;
    for e in embs {
        let u = normalized(e);
        match &mut sum {
            Some(s) => s.iter_mut().zip(&u).for_each(|(a, b)| *a += b),
            None => sum = Some(u),
        }
    }
    sum.map(|s| normalized(&s))
}

/// Unit-length mean embedding per cluster label (labels as returned by [`cluster`]).
pub fn centroids(embs: &[Vec<f32>], labels: &[u32]) -> std::collections::BTreeMap<u32, Vec<f32>> {
    let mut by_label: std::collections::BTreeMap<u32, Vec<&Vec<f32>>> = Default::default();
    for (e, l) in embs.iter().zip(labels) {
        by_label.entry(*l).or_default().push(e);
    }
    by_label.into_iter().filter_map(|(l, es)| mean_voice(es).map(|c| (l, c))).collect()
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

    // Fold tiny clusters into the nearest substantial one (by centroid similarity).
    let centroid = |members: &[usize]| {
        let mut c = vec![0f32; unit[0].len()];
        for &m in members {
            c.iter_mut().zip(&unit[m]).for_each(|(a, b)| *a += b);
        }
        normalized(&c)
    };
    let big: Vec<(usize, Vec<f32>)> = (0..n)
        .filter(|&i| active[i] && members[i].len() >= MIN_CLUSTER_SIZE)
        .map(|i| (i, centroid(&members[i])))
        .collect();
    if !big.is_empty() {
        for small in (0..n).filter(|&i| active[i] && members[i].len() < MIN_CLUSTER_SIZE).collect::<Vec<_>>() {
            let c = centroid(&members[small]);
            let best = big
                .iter()
                .map(|(root, bc)| (*root, c.iter().zip(bc).map(|(a, b)| a * b).sum::<f32>()))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((root, _)) = best.filter(|(_, sim)| *sim >= MIN_REASSIGN_SIMILARITY) {
                let moved = std::mem::take(&mut members[small]);
                members[root].extend(moved);
                active[small] = false;
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

    #[test]
    fn tiny_clusters_join_the_nearest_big_one() {
        // Two real speakers (4 segments each) and one stray segment closer to the first.
        let mut embs = vec![v(1.0, 0.0, 0.0); 4];
        embs.extend(vec![v(0.0, 1.0, 0.0); 4]);
        embs.push(v(0.3, 0.1, 0.95)); // cosine ≈ 0.30 / 0.10: below the threshold, nearer to the first
        let labels = cluster(&embs, SAME_SPEAKER_THRESHOLD);
        assert_eq!(labels, vec![0, 0, 0, 0, 1, 1, 1, 1, 0]);
    }
}
