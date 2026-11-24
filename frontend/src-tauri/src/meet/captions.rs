//! Google Meet live captions → (speaker, text) lines, from a small snapshot of the captions region.
//! Pure: the macOS reader (`meet::ax`) builds the snapshot, these rules read it.

use serde::Serialize;

/// One element of the captions region: its role and, for static text, the text.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Node {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
}

/// Controls inside the region ("Jump to bottom", caption settings): their labels are not captions.
const CONTROLS: &[&str] = &["AXButton", "AXMenuButton", "AXPopUpButton", "AXCheckBox", "AXLink"];

/// How Meet names the user's own captions (it + en). The mic already names the user, and on
/// system audio a "You" line is crosstalk: never a speaker for the others.
const SELF_LABELS: &[&str] = &["tu", "you", "tu (presentazione)", "you (presenting)"];

fn texts<'a>(n: &'a Node, out: &mut Vec<&'a str>) {
    if CONTROLS.contains(&n.role.as_str()) {
        return;
    }
    if n.role == "AXStaticText" {
        if let Some(t) = n.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            out.push(t);
        }
    }
    for c in &n.children {
        texts(c, out);
    }
}

fn text_count(n: &Node) -> usize {
    let mut v = Vec::new();
    texts(n, &mut v);
    v.len()
}

/// A caption entry: its first child carrying text holds exactly one text, the speaker's name
/// (avatar and name in one group, or the name's static text itself when Chrome flattens it).
fn is_entry(n: &Node) -> bool {
    n.children.iter().find(|c| text_count(c) > 0).is_some_and(|c| text_count(c) == 1)
}

/// Entries at any wrapper depth. A lone text (a notice, a stray label) is never an entry.
fn entries<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
    for c in &n.children {
        if CONTROLS.contains(&c.role.as_str()) || text_count(c) < 2 {
            continue;
        }
        if is_entry(c) {
            out.push(c);
        } else {
            entries(c, out);
        }
    }
}

/// The caption lines on screen, oldest first: (speaker, what they said). The user's own lines
/// ("Tu"/"You") are dropped.
pub fn caption_lines(region: &Node) -> Vec<(String, String)> {
    let mut found = Vec::new();
    entries(region, &mut found);
    found
        .into_iter()
        .filter_map(|e| {
            let mut parts = Vec::new();
            texts(e, &mut parts);
            let parts: Vec<String> = parts.into_iter().map(str::to_string).collect();
            crate::teams::captions::caption_line(&parts)
        })
        .filter(|(name, _)| !SELF_LABELS.contains(&name.to_lowercase().as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Node {
        Node { role: "AXStaticText".into(), text: Some(t.into()), children: vec![] }
    }
    fn group(children: Vec<Node>) -> Node {
        Node { role: "AXGroup".into(), text: None, children }
    }
    fn image() -> Node {
        Node { role: "AXImage".into(), ..Default::default() }
    }
    fn button(label: &str) -> Node {
        Node { role: "AXButton".into(), text: None, children: vec![text(label)] }
    }
    /// Meet's DOM shape: avatar and name in one group, the caption text in another.
    fn entry(name: &str, said: &[&str]) -> Node {
        group(vec![group(vec![image(), group(vec![text(name)])]), group(said.iter().map(|s| text(s)).collect())])
    }
    fn line(name: &str, said: &str) -> (String, String) {
        (name.into(), said.into())
    }

    #[test]
    fn one_entry_per_speaker() {
        let region = group(vec![entry("Partecipante Uno", &["buongiorno a tutti"]), entry("Partecipante Due", &["ciao"])]);
        assert_eq!(
            caption_lines(&region),
            vec![line("Partecipante Uno", "buongiorno a tutti"), line("Partecipante Due", "ciao")]
        );
    }

    #[test]
    fn a_single_speaker_is_not_split_into_name_and_text() {
        let region = group(vec![entry("Partecipante Uno", &["prima frase."])]);
        assert_eq!(caption_lines(&region), vec![line("Partecipante Uno", "prima frase.")]);
    }

    #[test]
    fn wrappers_and_flattened_entries() {
        // Scroll container around the entries; one entry flattened by Chrome (no name group).
        let flat = group(vec![image(), text("Partecipante Due"), text("seconda frase")]);
        let region = group(vec![group(vec![group(vec![entry("Partecipante Uno", &["prima"]), flat])])]);
        assert_eq!(caption_lines(&region), vec![line("Partecipante Uno", "prima"), line("Partecipante Due", "seconda frase")]);
    }

    #[test]
    fn sentences_in_separate_texts_are_joined() {
        let region = group(vec![entry("Partecipante Uno", &["Prima frase.", "Seconda frase."])]);
        assert_eq!(caption_lines(&region), vec![line("Partecipante Uno", "Prima frase. Seconda frase.")]);
    }

    #[test]
    fn controls_and_lone_texts_are_not_captions() {
        let region = group(vec![
            text("Sottotitoli attivi"),
            button("Vai in fondo"),
            entry("Partecipante Uno", &["ciao"]),
            group(vec![button("Impostazioni sottotitoli"), text("Italiano")]),
        ]);
        assert_eq!(caption_lines(&region), vec![line("Partecipante Uno", "ciao")]);
    }

    #[test]
    fn the_users_own_lines_are_dropped() {
        let region = group(vec![entry("Tu", &["parlo io"]), entry("You", &["me too"]), entry("Partecipante Uno", &["ok"])]);
        assert_eq!(caption_lines(&region), vec![line("Partecipante Uno", "ok")]);
    }

    #[test]
    fn empty_region_and_name_without_text() {
        assert!(caption_lines(&group(vec![])).is_empty());
        assert!(caption_lines(&group(vec![entry("Partecipante Uno", &[" "])])).is_empty());
    }

    /// The lines feed the same `CaptionLog` as Teams, so `captions.json` and naming are unchanged.
    #[test]
    fn lines_feed_the_caption_log() {
        let mut log = crate::teams::captions::CaptionLog::default();
        log.ingest(10.0, &caption_lines(&group(vec![entry("Partecipante Uno", &["ciao"])])));
        log.ingest(11.0, &caption_lines(&group(vec![entry("Partecipante Uno", &["ciao a tutti"])])));
        let turns = log.turns();
        assert_eq!(turns.len(), 1);
        assert_eq!((turns[0].name.as_str(), turns[0].text.as_str(), turns[0].end), ("Partecipante Uno", "ciao a tutti", 11.0));
    }
}
