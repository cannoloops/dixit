// Dixit: Google Meet (in the browser) live captions through the macOS Accessibility API.
// Meet captions are per user (no participant sees them): they are read only while the user
// has them on, never switched on by Dixit. Same `captions.json` as Teams, same naming.
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
    let mut seen = false;
    crate::teams::collect_captions(&folder, rec_start, &alive, || {
        let lines = reader.read();
        if !seen && !lines.is_empty() {
            seen = true;
            log::info!("Meet captions found: naming speakers from them");
        }
        lines
    });
}
