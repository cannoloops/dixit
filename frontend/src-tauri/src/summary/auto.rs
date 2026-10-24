//! Dixit: automatic summary once the final transcript is ready, and the meeting type commands.

use crate::database::repositories::{meeting::MeetingsRepository, setting::SettingsRepository};
use crate::state::AppState;
use crate::summary::classification::{MeetingKind, AUTO_TEMPLATE_ID};
use crate::summary::commands::start_summary;
use crate::summary::metadata::read_summary_language_from_metadata;
use crate::summary::service::SummaryService;
use log::{info, warn};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Mirrors the "Automatic summary" setting, which lives in the frontend.
static AUTO_SUMMARY: AtomicBool = AtomicBool::new(true);

#[tauri::command]
pub fn api_set_auto_summary(enabled: bool) {
    AUTO_SUMMARY.store(enabled, Ordering::SeqCst);
}

/// Real names only: the diarization's "Parlante X" / "Altri" labels end up listed as participants.
fn is_named(speaker: &str) -> bool {
    let s = speaker.trim();
    let generic = s == "Altri"
        || s.strip_prefix("Parlante ").is_some_and(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_uppercase()));
    !s.is_empty() && !generic
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

/// Runs after the final pass and speaker attribution of a recording.
pub async fn after_final_transcript<R: Runtime>(app: &AppHandle<R>, folder: &Path) {
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
    fn generic_speaker_labels_are_dropped() {
        assert!(super::is_named("Manuguerra"));
        assert!(super::is_named("Parlante Rossi"));
        for label in ["Parlante A", "Parlante AK", "Altri", " "] {
            assert!(!super::is_named(label), "{label}");
        }
    }
}
