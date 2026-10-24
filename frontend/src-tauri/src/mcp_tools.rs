//! Read-only MCP tools over the saved meetings: `list_meetings`, `get_meeting`,
//! `search_transcripts`. (`live_transcript`, about the call in progress, lives in `mcp.rs`.)

use crate::library::{self, Brief, Meeting};
use crate::notes::Note;
use crate::search::{self, Hit};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const DEFAULT_LIST: usize = 20;
const MAX_LIST: usize = 100;
const DEFAULT_HITS: usize = 10;
const MAX_HITS: usize = 50;
/// Silence longer than this starts a new turn even if the same person goes on speaking.
const TURN_GAP_SECS: f64 = 30.0;

pub fn definitions() -> Vec<Value> {
    let read_only = json!({"readOnlyHint": true});
    vec![
        json!({
            "name": "list_meetings",
            "description": "Meetings saved by Dixit, newest first: id, title, start, last change, duration, participants, \
                number of notes, whether a summary exists. Filter by period (`from`/`to` on the start) or by `updated_since` \
                (anything changed after it: transcript, summary or notes) to ingest only what is new. Paginated.",
            "inputSchema": {"type": "object", "properties": {
                "from": {"type": "string", "description": "Meetings that started at or after this (ISO 8601 date or date-time; a date is local midnight)"},
                "to": {"type": "string", "description": "Meetings that started before this (a date includes that whole day)"},
                "updated_since": {"type": "string", "description": "Only meetings changed after this ISO 8601 date-time (use the newest `updated_at` already ingested)"},
                "limit": {"type": "integer", "description": format!("Page size (default {DEFAULT_LIST}, max {MAX_LIST})")},
                "offset": {"type": "integer", "description": "Skip this many (default 0); `next_offset` in the reply continues"}
            }},
            "annotations": read_only,
        }),
        json!({
            "name": "get_meeting",
            "description": "One meeting in full: metadata, participants, transcript with speakers and times, summary \
                and the user's own notes (typed during the call, each with its instant). `format: markdown` is ready \
                to ingest into a wiki; `json` is structured.",
            "inputSchema": {"type": "object", "properties": {
                "id": {"type": "string", "description": "Meeting id from list_meetings or search_transcripts"},
                "include_notes": {"type": "boolean", "description": "Include the user's notes (default true)"},
                "format": {"type": "string", "enum": ["json", "markdown"], "description": "Default json"}
            }, "required": ["id"]},
            "annotations": read_only,
        }),
        json!({
            "name": "search_transcripts",
            "description": "Search what was said in all meetings (transcripts, the user's notes and summaries). \
                `classic` finds words, \"quoted phrases\" and word starts, ignoring accents; `smart` finds passages by \
                meaning (\"performance problems\" finds \"the system is slow\"); `hybrid` (default) merges both. Each result \
                has the meeting, the instant in it (mm:ss from the start) and an excerpt; fetch the rest with get_meeting.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "mode": {"type": "string", "enum": ["classic", "smart", "hybrid"], "description": "Default hybrid"},
                "limit": {"type": "integer", "description": format!("Default {DEFAULT_HITS}, max {MAX_HITS}")},
                "meeting_id": {"type": "string", "description": "Search only inside this meeting"}
            }, "required": ["query"]},
            "annotations": read_only,
        }),
    ]
}

/// None when `name` is not one of these tools; otherwise the text of the reply or an error message.
pub async fn call(pool: &SqlitePool, name: &str, args: &Value) -> Option<Result<String, String>> {
    Some(match name {
        "list_meetings" => list_meetings(pool, args).await,
        "get_meeting" => get_meeting(pool, args).await,
        "search_transcripts" => search_transcripts(pool, args).await,
        _ => return None,
    })
}

fn db_err(e: impl std::fmt::Display) -> String {
    format!("Database error: {e}")
}

/// Date (local midnight; `end` = the day after, so the day is included) or RFC 3339 date-time.
fn parse_bound(field: &str, s: &str, end: bool) -> Result<DateTime<Utc>, String> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| format!("`{field}` must be an ISO 8601 date or date-time, got '{s}'"))?;
    let date = if end { date + Duration::days(1) } else { date };
    Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .ok_or_else(|| format!("`{field}` is not a valid local time"))
}

fn arg_bound(args: &Value, field: &str, end: bool) -> Result<Option<DateTime<Utc>>, String> {
    args[field].as_str().map(|s| parse_bound(field, s, end)).transpose()
}

fn arg_count(args: &Value, field: &str, default: usize, max: usize) -> usize {
    args[field].as_u64().map_or(default, |n| (n as usize).clamp(1, max))
}

fn brief_json(b: &Brief) -> Value {
    json!({
        "id": b.id,
        "title": b.title,
        "created_at": b.created_at.to_rfc3339(),
        "updated_at": b.updated_at.to_rfc3339(),
        "duration_seconds": b.duration_secs.round() as i64,
        "participants": b.participants,
        "notes_count": b.notes_count,
        "has_summary": b.has_summary,
    })
}

async fn list_meetings(pool: &SqlitePool, args: &Value) -> Result<String, String> {
    let (from, to, since) = (arg_bound(args, "from", false)?, arg_bound(args, "to", true)?, arg_bound(args, "updated_since", false)?);
    let (limit, offset) = (arg_count(args, "limit", DEFAULT_LIST, MAX_LIST), args["offset"].as_u64().unwrap_or(0) as usize);
    let all: Vec<Brief> = library::briefs(pool)
        .await
        .map_err(db_err)?
        .into_iter()
        .filter(|b| from.is_none_or(|f| b.created_at >= f) && to.is_none_or(|t| b.created_at < t) && since.is_none_or(|s| b.updated_at > s))
        .collect();
    let page: Vec<Value> = all.iter().skip(offset).take(limit).map(brief_json).collect();
    let next = (offset + page.len() < all.len()).then_some(offset + page.len());
    Ok(json!({"total": all.len(), "offset": offset, "next_offset": next, "meetings": page}).to_string())
}

/// "mm:ss", or "h:mm:ss" past the hour.
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60) } else { format!("{:02}:{:02}", s / 60, s % 60) }
}

async fn get_meeting(pool: &SqlitePool, args: &Value) -> Result<String, String> {
    let id = args["id"].as_str().filter(|s| !s.is_empty()).ok_or("`id` is required")?;
    let include_notes = args["include_notes"].as_bool().unwrap_or(true);
    let markdown = match args["format"].as_str().unwrap_or("json") {
        "json" => false,
        "markdown" => true,
        other => return Err(format!("`format` must be json or markdown, got '{other}'")),
    };
    let m = library::load(pool, id).await.map_err(db_err)?.ok_or_else(|| format!("No meeting with id '{id}'"))?;
    Ok(if markdown { meeting_markdown(&m, include_notes, crate::i18n::it()) } else { meeting_json(&m, include_notes).to_string() })
}

fn note_json(n: &Note) -> Value {
    json!({"id": n.id, "at": clock(n.offset_ms as f64 / 1000.0), "offset_ms": n.offset_ms, "text": n.text, "created_at": n.created_at})
}

fn meeting_json(m: &Meeting, include_notes: bool) -> Value {
    let mut v = brief_json(&m.brief);
    v["summary"] = json!(m.summary);
    v["transcript"] = m
        .lines
        .iter()
        .map(|l| json!({"start": (l.start * 100.0).round() / 100.0, "end": (l.end * 100.0).round() / 100.0, "at": clock(l.start), "speaker": l.speaker, "text": l.text}))
        .collect();
    if include_notes {
        v["notes"] = m.notes.iter().map(note_json).collect();
    }
    v
}

/// Wiki-ready markdown: header, summary, then the transcript by turn with the notes in between.
pub fn meeting_markdown(m: &Meeting, include_notes: bool, it: bool) -> String {
    let tr = |a: &'static str, b: &'static str| if it { a } else { b };
    let b = &m.brief;
    let mut md = format!("# {}\n\n", b.title.trim());
    md.push_str(&format!("- **{}:** {}\n", tr("Data", "Date"), b.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M")));
    if b.duration_secs >= 1.0 {
        md.push_str(&format!("- **{}:** {}\n", tr("Durata", "Duration"), clock(b.duration_secs)));
    }
    if !b.participants.is_empty() {
        md.push_str(&format!("- **{}:** {}\n", tr("Partecipanti", "Participants"), b.participants.join(", ")));
    }
    md.push_str(&format!("- **ID:** {}\n", b.id));
    if let Some(summary) = &m.summary {
        md.push_str(&format!("\n## {}\n\n{}\n", tr("Riassunto", "Summary"), summary.trim()));
    }
    md.push_str(&format!("\n## {}\n", tr("Trascrizione", "Transcript")));
    let notes: Vec<&Note> = if include_notes { m.notes.iter().collect() } else { vec![] };
    let mut notes = notes.into_iter().peekable();
    let write_notes_before = |md: &mut String, notes: &mut std::iter::Peekable<std::vec::IntoIter<&Note>>, at: f64| {
        while let Some(n) = notes.next_if(|n| n.offset_ms as f64 / 1000.0 <= at) {
            md.push_str(&format!("\n> {} [{}] {}\n", tr("Nota", "Note"), clock(n.offset_ms as f64 / 1000.0), n.text.split_whitespace().collect::<Vec<_>>().join(" ")));
        }
    };
    let mut turn: Option<(f64, Option<&str>, String)> = None; // (start, speaker, text)
    let mut last_end = f64::MIN;
    let flush = |md: &mut String, turn: &mut Option<(f64, Option<&str>, String)>| {
        if let Some((start, speaker, text)) = turn.take() {
            match speaker {
                Some(s) => md.push_str(&format!("\n[{}] **{}:** {}\n", clock(start), s, text)),
                None => md.push_str(&format!("\n[{}] {}\n", clock(start), text)),
            }
        }
    };
    for l in m.lines.iter().filter(|l| !l.text.trim().is_empty()) {
        let text = l.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let breaks = l.start - last_end > TURN_GAP_SECS || notes.peek().is_some_and(|n| n.offset_ms as f64 / 1000.0 <= l.start);
        last_end = l.end;
        match &mut turn {
            Some((_, speaker, t)) if *speaker == l.speaker.as_deref() && !breaks => {
                t.push(' ');
                t.push_str(&text);
            }
            _ => {
                flush(&mut md, &mut turn);
                write_notes_before(&mut md, &mut notes, l.start);
                turn = Some((l.start, l.speaker.as_deref(), text));
            }
        }
    }
    flush(&mut md, &mut turn);
    write_notes_before(&mut md, &mut notes, f64::MAX);
    md
}

fn hit_json(h: &Hit, meetings: &[Brief]) -> Value {
    let b = meetings.iter().find(|b| b.id == h.meeting_id);
    json!({
        "meeting_id": h.meeting_id,
        "meeting_title": b.map(|b| b.title.as_str()),
        "meeting_date": b.map(|b| b.created_at.to_rfc3339()),
        "kind": h.kind,
        "at": h.start_ms.map(|ms| clock(ms as f64 / 1000.0)),
        "start_ms": h.start_ms,
        "excerpt": h.excerpt,
        "score": (h.score * 1e4).round() / 1e4,
        "matched_by": h.matched_by,
    })
}

async fn embed_query(q: &str) -> Result<Vec<f32>, String> {
    let dir = search::model_dir().ok_or("the embedding model is not installed")?;
    let q = q.to_string();
    tokio::task::spawn_blocking(move || crate::embedding::with_embedder(&dir, |e| e.embed_query(&q)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

async fn search_transcripts(pool: &SqlitePool, args: &Value) -> Result<String, String> {
    let q = args["query"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or("`query` is required")?;
    let mode = args["mode"].as_str().unwrap_or("hybrid");
    if !["classic", "smart", "hybrid"].contains(&mode) {
        return Err(format!("`mode` must be classic, smart or hybrid, got '{mode}'"));
    }
    let limit = arg_count(args, "limit", DEFAULT_HITS, MAX_HITS);
    let meeting = args["meeting_id"].as_str().filter(|s| !s.is_empty());
    let query_vec = if mode == "classic" { Err(String::new()) } else { embed_query(q).await };
    run_search(pool, q, mode, limit, meeting, query_vec).await
}

/// `query_vec` is the embedding of the query, or why there is none (not needed in classic mode).
async fn run_search(pool: &SqlitePool, q: &str, mode: &str, limit: usize, meeting: Option<&str>, query_vec: Result<Vec<f32>, String>) -> Result<String, String> {
    let mut warning = None;
    let wide = (limit * 3).max(30) as i64; // each list is longer than the final one, so the fusion has material
    let classic = if mode == "smart" { vec![] } else { search::search_classic(pool, q, wide, meeting).await.map_err(db_err)? };
    let smart = match (mode, query_vec) {
        ("classic", _) => vec![],
        (_, Ok(v)) => search::search_smart(pool, &v, wide as usize, meeting).await.map_err(db_err)?,
        ("smart", Err(e)) => return Err(format!("Semantic search unavailable: {e}")),
        (_, Err(e)) => {
            warning = Some(format!("Semantic search unavailable ({e}); showing exact matches only"));
            vec![]
        }
    };
    let hits = match mode {
        "classic" => classic.into_iter().take(limit).collect(),
        "smart" => smart.into_iter().take(limit).collect(),
        _ => search::fuse(vec![classic, smart], limit),
    };
    let meetings = library::briefs(pool).await.map_err(db_err)?;
    Ok(json!({"mode": mode, "warning": warning, "results": hits.iter().map(|h| hit_json(h, &meetings)).collect::<Vec<_>>()}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::fixtures::*;
    use crate::embedding;

    async fn fixture() -> SqlitePool {
        let p = pool().await;
        meeting(&p, "m1", "Sprint review", "2026-10-23T10:00:00+00:00", &[
            (1.0, "Rossi", "Buongiorno a tutti."),
            (4.0, "Rossi", "Partiamo dal rilascio."),
            (9.0, "Bianchi", "Il sistema è lento"),
            (3700.0, "Bianchi", "Chiudiamo qui"),
        ]).await;
        meeting(&p, "m2", "Daily", "2026-10-25T10:00:00+00:00", &[(2.0, "", "Pranzo insieme")]).await;
        sqlx::query("INSERT INTO meeting_note_entries VALUES ('n1', 'm1', 'chiedere   il budget', 6000, 'a', '2026-10-26T08:00:00+00:00')").execute(&p).await.unwrap();
        sqlx::query("INSERT INTO summary_processes VALUES ('m1', 'completed', '2026-10-24T08:00:00+00:00', ?)")
            .bind(r##"{"markdown": "**Decisioni**\n\nSi rilascia a luglio."}"##).execute(&p).await.unwrap();
        search::sync_text(&p).await.unwrap();
        p
    }

    fn parse(r: Option<Result<String, String>>) -> Value {
        serde_json::from_str(&r.unwrap().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn list_filters_by_period_and_updated_since_and_paginates() {
        let p = fixture().await;
        let all = parse(call(&p, "list_meetings", &json!({})).await);
        assert_eq!(all["total"], 2);
        assert_eq!(all["meetings"][0]["id"], "m2"); // newest first
        let m1 = &all["meetings"][1];
        assert_eq!((m1["participants"].clone(), m1["notes_count"].clone(), m1["has_summary"].clone()), (json!(["Bianchi", "Rossi"]), json!(1), json!(true)));
        assert_eq!(m1["updated_at"], "2026-10-26T08:00:00+00:00"); // the note is the latest change

        let ids = |v: &Value| v["meetings"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(ids(&parse(call(&p, "list_meetings", &json!({"from": "2026-10-24T00:00:00Z"})).await)), ["m2"]);
        assert_eq!(ids(&parse(call(&p, "list_meetings", &json!({"to": "2026-10-23T12:00:00Z"})).await)), ["m1"]);
        // a date-only `to` includes that whole (local) day
        assert_eq!(ids(&parse(call(&p, "list_meetings", &json!({"from": "2026-10-23", "to": "2026-10-25"})).await)).len(), 2);
        // ingest: only what changed after the last seen update
        assert_eq!(ids(&parse(call(&p, "list_meetings", &json!({"updated_since": "2026-10-26T08:00:00Z"})).await)), Vec::<String>::new());
        assert_eq!(ids(&parse(call(&p, "list_meetings", &json!({"updated_since": "2026-10-25T12:00:00Z"})).await)), ["m1", ]);
        let page = parse(call(&p, "list_meetings", &json!({"limit": 1})).await);
        assert_eq!((page["meetings"].as_array().unwrap().len(), page["next_offset"].clone()), (1, json!(1)));
        let last = parse(call(&p, "list_meetings", &json!({"limit": 1, "offset": 1})).await);
        assert_eq!((last["meetings"][0]["id"].clone(), last["next_offset"].clone()), (json!("m1"), Value::Null));
        assert!(call(&p, "list_meetings", &json!({"from": "ieri"})).await.unwrap().is_err());
    }

    #[tokio::test]
    async fn get_meeting_json_and_notes_switch() {
        let p = fixture().await;
        let m = parse(call(&p, "get_meeting", &json!({"id": "m1"})).await);
        assert_eq!(m["title"], "Sprint review");
        assert_eq!(m["transcript"][2], json!({"start": 9.0, "end": 12.0, "at": "00:09", "speaker": "Bianchi", "text": "Il sistema è lento"}));
        assert_eq!(m["summary"], "**Decisioni**\n\nSi rilascia a luglio.");
        assert_eq!((m["notes"][0]["at"].clone(), m["notes"][0]["text"].clone()), (json!("00:06"), json!("chiedere   il budget")));
        let m = parse(call(&p, "get_meeting", &json!({"id": "m1", "include_notes": false})).await);
        assert!(m.get("notes").is_none());
        assert!(call(&p, "get_meeting", &json!({"id": "nope"})).await.unwrap().is_err());
        assert!(call(&p, "get_meeting", &json!({})).await.unwrap().is_err());
        assert!(call(&p, "get_meeting", &json!({"id": "m1", "format": "xml"})).await.unwrap().is_err());
        assert!(call(&p, "non_existent", &json!({})).await.is_none());
    }

    #[tokio::test]
    async fn get_meeting_markdown_is_wiki_ready() {
        let p = fixture().await;
        let md = call(&p, "get_meeting", &json!({"id": "m1", "format": "markdown"})).await.unwrap().unwrap();
        let body = md.split("## Trascrizione\n").nth(1).unwrap();
        // consecutive lines of a speaker are one turn; the note (6 s) splits Rossi's turn at 4 s? no: it falls between 4 s and 9 s
        assert_eq!(
            body,
            "\n[00:01] **Rossi:** Buongiorno a tutti. Partiamo dal rilascio.\n\
             \n> Nota [00:06] chiedere il budget\n\
             \n[00:09] **Bianchi:** Il sistema è lento\n\
             \n[1:01:40] **Bianchi:** Chiudiamo qui\n"
        );
        assert!(md.starts_with("# Sprint review\n\n- **Data:** 2026-10-23 "), "{md}");
        assert!(md.contains("- **Partecipanti:** Bianchi, Rossi\n- **ID:** m1\n\n## Riassunto\n\n**Decisioni**\n\nSi rilascia a luglio.\n"));
        let without = call(&p, "get_meeting", &json!({"id": "m1", "format": "markdown", "include_notes": false})).await.unwrap().unwrap();
        assert!(!without.contains("Nota"));
        // the English labels
        assert!(meeting_markdown(&library::load(&p, "m1").await.unwrap().unwrap(), true, false).contains("## Transcript\n"));
    }

    #[test]
    fn markdown_puts_a_note_inside_a_turn_when_its_instant_falls_between_lines() {
        let line = |start: f64, text: &str| library::Line { start, end: start + 1.0, speaker: Some("Rossi".into()), text: text.into() };
        let mut m = Meeting {
            brief: Brief { id: "m".into(), title: "T".into(), created_at: Default::default(), updated_at: Default::default(), duration_secs: 0.0, participants: vec![], line_count: 2, notes_count: 1, has_summary: false },
            lines: vec![line(1.0, "uno"), line(10.0, "due")],
            notes: vec![Note::new("in mezzo", 5_000)],
            summary: None,
        };
        let md = meeting_markdown(&m, true, true);
        assert!(md.ends_with("\n[00:01] **Rossi:** uno\n\n> Nota [00:05] in mezzo\n\n[00:10] **Rossi:** due\n"), "{md}");
        m.notes[0].offset_ms = 99_000; // after everything: goes last
        assert!(meeting_markdown(&m, true, true).ends_with("due\n\n> Nota [01:39] in mezzo\n"));
    }

    #[tokio::test]
    async fn search_modes_and_degradation_without_a_model() {
        let p = fixture().await;
        let none = || Err::<Vec<f32>, String>("the embedding model is not installed".into());
        // classic: finds the transcript line, the note and the summary
        let r: Value = serde_json::from_str(&run_search(&p, "budget", "classic", 10, None, none()).await.unwrap()).unwrap();
        assert_eq!(r["results"][0]["kind"], "note");
        assert_eq!((r["results"][0]["meeting_title"].clone(), r["results"][0]["at"].clone()), (json!("Sprint review"), json!("00:06")));
        let r: Value = serde_json::from_str(&run_search(&p, "luglio", "classic", 10, None, none()).await.unwrap()).unwrap();
        assert_eq!((r["results"][0]["kind"].clone(), r["results"][0]["at"].clone()), (json!("summary"), Value::Null));
        // hybrid degrades to classic with a warning; smart refuses
        let r: Value = serde_json::from_str(&run_search(&p, "sistema", "hybrid", 10, None, none()).await.unwrap()).unwrap();
        assert_eq!(r["results"].as_array().unwrap().len(), 1);
        assert!(r["warning"].as_str().unwrap().contains("not installed"));
        assert!(run_search(&p, "sistema", "smart", 10, None, none()).await.unwrap_err().contains("unavailable"));
        // smart with a vector: the stored one decides
        sqlx::query("UPDATE search_chunks SET embedding = ? WHERE text LIKE '%sistema%'").bind(embedding::to_blob(&[1.0, 0.0])).execute(&p).await.unwrap();
        let r: Value = serde_json::from_str(&run_search(&p, "problemi di performance", "smart", 10, None, Ok(vec![1.0, 0.0])).await.unwrap()).unwrap();
        assert_eq!(r["results"][0]["matched_by"], json!(["smart"]));
        assert!(r["results"][0]["excerpt"].as_str().unwrap().contains("sistema è lento"));
        assert!(call(&p, "search_transcripts", &json!({"query": "  "})).await.unwrap().is_err());
        assert!(call(&p, "search_transcripts", &json!({"query": "x", "mode": "fuzzy"})).await.unwrap().is_err());
    }
}
