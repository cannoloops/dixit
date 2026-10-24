//! Notes typed during a recording, each with its instant (ms from recording start).
//! While recording they live in `notes.json` in the meeting folder (the meeting is not in the DB
//! yet); when the meeting is saved they move to `meeting_note_entries`, and from then on every
//! edit rewrites `notes.json` and the folder's `.md`.

use crate::database::repositories::transcript::TranscriptsRepository;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const NOTES_FILE: &str = "notes.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub offset_ms: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Note {
    pub fn new(text: &str, offset_ms: i64) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Note {
            id: format!("note-{}", uuid::Uuid::new_v4()),
            text: text.trim().to_string(),
            offset_ms: offset_ms.max(0),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

pub fn read_file(folder: &Path) -> Vec<Note> {
    std::fs::read_to_string(folder.join(NOTES_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| serde_json::from_value(v["notes"].clone()).ok())
        .unwrap_or_default()
}

pub fn write_file(folder: &Path, notes: &[Note]) -> anyhow::Result<()> {
    let tmp = folder.join(".notes.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&serde_json::json!({ "notes": notes }))?)?;
    std::fs::rename(&tmp, folder.join(NOTES_FILE))?;
    Ok(())
}

/// Notes as (seconds, text) for the transcript `.md`.
pub fn for_md(folder: &Path) -> Vec<(f64, String)> {
    read_file(folder).into_iter().map(|n| (n.offset_ms as f64 / 1000.0, n.text)).collect()
}

pub async fn list(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, text, offset_ms, created_at, updated_at FROM meeting_note_entries WHERE meeting_id = ? ORDER BY offset_ms, created_at",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
}

/// Moves the folder's notes into the DB (idempotent: same ids are kept).
pub async fn import_folder(pool: &SqlitePool, meeting_id: &str, folder: &Path) {
    for n in read_file(folder) {
        let r = sqlx::query(
            "INSERT OR IGNORE INTO meeting_note_entries (id, meeting_id, text, offset_ms, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&n.id)
        .bind(meeting_id)
        .bind(&n.text)
        .bind(n.offset_ms)
        .bind(&n.created_at)
        .bind(&n.updated_at)
        .execute(pool)
        .await;
        if let Err(e) = r {
            log::warn!("Could not save note {}: {e}", n.id);
        }
    }
}

/// `Ok(None)` when no note has that id.
pub async fn update(pool: &SqlitePool, id: &str, text: &str) -> Result<Option<String>, sqlx::Error> {
    let r = sqlx::query("UPDATE meeting_note_entries SET text = ?, updated_at = ? WHERE id = ?")
        .bind(text.trim())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;
    meeting_of(pool, id, r.rows_affected()).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<Option<String>, sqlx::Error> {
    let meeting = sqlx::query_scalar("SELECT meeting_id FROM meeting_note_entries WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    sqlx::query("DELETE FROM meeting_note_entries WHERE id = ?").bind(id).execute(pool).await?;
    Ok(meeting)
}

async fn meeting_of(pool: &SqlitePool, id: &str, changed: u64) -> Result<Option<String>, sqlx::Error> {
    if changed == 0 {
        return Ok(None);
    }
    sqlx::query_scalar("SELECT meeting_id FROM meeting_note_entries WHERE id = ?").bind(id).fetch_optional(pool).await
}

fn pool<R: Runtime>(app: &AppHandle<R>) -> SqlitePool {
    app.state::<crate::state::AppState>().db_manager.pool().clone()
}

async fn recording_folder() -> Result<PathBuf, String> {
    crate::audio::recording_commands::get_meeting_folder_path()
        .await?
        .map(PathBuf::from)
        .ok_or_else(|| "Nessuna registrazione in corso".to_string())
}

/// Saves a note at the current instant of the recording; returns all notes so far.
#[tauri::command]
pub async fn recording_note_add<R: Runtime>(app: AppHandle<R>, text: String) -> Result<Vec<Note>, String> {
    if text.trim().is_empty() {
        return Err("Nota vuota".into());
    }
    let folder = recording_folder().await?;
    let secs = crate::audio::recording_commands::get_recording_state().await["recording_duration"]
        .as_f64()
        .unwrap_or(0.0);
    let mut notes = read_file(&folder);
    notes.push(Note::new(&text, (secs * 1000.0).round() as i64));
    write_file(&folder, &notes).map_err(|e| e.to_string())?;
    let _ = app.emit("recording-notes", &notes);
    Ok(notes)
}

#[tauri::command]
pub async fn recording_notes_list() -> Result<Vec<Note>, String> {
    Ok(read_file(&recording_folder().await?))
}

#[tauri::command]
pub async fn meeting_notes_list<R: Runtime>(app: AppHandle<R>, meeting_id: String) -> Result<Vec<Note>, String> {
    list(&pool(&app), &meeting_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn meeting_note_update<R: Runtime>(app: AppHandle<R>, id: String, text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("Nota vuota".into());
    }
    let meeting = update(&pool(&app), &id, &text).await.map_err(|e| e.to_string())?;
    sync_folder(&app, meeting).await
}

#[tauri::command]
pub async fn meeting_note_delete<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    let meeting = delete(&pool(&app), &id).await.map_err(|e| e.to_string())?;
    sync_folder(&app, meeting).await
}

/// Rewrites notes.json and the .md of the meeting's folder after an edit.
async fn sync_folder<R: Runtime>(app: &AppHandle<R>, meeting: Option<String>) -> Result<(), String> {
    let Some(meeting) = meeting else { return Ok(()) };
    let pool = pool(app);
    let Some(folder) = TranscriptsRepository::folder_for_meeting(&pool, &meeting).await.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let folder = PathBuf::from(folder);
    if !folder.is_dir() {
        return Ok(());
    }
    let notes = list(&pool, &meeting).await.map_err(|e| e.to_string())?;
    write_file(&folder, &notes).map_err(|e| e.to_string())?;
    crate::search::kick();
    if folder.join("transcripts.json").exists() {
        crate::speakers::process_meeting(app.clone(), folder, false).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE meetings (id TEXT PRIMARY KEY, title TEXT, created_at TEXT, updated_at TEXT, folder_path TEXT)")
            .execute(&p)
            .await
            .unwrap();
        sqlx::query("INSERT INTO meetings VALUES ('m1', 'Sprint', '', '', '/rec/sprint')").execute(&p).await.unwrap();
        for stmt in include_str!("../migrations/20261024120000_add_meeting_note_entries.sql").split(';').filter(|s| s.trim().len() > 5) {
            sqlx::query(stmt).execute(&p).await.unwrap();
        }
        p
    }

    #[test]
    fn notes_survive_the_folder_roundtrip_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let notes = vec![Note::new("  prima ", 12_345), Note::new("seconda", 61_000)];
        write_file(dir.path(), &notes).unwrap();
        assert_eq!(read_file(dir.path()), notes);
        assert_eq!(read_file(dir.path())[0].text, "prima");
        assert_eq!(for_md(dir.path()), vec![(12.345, "prima".to_string()), (61.0, "seconda".to_string())]);
        assert!(read_file(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn negative_offsets_are_clamped() {
        assert_eq!(Note::new("x", -5).offset_ms, 0);
    }

    #[tokio::test]
    async fn import_edit_delete_in_the_db() {
        let p = pool().await;
        let dir = tempfile::tempdir().unwrap();
        let notes = vec![Note::new("tardi", 90_000), Note::new("presto", 5_000), Note::new("da togliere", 7_000)];
        write_file(dir.path(), &notes).unwrap();

        import_folder(&p, "m1", dir.path()).await;
        import_folder(&p, "m1", dir.path()).await; // rerun keeps the same rows
        let got = list(&p, "m1").await.unwrap();
        assert_eq!(got.iter().map(|n| n.text.as_str()).collect::<Vec<_>>(), ["presto", "da togliere", "tardi"]);
        assert_eq!(got[0].offset_ms, 5_000);

        assert_eq!(update(&p, &got[0].id, " corretto ").await.unwrap().as_deref(), Some("m1"));
        assert_eq!(update(&p, "nope", "x").await.unwrap(), None);
        assert_eq!(delete(&p, &got[1].id).await.unwrap().as_deref(), Some("m1"));
        let got = list(&p, "m1").await.unwrap();
        assert_eq!(got.iter().map(|n| n.text.as_str()).collect::<Vec<_>>(), ["corretto", "tardi"]);
    }
}
