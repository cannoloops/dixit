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
