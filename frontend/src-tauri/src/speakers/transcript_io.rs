//! Teams transcript files: import of downloaded `.docx`/`.vtt`, export of a Teams-style `.md`
//! (the format `minuta-call/scripts/parse_teams.py` reads: header of three lines, then blocks
//! separated by a blank line: "Cognome Nome   M:SS", line break, text).

use crate::teams::captions::Turn;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Local, Timelike};
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::Read;
use std::path::Path;

/// "Name   M:SS" or "Name   H:MM:SS".
static SPEAKER_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+?)\s+((?:\d+:)?\d{1,2}:\d{2})$").unwrap());
static DOCX_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>|<w:br[^>]*/>|<w:cr\s*/>|<w:tab\s*/>").unwrap());
static VTT_TIMING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+):(\d{2}):(\d{2})[.,](\d{3})\s+-->\s+(\d+):(\d{2}):(\d{2})[.,](\d{3})").unwrap());
static VTT_VOICE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^<v\s+([^>]+)>(.*?)(?:</v>)?$").unwrap());

// ponytail: the last turn has no successor; assume it lasted this long.
const LAST_TURN_SECS: f64 = 30.0;

const MONTHS: [&str; 12] = [
    "gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno",
    "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre",
];

/// "M:SS" up to 99 minutes (what parse_teams.py's `\d{1,2}:\d{2}` accepts), then "H:MM:SS".
pub fn format_offset(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let (m, sec) = (s / 60, s % 60);
    if m < 100 { format!("{m}:{sec:02}") } else { format!("{}:{:02}:{sec:02}", m / 60, m % 60) }
}

fn parse_offset(t: &str) -> f64 {
    t.split(':').fold(0.0, |acc, part| acc * 60.0 + part.parse::<f64>().unwrap_or(0.0))
}

/// Joins consecutive lines of the same speaker (keeps the first start).
pub fn merge_lines(lines: &[(String, f64, String)]) -> Vec<(String, f64, String)> {
    let mut out: Vec<(String, f64, String)> = Vec::new();
    for (name, start, text) in lines {
        match out.last_mut() {
            Some(last) if last.0 == *name => {
                last.2 = format!("{} {}", last.2, text.trim()).trim().to_string();
            }
            _ => out.push((name.clone(), *start, text.trim().to_string())),
        }
    }
    out
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Collapses any whitespace (newlines included) into single spaces: a turn is exactly two lines.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Teams-style transcript in Markdown: header (title, date, duration), then one block per turn.
/// Speaker lines stay plain (parse_teams.py's `SPEAKER_RE` takes no markup) and end with two
/// spaces, a Markdown hard line break before the text.
pub fn teams_md(title: &str, started: DateTime<Local>, duration_secs: f64, lines: &[(String, f64, String)]) -> String {
    let d = duration_secs.max(0.0) as u64;
    let duration = if d >= 3600 {
        format!("{}h {}m {}s", d / 3600, (d % 3600) / 60, d % 60)
    } else {
        format!("{}m {}s", d / 60, d % 60)
    };
    let mut md = format!(
        "# {}-{}-Trascrizione della riunione\n\n{} {} {}, {:02}:{:02}\n\n{duration}\n\n",
        one_line(title),
        started.format("%Y%m%d_%H%M%S"),
        started.day(),
        MONTHS[started.month0() as usize],
        started.year(),
        started.hour(),
        started.minute()
    );
    for (name, start, text) in merge_lines(lines) {
        let mut text = one_line(&text);
        // A text line ending with a time ("alle 10:30") would read as a speaker line.
        if SPEAKER_LINE.is_match(&text) {
            text.push('.');
        }
        md.push_str(&format!("{}   {}  \n{text}\n\n", one_line(&name), format_offset(start)));
    }
    md
}

pub fn write_teams_md(
    path: &Path,
    title: &str,
    started: DateTime<Local>,
    duration_secs: f64,
    lines: &[(String, f64, String)],
) -> Result<()> {
    let tmp = path.with_extension("md.part");
    std::fs::write(&tmp, teams_md(title, started, duration_secs, lines))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn docx_paragraphs(path: &Path) -> Result<Vec<String>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?).context("not a .docx (zip) file")?;
    let mut xml = String::new();
    archive.by_name("word/document.xml")?.read_to_string(&mut xml)?;
    Ok(xml
        .split("</w:p>")
        .map(|p| {
            DOCX_TOKEN
                .captures_iter(p)
                .map(|c| match c.get(1) {
                    Some(t) => xml_unescape(t.as_str()),
                    None if c[0].starts_with("<w:tab") => "\t".to_string(),
                    None => "\n".to_string(),
                })
                .collect::<String>()
        })
        .collect())
}

/// (title line, turns) of a Teams transcript `.docx`; errors when no speaker turn is found.
pub fn parse_teams_docx(path: &Path) -> Result<(String, Vec<Turn>)> {
    let paragraphs = docx_paragraphs(path)?;
    let mut non_empty = paragraphs.iter().map(|p| p.trim()).filter(|p| !p.is_empty());
    // Like parse_teams.py: the first three paragraphs are title, date/time and duration.
    let header: Vec<&str> = non_empty.by_ref().take(3).collect();
    let mut turns: Vec<Turn> = Vec::new();
    for p in non_empty {
        let mut lines = p.lines().map(str::trim).filter(|l| !l.is_empty());
        let first = lines.next().unwrap_or_default();
        if let Some(c) = SPEAKER_LINE.captures(first) {
            let text = lines.collect::<Vec<_>>().join(" ");
            let start = parse_offset(&c[2]);
            turns.push(Turn { name: c[1].trim().to_string(), text, start, end: start });
        } else if let Some(last) = turns.last_mut() {
            last.text = format!("{} {}", last.text, p.replace('\n', " ")).trim().to_string();
        }
    }
    if turns.is_empty() {
        bail!("{} is not a Teams transcript", path.display());
    }
    for i in 0..turns.len() {
        turns[i].end = turns.get(i + 1).map_or(turns[i].start + LAST_TURN_SECS, |n| n.start);
    }
    Ok((header.first().map(|h| h.to_string()).unwrap_or_default(), turns))
}

/// Turns of a Teams `.vtt` (`<v Name>text</v>` cues).
pub fn parse_vtt(text: &str) -> Vec<Turn> {
    let secs = |c: &regex::Captures, i: usize| {
        c[i].parse::<f64>().unwrap_or(0.0) * 3600.0
            + c[i + 1].parse::<f64>().unwrap_or(0.0) * 60.0
            + c[i + 2].parse::<f64>().unwrap_or(0.0)
            + c[i + 3].parse::<f64>().unwrap_or(0.0) / 1000.0
    };
    let mut turns = Vec::new();
    let mut timing: Option<(f64, f64)> = None;
    for line in text.lines().map(str::trim) {
        if let Some(c) = VTT_TIMING.captures(line) {
            timing = Some((secs(&c, 1), secs(&c, 5)));
        } else if line.is_empty() {
            timing = None;
        } else if let (Some((start, end)), Some(v)) = (timing, VTT_VOICE.captures(line)) {
            turns.push(Turn { name: v[1].trim().to_string(), text: v[2].trim().to_string(), start, end });
        }
    }
    turns
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::io::Write;

    #[test]
    fn formats_offsets_like_teams() {
        assert_eq!(format_offset(5.4), "0:05");
        assert_eq!(format_offset(65.0), "1:05");
        assert_eq!(format_offset(3725.0), "62:05"); // parse_teams.py accepts up to 2 minute digits
        assert_eq!(format_offset(6000.0), "1:40:00");
        assert_eq!(parse_offset("62:05"), 3725.0);
        assert_eq!(parse_offset("1:40:00"), 6000.0);
    }

    #[test]
    fn merges_consecutive_speaker_lines() {
        let lines = vec![
            ("Rossi Mario".to_string(), 5.0, "ciao a tutti".to_string()),
            ("Rossi Mario".to_string(), 8.0, "come va".to_string()),
            ("Bianchi Anna".to_string(), 65.0, "bene".to_string()),
        ];
        let merged = merge_lines(&lines);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], ("Rossi Mario".to_string(), 5.0, "ciao a tutti come va".to_string()));
    }

    /// Minimal valid Teams-style .docx (what Teams exports): one paragraph per entry, soft breaks inside.
    fn write_docx_paragraphs(path: &Path, paragraphs: &[Vec<String>]) {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let body: String = paragraphs
            .iter()
            .map(|lines| {
                let runs: Vec<String> =
                    lines.iter().map(|l| format!(r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#, esc(l))).collect();
                format!("<w:p>{}</w:p>", runs.join("<w:r><w:br/></w:r>"))
            })
            .collect();
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
        );
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        zip.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(document.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn parses_teams_docx() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.docx");
        let p = |lines: &[&str]| lines.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        write_docx_paragraphs(
            &path,
            &[
                p(&["Sprint review-20261023_100215-Trascrizione della riunione"]),
                p(&["23 ottobre 2026, 10:02"]),
                p(&["2m 5s"]),
                p(&["Rossi Mario   0:05", "ciao a tutti & benvenuti"]),
                p(&["Bianchi Anna   1:05", "bene <grazie>"]),
            ],
        );
        let (title, turns) = parse_teams_docx(&path).unwrap();
        assert_eq!(title, "Sprint review-20261023_100215-Trascrizione della riunione");
        let got: Vec<_> = turns.iter().map(|t| (t.name.as_str(), t.start, t.text.as_str())).collect();
        assert_eq!(got, vec![("Rossi Mario", 5.0, "ciao a tutti & benvenuti"), ("Bianchi Anna", 65.0, "bene <grazie>")]);
        assert_eq!(turns[0].end, 65.0);
    }

    #[test]
    fn rejects_non_teams_docx() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.docx");
        write_docx_paragraphs(&path, &[vec!["Verbale".into()], vec!["Solo testo libero.".into()]]);
        assert!(parse_teams_docx(&path).is_err());
        std::fs::write(dir.path().join("junk.docx"), b"not a zip").unwrap();
        assert!(parse_teams_docx(&dir.path().join("junk.docx")).is_err());
    }

    #[test]
    fn parses_vtt_voices() {
        let vtt = "WEBVTT\n\n0f1e-2/12-0\n00:00:01.000 --> 00:00:04.500\n<v Rossi Mario>Ciao a tutti</v>\n\n00:01:02.000 --> 00:01:05.000\n<v Bianchi Anna>Buongiorno</v>\n";
        let turns = parse_vtt(vtt);
        assert_eq!(turns.len(), 2);
        assert_eq!((turns[0].name.as_str(), turns[0].start, turns[0].end, turns[0].text.as_str()), ("Rossi Mario", 1.0, 4.5, "Ciao a tutti"));
        assert_eq!((turns[1].name.as_str(), turns[1].start), ("Bianchi Anna", 62.0));
    }

    /// The consumer's `SPEAKER_RE` (minuta-call/scripts/parse_teams.py), to check which lines it takes for turns.
    static PY_SPEAKER_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^([A-Za-zÀ-ÿ'\.\-]+(?:\s+[A-Za-zÀ-ÿ'\.\-()]+)*)\s+(\d{1,2}:\d{2}(?::\d{2})?)\s*$").unwrap()
    });

    fn started() -> DateTime<Local> {
        chrono::Local.with_ymd_and_hms(2026, 10, 23, 10, 2, 15).unwrap()
    }

    #[test]
    fn md_layout_matches_parse_teams() {
        let lines = vec![
            ("Rossi Mario".to_string(), 5.0, "ciao a tutti".to_string()),
            ("Rossi Mario".to_string(), 8.0, "come va".to_string()),
            ("Bianchi Anna".to_string(), 4512.0, "bene".to_string()),
        ];
        let md = teams_md("Sprint review", started(), 4600.0, &lines);
        assert_eq!(
            md,
            "# Sprint review-20261023_100215-Trascrizione della riunione\n\n\
             23 ottobre 2026, 10:02\n\n\
             1h 16m 40s\n\n\
             Rossi Mario   0:05  \nciao a tutti come va\n\n\
             Bianchi Anna   75:12  \nbene\n\n"
        );
        // Only the speaker lines look like turns to the consumer.
        let turn_lines: Vec<_> = md.lines().filter(|l| PY_SPEAKER_RE.is_match(l.trim())).collect();
        assert_eq!(turn_lines, vec!["Rossi Mario   0:05  ", "Bianchi Anna   75:12  "]);
    }

    #[test]
    fn md_text_is_one_line_and_never_a_speaker_line() {
        let lines = vec![
            ("Rossi Mario".to_string(), 5.0, "ci vediamo alle 10:30".to_string()),
            ("Bianchi Anna".to_string(), 65.0, "ok\n\n  fine\r\ncosì".to_string()),
            ("Verdi Luca".to_string(), 3700.0, "".to_string()),
        ];
        let md = teams_md("Daily", started(), 3725.0, &lines);
        assert!(md.contains("Rossi Mario   0:05  \nci vediamo alle 10:30.\n\n"));
        assert!(md.contains("Bianchi Anna   1:05  \nok fine così\n\n"));
        assert_eq!(md.lines().filter(|l| PY_SPEAKER_RE.is_match(l.trim())).count(), 3);
    }

    #[test]
    fn writes_md_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Daily-Trascrizione della riunione.md");
        let lines = vec![("Rossi Mario".to_string(), 5.0, "ciao".to_string())];
        write_teams_md(&path, "Daily", started(), 125.0, &lines).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), teams_md("Daily", started(), 125.0, &lines));
        let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1, "no leftover temp file: {names:?}");
    }

    /// Writes a sample for checking with the real consumer:
    /// `TRASCRIVI_MD_OUT=/tmp/x.md cargo test --lib write_sample_md -- --ignored`, then
    /// `python3 -I <minuta-call>/scripts/parse_teams.py /tmp/x.md`.
    #[test]
    #[ignore]
    fn write_sample_md() {
        let out = std::env::var("TRASCRIVI_MD_OUT").expect("set TRASCRIVI_MD_OUT");
        let lines = vec![
            ("Manuguerra Vito".to_string(), 3.0, "Buongiorno, partiamo dal rilascio.".to_string()),
            ("Rossi Mario".to_string(), 12.0, "Il deploy è pronto per venerdì.".to_string()),
            ("Rossi Mario".to_string(), 15.0, "Manca solo il collaudo, ci vediamo alle 10:30".to_string()),
            ("Bianchi Anna".to_string(), 4512.0, "Confermo.".to_string()),
            ("Parlante B".to_string(), 6100.0, "Anch'io.".to_string()),
        ];
        write_teams_md(std::path::Path::new(&out), "Sprint review", started(), 6200.0, &lines).unwrap();
    }
}
