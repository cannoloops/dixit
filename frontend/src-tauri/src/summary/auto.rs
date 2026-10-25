//! Dixit: automatic summary once the final transcript is ready, and the meeting type commands.

use crate::database::repositories::{
    meeting::MeetingsRepository, setting::SettingsRepository, summary::SummaryProcessesRepository,
};
use chrono::{DateTime, Utc};
use crate::state::AppState;
use crate::summary::classification::{MeetingKind, AUTO_TEMPLATE_ID};
use crate::summary::commands::start_summary;
use crate::summary::metadata::read_summary_language_from_metadata;
use crate::summary::service::SummaryService;
use log::{info, warn};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Mirrors the "Automatic summary" setting, which lives in the frontend.
static AUTO_SUMMARY: AtomicBool = AtomicBool::new(true);

#[tauri::command]
pub fn api_set_auto_summary(enabled: bool) {
    AUTO_SUMMARY.store(enabled, Ordering::SeqCst);
}

/// Real names only: the diarization's "Parlante X" / "Altri" labels end up listed as participants.
pub(crate) fn is_named(speaker: &str) -> bool {
    let s = speaker.trim();
    let generic = s == "Altri"
        || s.strip_prefix("Parlante ").is_some_and(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_uppercase()));
    !s.is_empty() && !generic
}

/// Recordings whose automatic summary is still on its way (final pass, speakers, start).
static PENDING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// Marks a stopped recording as waiting for its automatic summary until dropped.
pub struct Pending(Option<String>);

impl Pending {
    pub fn start(folder: &Path) -> Self {
        if !AUTO_SUMMARY.load(Ordering::SeqCst) {
            return Self(None);
        }
        let folder = folder.to_string_lossy().to_string();
        PENDING.lock().unwrap_or_else(|e| e.into_inner()).insert(folder.clone());
        Self(Some(folder))
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(folder) = &self.0 {
            PENDING.lock().unwrap_or_else(|e| e.into_inner()).remove(folder);
        }
    }
}

/// True while the meeting waits for large-v3, speaker attribution and its automatic summary.
#[tauri::command]
pub async fn api_auto_summary_pending(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<bool, String> {
    let folder = crate::database::repositories::transcript::TranscriptsRepository::folder_for_meeting(
        state.db_manager.pool(),
        &meeting_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(folder.is_some_and(|f| PENDING.lock().unwrap_or_else(|e| e.into_inner()).contains(&f)))
}

/// Transcript as the summary reads it: `[mm:ss] Speaker: text`, in time order.
pub(crate) async fn transcript_text(pool: &SqlitePool, meeting_id: &str) -> Result<String, sqlx::Error> {
    let rows: Vec<(String, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT transcript, audio_start_time, speaker FROM transcripts WHERE meeting_id = ? ORDER BY audio_start_time, timestamp",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(text, start, speaker)| {
            let secs = start.unwrap_or(0.0) as u64;
            let speaker = speaker.filter(|s| is_named(s)).map(|s| format!("{s}: ")).unwrap_or_default();
            format!("[{:02}:{:02}] {speaker}{}", secs / 60, secs % 60, text.trim())
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// A summary already pending, running or done since the recording stopped.
fn started_after_stop(status: &str, start_time: Option<DateTime<Utc>>, stopped_at: DateTime<Utc>) -> bool {
    let live = matches!(status.to_ascii_lowercase().as_str(), "pending" | "processing" | "completed");
    live && start_time.is_some_and(|t| t >= stopped_at)
}

/// Runs after the final pass and speaker attribution of a recording.
pub async fn after_final_transcript<R: Runtime>(app: &AppHandle<R>, folder: &Path, stopped_at: DateTime<Utc>) {
    if !AUTO_SUMMARY.load(Ordering::SeqCst) {
        return;
    }
    let pool = app.state::<AppState>().db_manager.pool().clone();
    let folder_str = folder.to_string_lossy().to_string();
    // The frontend saves the meeting right after the stop: wait for its row.
    let mut meeting_id = None;
    for _ in 0..120 {
        if let Ok(Some(id)) =
            crate::database::repositories::transcript::TranscriptsRepository::meeting_for_folder(&pool, &folder_str).await
        {
            meeting_id = Some(id);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    let Some(meeting_id) = meeting_id else {
        warn!("Auto summary skipped: meeting for {} not saved", folder.display());
        return;
    };
    // The first summary wins: one the user started meanwhile (Genera) is not replaced.
    // ponytail: check-then-start is not atomic; a click in the same instant can still race it.
    if let Ok(Some(process)) = SummaryProcessesRepository::get_summary_data(&pool, &meeting_id).await {
        if started_after_stop(&process.status, process.start_time, stopped_at) {
            return info!("Auto summary skipped for {meeting_id}: a summary is already {}", process.status);
        }
    }
    let text = match transcript_text(&pool, &meeting_id).await {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) => return,
        Err(e) => return warn!("Auto summary skipped for {meeting_id}: {e}"),
    };
    let Ok(Some(config)) = SettingsRepository::get_model_config(&pool).await else {
        return warn!("Auto summary skipped for {meeting_id}: no summary model configured");
    };
    // Same order as the frontend: language chosen for the meeting, else the detected one.
    let language = match read_summary_language_from_metadata(folder).ok().flatten() {
        Some(language) => Some(language),
        None => SummaryService::read_detected_summary_language(&pool, &meeting_id)
            .await
            .or_else(|| SummaryService::detect_summary_language_from_text(&text)),
    };
    info!("Auto summary for {meeting_id} with {}/{}", config.provider, config.model);
    match start_summary(
        app.clone(), pool, meeting_id.clone(), text, config.provider, config.model, 40000, 1000,
        String::new(), AUTO_TEMPLATE_ID.to_string(), language,
    )
    .await
    {
        Ok(started) => {
            let _ = app.emit(
                "summary-auto-started",
                serde_json::json!({ "meeting_id": meeting_id, "process_id": started.process_id }),
            );
        }
        Err(e) => warn!("Auto summary failed to start for {meeting_id}: {e}"),
    }
}

#[derive(Serialize)]
pub struct MeetingTypeInfo {
    #[serde(rename = "type")]
    pub kind: String,
    /// The classifier's reason; None when the user chose the type.
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn api_get_meeting_type(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Option<MeetingTypeInfo>, String> {
    MeetingsRepository::get_meeting_type(state.db_manager.pool(), &meeting_id)
        .await
        .map(|row| row.map(|(kind, reason)| MeetingTypeInfo { kind, reason }))
        .map_err(|e| e.to_string())
}

/// Manual choice: the next summary with the "auto" template follows it.
#[tauri::command]
pub async fn api_set_meeting_type(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    meeting_type: String,
) -> Result<(), String> {
    let kind = MeetingKind::from_id(&meeting_type).ok_or_else(|| format!("Unknown meeting type: {meeting_type}"))?;
    MeetingsRepository::set_meeting_type(state.db_manager.pool(), &meeting_id, kind.id(), None)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn auto_summary_yields_to_a_summary_started_after_the_stop() {
        let stop = chrono::Utc::now();
        let after = Some(stop + chrono::Duration::seconds(5));
        let before = Some(stop - chrono::Duration::hours(1));
        for status in ["pending", "processing", "completed", "PROCESSING"] {
            assert!(super::started_after_stop(status, after, stop), "{status}");
        }
        // Failed or cancelled attempts, and summaries of an older recording, do not block it.
        for status in ["failed", "error", "cancelled"] {
            assert!(!super::started_after_stop(status, after, stop), "{status}");
        }
        assert!(!super::started_after_stop("completed", before, stop));
        assert!(!super::started_after_stop("pending", None, stop));
    }

    #[test]
    fn pending_lasts_until_the_guard_is_dropped() {
        let folder = std::path::Path::new("/tmp/dixit-test-pending");
        let key = folder.to_string_lossy().to_string();
        let guard = super::Pending::start(folder);
        assert!(super::PENDING.lock().unwrap().contains(&key));
        drop(guard);
        assert!(!super::PENDING.lock().unwrap().contains(&key));
    }

    #[test]
    fn generic_speaker_labels_are_dropped() {
        assert!(super::is_named("Manuguerra"));
        assert!(super::is_named("Parlante Rossi"));
        for label in ["Parlante A", "Parlante AK", "Altri", " "] {
            assert!(!super::is_named(label), "{label}");
        }
    }
}
