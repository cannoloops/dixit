//! macOS glue: samples who holds the microphone (Core Audio) and which meeting windows are open (AX).
#![cfg(target_os = "macos")]

use super::platform::{self, AppKind, Platform};
use super::tracker::Observation;
use crate::ax_util;
use cidre::core_audio as ca;

const TEAMS_BUNDLE: &str = "com.microsoft.teams2";
const ZOOM_BUNDLE: &str = "us.zoom.xos";

/// App kinds of every process currently capturing audio input.
fn mic_users() -> Vec<AppKind> {
    let mut kinds = Vec::new();
    for p in ca::System::processes().unwrap_or_default() {
        if !p.is_running_input().unwrap_or(false) {
            continue;
        }
        let Ok(bundle) = p.bundle_id() else { continue };
        if let Some(k) = platform::platform_for_bundle(&bundle.to_string()) {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
        }
    }
    kinds
}

fn window_titles(bundle: &str) -> Vec<String> {
    ax_util::pids_for_bundle(bundle)
        .into_iter()
        .flat_map(|pid| ax_util::windows(&ax_util::app(pid)))
        .filter_map(|w| ax_util::string_attr(&w, "AXTitle"))
        .collect()
}

fn meet_window_title() -> Option<String> {
    platform::browser_bundles()
        .iter()
        .flat_map(|b| window_titles(b))
        .find(|t| platform::is_meet_window(t))
}

fn teams_call_window() -> bool {
    ax_util::pids_for_bundle(TEAMS_BUNDLE)
        .into_iter()
        .any(crate::teams::ax::has_call_controls)
}

/// Meeting title for the platform, read when a session starts (walks the Teams tree, not cheap).
pub fn title_for(platform: Platform) -> Option<String> {
    match platform {
        Platform::Teams => {
            let from_call = ax_util::pids_for_bundle(TEAMS_BUNDLE)
                .into_iter()
                .filter_map(crate::teams::ax::call_window)
                .filter_map(|w| ax_util::string_attr(&w, "AXTitle"))
                .find_map(|t| platform::meeting_title(Platform::Teams, &t));
            from_call.or_else(|| {
                window_titles(TEAMS_BUNDLE)
                    .iter()
                    .find_map(|t| platform::meeting_title(Platform::Teams, t))
            })
        }
        Platform::Meet => meet_window_title().and_then(|t| platform::meeting_title(Platform::Meet, &t)),
        Platform::Zoom => None,
    }
}

/// `in_session`: only then is the (costly) Teams call-window walk worth doing.
pub fn observe(in_session: bool) -> Observation {
    let users = mic_users();
    let meet_window = meet_window_title().is_some();

    let mic = if users.contains(&AppKind::Teams) {
        Some(Platform::Teams)
    } else if users.contains(&AppKind::Zoom) {
        Some(Platform::Zoom)
    } else if meet_window && users.iter().any(|k| matches!(k, AppKind::Browser(_))) {
        Some(Platform::Meet)
    } else {
        None
    };

    // Window presence only matters while nobody holds the mic (muted or reconnecting), and only
    // from unambiguous meeting UIs: a Teams window title can name a channel, so Teams counts
    // only when its call controls are visible.
    let window = if mic.is_some() {
        mic
    } else if window_titles(ZOOM_BUNDLE).iter().any(|t| platform::is_zoom_meeting_window(t)) {
        Some(Platform::Zoom)
    } else if meet_window {
        Some(Platform::Meet)
    } else if in_session && teams_call_window() {
        Some(Platform::Teams)
    } else {
        None
    };
    Observation { mic, window }
}

#[cfg(test)]
mod tests {
    /// Live smoke test against this Mac: `cargo test --lib probe_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_smoke() {
        let obs = super::observe(true);
        let title = obs.mic.and_then(super::title_for);
        println!("observation={obs:?} title={title:?} trusted={}", crate::ax_util::is_trusted(false));
    }
}
