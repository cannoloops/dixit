//! Thin helpers over the macOS Accessibility API (cidre::ax), shared by autopilot and teams.
#![cfg(target_os = "macos")]

use cidre::{arc, ax, cf, ns};

pub type El = arc::R<ax::UiElement>;

pub fn is_trusted(prompt: bool) -> bool {
    ax::is_process_trusted_with_prompt(prompt)
}

pub fn app(pid: i32) -> El {
    let el = ax::UiElement::with_app_pid(pid);
    // ponytail: a hung app must not block the autopilot loop.
    let _ = el.set_messaging_timeout_secs(1.0);
    el
}

/// PIDs of running apps with this exact bundle id.
pub fn pids_for_bundle(bundle_id: &str) -> Vec<i32> {
    let apps = ns::RunningApp::with_bundle_id(&ns::String::with_str(bundle_id));
    apps.iter().map(|a| a.pid()).collect()
}

fn attr_value(el: &ax::UiElement, name: &str) -> Option<arc::R<cf::Type>> {
    el.attr_value(ax::Attr::with_string(&cf::String::from_str(name))).ok()
}

pub fn string_attr(el: &ax::UiElement, name: &str) -> Option<String> {
    let v = attr_value(el, name)?;
    let s = v.try_as_string()?.to_string();
    (!s.is_empty()).then_some(s)
}

fn elements_attr(el: &ax::UiElement, name: &str) -> Vec<El> {
    let Some(v) = attr_value(el, name) else { return Vec::new() };
    if v.get_type_id() != cf::Array::type_id() {
        return Vec::new();
    }
    let arr: &cf::ArrayOf<ax::UiElement> = unsafe { std::mem::transmute(&*v) };
    arr.iter().map(|e| e.retained()).collect()
}

pub fn children(el: &ax::UiElement) -> Vec<El> {
    elements_attr(el, "AXChildren")
}

pub fn windows(app: &ax::UiElement) -> Vec<El> {
    elements_attr(app, "AXWindows")
}

pub fn role(el: &ax::UiElement) -> String {
    string_attr(el, "AXRole").unwrap_or_default()
}

/// Every human-readable label of an element (title, description, value, identifiers).
pub fn labels(el: &ax::UiElement) -> Vec<String> {
    ["AXTitle", "AXDescription", "AXValue", "AXIdentifier", "AXDOMIdentifier", "AXHelp"]
        .iter()
        .filter_map(|a| string_attr(el, a))
        .collect()
}

pub fn set_bool_attr(el: &ax::UiElement, name: &str, value: bool) -> bool {
    let val = if value { cf::Boolean::value_true() } else { cf::Boolean::value_false() };
    let mut el = el.retained();
    el.set_attr(ax::Attr::with_string(&cf::String::from_str(name)), val)
        .is_ok()
}

pub fn press(el: &ax::UiElement) -> bool {
    el.perform_action(ax::Action::with_string(&cf::String::from_str("AXPress")))
        .is_ok()
}

/// Depth-first search; `max_nodes` bounds the walk on huge web trees.
pub fn find_all(root: &ax::UiElement, max_nodes: usize, pred: &dyn Fn(&ax::UiElement) -> bool) -> Vec<El> {
    let mut out = Vec::new();
    let mut stack = vec![root.retained()];
    let mut seen = 0;
    while let Some(el) = stack.pop() {
        seen += 1;
        if seen > max_nodes {
            break;
        }
        if pred(&el) {
            out.push(el.retained());
        }
        let mut kids = children(&el);
        kids.reverse();
        stack.extend(kids);
    }
    out
}

/// JSON snapshot of the tree (diagnostics for selector tuning).
pub fn dump(el: &ax::UiElement, depth: usize) -> serde_json::Value {
    let kids = if depth == 0 {
        Vec::new()
    } else {
        children(el).iter().map(|c| dump(c, depth - 1)).collect()
    };
    serde_json::json!({
        "role": role(el),
        "subrole": string_attr(el, "AXSubrole"),
        "title": string_attr(el, "AXTitle"),
        "description": string_attr(el, "AXDescription"),
        "value": string_attr(el, "AXValue"),
        "identifier": string_attr(el, "AXIdentifier").or_else(|| string_attr(el, "AXDOMIdentifier")),
        "children": kids,
    })
}
