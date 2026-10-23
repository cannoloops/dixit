// Trascrivi autopilot: detects Teams/Zoom/Meet calls and starts/stops recording on its own.
pub mod banner;
pub mod settings;
pub mod tracker;
pub mod platform;
#[cfg(target_os = "macos")]
pub mod probe;

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use tracker::{Action, CallTracker, Platform};

/// The auto-started session behind the current recording, if any.
#[derive(Clone, Debug)]
pub struct Session {
    pub platform: Platform,
    pub title: String,
    pub started: Instant,
}

static SESSION: Lazy<Mutex<Option<Session>>> = Lazy::new(|| Mutex::new(None));
/// Folder of a recording the user cancelled: post-processing must skip it.
static CANCELLED_FOLDER: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

pub fn current_session() -> Option<Session> {
    SESSION.lock().unwrap().clone()
}

pub fn is_cancelled(folder: &Path) -> bool {
    CANCELLED_FOLDER.lock().unwrap().as_deref() == Some(folder)
}

pub(crate) fn notify<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("Notification failed: {e}");
    }
}

fn default_title(platform: Platform) -> String {
    format!("{} {}", platform.label(), chrono::Local::now().format("%d-%m-%Y %H.%M"))
}

/// Starts the polling loop (every 2 s) that drives automatic start/stop.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let mut knobs = (0, 0);
        let mut tracker = CallTracker::new(Duration::ZERO, Duration::ZERO);
        // Whether the auto-started recording has actually begun (to tell a manual stop apart).
        let mut seen_recording = false;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let s = settings::load(&app);
            if !s.auto_start {
                continue;
            }
            if knobs != (s.start_after_secs, s.stop_after_secs) {
                knobs = (s.start_after_secs, s.stop_after_secs);
                tracker = CallTracker::new(Duration::from_secs(knobs.0), Duration::from_secs(knobs.1));
            }
            let recording = crate::audio::recording_commands::is_recording().await;

            if tracker.is_auto_session() {
                if recording {
                    seen_recording = true;
                } else if seen_recording {
                    log::info!("Autopilot: recording stopped by the user, suppressing until the call ends");
                    crate::teams::stop_native_in_background();
                    tracker.manual_stop();
                    end_session();
                    seen_recording = false;
                    continue;
                }
            }

            #[cfg(target_os = "macos")]
            let obs = {
                let in_session = tracker.is_auto_session();
                tokio::task::spawn_blocking(move || probe::observe(in_session))
                    .await
                    .unwrap_or_default()
            };
            #[cfg(not(target_os = "macos"))]
            let obs = tracker::Observation::default();

            match tracker.observe(Instant::now(), &obs, recording) {
                Some(Action::Start(p)) => {
                    seen_recording = false;
                    start(&app, p).await;
                }
                Some(Action::Stop) => {
                    log::info!("Autopilot: call ended, stopping recording");
                    end_session();
                    seen_recording = false;
                    crate::tray::stop_recording_in_background(&app, false);
                }
                None => {}
            }
        }
    });
}

fn end_session() {
    SESSION.lock().unwrap().take();
}

async fn start<R: Runtime>(app: &AppHandle<R>, platform: Platform) {
    #[cfg(target_os = "macos")]
    let title = tokio::task::spawn_blocking(move || probe::title_for(platform))
        .await
        .ok()
        .flatten();
    #[cfg(not(target_os = "macos"))]
    let title: Option<String> = None;
    let title = title.unwrap_or_else(|| default_title(platform));
    log::info!("Autopilot: {} call detected, starting '{}'", platform.label(), title);

    *SESSION.lock().unwrap() = Some(Session { platform, title: title.clone(), started: Instant::now() });
    *CANCELLED_FOLDER.lock().unwrap() = None;

    // Same path as the tray's Start: the home page starts the recording from these flags.
    if let Some(window) = app.get_webview_window("main") {
        let js = format!(
            "sessionStorage.setItem('autoStartRecording','true');sessionStorage.setItem('autoStartMeetingTitle',{});window.location.assign('/')",
            serde_json::to_string(&title).unwrap_or_else(|_| "''".into())
        );
        if let Err(e) = window.eval(&js) {
            log::error!("Autopilot: failed to trigger recording start: {e}");
        }
    }
    banner::show(app, &title);
    if platform == Platform::Teams {
        crate::teams::on_session_start(app);
    }
    notify(app, &format!("Trascrizione avviata – {title}"), "Puoi annullarla dal banner o dal menu Trascrivi.");
    let _ = app.emit("autopilot-session", serde_json::json!({ "platform": platform, "title": title }));
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
    let folder = rc::get_meeting_folder_path().await?.map(PathBuf::from);
    *CANCELLED_FOLDER.lock().unwrap() = folder.clone();
    log::info!("Cancelling recording, folder {:?}", folder);
    banner::close(app);
    crate::teams::stop_native_in_background();
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
        let obs = probe::observe(true);
        let title = obs.mic.or(obs.window).and_then(probe::title_for);
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
