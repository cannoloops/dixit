//! Every Teams UI selector lives here: Teams updates break them, keep them in one place.

use crate::ax_util::{self, El};
use once_cell::sync::Lazy;
use regex::Regex;

/// Leave / hang-up button of a meeting or call window (it + en).
static LEAVE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(leave|lascia|esci|abbandona|hang up|riaggancia|termina)\b").unwrap());

/// Teams' web UI only builds its accessibility tree when an assistive client asks for it.
pub fn enable_web_accessibility(app: &El) {
    ax_util::set_bool_attr(app, "AXManualAccessibility", true);
    ax_util::set_bool_attr(app, "AXEnhancedUserInterface", true);
}

fn is_button_matching(el: &cidre::ax::UiElement, re: &Regex) -> bool {
    ax_util::role(el) == "AXButton" && ax_util::labels(el).iter().any(|l| re.is_match(l.trim()))
}

/// The window holding the Leave/hang-up button, if a call is on screen.
pub fn call_window(pid: i32) -> Option<El> {
    let app = ax_util::app(pid);
    enable_web_accessibility(&app);
    ax_util::windows(&app).into_iter().find(|w| {
        // ponytail: bounded walk, Teams' web tree is large; raise if the button is deeper.
        !ax_util::find_all(w, 3000, &|e| is_button_matching(e, &LEAVE)).is_empty()
    })
}

pub fn has_call_controls(pid: i32) -> bool {
    call_window(pid).is_some()
}

// --- Menus: captions and native transcription -------------------------------------------

static MORE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(more|more actions|altro|altre azioni)$|showMoreBtn").unwrap());
static LANGUAGE_SPEECH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(language and speech|lingua e voce)").unwrap());
static CAPTIONS_ON: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(turn on|show|attiva|mostra)\b.*(captions|sottotitoli)|^(live captions|sottotitoli in tempo reale)$").unwrap()
});
static CAPTIONS_ALREADY_ON: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(turn off|hide|disattiva|nascondi)\b.*(captions|sottotitoli)").unwrap());
static RECORD_TRANSCRIBE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(record and transcribe|registra e trascrivi)").unwrap());
static START_TRANSCRIPTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(start transcription|avvia trascrizione)").unwrap());
static STOP_TRANSCRIPTION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(stop transcription|interrompi trascrizione|arresta trascrizione)").unwrap());
static CONFIRM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^(confirm|conferma|start|avvia|stop|interrompi|arresta)$").unwrap());
/// Live-caption container (aria-label / DOM id / class of Teams' captions region).
static CAPTIONS_REGION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)caption|sottotitol").unwrap());

const PRESSABLE: &[&str] = &["AXButton", "AXMenuItem", "AXMenuButton", "AXCheckBox", "AXRadioButton", "AXPopUpButton"];
const STEP_DELAY: std::time::Duration = std::time::Duration::from_millis(600);

pub fn teams_pid() -> Option<i32> {
    ax_util::pids_for_bundle("com.microsoft.teams2").into_iter().next()
}

fn find_pressable(root: &El, re: &Regex) -> Option<El> {
    ax_util::find_all(root, 4000, &|e| {
        PRESSABLE.contains(&ax_util::role(e).as_str()) && ax_util::labels(e).iter().any(|l| re.is_match(l.trim()))
    })
    .into_iter()
    .next()
}

/// Posts Escape to Teams so no half-open menu is left on screen.
fn escape(pid: i32) {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        for down in [true, false] {
            if let Ok(ev) = CGEvent::new_keyboard_event(source.clone(), 53, down) {
                ev.post_to_pid(pid);
            }
        }
    }
}

enum Found {
    Pressed,
    AlreadyDone,
}

/// Serializes menu automation: a cancel's "stop" must not interleave with a running "start".
static AUTOMATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub enum NativeStart {
    Started,
    /// Someone (e.g. the organizer) is already transcribing: not ours to stop later.
    AlreadyRunning,
}

/// Presses `target`, opening `openers` one by one until it shows up. `done` = target's
/// "already active" twin (e.g. "Turn off captions"). Escapes and errors when nothing matches.
fn reveal_and_press(pid: i32, openers: &[&Regex], target: &Regex, done: Option<&Regex>) -> anyhow::Result<Found> {
    let _guard = AUTOMATION.lock().unwrap_or_else(|e| e.into_inner());
    let window = call_window(pid).ok_or_else(|| anyhow::anyhow!("no Teams call window"))?;
    let app = ax_util::app(pid);
    for i in 0..=openers.len() {
        if let Some(el) = find_pressable(&app, target) {
            ax_util::press(&el);
            return Ok(Found::Pressed);
        }
        if let Some(d) = done {
            if find_pressable(&app, d).is_some() {
                escape(pid);
                return Ok(Found::AlreadyDone);
            }
        }
        let Some(opener) = openers.get(i) else { break };
        let root = if i == 0 { &window } else { &app };
        let Some(el) = find_pressable(root, opener) else { break };
        ax_util::press(&el);
        std::thread::sleep(STEP_DELAY);
    }
    escape(pid);
    anyhow::bail!("Teams menu item {:?} not found", target.as_str())
}

/// The full meeting window has a More/Altro menu; the compact meeting view (seen live) does not,
/// so captions and transcription can only be switched on once the user expands the window.
pub fn has_menu(pid: i32) -> bool {
    call_window(pid).is_some_and(|w| find_pressable(&w, &MORE).is_some())
}

pub fn enable_live_captions(pid: i32) -> anyhow::Result<()> {
    reveal_and_press(pid, &[&MORE, &LANGUAGE_SPEECH], &CAPTIONS_ON, Some(&CAPTIONS_ALREADY_ON)).map(|_| ())
}

fn confirm_dialog_if_any(pid: i32) {
    std::thread::sleep(STEP_DELAY * 2);
    let app = ax_util::app(pid);
    if let Some(el) = find_pressable(&app, &CONFIRM) {
        ax_util::press(&el);
    }
}

/// Starts Teams' own (visible to everyone) transcription.
pub fn start_native_transcription(pid: i32) -> anyhow::Result<NativeStart> {
    match reveal_and_press(pid, &[&MORE, &RECORD_TRANSCRIBE], &START_TRANSCRIPTION, Some(&STOP_TRANSCRIPTION))? {
        Found::Pressed => {
            confirm_dialog_if_any(pid);
            Ok(NativeStart::Started)
        }
        Found::AlreadyDone => Ok(NativeStart::AlreadyRunning),
    }
}

pub fn stop_native_transcription(pid: i32) -> anyhow::Result<()> {
    if let Found::Pressed = reveal_and_press(pid, &[&MORE, &RECORD_TRANSCRIBE], &STOP_TRANSCRIPTION, None)? {
        confirm_dialog_if_any(pid);
    }
    Ok(())
}

// --- Live caption reading ----------------------------------------------------------------

/// Reads the visible caption lines; caches the captions region between calls and searches for
/// it only every few seconds while it's missing.
#[derive(Default)]
pub struct CaptionReader {
    region: Option<El>,
    search: super::captions::SearchThrottle,
}

fn texts(el: &El, out: &mut Vec<String>, budget: &mut usize) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    if ax_util::role(el) == "AXStaticText" {
        if let Some(t) = ax_util::string_attr(el, "AXValue").or_else(|| ax_util::string_attr(el, "AXTitle")) {
            let t = t.trim().to_string();
            if !t.is_empty() {
                out.push(t);
            }
        }
    }
    for c in ax_util::children(el) {
        texts(&c, out, budget);
    }
}

/// A container labelled as the captions region (structural labels only: visible text such as a
/// chat message mentioning "sottotitoli" must not match).
fn is_captions_region(e: &cidre::ax::UiElement) -> bool {
    ["AXGroup", "AXList", "AXScrollArea", "AXSection"].contains(&ax_util::role(e).as_str())
        && ["AXDescription", "AXTitle", "AXIdentifier", "AXDOMIdentifier"]
            .iter()
            .filter_map(|a| ax_util::string_attr(e, a))
            .any(|l| CAPTIONS_REGION.is_match(&l))
}

impl CaptionReader {
    pub fn read(&mut self, pid: i32) -> Vec<(String, String)> {
        let alive = self.region.as_ref().is_some_and(|r| !ax_util::role(r).is_empty());
        let now = std::time::Instant::now();
        if !alive {
            self.region = None;
            if self.search.due(now) {
                self.search.mark(now);
                self.region = call_window(pid).and_then(|w| {
                    ax_util::find_all(&w, 6000, &is_captions_region).into_iter().next()
                });
            }
        }
        let Some(region) = &self.region else { return Vec::new() };
        ax_util::children(region)
            .iter()
            .filter_map(|item| {
                let mut parts = Vec::new();
                let mut budget = 200;
                texts(item, &mut parts, &mut budget);
                // First text is the speaker, the rest is what they said.
                (parts.len() >= 2).then(|| (parts[0].clone(), parts[1..].join(" ")))
            })
            .collect()
    }
}

/// Full JSON snapshot of every Teams window, for tuning selectors after a Teams update.
pub fn dump_tree(pid: i32, depth: usize) -> serde_json::Value {
    let app = ax_util::app(pid);
    enable_web_accessibility(&app);
    serde_json::Value::Array(ax_util::windows(&app).iter().map(|w| ax_util::dump(w, depth)).collect())
}

#[cfg(test)]
mod tests {
    /// Live diagnostics while a Teams call is on screen:
    /// `TRASCRIVI_AX_OUT=/tmp/teams.json cargo test --lib dump_teams_tree -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_teams_tree() {
        let out = std::env::var("TRASCRIVI_AX_OUT").expect("set TRASCRIVI_AX_OUT");
        let pid = super::teams_pid().expect("Teams not running");
        std::fs::write(&out, serde_json::to_string_pretty(&super::dump_tree(pid, 60)).unwrap()).unwrap();
        println!("call window: {}", super::call_window(pid).is_some());
        println!("menu available: {}", super::has_menu(pid));
        println!("captions: {:?}", super::CaptionReader::default().read(pid));
    }
}
