// Trascrivi: post-call speaker attribution (who said what).
pub mod attribution;
pub mod cluster;
pub mod embedder;
pub mod fbank;
pub mod naming;
pub mod tracks;
pub mod transcript_io;
pub mod wav;

/// File-name-safe meeting title: no path/shell-hostile characters, single spaces, max 120 chars.
pub fn sanitize_title_for_file(title: &str) -> String {
    let replaced: String = title
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(120).collect()
}

use crate::database::repositories::transcript::TranscriptsRepository;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, Runtime};

fn models_dir<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("models"))
}

fn pool<R: Runtime>(app: &AppHandle<R>) -> sqlx::SqlitePool {
    app.state::<crate::state::AppState>().db_manager.pool().clone()
}

/// Attributes speakers for a finished meeting and updates transcripts.json, the DB and the .docx.
/// Safe to rerun (renames, imported Teams transcript).
pub async fn process_meeting<R: Runtime>(app: AppHandle<R>, folder: PathBuf, notify: bool) -> anyhow::Result<()> {
    let f = folder.clone();
    tokio::task::spawn_blocking(move || {
        tracks::wait_finished(&f);
        crate::teams::wait_session_finished(&f);
    })
    .await?;

    let needs_embedder =
        !folder.join(attribution::DIARIZATION_FILE).exists() && folder.join(tracks::SYS_TRACK).exists();
    let model = if needs_embedder {
        match embedder::ensure_model(&models_dir(&app)?).await {
            Ok(m) => Some(m),
            Err(e) => {
                log::warn!("Speaker model unavailable, no voice clustering: {e:#}");
                None
            }
        }
    } else {
        None
    };
    let me = crate::autopilot::settings::load(&app).me();
    let f = folder.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let mut emb = model.and_then(|m| embedder::Embedder::new(&m).map_err(|e| log::warn!("{e:#}")).ok());
        attribution::attribute_folder(&f, &me, emb.as_mut())
    })
    .await??;

    let updated = TranscriptsRepository::set_speakers_for_folder(&pool(&app), &folder.to_string_lossy(), &outcome.speakers).await?;
    log::info!("Speakers attributed for '{}' ({} DB rows)", outcome.title, updated);
    if notify {
        crate::autopilot::notify(
            &app,
            &format!("Trascrizione salvata – {}", outcome.title),
            "Parlanti riconosciuti; il file Word è nella cartella della riunione.",
        );
    }
    let _ = app.emit("speakers-updated", serde_json::json!({ "folder": folder }));
    Ok(())
}

/// After the frontend inserts a meeting: copy speakers already attributed (if any) into the DB.
pub async fn apply_saved_speakers(pool: &sqlx::SqlitePool, folder: &Path) {
    let Ok(text) = std::fs::read_to_string(folder.join("transcripts.json")) else { return };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return };
    let speakers: Vec<(f64, String)> = json["segments"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| Some((s["audio_start_time"].as_f64()?, s["speaker"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    if !speakers.is_empty() {
        if let Err(e) = TranscriptsRepository::set_speakers_for_folder(pool, &folder.to_string_lossy(), &speakers).await {
            log::warn!("Could not apply saved speakers: {e}");
        }
    }
}

/// Runs post-call attribution whenever a recording stops (except cancelled ones).
pub fn listen_for_stops<R: Runtime>(app: &AppHandle<R>) {
    use tauri::Listener;
    let handle = app.clone();
    app.listen_any("recording-stopped", move |event| {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) else { return };
        let Some(folder) = payload["folder_path"].as_str().map(PathBuf::from) else { return };
        if crate::autopilot::is_cancelled(&folder) {
            return;
        }
        let app = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = process_meeting(app, folder, true).await {
                log::error!("Speaker attribution failed: {e:#}");
            }
        });
    });
}

/// Renames a speaker label of a meeting (DB right away; files and .docx via a rerun).
#[tauri::command]
pub async fn speakers_rename<R: Runtime>(app: AppHandle<R>, meeting_id: String, from: String, to: String) -> Result<(), String> {
    let to = to.trim().to_string();
    if to.is_empty() || to == from {
        return Ok(());
    }
    let pool = pool(&app);
    TranscriptsRepository::rename_speaker(&pool, &meeting_id, &from, &to).await.map_err(|e| e.to_string())?;
    let Some(folder) = TranscriptsRepository::folder_for_meeting(&pool, &meeting_id).await.map_err(|e| e.to_string())?
    else {
        return Ok(()); // meeting without a folder: DB rename only
    };
    let folder = PathBuf::from(folder);
    let path = folder.join(attribution::SPEAKERS_FILE);
    let mut renames: std::collections::HashMap<String, String> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| serde_json::from_value(v["renames"].clone()).ok())
        .unwrap_or_default();
    // Chain earlier renames that produced `from`, then map `from` itself.
    for v in renames.values_mut().filter(|v| **v == from) {
        *v = to.clone();
    }
    renames.insert(from, to);
    std::fs::write(&path, serde_json::json!({ "renames": renames }).to_string()).map_err(|e| e.to_string())?;
    if folder.join("transcripts.json").exists() {
        process_meeting(app, folder, false).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Reruns attribution for a meeting (e.g. after importing its Teams transcript).
#[tauri::command]
pub async fn speakers_reprocess<R: Runtime>(app: AppHandle<R>, meeting_id: String) -> Result<(), String> {
    let folder = TranscriptsRepository::folder_for_meeting(&pool(&app), &meeting_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Riunione senza cartella")?;
    process_meeting(app, PathBuf::from(folder), false).await.map_err(|e| e.to_string())
}

/// Downloads the speaker model (onboarding), returns its path.
#[tauri::command]
pub async fn speakers_ensure_model<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let dir = models_dir(&app).map_err(|e| e.to_string())?;
    embedder::ensure_model(&dir).await.map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_title_for_file_cases() {
        assert_eq!(sanitize_title_for_file("A/B: c | d"), "A-B- c - d");
        assert_eq!(sanitize_title_for_file("  Riunione   lunga  "), "Riunione lunga");
        assert_eq!(sanitize_title_for_file(&"x".repeat(300)).chars().count(), 120);
    }
}
