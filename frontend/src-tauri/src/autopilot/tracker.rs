//! Pure call-tracking state machine: turns periodic observations into Start/Stop actions.

use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Teams,
    Zoom,
    Meet,
}

impl Platform {
    pub fn label(&self) -> &'static str {
        match self {
            Platform::Teams => "Teams",
            Platform::Zoom => "Zoom",
            Platform::Meet => "Meet",
        }
    }
}

/// One sample of the world: which meeting app captures the mic, which has a meeting window open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
pub struct Observation {
    pub mic: Option<Platform>,
    pub window: Option<Platform>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start(Platform),
    Stop,
}

pub struct CallTracker {
    start_after: Duration,
    stop_after: Duration,
    /// Platform currently holding the mic and since when (continuously).
    mic_since: Option<(Platform, Instant)>,
    /// Last time any call signal (mic or window) was present.
    last_seen: Option<Instant>,
    /// Active auto-started session.
    session: Option<Platform>,
    /// Set by a manual stop; cleared once the call is gone for `stop_after`.
    suppressed: bool,
}

impl CallTracker {
    pub fn new(start_after: Duration, stop_after: Duration) -> Self {
        Self { start_after, stop_after, mic_since: None, last_seen: None, session: None, suppressed: false }
    }

    pub fn is_auto_session(&self) -> bool {
        self.session.is_some()
    }

    /// The user stopped the auto-started recording: don't restart until this call ends.
    pub fn manual_stop(&mut self) {
        self.session = None;
        self.suppressed = true;
    }

    pub fn observe(&mut self, now: Instant, obs: &Observation, recording: bool) -> Option<Action> {
        self.mic_since = match (obs.mic, self.mic_since) {
            (Some(p), Some((q, since))) if p == q => Some((q, since)),
            (Some(p), _) => Some((p, now)),
            (None, _) => None,
        };
        if obs.mic.is_some() || obs.window.is_some() {
            self.last_seen = Some(now);
        }
        let call_gone = self
            .last_seen
            .map_or(true, |t| now.saturating_duration_since(t) >= self.stop_after);
        if self.suppressed && call_gone {
            self.suppressed = false;
        }

        if let Some(active) = self.session {
            let switched = matches!(obs.mic, Some(p) if p != active);
            if switched || call_gone {
                self.session = None;
                return Some(Action::Stop);
            }
            return None;
        }

        if recording || self.suppressed {
            return None;
        }
        match self.mic_since {
            Some((p, since)) if now.saturating_duration_since(since) >= self.start_after => {
                self.session = Some(p);
                Some(Action::Start(p))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    const MIC_TEAMS: Observation = Observation { mic: Some(Platform::Teams), window: None };
    const MIC_ZOOM: Observation = Observation { mic: Some(Platform::Zoom), window: None };
    const WINDOW_TEAMS: Observation = Observation { mic: None, window: Some(Platform::Teams) };
    const NOTHING: Observation = Observation { mic: None, window: None };

    fn tracker() -> CallTracker {
        CallTracker::new(Duration::from_secs(5), Duration::from_secs(45))
    }

    /// Observes once per second for t in [from, to); returns (t, action) pairs.
    fn run(
        tr: &mut CallTracker,
        t0: Instant,
        from: u64,
        to: u64,
        obs: &Observation,
        recording: bool,
    ) -> Vec<(u64, Action)> {
        (from..to)
            .filter_map(|t| {
                tr.observe(t0 + Duration::from_secs(t), obs, recording)
                    .map(|a| (t, a))
            })
            .collect()
    }

    #[test]
    fn starts_after_stable_mic() {
        let (mut tr, t0) = (tracker(), Instant::now());
        assert!(run(&mut tr, t0, 0, 5, &MIC_TEAMS, false).is_empty());
        assert_eq!(run(&mut tr, t0, 5, 6, &MIC_TEAMS, false), vec![(5, Action::Start(Platform::Teams))]);
        assert!(tr.is_auto_session());
    }

    #[test]
    fn ignores_short_blip() {
        let (mut tr, t0) = (tracker(), Instant::now());
        assert!(run(&mut tr, t0, 0, 3, &MIC_TEAMS, false).is_empty());
        assert!(run(&mut tr, t0, 3, 120, &NOTHING, false).is_empty());
    }

    #[test]
    fn stops_after_absence() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 11, &MIC_TEAMS, false); // last seen at t=10
        assert!(run(&mut tr, t0, 11, 55, &NOTHING, true).is_empty());
        assert_eq!(run(&mut tr, t0, 55, 56, &NOTHING, true), vec![(55, Action::Stop)]);
        assert!(!tr.is_auto_session());
    }

    #[test]
    fn survives_gap_shorter_than_stop_after() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 10, &MIC_TEAMS, false);
        assert!(run(&mut tr, t0, 10, 40, &NOTHING, true).is_empty());
        assert!(run(&mut tr, t0, 40, 100, &MIC_TEAMS, true).is_empty());
    }

    #[test]
    fn window_presence_keeps_call_alive() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 10, &MIC_TEAMS, false);
        assert!(run(&mut tr, t0, 10, 610, &WINDOW_TEAMS, true).is_empty());
    }

    #[test]
    fn manual_stop_suppresses_until_call_ends() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 6, &MIC_TEAMS, false); // started at 5
        tr.manual_stop();
        assert!(!tr.is_auto_session());
        assert!(run(&mut tr, t0, 6, 21, &MIC_TEAMS, false).is_empty()); // last seen 20
        assert!(run(&mut tr, t0, 21, 70, &NOTHING, false).is_empty()); // call ended at 65
        assert_eq!(run(&mut tr, t0, 70, 76, &MIC_TEAMS, false), vec![(75, Action::Start(Platform::Teams))]);
    }

    #[test]
    fn does_not_touch_manual_recording() {
        let (mut tr, t0) = (tracker(), Instant::now());
        assert!(run(&mut tr, t0, 0, 60, &MIC_TEAMS, true).is_empty());
        assert!(run(&mut tr, t0, 60, 200, &NOTHING, true).is_empty());
    }

    #[test]
    fn platform_switch_restarts() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 20, &MIC_TEAMS, false);
        assert_eq!(run(&mut tr, t0, 20, 21, &MIC_ZOOM, true), vec![(20, Action::Stop)]);
        assert_eq!(run(&mut tr, t0, 21, 26, &MIC_ZOOM, false), vec![(25, Action::Start(Platform::Zoom))]);
    }
}
