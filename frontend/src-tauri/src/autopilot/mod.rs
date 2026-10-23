// Trascrivi autopilot: detects Teams/Zoom/Meet calls and starts/stops recording on its own.
pub mod banner;
pub mod settings;
pub mod tracker;
pub mod platform;
#[cfg(target_os = "macos")]
pub mod probe;

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;
use tracker::{Action, CallTracker, Platform};

/// The auto-started session behind the current recording, if any.
#[derive(Clone, Debug)]
pub struct Session {
    pub id: u64,
    pub platform: Platform,
    pub title: String,
    pub started: Instant,
}

static SESSION: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
/// Folder of a recording the user cancelled: post-processing must skip it.
static CANCELLED_FOLDER: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
/// "Annulla" pressed while the auto-start was still bringing the recording up.
static PENDING_CANCEL: AtomicBool = AtomicBool::new(false);

/// How often a running (or suppressed) call is checked for having become a different call.
const IDENTITY_CHECK: Duration = Duration::from_secs(30);
const MAX_TITLE_CHARS: usize = 100;

pub fn current_session() -> Option<Session> {
    SESSION.lock().unwrap().clone()
}

pub fn is_session_alive(id: u64) -> bool {
    current_session().is_some_and(|s| s.id == id)
}

pub fn is_cancel_pending() -> bool {
    PENDING_CANCEL.load(Ordering::SeqCst)
}

pub fn is_cancelled(folder: &Path) -> bool {
    CANCELLED_FOLDER.lock().unwrap().as_deref() == Some(folder)
}

pub(crate) fn notify<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("Notification failed: {e}");
    }
}

/// Meeting name for an auto-started recording: the call's title, or "<Platform> <date time>";
/// bounded because it becomes a folder name.
pub fn session_title(raw: Option<String>, platform: Platform) -> String {
    let title = raw
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("{} {}", platform.label(), chrono::Local::now().format("%d-%m-%Y %H.%M")));
    title.chars().take(MAX_TITLE_CHARS).collect()
}

#[cfg(target_os = "macos")]
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(f).await.ok()
}

/// Starts the polling loop (every 2 s) that drives automatic start/stop.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let initial = settings::load(&app);
        let mut tracker = CallTracker::new(
            Duration::from_secs(initial.start_after_secs),
            Duration::from_secs(initial.stop_after_secs),
        );
        // Whether the auto-started recording is up (to tell a manual stop apart).
        let mut seen_recording = false;
        // (platform, call title) of the running or suppressed call, to split back-to-back calls.
        let mut identity: Option<(Platform, String)> = None;
        let mut last_identity_check = Instant::now();
        let mut last_obs = tracker::Observation::default();
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let s = settings::load(&app);
            tracker.set_thresholds(Duration::from_secs(s.start_after_secs), Duration::from_secs(s.stop_after_secs));
            // Turning auto-start off never orphans a running session: it still stops on its own.
            if !s.auto_start && !tracker.is_auto_session() {
                continue;
            }
            let recording = crate::audio::recording_commands::is_recording().await;

            if tracker.is_auto_session() {
                if recording {
                    seen_recording = true;
                } else if seen_recording {
                    log::info!("Autopilot: recording stopped by the user, suppressing until the call ends");
                    if let Some(s) = end_session() {
                        crate::teams::stop_native_in_background(s.id);
                    }
                    tracker.manual_stop();
                    seen_recording = false;
                    continue;
                }
            }

            #[cfg(target_os = "macos")]
            let obs = {
                let session = current_session().map(|s| s.platform);
                // A probe panic must not look like "call ended": keep the last observation.
                blocking(move || probe::observe(session)).await.unwrap_or(last_obs)
            };
            #[cfg(not(target_os = "macos"))]
            let obs = tracker::Observation::default();
            last_obs = obs;

            // Same app, different call (e.g. next meeting joined right after leaving the last).
            #[cfg(target_os = "macos")]
            if (tracker.is_auto_session() || tracker.is_suppressed()) && last_identity_check.elapsed() >= IDENTITY_CHECK {
                last_identity_check = Instant::now();
                if let Some(p) = obs.mic {
                    let title = blocking(move || probe::call_title(p)).await.flatten();
                    match (&identity, title) {
                        (Some((ip, it)), Some(t)) if *ip == p && *it != t => {
                            log::info!("Autopilot: '{it}' became '{t}', treating it as a new call");
                            identity = None;
                            if tracker.new_call() == Some(Action::Stop) {
                                stop_session(&app, seen_recording);
                                seen_recording = false;
                            }
                            continue;
                        }
                        (None, Some(t)) => identity = Some((p, t)),
                        _ => {}
                    }
                }
            }

            match tracker.observe(Instant::now(), &obs, recording) {
                Some(Action::Start(p)) => {
                    seen_recording = false;
                    #[cfg(target_os = "macos")]
                    {
                        identity = blocking(move || probe::call_title(p)).await.flatten().map(|t| (p, t));
                    }
                    match start(&app, p).await {
                        Ok(()) if PENDING_CANCEL.swap(false, Ordering::SeqCst) => {
                            log::info!("Autopilot: honouring an Annulla pressed during start");
                            if let Err(e) = cancel(&app).await {
                                log::error!("Pending cancel failed: {e}");
                            }
                            tracker.manual_stop();
                        }
                        Ok(()) => seen_recording = true,
                        Err(e) => {
                            log::error!("Autopilot: recording did not start: {e}");
                            notify(&app, "Avvio automatico non riuscito", &e);
                            // Don't retry every 2 s for the rest of this call.
                            tracker.manual_stop();
                        }
                    }
                    last_identity_check = Instant::now();
                }
                Some(Action::Stop) => {
                    log::info!("Autopilot: call ended, stopping recording");
                    identity = None;
                    stop_session(&app, seen_recording);
                    seen_recording = false;
                }
                None => {}
            }
        }
    });
}

fn end_session() -> Option<Session> {
    SESSION.lock().unwrap().take()
}

/// Ends the session; stops (and saves) the recording only if it actually started.
fn stop_session<R: Runtime>(app: &AppHandle<R>, recording_started: bool) {
    end_session();
    PENDING_CANCEL.store(false, Ordering::SeqCst);
    banner::close(app);
    if recording_started {
        crate::tray::stop_recording_in_background(app, false);
    }
}

/// Starts the recording from Rust (no page navigation, so unsaved edits in the UI survive).
async fn start<R: Runtime>(app: &AppHandle<R>, platform: Platform) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let raw = blocking(move || probe::title_for(platform)).await.flatten();
    #[cfg(not(target_os = "macos"))]
    let raw: Option<String> = None;
    let title = session_title(raw, platform);
    log::info!("Autopilot: {} call detected, starting '{}'", platform.label(), title);

    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst);
    *SESSION.lock().unwrap() = Some(Session { id, platform, title: title.clone(), started: Instant::now() });
    *CANCELLED_FOLDER.lock().unwrap() = None;
    PENDING_CANCEL.store(false, Ordering::SeqCst);

    // Banner first: the user can say no while the recorder is still starting.
    banner::show(app, &title);
    notify(app, &format!("Trascrizione avviata – {title}"), "Puoi annullarla dal banner o dal menu Trascrivi.");
    if let Err(e) = crate::audio::recording_commands::start_recording_with_meeting_name(app.clone(), Some(title.clone())).await {
        banner::close(app);
        end_session();
        return Err(e);
    }
    if platform == Platform::Teams {
        crate::teams::on_session_start(app, id);
    }
    let _ = app.emit("autopilot-session", serde_json::json!({ "platform": platform, "title": title }));
    Ok(())
}

/// Stops the current recording and throws it away (audio, transcript, folder).
pub(crate) fn cancel_in_background<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = cancel(&app).await {
            log::error!("Cancel failed: {e}");
        }
    });
}

async fn cancel<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    use crate::audio::recording_commands as rc;
    banner::close(app);
    let session = current_session();
    if let Some(s) = &session {
        crate::teams::stop_native_in_background(s.id);
    }
    if !rc::is_recording().await {
        if session.is_some() {
            // The auto-start is still bringing the recorder up: the loop cancels right after.
            log::info!("Cancel requested during auto-start; will cancel once recording is up");
            PENDING_CANCEL.store(true, Ordering::SeqCst);
        }
        return Ok(());
    }
    let folder = rc::get_meeting_folder_path().await?.map(PathBuf::from);
    *CANCELLED_FOLDER.lock().unwrap() = folder.clone();
    log::info!("Cancelling recording, folder {:?}", folder);
    rc::stop_recording(app.clone(), rc::RecordingArgs { save_path: String::new() }).await?;
    end_session();
    if let Some(folder) = folder {
        if let Err(e) = std::fs::remove_dir_all(&folder) {
            log::warn!("Could not delete cancelled meeting folder {}: {e}", folder.display());
        }
    }
    app.emit("recording-cancelled", ()).map_err(|e| e.to_string())?;
    crate::tray::update_tray_menu(app);
    Ok(())
}

#[tauri::command]
pub async fn autopilot_cancel<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    cancel(&app).await
}

/// Debug/diagnostics: what the autopilot sees right now.
#[tauri::command]
pub async fn autopilot_probe() -> serde_json::Value {
    #[cfg(target_os = "macos")]
    {
        let (obs, title) = tokio::task::spawn_blocking(|| {
            let obs = probe::observe(Some(Platform::Teams));
            (obs, obs.mic.or(obs.window).and_then(probe::title_for))
        })
        .await
        .unwrap_or_default();
        serde_json::json!({
            "mic": obs.mic,
            "window": obs.window,
            "title": title,
            "accessibility": crate::ax_util::is_trusted(false),
        })
    }
    #[cfg(not(target_os = "macos"))]
    serde_json::json!({})
}

/// Returns whether Trascrivi is trusted for Accessibility; `prompt` opens the system dialog.
#[tauri::command]
pub async fn autopilot_request_accessibility(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    return crate::ax_util::is_trusted(prompt);
    #[cfg(not(target_os = "macos"))]
    return true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_title_is_bounded_and_has_fallback() {
        let long = "è".repeat(300);
        let t = session_title(Some(long), Platform::Teams);
        assert_eq!(t.chars().count(), 100);
        assert_eq!(session_title(Some("  Sprint review ".into()), Platform::Teams), "Sprint review");
        assert!(session_title(None, Platform::Meet).starts_with("Meet "));
        assert!(session_title(Some("   ".into()), Platform::Zoom).starts_with("Zoom "));
    }
}
