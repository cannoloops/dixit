//! Turns a stream of Teams live-caption snapshots into timed speaker turns.

use serde::{Deserialize, Serialize};

/// A timed utterance with a speaker name (live captions or native Teams transcript).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Turn {
    pub name: String,
    pub text: String,
    /// Seconds from recording start.
    pub start: f64,
    pub end: f64,
}

// ponytail: captions trail the voice by about this much; calibrate on real calls.
pub const CAPTION_LAG_SECS: f64 = 1.5;

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Same utterance seen again: grown, trimmed or revised at the end.
fn continues(previous: &str, current: &str) -> bool {
    let (p, c) = (previous.to_lowercase(), current.to_lowercase());
    let head = |s: &str| s.chars().take(30).collect::<String>();
    c.starts_with(&p) || p.starts_with(&c) || (p.chars().count() >= 30 && head(&p) == head(&c))
}

#[derive(Default)]
pub struct CaptionLog {
    turns: Vec<Turn>,
    /// Turns that were on screen in the previous snapshot (candidates for growth).
    open: Vec<usize>,
}

impl CaptionLog {
    /// `t`: seconds from recording start; `visible`: (speaker, text) lines currently on screen.
    pub fn ingest(&mut self, t: f64, visible: &[(String, String)]) {
        let mut still_open = Vec::new();
        for (name, text) in visible {
            let text = normalize(text);
            if text.is_empty() {
                continue;
            }
            let matched = self.open.iter().copied().find(|&i| {
                !still_open.contains(&i) && self.turns[i].name == *name && continues(&self.turns[i].text, &text)
            });
            match matched {
                Some(i) => {
                    self.turns[i].text = text;
                    self.turns[i].end = t;
                    still_open.push(i);
                }
                None => {
                    self.turns.push(Turn { name: name.clone(), text, start: (t - CAPTION_LAG_SECS).max(0.0), end: t });
                    still_open.push(self.turns.len() - 1);
                }
            }
        }
        self.open = still_open;
    }

    pub fn turns(&self) -> Vec<Turn> {
        self.turns.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(name: &str, text: &str) -> (String, String) {
        (name.to_string(), text.to_string())
    }

    #[test]
    fn growing_caption_updates_in_place() {
        let mut log = CaptionLog::default();
        log.ingest(10.0, &[line("Rossi Mario", "ciao")]);
        log.ingest(11.0, &[line("Rossi Mario", "ciao a tutti")]);
        log.ingest(12.0, &[line("Rossi Mario", "ciao a tutti,  oggi")]);
        log.ingest(13.0, &[line("Rossi Mario", "ciao a tutti, oggi")]);
        assert_eq!(
            log.turns(),
            vec![Turn { name: "Rossi Mario".into(), text: "ciao a tutti, oggi".into(), start: 8.5, end: 13.0 }]
        );
    }

    #[test]
    fn two_speakers_two_turns() {
        let mut log = CaptionLog::default();
        log.ingest(10.0, &[line("A", "x")]);
        log.ingest(11.0, &[line("A", "x y"), line("B", "hello")]);
        let turns = log.turns();
        assert_eq!(turns.len(), 2);
        assert_eq!((turns[0].name.as_str(), turns[0].start, turns[0].end), ("A", 8.5, 11.0));
        assert_eq!((turns[1].name.as_str(), turns[1].start, turns[1].end), ("B", 9.5, 11.0));
    }

    #[test]
    fn same_speaker_new_sentence_new_turn() {
        let mut log = CaptionLog::default();
        log.ingest(10.0, &[line("A", "prima frase")]);
        log.ingest(14.0, &[line("A", "prima frase"), line("A", "seconda frase")]);
        let texts: Vec<_> = log.turns().into_iter().map(|t| t.text).collect();
        assert_eq!(texts, vec!["prima frase", "seconda frase"]);
    }

    #[test]
    fn start_is_shifted_by_lag() {
        let mut log = CaptionLog::default();
        log.ingest(0.5, &[line("A", "x")]);
        assert_eq!(log.turns()[0].start, 0.0); // clamped at recording start
        log.ingest(20.0, &[line("B", "y")]);
        assert_eq!(log.turns()[1].start, 20.0 - CAPTION_LAG_SECS);
    }
}
