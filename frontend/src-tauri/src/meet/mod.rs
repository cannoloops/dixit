// Dixit: Google Meet (in the browser) live captions through the macOS Accessibility API.
// Meet captions are per user (no participant sees them): Dixit switches them on at the start of
// the call unless they already are on, and never again once the user turns them off.
// Same `captions.json` as Teams, same naming.
#[cfg(target_os = "macos")]
pub mod ax;
pub mod captions;

use tauri::{AppHandle, Runtime};

/// Called when the autopilot starts (or adopts) a Meet session: collects the captions on screen
/// until the recording ends. Every failure is non-fatal.
pub fn on_session_start<R: Runtime>(app: &AppHandle<R>, session: u64) {
    let _ = app;
    #[cfg(target_os = "macos")]
    std::thread::spawn(move || run_session(session));
    #[cfg(not(target_os = "macos"))]
    let _ = session;
}

#[cfg(target_os = "macos")]
fn run_session(session: u64) {
    use crate::audio::recording_commands as rc;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use tauri::async_runtime::block_on;
    let alive = || crate::autopilot::is_session_alive(session) && block_on(rc::is_recording());

    let state = block_on(rc::get_recording_state());
    let Some(folder) = block_on(rc::get_meeting_folder_path()).ok().flatten().map(PathBuf::from) else { return };
    let elapsed = state["recording_duration"].as_f64().unwrap_or(0.0);
    let rec_start = Instant::now() - Duration::from_secs_f64(elapsed);
    if !crate::ax_util::is_trusted(false) {
        log::warn!("Accessibility not granted: no Meet captions or speaker names");
        return;
    }
    let mut reader = ax::CaptionReader::default();
    let mut switch = CaptionsSwitch::default();
    let mut seen = false;
    crate::teams::collect_captions(&folder, rec_start, &alive, || {
        let now = Instant::now();
        if switch.due(now) {
            let result = ax::enable_captions();
            match &result {
                Ok(ax::Switched::TurnedOn) => log::info!("Meet captions switched on"),
                Ok(ax::Switched::AlreadyOn) => log::info!("Meet captions already on"),
                Err(e) => log::warn!("Meet captions not switched on (try {}): {e:#}", switch.tries + 1),
            }
            switch.record(now, result.is_ok());
        }
        let lines = reader.read();
        if !seen && !lines.is_empty() {
            seen = true;
            log::info!("Meet captions found: naming speakers from them");
        }
        lines
    });
}

/// When to try switching Meet's captions on: until it works once (afterwards captions the user
/// turns off stay off), every `RETRY_EVERY` while the button is missing (lobby, another tab in
/// front), at most `MAX_TRIES` times since each try walks Meet's large tree.
#[derive(Default)]
struct CaptionsSwitch {
    done: bool,
    tries: u32,
    last: Option<std::time::Instant>,
}

const MAX_TRIES: u32 = 10;
const RETRY_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

impl CaptionsSwitch {
    fn due(&self, now: std::time::Instant) -> bool {
        !self.done && self.tries < MAX_TRIES && self.last.map_or(true, |t| now.saturating_duration_since(t) >= RETRY_EVERY)
    }

    fn record(&mut self, now: std::time::Instant, ok: bool) {
        self.tries += 1;
        self.last = Some(now);
        self.done |= ok;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn captions_are_switched_on_once() {
        let t0 = Instant::now();
        let mut s = CaptionsSwitch::default();
        assert!(s.due(t0));
        s.record(t0, true);
        // The user turning them off later must not see them come back.
        assert!(!s.due(t0 + RETRY_EVERY * 5));
    }

    #[test]
    fn a_missing_button_is_retried_a_few_times_spaced_out() {
        let t0 = Instant::now();
        let mut s = CaptionsSwitch::default();
        s.record(t0, false);
        assert!(!s.due(t0 + Duration::from_secs(5)));
        let mut t = t0;
        for _ in 1..MAX_TRIES {
            t += RETRY_EVERY;
            assert!(s.due(t));
            s.record(t, false);
        }
        assert!(!s.due(t + RETRY_EVERY * 10));
    }
}
