//! Picks up native Teams transcripts the user downloads (`~/Downloads`) and links them to the
//! meeting they belong to, then reruns speaker attribution with them. A new file is linked only
//! after the user confirms it.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::{AppHandle, Manager, Runtime};

pub struct MeetingRef {
    pub id: String,
    pub title: String,
    pub folder: PathBuf,
    pub created_at: DateTime<Utc>,
    pub is_teams: bool,
}

pub fn is_candidate(file_name: &str) -> bool {
    let n = file_name.to_lowercase();
    (n.ends_with(".docx") || n.ends_with(".vtt")) && (n.contains("trascrizione") || n.contains("transcript"))
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The meeting a transcript belongs to: latest meeting (before the download) whose title appears
/// in the file's title; else the latest Teams meeting of the previous 24 h.
pub fn match_meeting<'a>(file_title: &str, file_mtime: DateTime<Utc>, meetings: &'a [MeetingRef]) -> Option<&'a MeetingRef> {
    let file_title = normalize(file_title);
    let before = || meetings.iter().filter(move |m| m.created_at <= file_mtime);
    before()
        .filter(|m| {
            let t = normalize(&m.title);
            !t.is_empty() && file_title.contains(&t)
        })
        .max_by_key(|m| m.created_at)
        .or_else(|| {
            before()
                .filter(|m| m.is_teams && file_mtime - m.created_at <= Duration::hours(24))
                .max_by_key(|m| m.created_at)
        })
}

async fn meetings<R: Runtime>(app: &AppHandle<R>) -> Vec<MeetingRef> {
    let pool = app.state::<crate::state::AppState>().db_manager.pool().clone();
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, title, folder_path, created_at FROM meetings WHERE folder_path IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    rows.into_iter()
        .filter_map(|(id, title, folder, created)| {
            let folder = PathBuf::from(folder);
            let platform = std::fs::read_to_string(folder.join("metadata.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v["platform"].as_str().map(str::to_string));
            Some(MeetingRef {
                id,
                title,
                created_at: DateTime::parse_from_rfc3339(&created).ok()?.with_timezone(&Utc),
                is_teams: platform.as_deref() == Some("teams"),
                folder,
            })
        })
        .collect()
}

fn already_linked(path: &Path, target: &Path) -> bool {
    matches!((std::fs::read(path), std::fs::read(target)), (Ok(a), Ok(b)) if a == b)
}

/// Asks before linking. "Ignora" is the default button and anything but an explicit
/// "Collega" (closing, an error) counts as no.
async fn confirm<R: Runtime>(app: &AppHandle<R>, file: &str, meeting: &str, replaces: bool) -> bool {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};
    let it = crate::i18n::it();
    let (title, ignore, link) = if it {
        ("Collegare la trascrizione Teams?", "Ignora", "Collega")
    } else {
        ("Link the Teams transcript?", "Ignore", "Link")
    };
    let mut msg = if it {
        format!("«{file}» in Download verrà usato per i nomi dei parlanti della riunione «{meeting}». Non è possibile verificare da dove arriva il file: collegalo solo se l'hai scaricato tu da Teams.")
    } else {
        format!("“{file}” in Downloads will set the speaker names of the meeting “{meeting}”. Its origin cannot be verified: link it only if you downloaded it from Teams yourself.")
    };
    if replaces {
        msg.push_str(if it { " Sostituisce la trascrizione Teams già collegata." } else { " It replaces the Teams transcript already linked." });
    }
    let dialog = app
        .dialog()
        .message(msg)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(ignore.into(), link.into()));
    let link = link.to_string();
    tauri::async_runtime::spawn_blocking(move || dialog.blocking_show_with_result())
        .await
        .map(|r| r == MessageDialogResult::Custom(link))
        .unwrap_or(false)
}

async fn link<R: Runtime>(app: &AppHandle<R>, path: &Path, mtime: DateTime<Utc>) -> anyhow::Result<()> {
    use crate::speakers::{attribution, transcript_io};
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let is_docx = name.to_lowercase().ends_with(".docx");
    // Only real Teams transcripts: the docx must have speaker turns, the vtt <v> voices.
    let file_title = if is_docx {
        format!("{} {}", transcript_io::parse_teams_docx(path)?.0, name)
    } else {
        if transcript_io::parse_vtt(&std::fs::read_to_string(path)?).is_empty() {
            anyhow::bail!("no speaker cues");
        }
        name.clone()
    };
    let all = meetings(app).await;
    let Some(m) = match_meeting(&file_title, mtime, &all) else {
        anyhow::bail!("no matching meeting");
    };
    let target = m.folder.join(if is_docx { attribution::NATIVE_DOCX } else { attribution::NATIVE_VTT });
    // Anyone can drop a file named like a transcript into Downloads (a web page, a chat
    // attachment), so a new one is linked only after the user confirms it. A file already
    // linked byte for byte is re-applied silently, as before.
    if !already_linked(path, &target) {
        let replaces = [attribution::NATIVE_DOCX, attribution::NATIVE_VTT].iter().any(|f| m.folder.join(f).exists());
        if !confirm(app, &name, &m.title, replaces).await {
            anyhow::bail!("not confirmed by the user");
        }
    }
    std::fs::copy(path, &target)?;
    log::info!("Linked Teams transcript {name} to meeting '{}'", m.title);
    crate::speakers::process_meeting(app.clone(), m.folder.clone(), false).await?;
    let (head, body) = if crate::i18n::it() {
        (format!("Trascrizione Teams collegata – {}", m.title), "Nomi dei parlanti aggiornati.")
    } else {
        (format!("Teams transcript linked – {}", m.title), "Speaker names updated.")
    };
    crate::autopilot::notify(app, &head, body);
    Ok(())
}

/// Polls `~/Downloads` every 30 s for new Teams transcripts.
// ponytail: polling, not FSEvents; a 30 s delay is irrelevant for a downloaded transcript.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    let Some(dir) = dirs::download_dir() else { return };
    tauri::async_runtime::spawn(async move {
        let mut since = SystemTime::now() - std::time::Duration::from_secs(24 * 3600);
        let mut seen: HashSet<PathBuf> = HashSet::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let scan_start = SystemTime::now();
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) else { continue };
                if !is_candidate(&name) || mtime < since || seen.contains(&path) {
                    continue;
                }
                seen.insert(path.clone());
                if let Err(e) = link(&app, &path, mtime.into()).await {
                    log::info!("Download {name} not linked: {e:#}");
                }
            }
            since = scan_start;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 23, h, 0, 0).unwrap()
    }
    fn meeting(id: &str, title: &str, h: u32, is_teams: bool) -> MeetingRef {
        MeetingRef { id: id.into(), title: title.into(), folder: PathBuf::from(format!("/rec/{id}")), created_at: at(h), is_teams }
    }

    #[test]
    fn candidate_names() {
        assert!(is_candidate("Sprint review-20261023_100215-Trascrizione della riunione.docx"));
        assert!(is_candidate("Meeting Transcript.VTT"));
        assert!(!is_candidate("report.docx"));
        assert!(!is_candidate("trascrizione.pdf"));
    }

    #[test]
    fn matches_by_title() {
        let ms = [meeting("a", "Sprint review", 10, true), meeting("b", "Daily", 11, true)];
        let m = match_meeting("Sprint review-20261023_100215-Trascrizione della riunione", at(12), &ms);
        assert_eq!(m.map(|m| m.id.as_str()), Some("a"));
    }

    #[test]
    fn falls_back_to_latest_teams_meeting() {
        let ms = [meeting("a", "Uno", 9, true), meeting("b", "Due", 11, false), meeting("c", "Tre", 10, true)];
        assert_eq!(match_meeting("Trascrizione", at(12), &ms).map(|m| m.id.as_str()), Some("c"));
        assert!(match_meeting("Trascrizione", at(12) + chrono::Duration::days(2), &ms).is_none());
    }

    #[test]
    fn already_linked_only_for_identical_file() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.vtt"), dir.path().join("b.vtt"));
        std::fs::write(&a, "WEBVTT").unwrap();
        assert!(!already_linked(&a, &b));
        std::fs::write(&b, "WEBVTT x").unwrap();
        assert!(!already_linked(&a, &b));
        std::fs::write(&b, "WEBVTT").unwrap();
        assert!(already_linked(&a, &b));
    }

    #[test]
    fn ignores_future_meetings() {
        let ms = [meeting("a", "Sprint review", 13, true)];
        assert!(match_meeting("Sprint review-Trascrizione", at(12), &ms).is_none());
    }
}
