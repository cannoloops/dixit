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
    /// The meeting app itself says the call is over (Teams with Accessibility: mic released and
    /// no call controls on screen), not just a quiet moment.
    pub call_ended: bool,
}

// ponytail: seen live on Teams, the mic is released and the call window closes the moment a call
// ends, while during a call the window may vanish for one 5 s sample but the mic stays: two
// absences together are the end. Short enough to split a call-back from the same person.
const CONFIRMED_END: Duration = Duration::from_secs(10);

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
    /// Recording state at the previous observation (to spot manual stops of manual recordings).
    was_recording: bool,
    /// A Stop was issued by the tracker: the next recording→stopped transition is ours.
    expecting_stop: bool,
}

impl CallTracker {
    pub fn new(start_after: Duration, stop_after: Duration) -> Self {
        Self { start_after, stop_after, mic_since: None, last_seen: None, session: None, suppressed: false, was_recording: false, expecting_stop: false }
    }

    pub fn is_auto_session(&self) -> bool {
        self.session.is_some()
    }

    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    /// Settings changed: new thresholds, same session.
    pub fn set_thresholds(&mut self, start_after: Duration, stop_after: Duration) {
        self.start_after = start_after;
        self.stop_after = stop_after;
    }

    /// The meeting app is now in a different call (title changed): end the session or the
    /// suppression that belonged to the previous call, so the new one is recorded on its own.
    pub fn new_call(&mut self) -> Option<Action> {
        self.suppressed = false;
        let stop = self.session.take().map(|_| Action::Stop);
        self.expecting_stop |= stop.is_some();
        stop
    }

    /// The user started recording during this call: it becomes the session, stopped and split
    /// like an auto-started one.
    pub fn adopt(&mut self, p: Platform) {
        self.session = Some(p);
        self.suppressed = false;
        self.expecting_stop = false;
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
        let gone_after = if obs.call_ended { self.stop_after.min(CONFIRMED_END) } else { self.stop_after };
        let call_gone = self
            .last_seen
            .map_or(true, |t| now.saturating_duration_since(t) >= gone_after);
        if self.suppressed && call_gone {
            self.suppressed = false;
        }
        // A recording the user started by hand and then stopped during the call: same rule as a
        // manual stop of an auto-started one, no restart until this call ends.
        let stopped_now = self.was_recording && !recording;
        self.was_recording = recording;
        if stopped_now && self.expecting_stop {
            self.expecting_stop = false;
        } else if stopped_now && self.session.is_none() && !call_gone {
            self.suppressed = true;
        }

        if let Some(active) = self.session {
            let switched = matches!(obs.mic, Some(p) if p != active);
            if switched || call_gone {
                self.session = None;
                self.expecting_stop = true;
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
                self.expecting_stop = false;
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

    const MIC_TEAMS: Observation = Observation { mic: Some(Platform::Teams), window: None, call_ended: false };
    const MIC_ZOOM: Observation = Observation { mic: Some(Platform::Zoom), window: None, call_ended: false };
    const WINDOW_TEAMS: Observation = Observation { mic: None, window: Some(Platform::Teams), call_ended: false };
    const NOTHING: Observation = Observation { mic: None, window: None, call_ended: false };
    const TEAMS_ENDED: Observation = Observation { mic: None, window: None, call_ended: true };

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
    fn teams_confirmed_end_stops_after_ten_seconds() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 11, &MIC_TEAMS, false); // last seen at t=10
        assert!(run(&mut tr, t0, 11, 20, &TEAMS_ENDED, true).is_empty());
        assert_eq!(run(&mut tr, t0, 20, 21, &TEAMS_ENDED, true), vec![(20, Action::Stop)]);
        // Quick call-back: a new session, not the old one stretched.
        assert_eq!(run(&mut tr, t0, 30, 36, &MIC_TEAMS, false), vec![(35, Action::Start(Platform::Teams))]);
    }

    #[test]
    fn short_teams_dropout_keeps_the_session() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 11, &MIC_TEAMS, false);
        assert!(run(&mut tr, t0, 11, 16, &TEAMS_ENDED, true).is_empty()); // 5 s: a flicker, not the end
        assert!(run(&mut tr, t0, 16, 60, &MIC_TEAMS, true).is_empty());
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

    #[test]
    fn stopping_a_manual_recording_mid_call_does_not_restart() {
        let (mut tr, t0) = (tracker(), Instant::now());
        assert!(run(&mut tr, t0, 0, 30, &MIC_TEAMS, true).is_empty()); // manual recording during the call
        assert!(run(&mut tr, t0, 30, 120, &MIC_TEAMS, false).is_empty()); // user stopped it: no auto restart
        assert!(run(&mut tr, t0, 120, 170, &NOTHING, false).is_empty()); // call ends at 165
        assert_eq!(run(&mut tr, t0, 170, 176, &MIC_TEAMS, false), vec![(175, Action::Start(Platform::Teams))]);
    }

    #[test]
    fn threshold_change_keeps_the_session() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 10, &MIC_TEAMS, false);
        tr.set_thresholds(Duration::from_secs(3), Duration::from_secs(20));
        assert!(tr.is_auto_session());
        assert!(run(&mut tr, t0, 10, 29, &NOTHING, true).is_empty());
        assert_eq!(run(&mut tr, t0, 29, 30, &NOTHING, true), vec![(29, Action::Stop)]); // 20 s after last seen (9)
    }

    #[test]
    fn new_call_splits_the_session() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 60, &MIC_TEAMS, false);
        assert_eq!(tr.new_call(), Some(Action::Stop));
        assert_eq!(run(&mut tr, t0, 60, 61, &MIC_TEAMS, false), vec![(60, Action::Start(Platform::Teams))]);
    }

    #[test]
    fn adopted_manual_recording_stops_with_the_call() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 6, &MIC_TEAMS, false);
        tr.manual_stop(); // earlier recording stopped by hand: suppressed
        assert!(run(&mut tr, t0, 6, 30, &MIC_TEAMS, true).is_empty()); // "Avvia registrazione"
        tr.adopt(Platform::Teams);
        assert!(tr.is_auto_session() && !tr.is_suppressed());
        assert!(run(&mut tr, t0, 30, 74, &NOTHING, true).is_empty()); // last seen 29
        assert_eq!(run(&mut tr, t0, 74, 75, &NOTHING, true), vec![(74, Action::Stop)]);
        assert!(run(&mut tr, t0, 75, 80, &NOTHING, false).is_empty()); // our stop, no suppression
        assert!(!tr.is_suppressed());
    }

    #[test]
    fn new_call_lifts_manual_suppression() {
        let (mut tr, t0) = (tracker(), Instant::now());
        run(&mut tr, t0, 0, 6, &MIC_TEAMS, false);
        tr.manual_stop();
        assert!(tr.is_suppressed());
        assert!(run(&mut tr, t0, 6, 30, &MIC_TEAMS, false).is_empty());
        assert_eq!(tr.new_call(), None);
        assert_eq!(run(&mut tr, t0, 30, 31, &MIC_TEAMS, false), vec![(30, Action::Start(Platform::Teams))]);
    }
}
