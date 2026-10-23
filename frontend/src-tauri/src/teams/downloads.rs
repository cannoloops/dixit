//! Picks up native Teams transcripts the user downloads (`~/Downloads`) and links them to the
//! meeting they belong to, then reruns speaker attribution with them.

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
    std::fs::copy(path, &target)?;
    log::info!("Linked Teams transcript {name} to meeting '{}'", m.title);
    crate::speakers::process_meeting(app.clone(), m.folder.clone(), false).await?;
    crate::autopilot::notify(app, &format!("Trascrizione Teams collegata – {}", m.title), "Nomi dei parlanti aggiornati.");
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
    fn ignores_future_meetings() {
        let ms = [meeting("a", "Sprint review", 13, true)];
        assert!(match_meeting("Sprint review-Trascrizione", at(12), &ms).is_none());
    }
}
