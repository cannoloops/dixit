//! Voice memory: names of voices heard in past meetings, so a known colleague is named in the next
//! meeting without Teams captions. Learned from every named remote speaker (Teams turns, captions,
//! manual renames); each meeting contributes one print per name and a rerun replaces it.

use super::cluster::{mean_voice, normalized};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// In the app data folder.
pub const VOICES_FILE: &str = "voices.json";
// ponytail: calibration knobs from `bench_voice_memory` (79 cluster prints of 30 real Teams
// meetings, centered): different voices of one meeting score p50 0.14, p90 0.56; at 0.7 only a
// few pairs pass (mostly one voice split in two clusters) while 31/79 clusters find themselves in
// another meeting. A wrong name is worse than "Parlante X", hence the margin over the next name.
pub const KNOWN_VOICE_THRESHOLD: f32 = 0.7;
const MIN_MARGIN: f32 = 0.1;
/// Mean wespeaker print of real Teams voices (|mean| ≈ 0.63): the component every voice shares
/// (model bias, codec), removed before comparing. Regenerate with `bench_voice_memory`.
// ponytail: fixed, from Vito's meetings; recompute from the memory's own voices if it drifts.
static BACKGROUND: Lazy<Vec<f32>> =
    Lazy::new(|| serde_json::from_str(include_str!("voice_mean.json")).expect("voice_mean.json"));

/// Unit print with the shared component removed (only for prints of the background's model).
pub(crate) fn centered(v: &[f32]) -> Vec<f32> {
    if v.len() != BACKGROUND.len() {
        return normalized(v);
    }
    normalized(&v.iter().zip(BACKGROUND.iter()).map(|(a, b)| a - b).collect::<Vec<_>>())
}
/// Most recent meetings kept per name (folder names sort by date): voices drift with mics and rooms.
const MAX_MEETINGS_PER_NAME: usize = 20;

/// name → meeting folder → that meeting's print of the voice.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Voices(BTreeMap<String, BTreeMap<String, Vec<f32>>>);

impl Voices {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The known name of `voice`, when clearly one; never from `meeting`'s own contribution.
    pub fn recognize(&self, voice: &[f32], meeting: &str) -> Option<String> {
        let voice = centered(voice);
        let mut scored: Vec<(f32, &str)> = self
            .0
            .iter()
            .filter_map(|(name, prints)| {
                let print = centered(&mean_voice(prints.iter().filter(|(m, _)| m.as_str() != meeting).map(|(_, p)| p))?);
                Some((print.iter().zip(&voice).map(|(a, b)| a * b).sum(), name.as_str()))
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (best, name) = *scored.first()?;
        let second = scored.get(1).map_or(-1.0, |s| s.0);
        (best >= KNOWN_VOICE_THRESHOLD && best - second >= MIN_MARGIN).then(|| name.to_string())
    }

    /// `meeting`'s named voices replace whatever it taught before (a rename moves its print).
    pub fn learn(&mut self, meeting: &str, named: &[(String, Vec<f32>)]) {
        for prints in self.0.values_mut() {
            prints.remove(meeting);
        }
        for (name, voice) in named {
            let prints = self.0.entry(name.clone()).or_default();
            prints.insert(meeting.to_string(), normalized(voice));
            while prints.len() > MAX_MEETINGS_PER_NAME {
                prints.pop_first();
            }
        }
        self.0.retain(|_, prints| !prints.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit vector mostly along `axis`, a little along `noise_axis`.
    fn voice(axis: usize, noise_axis: usize, noise: f32) -> Vec<f32> {
        let mut v = vec![0.0; 8];
        v[axis] = 1.0;
        v[noise_axis] += noise;
        v
    }

    #[test]
    fn recognizes_a_learned_voice_in_another_meeting() {
        let mut m = Voices::default();
        m.learn("2026-10-01 SAL", &[("Rossi Mario".into(), voice(0, 7, 0.1)), ("Bianchi Anna".into(), voice(1, 7, 0.1))]);
        assert_eq!(m.recognize(&voice(0, 6, 0.2), "2026-10-02 SAL").as_deref(), Some("Rossi Mario"));
        assert_eq!(m.recognize(&voice(1, 5, 0.2), "2026-10-02 SAL").as_deref(), Some("Bianchi Anna"));
        assert_eq!(m.recognize(&voice(2, 7, 0.1), "2026-10-02 SAL"), None); // a stranger
    }

    #[test]
    fn ignores_the_meeting_s_own_contribution() {
        let mut m = Voices::default();
        m.learn("2026-10-01 SAL", &[("Rossi Mario".into(), voice(0, 7, 0.1))]);
        assert_eq!(m.recognize(&voice(0, 7, 0.1), "2026-10-01 SAL"), None);
    }

    #[test]
    fn ambiguous_voice_stays_unnamed() {
        let mut m = Voices::default();
        m.learn("a", &[("Rossi Mario".into(), voice(0, 1, 0.9))]);
        m.learn("b", &[("Verdi Luca".into(), voice(1, 0, 0.9))]);
        assert_eq!(m.recognize(&voice(0, 1, 1.0), "c"), None); // equally close to both
    }

    #[test]
    fn rerun_after_rename_moves_the_print() {
        let mut m = Voices::default();
        m.learn("2026-10-01 SAL", &[("Parlante sbagliato".into(), voice(0, 7, 0.1))]);
        m.learn("2026-10-01 SAL", &[("Rossi Mario".into(), voice(0, 7, 0.1))]);
        assert_eq!(m.0.keys().collect::<Vec<_>>(), ["Rossi Mario"]);
        assert_eq!(m.recognize(&voice(0, 6, 0.2), "2026-10-02 SAL").as_deref(), Some("Rossi Mario"));
    }

    #[test]
    fn shared_component_is_removed_before_comparing() {
        // Like real prints: shared background + an own part, so raw cosine ≈ 0.4 + 0.6·cos(own).
        let print = |own: &[(usize, f32)]| -> Vec<f32> {
            let scale = (1.0 - BACKGROUND.iter().map(|b| b * b).sum::<f32>()).sqrt();
            let mut v = BACKGROUND.clone();
            for &(axis, w) in own {
                v[axis] += scale * w;
            }
            v
        };
        let rossi = print(&[(0, 1.0)]);
        let other = print(&[(0, 0.6), (1, 0.8)]); // own parts at 0.6: raw ≈ 0.76, centered ≈ 0.6
        let raw: f32 = normalized(&rossi).iter().zip(normalized(&other)).map(|(x, y)| x * y).sum();
        assert!(raw >= KNOWN_VOICE_THRESHOLD, "raw {raw}");
        let mut m = Voices::default();
        m.learn("2026-10-01 SAL", &[("Rossi Mario".into(), rossi)]);
        assert_eq!(m.recognize(&other, "2026-10-02 SAL"), None);
        assert_eq!(m.recognize(&print(&[(0, 1.0), (2, 0.1)]), "2026-10-02 SAL").as_deref(), Some("Rossi Mario"));
    }

    #[test]
    fn keeps_only_the_latest_meetings_per_name_and_round_trips() {
        let mut m = Voices::default();
        for day in 1..=25 {
            m.learn(&format!("2026-10-{day:02} SAL"), &[("Rossi Mario".into(), voice(0, 7, 0.1))]);
        }
        let prints = &m.0["Rossi Mario"];
        assert_eq!(prints.len(), MAX_MEETINGS_PER_NAME);
        assert_eq!(prints.keys().next().unwrap(), "2026-10-06 SAL");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(VOICES_FILE);
        m.save(&path).unwrap();
        assert_eq!(Voices::load(&path).0["Rossi Mario"].len(), MAX_MEETINGS_PER_NAME);
        assert!(Voices::load(&dir.path().join("missing.json")).0.is_empty());
    }
}
