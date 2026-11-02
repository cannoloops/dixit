// Trascrivi: post-call speaker attribution (who said what).
pub mod attribution;
pub mod cluster;
pub mod embedder;
pub mod fbank;
pub mod naming;
pub mod tracks;
pub mod transcript_io;
pub mod voices;
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

/// One attribution at a time reads and rewrites the voice memory (a rename can run alongside the
/// attribution of the call that just ended).
static VOICES_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Attributes speakers for a finished meeting and updates transcripts.json, the DB and the .md.
/// Safe to rerun (renames, imported Teams transcript).
pub async fn process_meeting<R: Runtime>(app: AppHandle<R>, folder: PathBuf, notify: bool) -> anyhow::Result<()> {
    let f = folder.clone();
    tokio::task::spawn_blocking(move || {
        tracks::wait_finished(&f);
        crate::teams::wait_session_finished(&f);
    })
    .await?;

    let diarization = folder.join(attribution::DIARIZATION_FILE);
    // Also for a meeting clustered before voice prints were kept: they come from its audio.mp4.
    let needs_embedder = if diarization.exists() {
        folder.join("audio.mp4").exists()
            && std::fs::read_to_string(&diarization)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .is_some_and(|v| v["voices"].is_null())
    } else {
        folder.join(tracks::SYS_TRACK).exists()
    };
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
    let voices_path = app.path().app_data_dir()?.join(voices::VOICES_FILE);
    let f = folder.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let mut emb = model.and_then(|m| embedder::Embedder::new(&m).map_err(|e| log::warn!("{e:#}")).ok());
        let _lock = VOICES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut memory = voices::Voices::load(&voices_path);
        let outcome = attribution::attribute_folder(&f, &me, emb.as_mut(), &memory)?;
        let meeting = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        memory.learn(&meeting, &outcome.voices);
        if let Err(e) = memory.save(&voices_path) {
            log::warn!("Voice memory not saved: {e:#}");
        }
        anyhow::Ok(outcome)
    })
    .await??;

    let updated = TranscriptsRepository::set_speakers_for_folder(&pool(&app), &folder.to_string_lossy(), &outcome.speakers).await?;
    log::info!("Speakers attributed for '{}' ({} DB rows)", outcome.title, updated);
    if notify {
        let (head, body) = if crate::i18n::it() {
            (
                format!("Trascrizione salvata – {}", outcome.title),
                "Parlanti riconosciuti; la trascrizione .md è nella cartella della riunione.",
            )
        } else {
            (
                format!("Transcript saved – {}", outcome.title),
                "Speakers identified; the .md transcript is in the meeting folder.",
            )
        };
        crate::autopilot::notify(&app, &head, body);
    }
    crate::search::kick(); // names changed
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
        let stopped_at = chrono::Utc::now();
        let pending = crate::summary::auto::Pending::start(&folder);
        tauri::async_runtime::spawn(async move {
            let _pending = pending; // the view shows "automatic summary on its way" until this ends
            crate::audio::retranscription::final_pass(&app, &folder).await;
            if let Err(e) = process_meeting(app.clone(), folder.clone(), true).await {
                log::error!("Speaker attribution failed: {e:#}");
            }
            // Dixit: the summary starts only on the final, attributed transcript.
            crate::summary::auto::after_final_transcript(&app, &folder, stopped_at).await;
        });
    });
}

/// Meetings stopped cleanly whose post-call work never finished (Dixit closed during large-v3,
/// attribution or the summary): attribution deletes the speaker tracks, so they are still there.
/// A recording never stopped cleanly (status "recording") is left alone: its audio may be cut.
fn interrupted_meetings(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|f| f.join(tracks::SYS_TRACK).exists() && f.join("transcripts.json").exists())
        .filter(|f| {
            std::fs::read_to_string(f.join("metadata.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .is_some_and(|m| m["status"] == "completed")
        })
        .collect();
    folders.sort();
    folders
}

/// At startup, in the background and one meeting at a time, finishes interrupted post-call work:
/// the final pass if it never ran, then speakers and the automatic summary (which replaces one
/// written on the live transcript before the interruption).
pub fn resume_interrupted<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The database opens after setup (later still on a first launch): wait, or retry next start.
        let mut ready = false;
        for _ in 0..60 {
            if app.try_state::<crate::state::AppState>().is_some() {
                ready = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        if !ready {
            return;
        }
        let root = crate::audio::recording_preferences::load_recording_preferences(&app)
            .await
            .map(|p| p.save_folder)
            .unwrap_or_else(|_| crate::audio::recording_preferences::get_default_recordings_folder());
        for folder in interrupted_meetings(&root) {
            log::info!("Resuming the post-call work of {}", folder.display());
            let _pending = crate::summary::auto::Pending::start(&folder);
            let resumed_at = chrono::Utc::now();
            if needs_final_pass(&app, &folder).await {
                crate::audio::retranscription::run_final_pass(&app, &folder).await;
            }
            if let Err(e) = process_meeting(app.clone(), folder.clone(), true).await {
                log::error!("Speaker attribution failed: {e:#}");
            }
            crate::summary::auto::after_final_transcript(&app, &folder, resumed_at).await;
        }
    });
}

/// The final pass runs when Whisper is the configured model (live was on Parakeet) and it has not
/// rewritten this transcript yet (it stamps `retranscribed_at`).
async fn needs_final_pass<R: Runtime>(app: &AppHandle<R>, folder: &Path) -> bool {
    let done = std::fs::read_to_string(folder.join("metadata.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .is_some_and(|m| !m["retranscribed_at"].is_null());
    !done
        && crate::api::api::api_get_transcript_config(app.clone(), app.state(), None)
            .await
            .ok()
            .flatten()
            .is_some_and(|c| c.provider == "localWhisper")
}

/// Renames a speaker label of a meeting (DB right away; files, .md and voice memory via a rerun in
/// the background, which for an older meeting first computes its voice prints from audio.mp4).
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
        // The view already shows the new name from the DB; `speakers-updated` follows the rerun.
        tauri::async_runtime::spawn(async move {
            if let Err(e) = process_meeting(app, folder, false).await {
                log::error!("Speaker rerun after rename failed: {e:#}");
            }
        });
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
    fn only_cleanly_stopped_meetings_with_tracks_are_resumed() {
        let root = tempfile::tempdir().unwrap();
        let meeting = |name: &str, status: &str, tracks: bool| {
            let f = root.path().join(name);
            std::fs::create_dir(&f).unwrap();
            std::fs::write(f.join("metadata.json"), serde_json::json!({ "status": status }).to_string()).unwrap();
            std::fs::write(f.join("transcripts.json"), "{}").unwrap();
            if tracks {
                std::fs::write(f.join(tracks::SYS_TRACK), "").unwrap();
            }
        };
        meeting("2026-11-02 09.34 Nuove attività", "completed", true); // closed during large-v3
        meeting("2026-10-30 12.02 Teams", "recording", true); // never stopped cleanly
        meeting("2026-10-30 15.56 Benè Elena", "completed", false); // done
        let found = interrupted_meetings(root.path());
        assert_eq!(found, vec![root.path().join("2026-11-02 09.34 Nuove attività")]);
        assert!(interrupted_meetings(&root.path().join("missing")).is_empty());
    }

    #[test]
    fn sanitize_title_for_file_cases() {
        assert_eq!(sanitize_title_for_file("A/B: c | d"), "A-B- c - d");
        assert_eq!(sanitize_title_for_file("  Riunione   lunga  "), "Riunione lunga");
        assert_eq!(sanitize_title_for_file(&"x".repeat(300)).chars().count(), 120);
    }
}
