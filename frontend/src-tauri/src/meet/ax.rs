//! Every Google Meet (in the browser) accessibility selector lives here: Meet updates break them.

use super::captions::Node;
use crate::autopilot::platform;
use crate::ax_util::{self, El};
use once_cell::sync::Lazy;
use regex::Regex;

/// aria-label of Meet's captions region (it + en). Anchored: the "Attiva sottotitoli" button and
/// the captions settings are not the region.
static CAPTIONS_REGION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(captions|live captions|sottotitoli|sottotitoli in tempo reale)$").unwrap());

/// Browser process and window whose active tab is a Meet call.
pub fn meet_window() -> Option<(i32, El)> {
    platform::browser_bundles().iter().flat_map(|b| ax_util::pids_for_bundle(b)).find_map(|pid| {
        ax_util::windows(&ax_util::app(pid))
            .into_iter()
            .find(|w| ax_util::string_attr(w, "AXTitle").is_some_and(|t| platform::is_meet_window(&t)))
            .map(|w| (pid, w))
    })
}

/// Chromium builds the web page's accessibility tree only when an assistive client asks for it.
// ponytail: AXManualAccessibility only; AXEnhancedUserInterface (what Teams gets) also changes
// window animations for the whole browser. Add it if the live smoke test finds no web content.
pub fn enable_web_accessibility(pid: i32) {
    ax_util::set_bool_attr(&ax_util::app(pid), "AXManualAccessibility", true);
}

fn is_captions_region(e: &cidre::ax::UiElement) -> bool {
    ["AXGroup", "AXList", "AXScrollArea", "AXSection"].contains(&ax_util::role(e).as_str())
        && ["AXDescription", "AXTitle"]
            .iter()
            .filter_map(|a| ax_util::string_attr(e, a))
            .any(|l| CAPTIONS_REGION.is_match(l.trim()))
}

// ponytail: bounded walk; Meet's tree with every video tile is large, raise if the region is deeper.
const REGION_SEARCH_NODES: usize = 12000;
const SNAPSHOT_NODES: usize = 600;

pub fn captions_region(window: &El) -> Option<El> {
    ax_util::find_all(window, REGION_SEARCH_NODES, &is_captions_region).into_iter().next()
}

// --- Switching captions on --------------------------------------------------------------

/// Meet's captions button names what a press does (it + en, shortcut "(c)" after it): it is a
/// toggle, so only a "turn on" label is ever pressed and captions the user has on stay on.
static TURN_ON: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(turn on (live )?captions|attiva (i )?sottotitoli|mostra (i )?sottotitoli)\b").unwrap());
static TURN_OFF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(turn off (live )?captions|disattiva (i )?sottotitoli|nascondi (i )?sottotitoli)\b").unwrap());

const TOGGLES: &[&str] = &["AXButton", "AXCheckBox", "AXToggle"];

/// From the captions button's labels: Some(true) = captions on, Some(false) = off, None = not it.
pub fn toggle_state(labels: &[String]) -> Option<bool> {
    let any = |re: &Regex| labels.iter().any(|l| re.is_match(l.trim()));
    if any(&TURN_OFF) {
        Some(true)
    } else if any(&TURN_ON) {
        Some(false)
    } else {
        None
    }
}

pub enum CaptionsState {
    On,
    /// Off, with the button that turns them on.
    Off(El),
    /// Neither the region nor the button on screen (lobby, another tab in front, Meet UI change).
    Unknown,
}

/// One bounded walk: the captions region or a "turn off" button = on; a "turn on" button = off.
pub fn captions_state(window: &El) -> CaptionsState {
    let found = ax_util::find_all(window, REGION_SEARCH_NODES, &|e| {
        is_captions_region(e) || (TOGGLES.contains(&ax_util::role(e).as_str()) && toggle_state(&ax_util::labels(e)).is_some())
    });
    let mut off = None;
    for e in found {
        match toggle_state(&ax_util::labels(&e)) {
            Some(false) => off = off.or(Some(e)),
            _ => return CaptionsState::On, // region or "turn off" button
        }
    }
    off.map_or(CaptionsState::Unknown, CaptionsState::Off)
}

pub enum Switched {
    TurnedOn,
    AlreadyOn,
}

/// Switches Meet's captions on unless they already are. Errors when the button is not on screen.
pub fn enable_captions() -> anyhow::Result<Switched> {
    let (pid, window) = meet_window().ok_or_else(|| anyhow::anyhow!("no Meet tab in front"))?;
    enable_web_accessibility(pid);
    match captions_state(&window) {
        CaptionsState::On => Ok(Switched::AlreadyOn),
        CaptionsState::Off(button) => {
            anyhow::ensure!(ax_util::press(&button), "captions button did not take the press");
            Ok(Switched::TurnedOn)
        }
        CaptionsState::Unknown => anyhow::bail!("Meet captions button not found"),
    }
}

/// Roles and static texts of the region, depth-first, at most `SNAPSHOT_NODES` elements.
pub fn snapshot(el: &El) -> Node {
    fn walk(el: &El, budget: &mut usize) -> Node {
        *budget = budget.saturating_sub(1);
        let role = ax_util::role(el);
        let text = (role == "AXStaticText")
            .then(|| ax_util::string_attr(el, "AXValue").or_else(|| ax_util::string_attr(el, "AXTitle")))
            .flatten();
        let mut children = Vec::new();
        for c in ax_util::children(el) {
            if *budget == 0 {
                break;
            }
            children.push(walk(&c, budget));
        }
        Node { role, text, children }
    }
    let mut budget = SNAPSHOT_NODES;
    walk(el, &mut budget)
}

/// Reads the caption lines on screen; caches the region between calls and searches for it
/// only every few seconds while it's missing (captions off, another tab in front).
#[derive(Default)]
pub struct CaptionReader {
    region: Option<El>,
    search: crate::teams::captions::SearchThrottle,
}

impl CaptionReader {
    pub fn read(&mut self) -> Vec<(String, String)> {
        let alive = self.region.as_ref().is_some_and(|r| !ax_util::role(r).is_empty());
        let now = std::time::Instant::now();
        if !alive {
            self.region = None;
            if self.search.due(now) {
                self.search.mark(now);
                self.region = meet_window().and_then(|(pid, w)| {
                    enable_web_accessibility(pid);
                    captions_region(&w)
                });
            }
        }
        let Some(region) = &self.region else { return Vec::new() };
        super::captions::caption_lines(&snapshot(region))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Labels of Meet's captions button, with the keyboard shortcut Meet appends.
    #[test]
    fn captions_button_state_from_its_label() {
        let state = |l: &str| toggle_state(&[l.to_string()]);
        for l in ["Attiva sottotitoli (c)", "Attiva i sottotitoli", "Turn on captions (c)", "Turn on live captions"] {
            assert_eq!(state(l), Some(false), "{l}");
        }
        for l in ["Disattiva sottotitoli (c)", "Disattiva i sottotitoli", "Turn off captions (c)"] {
            assert_eq!(state(l), Some(true), "{l}");
        }
        for l in ["Sottotitoli", "Impostazioni sottotitoli", "Attiva microfono", "Captions settings"] {
            assert_eq!(state(l), None, "{l}");
        }
        // Both labels on one element (title + help): "turn off" wins, never pressed.
        assert_eq!(toggle_state(&["Disattiva sottotitoli (c)".into(), "Attiva sottotitoli".into()]), Some(true));
    }

    #[test]
    fn region_label_is_exact() {
        for l in ["Captions", "Sottotitoli", "sottotitoli in tempo reale"] {
            assert!(CAPTIONS_REGION.is_match(l), "{l}");
        }
        for l in ["Attiva sottotitoli", "Turn on captions (c)", "Impostazioni sottotitoli", "Captions settings"] {
            assert!(!CAPTIONS_REGION.is_match(l), "{l}");
        }
    }
}
