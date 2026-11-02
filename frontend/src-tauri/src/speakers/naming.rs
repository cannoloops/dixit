//! Pure speaker naming: who said each segment, from the strongest evidence available.

pub use crate::teams::captions::Turn;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The local microphone (the user).
    Me,
    /// Remote participants (system audio).
    Others,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentInfo {
    pub seq: u64,
    pub start: f64,
    pub end: f64,
    pub source: Source,
    pub cluster: Option<u32>,
}

// ponytail: calibration knobs (mic is EBU-normalized, system audio raw); tune on real calls.
const MIC_FLOOR_RMS: f32 = 0.005;
const MIC_OVER_SYSTEM: f32 = 2.0;

pub fn classify_source(mic_rms: f32, sys_rms: f32) -> Source {
    if mic_rms > MIC_FLOOR_RMS && mic_rms > MIC_OVER_SYSTEM * sys_rms { Source::Me } else { Source::Others }
}

/// 0 → "A", 25 → "Z", 26 → "AA" (no digits: parse_teams.py rejects them in names).
pub fn speaker_letter(mut n: u32) -> String {
    let mut out = Vec::new();
    loop {
        out.push((b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.iter().rev().collect()
}

fn best_overlap(turns: &[Turn], seg: &SegmentInfo) -> Option<String> {
    turns
        .iter()
        .map(|t| (t, seg.end.min(t.end) - seg.start.max(t.start)))
        .filter(|(_, overlap)| *overlap > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, _)| t.name.clone())
}

/// Labels no real person carries ("Parlante A", "Altri"): never learned as a voice.
pub fn is_generic(name: &str) -> bool {
    name == "Altri" || name.starts_with("Parlante ")
}

/// `name` (from Teams) is the user: it carries every word of `me` ("Manuguerra" in "Manuguerra Vito").
pub fn is_me(name: &str, me: &str) -> bool {
    let words = |s: &str| -> HashSet<String> {
        s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
    };
    let me = words(me);
    !me.is_empty() && me.is_subset(&words(name))
}

/// One label per segment. Precedence: the mic is `me`; others take the best-overlapping native
/// Teams turn, else caption turn, else their cluster's majority name, else the cluster's `known`
/// voice (voice memory), else "Parlante X"/"Altri". `renames` (manual, from → to) apply last.
pub fn assign_names(
    segs: &[SegmentInfo],
    native: &[Turn],
    captions: &[Turn],
    known: &HashMap<u32, String>,
    renames: &HashMap<String, String>,
    me: &str,
) -> Vec<String> {
    let direct: Vec<Option<String>> = segs
        .iter()
        .map(|s| match s.source {
            Source::Me => Some(me.to_string()),
            // Teams names the user's own turns too: on system audio that is crosstalk or caption lag.
            Source::Others => best_overlap(native, s)
                .or_else(|| best_overlap(captions, s))
                .map(|n| if is_me(&n, me) { me.to_string() } else { n }),
        })
        .collect();

    let mut votes: HashMap<u32, HashMap<&str, usize>> = HashMap::new();
    for (s, name) in segs.iter().zip(&direct) {
        if let (Source::Others, Some(c), Some(n)) = (s.source, s.cluster, name) {
            *votes.entry(c).or_default().entry(n.as_str()).or_default() += 1;
        }
    }
    let cluster_name: HashMap<u32, String> = votes
        .into_iter()
        .filter_map(|(c, v)| {
            // Most votes; ties broken alphabetically so reruns are stable.
            v.into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
                .map(|(name, _)| (c, name.to_string()))
        })
        .collect();

    segs.iter()
        .zip(direct)
        .map(|(s, name)| {
            let label = name
                .or_else(|| s.cluster.and_then(|c| cluster_name.get(&c).or_else(|| known.get(&c)).cloned()))
                .unwrap_or_else(|| match s.cluster {
                    Some(c) => format!("Parlante {}", speaker_letter(c)),
                    None => "Altri".to_string(),
                });
            renames.get(&label).cloned().unwrap_or(label)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(seq: u64, start: f64, end: f64, source: Source, cluster: Option<u32>) -> SegmentInfo {
        SegmentInfo { seq, start, end, source, cluster }
    }
    fn turn(name: &str, start: f64, end: f64) -> Turn {
        Turn { name: name.into(), text: String::new(), start, end }
    }
    fn names(segs: &[SegmentInfo], native: &[Turn], captions: &[Turn], renames: &[(&str, &str)]) -> Vec<String> {
        let renames: HashMap<String, String> = renames.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        assign_names(segs, native, captions, &HashMap::new(), &renames, "Manuguerra Vito")
    }

    #[test]
    fn known_voice_names_a_cluster_teams_did_not() {
        let segs = [seg(1, 0.0, 2.0, Source::Others, Some(0)), seg(2, 3.0, 5.0, Source::Others, Some(1))];
        let known = HashMap::from([(0, "Rossi Mario".to_string()), (1, "Verdi Luca".to_string())]);
        let captions = [turn("Bianchi Anna", 3.0, 5.0)]; // Teams evidence wins over the voice memory
        let renames = HashMap::from([("Rossi Mario".to_string(), "Rossi Maria".to_string())]);
        let got = assign_names(&segs, &[], &captions, &known, &renames, "Manuguerra Vito");
        assert_eq!(got, vec!["Rossi Maria", "Bianchi Anna"]);
        assert!(is_generic("Parlante B") && is_generic("Altri") && !is_generic("Rossi Mario"));
    }

    #[test]
    fn me_segments_get_my_name() {
        let segs = [seg(1, 0.0, 2.0, Source::Me, None)];
        assert_eq!(names(&segs, &[], &[turn("Rossi Mario", 0.0, 2.0)], &[]), vec!["Manuguerra Vito"]);
    }

    #[test]
    fn my_teams_name_on_system_audio_is_me() {
        // Gallo call, 2 Nov: Teams captions the user too ("Manuguerra Vito"), settings say "Manuguerra".
        let segs = [seg(1, 0.0, 2.0, Source::Me, None), seg(2, 3.0, 5.0, Source::Others, Some(0))];
        let captions = [turn("Manuguerra Vito", 3.0, 5.0)];
        let got = assign_names(&segs, &[], &captions, &HashMap::new(), &HashMap::new(), "Manuguerra");
        assert_eq!(got, vec!["Manuguerra", "Manuguerra"]);
        assert!(is_me("Manuguerra, Vito (Dixit)", "Manuguerra") && !is_me("Rossi Mario", "Manuguerra"));
    }

    #[test]
    fn native_beats_captions() {
        let segs = [seg(1, 10.0, 12.0, Source::Others, Some(0))];
        let native = [turn("Bianchi Anna", 9.0, 13.0)];
        let captions = [turn("Rossi Mario", 9.0, 13.0)];
        assert_eq!(names(&segs, &native, &captions, &[]), vec!["Bianchi Anna"]);
    }

    #[test]
    fn captions_name_others() {
        let segs = [seg(1, 10.0, 12.0, Source::Others, Some(0)), seg(2, 20.0, 22.0, Source::Others, Some(1))];
        let captions = [turn("Rossi Mario", 9.5, 12.5), turn("Bianchi Anna", 19.0, 21.0)];
        assert_eq!(names(&segs, &[], &captions, &[]), vec!["Rossi Mario", "Bianchi Anna"]);
    }

    #[test]
    fn best_overlap_wins() {
        let segs = [seg(1, 10.0, 14.0, Source::Others, None)];
        let captions = [turn("Rossi Mario", 8.0, 11.0), turn("Bianchi Anna", 11.0, 15.0)];
        assert_eq!(names(&segs, &[], &captions, &[]), vec!["Bianchi Anna"]);
    }

    #[test]
    fn cluster_majority_fills_gaps() {
        let segs = [
            seg(1, 10.0, 12.0, Source::Others, Some(0)),
            seg(2, 30.0, 32.0, Source::Others, Some(0)), // no caption here
            seg(3, 40.0, 42.0, Source::Others, Some(0)),
        ];
        let captions = [turn("Rossi Mario", 10.0, 12.0), turn("Rossi Mario", 40.0, 42.0)];
        assert_eq!(names(&segs, &[], &captions, &[]), vec!["Rossi Mario"; 3]);
    }

    #[test]
    fn unknown_cluster_gets_parlante_letter() {
        let segs = [seg(1, 0.0, 1.0, Source::Others, Some(0)), seg(2, 2.0, 3.0, Source::Others, Some(1))];
        assert_eq!(names(&segs, &[], &[], &[]), vec!["Parlante A", "Parlante B"]);
        assert_eq!(speaker_letter(26), "AA");
    }

    #[test]
    fn no_cluster_is_altri() {
        let segs = [seg(1, 0.0, 1.0, Source::Others, None)];
        assert_eq!(names(&segs, &[], &[], &[]), vec!["Altri"]);
    }

    #[test]
    fn renames_apply_last() {
        let segs = [seg(1, 0.0, 1.0, Source::Others, Some(0)), seg(2, 2.0, 3.0, Source::Me, None)];
        let out = names(&segs, &[], &[], &[("Parlante A", "Verdi Luca"), ("Manuguerra Vito", "Vito")]);
        assert_eq!(out, vec!["Verdi Luca", "Vito"]);
    }

    #[test]
    fn quiet_mic_is_others() {
        assert_eq!(classify_source(0.001, 0.0), Source::Others);
    }

    #[test]
    fn loud_mic_quiet_system_is_me() {
        assert_eq!(classify_source(0.08, 0.002), Source::Me);
    }

    #[test]
    fn bleed_is_others() {
        assert_eq!(classify_source(0.05, 0.08), Source::Others);
    }
}
