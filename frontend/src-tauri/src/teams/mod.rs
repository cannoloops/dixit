// Trascrivi: Microsoft Teams integration through the macOS Accessibility API.
#[cfg(target_os = "macos")]
pub mod ax;
pub mod captions;
pub mod downloads;

use captions::Turn;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};

pub const CAPTIONS_FILE: &str = "captions.json";
/// Per-meeting Teams facts (kept out of metadata.json, which the recorder rewrites on stop).
pub const SESSION_FILE: &str = "teams-session.json";

/// Meeting folders whose Teams session thread is still collecting captions.
static ACTIVE: Lazy<Mutex<HashSet<PathBuf>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static NATIVE: Lazy<Mutex<NativeState>> = Lazy::new(|| Mutex::new(NativeState::default()));

pub fn read_captions(folder: &Path) -> Vec<Turn> {
    std::fs::read_to_string(folder.join(CAPTIONS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Seconds from recording start at which Trascrivi started the native Teams transcription.
pub fn native_offset(folder: &Path) -> Option<f64> {
    let s = std::fs::read_to_string(folder.join(SESSION_FILE)).ok()?;
    serde_json::from_str::<serde_json::Value>(&s).ok()?["native_transcription_offset"].as_f64()
}

fn write_json(path: &Path, value: &impl serde::Serialize) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            if let Err(e) = std::fs::write(path, s) {
                log::warn!("Could not write {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("Could not serialize {}: {e}", path.display()),
    }
}

/// Waits (up to 15 s) for the folder's caption collector to flush `captions.json`.
pub fn wait_session_finished(folder: &Path) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while ACTIVE.lock().unwrap().contains(folder) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Which auto session started the native Teams transcription (only that one may stop it).
#[derive(Default)]
pub struct NativeState {
    owner: Option<u64>,
}

impl NativeState {
    pub fn started_by(&mut self, session: u64) {
        self.owner = Some(session);
    }

    /// True (once) when `session` started the transcription and should now stop it.
    pub fn take_if_owned(&mut self, session: u64) -> bool {
        let owned = self.owner == Some(session);
        if owned {
            self.owner = None;
        }
        owned
    }

    pub fn session_ended(&mut self, session: u64) {
        if self.owner == Some(session) {
            self.owner = None;
        }
    }
}

/// Called when the autopilot starts a Teams session: captions on, native transcription (if
/// enabled), caption collection until the recording ends. Every failure is non-fatal.
pub fn on_session_start<R: Runtime>(app: &AppHandle<R>, session: u64) {
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        std::thread::spawn(move || run_session(app, session));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, session);
}

#[cfg(target_os = "macos")]
fn run_session<R: Runtime>(app: AppHandle<R>, session: u64) {
    use crate::audio::recording_commands as rc;
    use tauri::async_runtime::block_on;
    let alive = || crate::autopilot::is_session_alive(session) && block_on(rc::is_recording());

    // Anchor caption times on the recording (already started by the autopilot).
    let state = block_on(rc::get_recording_state());
    let Some(folder) = block_on(rc::get_meeting_folder_path()).ok().flatten().map(PathBuf::from) else { return };
    let elapsed = state["recording_duration"].as_f64().unwrap_or(0.0);
    let rec_start = Instant::now() - Duration::from_secs_f64(elapsed);
    let Some(pid) = ax::teams_pid() else { return };
    ACTIVE.lock().unwrap().insert(folder.clone());

    let mut reader = ax::CaptionReader::default();
    let mut log = captions::CaptionLog::default();
    let mut last_write = Instant::now();
    // Captions/native need the meeting menu, which may appear only after the lobby or when the
    // user expands the compact view: keep checking for the whole call, act once.
    let mut menus_done = false;
    let mut menu_check = captions::SearchThrottle::default();
    while alive() {
        std::thread::sleep(Duration::from_secs(1));
        let now = Instant::now();
        if !menus_done && menu_check.due(now) {
            menu_check.mark(now);
            if ax::has_menu(pid) {
                menus_done = true;
                enable_captions_and_native(&app, session, pid, &folder, rec_start, &alive);
            }
        }
        log.ingest(rec_start.elapsed().as_secs_f64(), &reader.read(pid));
        if last_write.elapsed() > Duration::from_secs(10) {
            write_json(&folder.join(CAPTIONS_FILE), &log.turns());
            last_write = Instant::now();
        }
    }
    write_json(&folder.join(CAPTIONS_FILE), &log.turns());
    NATIVE.lock().unwrap().session_ended(session);
    ACTIVE.lock().unwrap().remove(&folder);
}

#[cfg(target_os = "macos")]
fn enable_captions_and_native<R: Runtime>(
    app: &AppHandle<R>,
    session: u64,
    pid: i32,
    folder: &Path,
    rec_start: Instant,
    alive: &dyn Fn() -> bool,
) {
    if let Err(e) = ax::enable_live_captions(pid) {
        log::warn!("Teams live captions not enabled: {e:#}");
    }
    // Re-check right before a visible action: the user may have cancelled meanwhile.
    if !(crate::autopilot::settings::load(app).native_teams_transcription
        && alive()
        && !crate::autopilot::is_cancel_pending())
    {
        return;
    }
    NATIVE.lock().unwrap().started_by(session);
    match ax::start_native_transcription(pid) {
        Ok(ax::NativeStart::Started) => {
            let offset = rec_start.elapsed().as_secs_f64();
            write_json(&folder.join(SESSION_FILE), &serde_json::json!({ "native_transcription_offset": offset }));
            use crate::i18n::tr;
            crate::autopilot::notify(
                app,
                tr("Trascrizione Teams avviata (visibile a tutti)", "Teams transcription started (visible to everyone)"),
                tr(
                    "Teams mostra ai partecipanti che la riunione viene trascritta.",
                    "Teams tells participants that the meeting is being transcribed.",
                ),
            );
        }
        Ok(ax::NativeStart::AlreadyRunning) => {
            log::info!("Teams transcription already running (started by someone else)");
            NATIVE.lock().unwrap().session_ended(session);
        }
        Err(e) => {
            log::warn!("Teams native transcription not started: {e:#}");
            NATIVE.lock().unwrap().session_ended(session);
            use crate::i18n::tr;
            crate::autopilot::notify(
                app,
                tr("Trascrizione Teams non avviata", "Teams transcription not started"),
                tr("Continuo con la trascrizione locale.", "Continuing with local transcription."),
            );
        }
    }
}

/// Cancel / manual stop of `session`: stop the native Teams transcription only if this session
/// started it (never one somebody else started, never one from an earlier call).
pub fn stop_native_in_background(session: u64) {
    if !NATIVE.lock().unwrap().take_if_owned(session) {
        return;
    }
    #[cfg(target_os = "macos")]
    std::thread::spawn(|| {
        if let Some(pid) = ax::teams_pid() {
            if let Err(e) = ax::stop_native_transcription(pid) {
                log::warn!("Teams native transcription not stopped: {e:#}");
            }
        }
    });
}

/// Settings → Diagnostica Teams: writes the Teams accessibility tree to a JSON file.
#[tauri::command]
pub async fn teams_dump_ax_tree<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let pid = ax::teams_pid().ok_or("Teams non è in esecuzione")?;
        let tree = tokio::task::spawn_blocking(move || ax::dump_tree(pid, 60))
            .await
            .map_err(|e| e.to_string())?;
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("diagnostics");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("teams-ax-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S")));
        write_json(&path, &tree);
        Ok(path.to_string_lossy().to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Solo macOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_running_transcription_is_never_stopped() {
        let mut n = NativeState::default();
        assert!(!n.take_if_owned(1)); // someone else started it: not ours to stop
    }

    #[test]
    fn owner_session_stops_once() {
        let mut n = NativeState::default();
        n.started_by(1);
        assert!(n.take_if_owned(1));
        assert!(!n.take_if_owned(1));
    }

    #[test]
    fn stale_session_does_not_stop_a_later_call() {
        let mut n = NativeState::default();
        n.started_by(1);
        n.session_ended(1); // call 1 auto-stopped
        assert!(!n.take_if_owned(2));
        n.started_by(2);
        n.session_ended(1); // late cleanup of an old session must not clear the new owner
        assert!(n.take_if_owned(2));
    }
}
