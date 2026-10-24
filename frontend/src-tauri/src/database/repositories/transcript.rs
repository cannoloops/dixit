use crate::api::{TranscriptSearchResult, TranscriptSegment};
use chrono::Utc;
use sqlx::{Connection, Error as SqlxError, SqlitePool};
use tracing::{error, info};
use uuid::Uuid;

pub struct TranscriptsRepository;

/// When the recording began, from the meeting folder's metadata.json (saved at recording start).
/// Meetings are dated by their start, not by when the transcript was saved at the end of the call.
fn started_at(folder_path: Option<&str>) -> Option<chrono::DateTime<Utc>> {
    let text = std::fs::read_to_string(std::path::Path::new(folder_path?).join("metadata.json")).ok()?;
    let metadata: serde_json::Value = serde_json::from_str(&text).ok()?;
    chrono::DateTime::parse_from_rfc3339(metadata["created_at"].as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

impl TranscriptsRepository {
    /// Saves a new meeting and its associated transcript segments.
    /// This function uses a transaction to ensure that either both the meeting
    /// and all its transcripts are saved, or none of them are.
    pub async fn save_transcript(
        pool: &SqlitePool,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
    ) -> Result<String, SqlxError> {
        let meeting_id = format!("meeting-{}", Uuid::new_v4());

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        let now = Utc::now();
        let created_at = started_at(folder_path.as_deref()).unwrap_or(now);

        // 1. Create the new meeting
        let result = sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at, folder_path) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&meeting_id)
        .bind(meeting_title)
        .bind(created_at)
        .bind(now)
        .bind(&folder_path)
        .execute(&mut *transaction)
        .await;

        if let Err(e) = result {
            error!("Failed to create meeting '{}': {}", meeting_title, e);
            transaction.rollback().await?;
            return Err(e);
        }

        info!("Successfully created meeting with id: {}", meeting_id);

        // 2. Save each transcript segment with audio timing fields
        for segment in transcripts {
            let transcript_id = format!("transcript-{}", Uuid::new_v4());
            let result = sqlx::query(
                "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&transcript_id)
            .bind(&meeting_id)
            .bind(&segment.text)
            .bind(&segment.timestamp)
            .bind(segment.audio_start_time)
            .bind(segment.audio_end_time)
            .bind(segment.duration)
            .execute(&mut *transaction)
            .await;

            if let Err(e) = result {
                error!(
                    "Failed to save transcript segment for meeting {}: {}",
                    meeting_id, e
                );
                transaction.rollback().await?;
                return Err(e);
            }
        }

        info!(
            "Successfully saved {} transcript segments for meeting {}",
            transcripts.len(),
            meeting_id
        );

        // Commit the transaction
        transaction.commit().await?;

        Ok(meeting_id)
    }

    /// Trascrivi: writes speaker labels onto the transcripts of the meeting recorded in
    /// `folder_path`, matching segments by `audio_start_time`. Returns rows updated (0 when the
    /// frontend has not saved the meeting yet).
    pub async fn set_speakers_for_folder(
        pool: &SqlitePool,
        folder_path: &str,
        speakers: &[(f64, String)],
    ) -> Result<u64, SqlxError> {
        let mut tx = pool.begin().await?;
        let mut updated = 0;
        for (start, name) in speakers {
            updated += sqlx::query(
                "UPDATE transcripts SET speaker = ?
                 WHERE meeting_id IN (SELECT id FROM meetings WHERE folder_path = ?)
                   AND ABS(audio_start_time - ?) < 0.01",
            )
            .bind(name)
            .bind(folder_path)
            .bind(start)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        }
        tx.commit().await?;
        Ok(updated)
    }

    /// Trascrivi: manual rename of one speaker label within a meeting.
    pub async fn rename_speaker(pool: &SqlitePool, meeting_id: &str, from: &str, to: &str) -> Result<u64, SqlxError> {
        Ok(sqlx::query("UPDATE transcripts SET speaker = ? WHERE meeting_id = ? AND speaker = ?")
            .bind(to)
            .bind(meeting_id)
            .bind(from)
            .execute(pool)
            .await?
            .rows_affected())
    }

    pub async fn meeting_for_folder(pool: &SqlitePool, folder_path: &str) -> Result<Option<String>, SqlxError> {
        sqlx::query_scalar("SELECT id FROM meetings WHERE folder_path = ?").bind(folder_path).fetch_optional(pool).await
    }

    pub async fn folder_for_meeting(pool: &SqlitePool, meeting_id: &str) -> Result<Option<String>, SqlxError> {
        let folder: Option<Option<String>> = sqlx::query_scalar("SELECT folder_path FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?;
        Ok(folder.flatten())
    }

    /// Searches for a query string within the transcripts.
    /// It returns a list of matching transcripts with context.
    pub async fn search_transcripts(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<Vec<TranscriptSearchResult>, SqlxError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let search_query = format!("%{}%", query.to_lowercase());

        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT m.id, m.title, t.transcript, t.timestamp
             FROM meetings m
             JOIN transcripts t ON m.id = t.meeting_id
             WHERE LOWER(t.transcript) LIKE ?",
        )
        .bind(&search_query)
        .fetch_all(pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|(id, title, transcript, timestamp)| {
                let match_context = Self::get_match_context(&transcript, query);
                TranscriptSearchResult {
                    id,
                    title,
                    match_context,
                    timestamp,
                }
            })
            .collect();

        Ok(results)
    }

    /// Helper function to extract a snippet of text around the first match of a query.
    fn get_match_context(transcript: &str, query: &str) -> String {
        let transcript_lower = transcript.to_lowercase();
        let query_lower = query.to_lowercase();

        match transcript_lower.find(&query_lower) {
            Some(match_index) => {
                let start_index = match_index.saturating_sub(100);
                let end_index = (match_index + query.len() + 100).min(transcript.len());

                let mut context = String::new();
                if start_index > 0 {
                    context.push_str("...");
                }
                context.push_str(&transcript[start_index..end_index]);
                if end_index < transcript.len() {
                    context.push_str("...");
                }
                context
            }
            None => transcript.chars().take(200).collect(), // Fallback to the start of the transcript
        }
    }
}

#[cfg(test)]
mod speaker_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        for sql in [
            "CREATE TABLE meetings (id TEXT PRIMARY KEY, title TEXT, created_at TEXT, updated_at TEXT, folder_path TEXT)",
            "CREATE TABLE transcripts (id TEXT PRIMARY KEY, meeting_id TEXT, transcript TEXT, timestamp TEXT, audio_start_time REAL, audio_end_time REAL, duration REAL, speaker TEXT)",
            "INSERT INTO meetings VALUES ('m1', 'Sprint', '', '', '/rec/sprint')",
            "INSERT INTO transcripts VALUES ('t1', 'm1', 'ciao', '', 1.0, 2.0, 1.0, NULL)",
            "INSERT INTO transcripts VALUES ('t2', 'm1', 'come va', '', 2.5, 4.0, 1.5, NULL)",
        ] {
            sqlx::query(sql).execute(&p).await.unwrap();
        }
        p
    }

    async fn speakers(p: &SqlitePool) -> Vec<Option<String>> {
        sqlx::query_scalar("SELECT speaker FROM transcripts ORDER BY audio_start_time").fetch_all(p).await.unwrap()
    }

    #[tokio::test]
    async fn sets_speakers_by_folder_and_start_time() {
        let p = pool().await;
        let n = TranscriptsRepository::set_speakers_for_folder(&p, "/rec/sprint", &[(1.0, "A".into()), (2.5, "B".into())])
            .await
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(speakers(&p).await, vec![Some("A".into()), Some("B".into())]);
        let none = TranscriptsRepository::set_speakers_for_folder(&p, "/other", &[(1.0, "X".into())]).await.unwrap();
        assert_eq!(none, 0);
    }

    #[tokio::test]
    async fn renames_speaker_and_finds_folder() {
        let p = pool().await;
        TranscriptsRepository::set_speakers_for_folder(&p, "/rec/sprint", &[(1.0, "A".into()), (2.5, "A".into())]).await.unwrap();
        assert_eq!(TranscriptsRepository::rename_speaker(&p, "m1", "A", "Rossi Mario").await.unwrap(), 2);
        assert_eq!(speakers(&p).await, vec![Some("Rossi Mario".into()); 2]);
        assert_eq!(TranscriptsRepository::folder_for_meeting(&p, "m1").await.unwrap().as_deref(), Some("/rec/sprint"));
    }
}

#[cfg(test)]
mod started_at_tests {
    use super::started_at;

    #[test]
    fn reads_recording_start_from_metadata() {
        let dir = std::env::temp_dir().join(format!("dixit-started-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("metadata.json"), r#"{"created_at":"2026-10-23T13:18:05+00:00"}"#).unwrap();
        let t = started_at(dir.to_str()).unwrap();
        assert_eq!(t.to_rfc3339(), "2026-10-23T13:18:05+00:00");
        assert!(started_at(Some("/nonexistent/dixit")).is_none());
        assert!(started_at(None).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
