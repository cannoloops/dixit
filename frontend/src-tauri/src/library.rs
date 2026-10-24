//! Read-only view of the saved meetings (metadata, participants, transcript, summary, notes),
//! shared by the MCP tools and the search index.

use crate::notes::{self, Note};
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{FromRow, SqlitePool};

pub struct Line {
    pub start: f64,
    pub end: f64,
    pub speaker: Option<String>,
    pub text: String,
}

/// What lists and the index need to know about a meeting, without its transcript.
pub struct Brief {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    /// Latest change of the meeting itself, its summary or its notes.
    pub updated_at: DateTime<Utc>,
    pub duration_secs: f64,
    pub participants: Vec<String>,
    pub line_count: i64,
    pub notes_count: i64,
    pub has_summary: bool,
}

impl Brief {
    /// Changes whenever anything the index covers changes.
    pub fn stamp(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.updated_at.to_rfc3339(),
            self.line_count,
            self.notes_count,
            self.participants.join(","),
            self.title
        )
    }
}

pub struct Meeting {
    pub brief: Brief,
    pub lines: Vec<Line>,
    pub notes: Vec<Note>,
    /// Markdown of the saved summary.
    pub summary: Option<String>,
}

/// RFC 3339, or SQLite's `YYYY-MM-DD HH:MM:SS[.f]` (UTC); unreadable values count as the epoch.
pub fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f").map(|n| n.and_utc()))
        .unwrap_or_default()
}

#[derive(FromRow)]
struct BriefRow {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    summary_updated: Option<String>,
    notes_updated: Option<String>,
    notes_count: i64,
    has_summary: i64,
    duration: Option<f64>,
    line_count: i64,
    speakers: Option<String>,
}

/// Every meeting, newest first. (Fine up to thousands of meetings; paginate in SQL beyond that.)
pub async fn briefs(pool: &SqlitePool) -> Result<Vec<Brief>, sqlx::Error> {
    let rows: Vec<BriefRow> = sqlx::query_as(
        "SELECT m.id, m.title, CAST(m.created_at AS TEXT) AS created_at, CAST(m.updated_at AS TEXT) AS updated_at,
                (SELECT CAST(s.updated_at AS TEXT) FROM summary_processes s WHERE s.meeting_id = m.id) AS summary_updated,
                (SELECT MAX(n.updated_at) FROM meeting_note_entries n WHERE n.meeting_id = m.id) AS notes_updated,
                (SELECT COUNT(*) FROM meeting_note_entries n WHERE n.meeting_id = m.id) AS notes_count,
                (SELECT COUNT(*) FROM summary_processes s WHERE s.meeting_id = m.id AND s.status = 'completed' AND s.result IS NOT NULL) AS has_summary,
                (SELECT MAX(t.audio_end_time) FROM transcripts t WHERE t.meeting_id = m.id) AS duration,
                (SELECT COUNT(*) FROM transcripts t WHERE t.meeting_id = m.id) AS line_count,
                (SELECT group_concat(DISTINCT t.speaker) FROM transcripts t WHERE t.meeting_id = m.id AND t.speaker <> '') AS speakers
         FROM meetings m ORDER BY m.created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let created_at = parse_ts(&r.created_at);
            let updated_at = [Some(parse_ts(&r.updated_at)), r.summary_updated.as_deref().map(parse_ts), r.notes_updated.as_deref().map(parse_ts)]
                .into_iter()
                .flatten()
                .max()
                .unwrap_or(created_at);
            let mut participants: Vec<String> =
                r.speakers.unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            participants.sort();
            Brief {
                id: r.id,
                title: r.title,
                created_at,
                updated_at,
                duration_secs: r.duration.unwrap_or(0.0),
                participants,
                line_count: r.line_count,
                notes_count: r.notes_count,
                has_summary: r.has_summary > 0,
            }
        })
        .collect())
}

/// The markdown of a stored summary result (`{"markdown": "..."}`), if it has any text.
pub fn summary_markdown(result: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(result).ok()?;
    let md = v["markdown"].as_str()?.trim();
    (!md.is_empty()).then(|| md.to_string())
}

pub async fn load(pool: &SqlitePool, id: &str) -> Result<Option<Meeting>, sqlx::Error> {
    let Some(brief) = briefs(pool).await?.into_iter().find(|b| b.id == id) else { return Ok(None) };
    let lines: Vec<(Option<f64>, Option<f64>, Option<String>, String)> = sqlx::query_as(
        "SELECT audio_start_time, audio_end_time, speaker, transcript FROM transcripts WHERE meeting_id = ?
         ORDER BY audio_start_time, timestamp",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let result: Option<Option<String>> =
        sqlx::query_scalar("SELECT result FROM summary_processes WHERE meeting_id = ?").bind(id).fetch_optional(pool).await?;
    Ok(Some(Meeting {
        lines: lines
            .into_iter()
            .map(|(start, end, speaker, text)| Line {
                start: start.unwrap_or(0.0),
                end: end.or(start).unwrap_or(0.0),
                speaker: speaker.filter(|s| !s.is_empty()),
                text,
            })
            .collect(),
        notes: notes::list(pool, id).await?,
        summary: result.flatten().and_then(|r| summary_markdown(&r)),
        brief,
    }))
}

#[cfg(test)]
pub mod fixtures {
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

    /// In-memory DB with the app's real tables (the subset the library and the index use).
    pub async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        for sql in [
            "CREATE TABLE meetings (id TEXT PRIMARY KEY, title TEXT, created_at TEXT, updated_at TEXT, folder_path TEXT)",
            "CREATE TABLE transcripts (id TEXT PRIMARY KEY, meeting_id TEXT, transcript TEXT, timestamp TEXT, audio_start_time REAL, audio_end_time REAL, duration REAL, speaker TEXT)",
            "CREATE TABLE summary_processes (meeting_id TEXT PRIMARY KEY, status TEXT, updated_at TEXT, result TEXT)",
        ] {
            sqlx::query(sql).execute(&p).await.unwrap();
        }
        for sql in [
            include_str!("../migrations/20261024120000_add_meeting_note_entries.sql"),
            include_str!("../migrations/20261024130000_add_search_index.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&p).await.unwrap();
        }
        p
    }

    pub async fn meeting(p: &SqlitePool, id: &str, title: &str, created: &str, lines: &[(f64, &str, &str)]) {
        sqlx::query("INSERT INTO meetings VALUES (?, ?, ?, ?, NULL)").bind(id).bind(title).bind(created).bind(created).execute(p).await.unwrap();
        for (i, (start, speaker, text)) in lines.iter().enumerate() {
            sqlx::query("INSERT INTO transcripts VALUES (?, ?, ?, '', ?, ?, 1.0, ?)")
                .bind(format!("{id}-{i}"))
                .bind(id)
                .bind(text)
                .bind(start)
                .bind(start + 3.0)
                .bind(if speaker.is_empty() { None } else { Some(*speaker) })
                .execute(p)
                .await
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn parses_both_timestamp_formats() {
        assert_eq!(parse_ts("2026-10-23T14:37:33.883356+00:00").to_rfc3339(), "2026-10-23T14:37:33.883356+00:00");
        assert_eq!(parse_ts("2026-10-23 12:38:59").to_rfc3339(), "2026-10-23T12:38:59+00:00");
        assert_eq!(parse_ts("boh").timestamp(), 0);
    }

    #[test]
    fn summary_markdown_reads_the_result_json() {
        assert_eq!(summary_markdown(r##"{"markdown": "# Titolo\n\nTesto"}"##).as_deref(), Some("# Titolo\n\nTesto"));
        assert_eq!(summary_markdown(r#"{"markdown": "  "}"#), None);
        assert_eq!(summary_markdown("non json"), None);
    }

    #[tokio::test]
    async fn briefs_and_load_collect_everything_about_a_meeting() {
        let p = pool().await;
        meeting(&p, "m1", "Sprint", "2026-10-23T10:00:00+00:00", &[(1.0, "Rossi", "ciao"), (5.0, "Bianchi", "buongiorno"), (9.0, "Rossi", "ok")]).await;
        meeting(&p, "m2", "Daily", "2026-10-24T10:00:00+00:00", &[(1.0, "", "solo")]).await;
        sqlx::query("INSERT INTO summary_processes VALUES ('m1', 'completed', '2026-10-25T08:00:00+00:00', ?)")
            .bind(r##"{"markdown": "# Riassunto"}"##)
            .execute(&p)
            .await
            .unwrap();
        sqlx::query("INSERT INTO meeting_note_entries VALUES ('n1', 'm1', 'chiedere budget', 4000, 'a', '2026-10-26T08:00:00+00:00')")
            .execute(&p)
            .await
            .unwrap();

        let all = briefs(&p).await.unwrap();
        assert_eq!(all.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["m2", "m1"]); // newest first
        let m1 = &all[1];
        assert_eq!(m1.participants, ["Bianchi", "Rossi"]);
        assert_eq!((m1.line_count, m1.notes_count, m1.has_summary), (3, 1, true));
        assert_eq!(m1.duration_secs, 12.0);
        // The summary and the note are later than the meeting row: the latest wins.
        assert_eq!(m1.updated_at.to_rfc3339(), "2026-10-26T08:00:00+00:00");
        assert!(all[0].participants.is_empty() && !all[0].has_summary);

        let full = load(&p, "m1").await.unwrap().unwrap();
        assert_eq!(full.lines.len(), 3);
        assert_eq!(full.lines[1].speaker.as_deref(), Some("Bianchi"));
        assert_eq!(full.notes[0].text, "chiedere budget");
        assert_eq!(full.summary.as_deref(), Some("# Riassunto"));
        assert!(load(&p, "nope").await.unwrap().is_none());
    }
}
