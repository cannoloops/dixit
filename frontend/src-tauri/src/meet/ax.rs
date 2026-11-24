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
// ponytail: about the last 40 entries; a few are on screen, the rest is history.
const SNAPSHOT_NODES: usize = 400;

pub fn captions_region(window: &El) -> Option<El> {
    ax_util::find_all(window, REGION_SEARCH_NODES, &is_captions_region).into_iter().next()
}

// --- Switching captions on --------------------------------------------------------------

/// The button that switches captions on or off names what a press does (it + en, shortcut "(c)"
/// after it). Seen live (Nov 2026): "Attiva sottotitoli" inside the "Sottotitoli e traduzione" panel.
static TURN_ON: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(turn on (live )?captions|attiva (i )?sottotitoli|mostra (i )?sottotitoli)\b").unwrap());
static TURN_OFF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(turn off (live )?captions|disattiva (i )?sottotitoli|nascondi (i )?sottotitoli)\b").unwrap());
/// Seen live (Nov 2026): a toggle (AXCheckBox, subrole AXToggleButton) "Sottotitoli e traduzione"
/// in the call controls opens a panel (AXGroup, subrole AXApplicationDialog, same label) holding
/// "Attiva sottotitoli" / "Disattiva i sottotitoli in tempo reale". The toggle's AXValue stays
/// false with the panel open: it tells neither the panel nor the captions state.
static PANEL_OPENER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(sottotitoli e traduzion[ei]|captions (and|&) translations?)(\s*\(.*\))?$").unwrap());

const PRESSABLE: &[&str] = &["AXButton", "AXMenuItem", "AXCheckBox", "AXRadioButton"];

/// From a control's labels: Some(true) = it turns captions off (so they are on), Some(false) =
/// it turns them on (they are off), None = not the captions switch.
pub fn switch_state(labels: &[String]) -> Option<bool> {
    let any = |re: &Regex| labels.iter().any(|l| re.is_match(l.trim()));
    if any(&TURN_OFF) {
        Some(true)
    } else if any(&TURN_ON) {
        Some(false)
    } else {
        None
    }
}

fn control_state(e: &cidre::ax::UiElement) -> Option<bool> {
    PRESSABLE.contains(&ax_util::role(e).as_str()).then(|| switch_state(&ax_util::labels(e))).flatten()
}

fn is_panel_opener(e: &cidre::ax::UiElement) -> bool {
    PRESSABLE.contains(&ax_util::role(e).as_str()) && ax_util::labels(e).iter().any(|l| PANEL_OPENER.is_match(l.trim()))
}

fn is_panel(e: &cidre::ax::UiElement) -> bool {
    ax_util::string_attr(e, "AXSubrole").as_deref() == Some("AXApplicationDialog")
        && ax_util::labels(e).iter().any(|l| PANEL_OPENER.is_match(l.trim()))
}

fn panel_open(window: &El) -> bool {
    !ax_util::find_all(window, REGION_SEARCH_NODES, &is_panel).is_empty()
}

pub enum CaptionsState {
    On,
    /// Off, with the button that turns them on.
    Off(El),
    /// Neither the region nor the switch on screen (panel closed, lobby, another tab in front).
    Unknown,
}

/// One bounded walk: the captions region or a "turn off" button = on; a "turn on" button = off.
pub fn captions_state(window: &El) -> CaptionsState {
    let found = ax_util::find_all(window, REGION_SEARCH_NODES, &|e| is_captions_region(e) || control_state(e).is_some());
    let mut off = None;
    for e in found {
        match control_state(&e) {
            Some(false) => off = off.or(Some(e)),
            _ => return CaptionsState::On, // region, or a "turn off" button
        }
    }
    off.map_or(CaptionsState::Unknown, CaptionsState::Off)
}

pub enum Switched {
    TurnedOn,
    AlreadyOn,
}

const PANEL_DELAY: std::time::Duration = std::time::Duration::from_millis(700);

/// Switches Meet's captions on unless they already are: the switch right away if on screen,
/// else through the "Sottotitoli e traduzione" panel, closed again if it stays open.
/// Errors when no switch is found.
pub fn enable_captions() -> anyhow::Result<Switched> {
    let (pid, window) = meet_window().ok_or_else(|| anyhow::anyhow!("no Meet tab in front"))?;
    enable_web_accessibility(pid);
    let mut opened = None;
    let state = match captions_state(&window) {
        CaptionsState::Unknown => {
            let opener = ax_util::find_all(&window, REGION_SEARCH_NODES, &is_panel_opener)
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Meet captions panel button not found"))?;
            ax_util::press(&opener);
            std::thread::sleep(PANEL_DELAY);
            opened = Some(opener);
            captions_state(&window)
        }
        known => known,
    };
    let result = match state {
        CaptionsState::On => Ok(Switched::AlreadyOn),
        CaptionsState::Off(button) => {
            if ax_util::press(&button) {
                Ok(Switched::TurnedOn)
            } else {
                Err(anyhow::anyhow!("captions button did not take the press"))
            }
        }
        CaptionsState::Unknown => Err(anyhow::anyhow!("no captions switch in Meet's captions panel")),
    };
    if let Some(opener) = opened {
        std::thread::sleep(PANEL_DELAY);
        if panel_open(&window) {
            ax_util::press(&opener); // close the panel we opened
        }
    }
    result
}

// --- Caption language ---------------------------------------------------------------------

/// Seen live (Nov 2026): AXComboBox "Lingua della riunione", AXValue the language ("Inglese"),
/// in the captions settings next to the captions region.
static LANGUAGE_BOX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(lingua della riunione|meeting language|spoken language|lingua parlata)$").unwrap());
/// Seen live (Nov 2026): AXPopUpButton "Apri impostazioni sottotitoli codificati" next to the
/// captions region opens the settings holding the language menu.
static SETTINGS_OPENER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(apri (le )?impostazioni (dei )?sottotitoli( codificati)?|open caption(s)? settings|caption(s)? settings)$").unwrap()
});
const OPTION_ROLES: &[&str] = &["AXMenuItem", "AXRadioButton", "AXRow", "AXCell", "AXStaticText", "AXGroup"];

pub fn language_box(window: &El) -> Option<El> {
    ax_util::find_all(window, REGION_SEARCH_NODES, &|e| {
        ax_util::role(e) == "AXComboBox" && ax_util::labels(e).iter().any(|l| LANGUAGE_BOX.is_match(l.trim()))
    })
    .into_iter()
    .next()
}

fn settings_opener(window: &El) -> Option<El> {
    ax_util::find_all(window, REGION_SEARCH_NODES, &|e| {
        PRESSABLE.contains(&ax_util::role(e).as_str()) || ax_util::role(e) == "AXPopUpButton"
    })
    .into_iter()
    .find(|e| ax_util::labels(e).iter().any(|l| SETTINGS_OPENER.is_match(l.trim())))
}

/// `defaults read -g` of AppleLanguages and AppleLocale, for `captions::mac_speaks_italian`.
pub fn mac_speaks_italian() -> bool {
    let read = |key: &str| {
        std::process::Command::new("defaults")
            .args(["read", "-g", key])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    };
    super::captions::mac_speaks_italian(&read("AppleLanguages"), &read("AppleLocale"))
}

pub fn meeting_language() -> Option<String> {
    let (_, window) = meet_window()?;
    language_box(&window).and_then(|b| ax_util::string_attr(&b, "AXValue"))
}

pub enum LanguageSet {
    Changed,
    AlreadySet,
}

/// Sets Meet's "meeting language" (what captions transcribe) to one of `names`: opens the menu,
/// presses the first matching option (the most specific role first), checks the value, and
/// escapes when it can't. Needs captions on (the menu sits next to them).
pub fn set_meeting_language(names: &[&str]) -> anyhow::Result<LanguageSet> {
    let (pid, window) = meet_window().ok_or_else(|| anyhow::anyhow!("no Meet tab in front"))?;
    let mut opened_settings = false;
    let combo = match language_box(&window) {
        Some(c) => c,
        None => {
            let opener = settings_opener(&window).ok_or_else(|| anyhow::anyhow!("Meet captions settings button not found"))?;
            ax_util::press(&opener);
            std::thread::sleep(PANEL_DELAY);
            opened_settings = true;
            language_box(&window).ok_or_else(|| {
                escape(pid);
                anyhow::anyhow!("Meet meeting-language menu not found in the captions settings")
            })?
        }
    };
    let result = choose_language(pid, &combo, names);
    if opened_settings {
        std::thread::sleep(PANEL_DELAY);
        if language_box(&window).is_some() {
            escape(pid); // close the captions settings we opened
        }
    }
    result
}

fn choose_language(pid: i32, combo: &El, names: &[&str]) -> anyhow::Result<LanguageSet> {
    use super::captions::is_language;
    let current = ax_util::string_attr(combo, "AXValue").unwrap_or_default();
    if is_language(&current, names) {
        return Ok(LanguageSet::AlreadySet);
    }
    ax_util::press(combo);
    std::thread::sleep(PANEL_DELAY);
    let options = ax_util::find_all(&ax_util::app(pid), 20_000, &|e| {
        OPTION_ROLES.contains(&ax_util::role(e).as_str()) && ax_util::labels(e).iter().any(|l| is_language(l, names))
    });
    log::debug!(
        "Meet language options: {:?}",
        options.iter().map(|e| (ax_util::role(e), ax_util::labels(e))).collect::<Vec<_>>()
    );
    let best = OPTION_ROLES.iter().find_map(|r| options.iter().find(|e| ax_util::role(e) == *r));
    let Some(option) = best else {
        escape(pid);
        anyhow::bail!("language {:?} not in Meet's menu (now {current:?})", names[0]);
    };
    ax_util::press(option);
    std::thread::sleep(PANEL_DELAY);
    let now = ax_util::string_attr(combo, "AXValue").unwrap_or_default();
    if is_language(&now, names) {
        Ok(LanguageSet::Changed)
    } else {
        escape(pid);
        anyhow::bail!("Meet meeting language still {now:?} after choosing {:?}", names[0])
    }
}

fn escape(pid: i32) {
    crate::teams::ax::escape(pid);
}

/// Roles and static texts of the region, at most `SNAPSHOT_NODES` elements. Meet keeps the whole
/// call's captions in the region (seen live, Nov 2026): the walk starts from the newest (last)
/// children, and an element the budget cuts in half is dropped whole, so the oldest entry
/// vanishes instead of turning into a wrong (name, text) pair.
pub fn snapshot(el: &El) -> Node {
    fn walk(el: &El, budget: &mut usize) -> Option<Node> {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        let role = ax_util::role(el);
        let text = (role == "AXStaticText")
            .then(|| ax_util::string_attr(el, "AXValue").or_else(|| ax_util::string_attr(el, "AXTitle")))
            .flatten();
        let mut children = Vec::new();
        for c in ax_util::children(el).iter().rev() {
            children.push(walk(c, budget)?);
        }
        children.reverse();
        Some(Node { role, text, children })
    }
    let mut budget = SNAPSHOT_NODES;
    let role = ax_util::role(el);
    let mut children: Vec<Node> = ax_util::children(el).iter().rev().map_while(|c| walk(c, &mut budget)).collect();
    children.reverse();
    Node { role, text: None, children }
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

    /// Seen live (Nov 2026): "Attiva sottotitoli" inside the panel, plus older/English labels.
    #[test]
    fn captions_switch_state_from_its_label() {
        let state = |l: &str| switch_state(&[l.to_string()]);
        for l in ["Attiva sottotitoli", "Attiva sottotitoli (c)", "Attiva i sottotitoli", "Turn on captions (c)", "Turn on live captions"] {
            assert_eq!(state(l), Some(false), "{l}");
        }
        for l in ["Disattiva sottotitoli", "Disattiva sottotitoli (c)", "Turn off captions (c)"] {
            assert_eq!(state(l), Some(true), "{l}");
        }
        // The panel opener and neighbours in the call controls are not the switch.
        for l in ["Sottotitoli e traduzione", "Impostazioni sottotitoli", "Attiva microfono", "Invia una reazione"] {
            assert_eq!(state(l), None, "{l}");
        }
        // Both labels on one element (title + help): "turn off" wins, never pressed.
        assert_eq!(switch_state(&["Disattiva sottotitoli (c)".into(), "Attiva sottotitoli".into()]), Some(true));
    }

    #[test]
    fn panel_opener_label() {
        for l in ["Sottotitoli e traduzione", "Captions and translations", "Captions & translation"] {
            assert!(PANEL_OPENER.is_match(l), "{l}");
        }
        for l in ["Sottotitoli", "Attiva sottotitoli", "Impostazioni sottotitoli"] {
            assert!(!PANEL_OPENER.is_match(l), "{l}");
        }
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
