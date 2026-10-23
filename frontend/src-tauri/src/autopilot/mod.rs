// Trascrivi autopilot: detects Teams/Zoom/Meet calls and starts/stops recording on its own.
pub mod settings;
pub mod tracker;
pub mod platform;
#[cfg(target_os = "macos")]
pub mod probe;

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
