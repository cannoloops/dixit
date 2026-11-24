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

/// Title of the call on screen, only from unambiguous sources (Teams call window, Meet tab):
/// used to tell one call from the next. Walks the Teams tree, not cheap.
pub fn call_title(platform: Platform) -> Option<String> {
    match platform {
        Platform::Teams => ax_util::pids_for_bundle(TEAMS_BUNDLE)
            .into_iter()
            .filter_map(crate::teams::ax::call_window)
            .filter_map(|w| ax_util::string_attr(&w, "AXTitle"))
            .find_map(|t| platform::meeting_title(Platform::Teams, &t)),
        Platform::Meet => meet_window_title().and_then(|t| platform::meeting_title(Platform::Meet, &t)),
        Platform::Zoom => None,
    }
}

/// Whole title of the call window (Teams call window, Meet tab), when it names a meeting: what
/// the identity check compares with `platform::same_call`. Walks the Teams tree, not cheap.
pub fn call_window_title(platform: Platform) -> Option<String> {
    match platform {
        Platform::Teams => ax_util::pids_for_bundle(TEAMS_BUNDLE)
            .into_iter()
            .filter_map(crate::teams::ax::call_window)
            .filter_map(|w| ax_util::string_attr(&w, "AXTitle"))
            .find(|t| platform::meeting_title(Platform::Teams, t).is_some()),
        Platform::Meet => meet_window_title().filter(|t| platform::meeting_title(Platform::Meet, t).is_some()),
        Platform::Zoom => None,
    }
}

/// Meeting title for the platform, read when a session starts (walks the Teams tree, not cheap).
pub fn title_for(platform: Platform) -> Option<String> {
    match platform {
        Platform::Teams => call_title(Platform::Teams).or_else(|| {
            window_titles(TEAMS_BUNDLE)
                .iter()
                .find_map(|t| platform::meeting_title(Platform::Teams, t))
        }),
        other => call_title(other),
    }
}

/// `session`: platform of the running auto session; only then is the (costly) Teams call-window
/// walk worth doing, and a browser keeps counting as Meet when another tab is active.
pub fn observe(session: Option<Platform>) -> Observation {
    let users = mic_users();
    let meet_window = meet_window_title().is_some();
    let mic = platform::mic_platform(&users, meet_window, session);

    // Window presence only matters while nobody holds the mic (muted or reconnecting), and only
    // from unambiguous meeting UIs: a Teams window title can name a channel, so Teams counts
    // only when its call controls are visible.
    let window = if mic.is_some() {
        mic
    } else if window_titles(ZOOM_BUNDLE).iter().any(|t| platform::is_zoom_meeting_window(t)) {
        Some(Platform::Zoom)
    } else if meet_window {
        Some(Platform::Meet)
    } else if session.is_some() && teams_call_window() {
        Some(Platform::Teams)
    } else {
        None
    };
    // Only Accessibility makes "no call controls" a fact rather than a failed look.
    let call_ended =
        session == Some(Platform::Teams) && mic.is_none() && window.is_none() && ax_util::is_trusted(false);
    Observation { mic, window, call_ended }
}

#[cfg(test)]
mod tests {
    /// Live smoke test against this Mac: `cargo test --lib probe_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_smoke() {
        let obs = super::observe(Some(super::Platform::Teams));
        let title = obs.mic.and_then(super::title_for);
        println!("observation={obs:?} title={title:?} trusted={}", crate::ax_util::is_trusted(false));
        // Raw signals behind a call boundary: who holds the mic, Teams call controls, call title.
        println!(
            "PROBE mic_users={:?} teams_call_window={} call_title={:?} call_window_title={:?}",
            super::mic_users(),
            super::teams_call_window(),
            super::call_title(super::Platform::Teams),
            super::call_window_title(super::Platform::Teams)
        );
    }

    /// During a Teams call: is the meeting menu reachable, what the caption reader sees, and the
    /// AX tree in `$DIXIT_AX_DUMP` if set. `cargo test --lib teams_captions_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn teams_captions_smoke() {
        use crate::teams::ax;
        let pid = ax::teams_pid().expect("Teams not running");
        let mut reader = ax::CaptionReader::default();
        println!("has_menu={} captions={:?}", ax::has_menu(pid), reader.read(pid));
        if let Ok(path) = std::env::var("DIXIT_AX_DUMP") {
            std::fs::write(&path, ax::dump_tree(pid, 60).to_string()).unwrap();
            println!("tree written to {path}");
        }
    }

    /// During a Google Meet call in the browser, with someone else talking: what the accessibility
    /// tree exposes and what it costs. `cargo test --lib meet_captions_smoke -- --ignored --nocapture`;
    /// `DIXIT_MEET_ENABLE=1` also switches captions on as a session would (never off);
    /// `DIXIT_AX_DUMP=/tmp/meet.json` writes the Meet window's whole tree (it holds participants'
    /// names: keep it local).
    #[test]
    #[ignore]
    fn meet_captions_smoke() {
        use crate::meet::{ax, captions};
        use std::time::Instant;
        println!("trusted={}", crate::ax_util::is_trusted(false));
        for b in super::platform::browser_bundles() {
            for pid in crate::ax_util::pids_for_bundle(b) {
                let titles: Vec<_> = crate::ax_util::windows(&crate::ax_util::app(pid))
                    .iter()
                    .map(|w| {
                        let nodes = crate::ax_util::find_all(w, 50_000, &|_| true).len();
                        let webs = crate::ax_util::find_all(w, 50_000, &|e| crate::ax_util::role(e) == "AXWebArea").len();
                        (crate::ax_util::string_attr(w, "AXTitle"), nodes, webs)
                    })
                    .collect();
                println!("{b} pid={pid} windows={titles:?}");
            }
        }
        let (pid, window) = ax::meet_window().expect("no browser window with a Meet tab in front");
        println!("meet window pid={pid} title={:?}", crate::ax_util::string_attr(&window, "AXTitle"));
        let count = |w| crate::ax_util::find_all(w, 50_000, &|_| true).len();
        let web = |w| crate::ax_util::find_all(w, 50_000, &|e| crate::ax_util::role(e) == "AXWebArea").len();
        println!("before: nodes={} web_areas={}", count(&window), web(&window));
        ax::enable_web_accessibility(pid);
        std::thread::sleep(std::time::Duration::from_secs(2));
        let t = Instant::now();
        let n = count(&window);
        println!("after AXManualAccessibility: nodes={n} web_areas={} full_walk={:?}", web(&window), t.elapsed());
        if std::env::var("DIXIT_AX_ENHANCED").is_ok() {
            crate::ax_util::set_bool_attr(&crate::ax_util::app(pid), "AXEnhancedUserInterface", true);
            std::thread::sleep(std::time::Duration::from_secs(2));
            let t = Instant::now();
            let n = count(&window);
            println!("after AXEnhancedUserInterface: nodes={n} web_areas={} full_walk={:?}", web(&window), t.elapsed());
        }
        let t = Instant::now();
        let state = match ax::captions_state(&window) {
            ax::CaptionsState::On => "on".to_string(),
            ax::CaptionsState::Off(b) => format!("off, button labels {:?}", crate::ax_util::labels(&b)),
            ax::CaptionsState::Unknown => "unknown: no region, no captions button".to_string(),
        };
        println!("captions state: {state} ({:?})", t.elapsed());
        let buttons: Vec<_> = crate::ax_util::find_all(&window, 50_000, &|e| {
            crate::ax_util::labels(e).iter().any(|l| l.to_lowercase().contains("caption") || l.to_lowercase().contains("sottotitol"))
        })
        .iter()
        .map(|e| (crate::ax_util::role(e), crate::ax_util::labels(e), crate::ax_util::bool_attr(e, "AXValue")))
        .collect();
        println!("elements mentioning captions: {buttons:?}");
        if std::env::var("DIXIT_MEET_ENABLE").is_ok() {
            match ax::enable_captions() {
                Ok(ax::Switched::TurnedOn) => println!("enable_captions: switched on"),
                Ok(ax::Switched::AlreadyOn) => println!("enable_captions: already on"),
                Err(e) => println!("enable_captions: {e:#}"),
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
        // DIXIT_MEET_LANGUAGE=1: the language a session would pick; =en / =it: that one.
        if let Ok(asked) = std::env::var("DIXIT_MEET_LANGUAGE") {
            use crate::meet::captions as c;
            let pref = crate::get_language_preference_internal().unwrap_or_default();
            let code = if asked == "1" { c::wanted_language(&pref, ax::mac_speaks_italian()).to_string() } else { asked };
            let names = c::language_names(&code).expect("language not mapped");
            println!("meeting language now {:?}, wanted {names:?}", ax::meeting_language());
            match ax::set_meeting_language(names) {
                Ok(ax::LanguageSet::Changed) => println!("set_meeting_language: changed -> {:?}", ax::meeting_language()),
                Ok(ax::LanguageSet::AlreadySet) => println!("set_meeting_language: already set"),
                Err(e) => {
                    println!("set_meeting_language: {e:#}");
                    // What the open menu offers: roles and labels mentioning a few languages.
                    if let Some(combo) = ax::language_box(&window) {
                        crate::ax_util::press(&combo);
                        std::thread::sleep(std::time::Duration::from_millis(900));
                        let hits: Vec<_> = crate::ax_util::find_all(&crate::ax_util::app(pid), 50_000, &|e| {
                            crate::ax_util::labels(e).iter().any(|l| {
                                let l = l.to_lowercase();
                                ["italian", "inglese", "english", "francese", "tedesco"].iter().any(|k| l.starts_with(k))
                            })
                        })
                        .iter()
                        .take(30)
                        .map(|e| (crate::ax_util::role(e), crate::ax_util::string_attr(e, "AXSubrole"), crate::ax_util::labels(e)))
                        .collect();
                        println!("menu offers: {hits:?}");
                        crate::teams::ax::escape(pid);
                    }
                }
            }
        }
        let t = Instant::now();
        let region = ax::captions_region(&window);
        println!("region found={} search={:?}", region.is_some(), t.elapsed());
        if let Ok(path) = std::env::var("DIXIT_AX_DUMP") {
            std::fs::write(&path, crate::ax_util::dump(&window, 80).to_string()).unwrap();
            println!("window tree written to {path}");
        }
        let Some(region) = region else {
            println!("no captions region: are captions on? (try DIXIT_MEET_ENABLE=1; else AXEnhancedUserInterface may be needed)");
            return;
        };
        let snap = ax::snapshot(&region);
        println!("region snapshot:\n{}", serde_json::to_string_pretty(&snap).unwrap());
        // Shape of the region with texts reduced to their length: shareable, no names in it.
        fn shape(n: &captions::Node, d: usize, out: &mut String) {
            let text = n.text.as_ref().map(|t| format!(" <{} chars>", t.chars().count())).unwrap_or_default();
            out.push_str(&format!("{}{}{text}\n", "  ".repeat(d), n.role));
            n.children.iter().for_each(|c| shape(c, d + 1, out));
        }
        let mut shown = false;
        for _ in 0..10 {
            let t = Instant::now();
            let snap = ax::snapshot(&region);
            let lines = captions::caption_lines(&snap);
            println!("read={:?} lines={lines:?}", t.elapsed());
            if !shown && !lines.is_empty() {
                shown = true;
                let mut out = String::new();
                shape(&snap, 0, &mut out);
                println!("SHAPE\n{out}END SHAPE");
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        let mut reader = ax::CaptionReader::default();
        println!("CaptionReader: {:?}", reader.read());
    }
}
