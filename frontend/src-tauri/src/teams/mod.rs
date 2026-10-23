// Trascrivi: Microsoft Teams integration through the macOS Accessibility API.
#[cfg(target_os = "macos")]
pub mod ax;
pub mod captions;
pub mod downloads;

use captions::Turn;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime};

pub const CAPTIONS_FILE: &str = "captions.json";
/// Per-meeting Teams facts (kept out of metadata.json, which the recorder rewrites on stop).
pub const SESSION_FILE: &str = "teams-session.json";

/// Meeting folders whose Teams session thread is still collecting captions.
static ACTIVE: Lazy<Mutex<HashSet<PathBuf>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static NATIVE_STARTED: AtomicBool = AtomicBool::new(false);

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

/// Called when the autopilot starts a Teams session: captions on, native transcription (if
/// enabled), caption collection until the recording ends. Every failure is non-fatal.
pub fn on_session_start<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        std::thread::spawn(move || run_session(app));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(target_os = "macos")]
fn run_session<R: Runtime>(app: AppHandle<R>) {
    use crate::audio::recording_commands as rc;
    use tauri::async_runtime::block_on;

    // Wait for the recording (started by the frontend) to exist, to anchor caption times.
    let deadline = Instant::now() + Duration::from_secs(60);
    let (folder, rec_start) = loop {
        if Instant::now() > deadline || crate::autopilot::current_session().is_none() {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
        let state = block_on(rc::get_recording_state());
        let folder = block_on(rc::get_meeting_folder_path()).ok().flatten();
        if let (true, Some(folder)) = (state["is_recording"].as_bool() == Some(true), folder) {
            let elapsed = state["recording_duration"].as_f64().unwrap_or(0.0);
            break (PathBuf::from(folder), Instant::now() - Duration::from_secs_f64(elapsed));
        }
    };
    let Some(pid) = ax::teams_pid() else { return };
    ACTIVE.lock().unwrap().insert(folder.clone());

    std::thread::sleep(Duration::from_secs(3)); // let the meeting UI settle
    if let Err(e) = ax::enable_live_captions(pid) {
        log::warn!("Teams live captions not enabled: {e:#}");
    }
    if crate::autopilot::settings::load(&app).native_teams_transcription {
        match ax::start_native_transcription(pid) {
            Ok(()) => {
                NATIVE_STARTED.store(true, Ordering::SeqCst);
                let offset = rec_start.elapsed().as_secs_f64();
                write_json(&folder.join(SESSION_FILE), &serde_json::json!({ "native_transcription_offset": offset }));
                crate::autopilot::notify(
                    &app,
                    "Trascrizione Teams avviata (visibile a tutti)",
                    "Teams mostra ai partecipanti che la riunione viene trascritta.",
                );
            }
            Err(e) => {
                log::warn!("Teams native transcription not started: {e:#}");
                crate::autopilot::notify(&app, "Trascrizione Teams non avviata", "Continuo con la trascrizione locale.");
            }
        }
    }

    let mut reader = ax::CaptionReader::default();
    let mut log = captions::CaptionLog::default();
    let mut last_write = Instant::now();
    while crate::autopilot::current_session().is_some() && block_on(rc::is_recording()) {
        std::thread::sleep(Duration::from_secs(1));
        log.ingest(rec_start.elapsed().as_secs_f64(), &reader.read(pid));
        if last_write.elapsed() > Duration::from_secs(10) {
            write_json(&folder.join(CAPTIONS_FILE), &log.turns());
            last_write = Instant::now();
        }
    }
    write_json(&folder.join(CAPTIONS_FILE), &log.turns());
    ACTIVE.lock().unwrap().remove(&folder);
}

/// Cancel / manual stop: also stop the native Teams transcription if Trascrivi started it.
pub fn stop_native_in_background() {
    #[cfg(target_os = "macos")]
    if NATIVE_STARTED.swap(false, Ordering::SeqCst) {
        std::thread::spawn(|| {
            if let Some(pid) = ax::teams_pid() {
                if let Err(e) = ax::stop_native_transcription(pid) {
                    log::warn!("Teams native transcription not stopped: {e:#}");
                }
            }
        });
    }
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
