//! Search over saved meetings: transcript, notes and summaries are cut into chunks stored in
//! SQLite. Exact search is FTS5 (phrases, prefixes, accents folded); semantic search compares
//! the query's embedding with the chunks' (no LLM); `fuse` merges both with reciprocal rank fusion.

use crate::embedding;
use crate::library::{self, Meeting};
use anyhow::Result;
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Notify;

/// A chunk stops growing past this many characters (at a line boundary): ~3-4 spoken sentences,
/// so a hit lands within about half a minute of the instant it points to.
const CHUNK_CHARS: usize = 300;
const EXCERPT_CHARS: usize = 240;
/// Semantic hits below this cosine similarity are noise (EmbeddingGemma: unrelated text ≈ 0.2–0.3).
const MIN_COSINE: f32 = 0.35;
/// Reciprocal rank fusion constant (the usual 60).
const RRF_K: f64 = 60.0;
const SYNC_EVERY: Duration = Duration::from_secs(120);

#[derive(Debug, PartialEq)]
pub struct Chunk {
    pub kind: &'static str,
    pub start_ms: Option<i64>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub chunk_id: i64,
    pub meeting_id: String,
    pub kind: String,
    pub start_ms: Option<i64>,
    pub excerpt: String,
    pub score: f64,
    pub matched_by: Vec<&'static str>,
}

fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", text[..i].trim_end()),
        None => text.to_string(),
    }
}

/// Transcript lines grouped up to `CHUNK_CHARS` ("Speaker: text" where the speaker changes), one
/// chunk per note, summary paragraphs grouped the same way.
pub fn chunks(m: &Meeting) -> Vec<Chunk> {
    let mut out = Vec::new();
    let (mut text, mut start, mut speaker): (String, i64, Option<&str>) = (String::new(), 0, None);
    for l in m.lines.iter().filter(|l| !l.text.trim().is_empty()) {
        if text.chars().count() >= CHUNK_CHARS {
            out.push(Chunk { kind: "transcript", start_ms: Some(start), text: std::mem::take(&mut text) });
            speaker = None;
        }
        if text.is_empty() {
            start = (l.start * 1000.0).round() as i64;
        }
        if l.speaker.as_deref() != speaker || text.is_empty() {
            if let Some(name) = &l.speaker {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&format!("{name}: "));
            }
            speaker = l.speaker.as_deref();
        } else {
            text.push(' ');
        }
        text.push_str(l.text.trim());
    }
    if !text.is_empty() {
        out.push(Chunk { kind: "transcript", start_ms: Some(start), text });
    }
    out.extend(m.notes.iter().filter(|n| !n.text.trim().is_empty()).map(|n| Chunk {
        kind: "note",
        start_ms: Some(n.offset_ms),
        text: n.text.trim().to_string(),
    }));
    let mut para = String::new();
    for p in m.summary.as_deref().unwrap_or_default().split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        if !para.is_empty() && para.chars().count() + p.chars().count() > 2 * CHUNK_CHARS {
            out.push(Chunk { kind: "summary", start_ms: None, text: std::mem::take(&mut para) });
        }
        if !para.is_empty() {
            para.push_str("\n\n");
        }
        para.push_str(p);
    }
    if !para.is_empty() {
        out.push(Chunk { kind: "summary", start_ms: None, text: para });
    }
    out
}

/// FTS5 expression for what the user typed: "quoted" words are a phrase, every other word is a
/// prefix ("perf" finds "performance"), all joined with AND (`any` = OR). None when empty.
pub fn fts_query(q: &str, any: bool) -> Option<String> {
    let mut terms = Vec::new();
    for (i, part) in q.split('"').enumerate() {
        let words: Vec<&str> = part.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
        if i % 2 == 1 && !words.is_empty() {
            terms.push(format!("\"{}\"", words.join(" ")));
        } else if i % 2 == 0 {
            // Short words stay exact: "di"* would match half the transcript.
            terms.extend(words.iter().map(|w| if w.chars().count() >= 3 { format!("\"{w}\"*") } else { format!("\"{w}\"") }));
        }
    }
    (!terms.is_empty()).then(|| terms.join(if any { " OR " } else { " AND " }))
}

#[derive(FromRow)]
struct ClassicRow {
    id: i64,
    meeting_id: String,
    kind: String,
    start_ms: Option<i64>,
    excerpt: String,
    rank: f64,
}

pub async fn search_classic(pool: &SqlitePool, q: &str, limit: i64, meeting: Option<&str>) -> Result<Vec<Hit>> {
    // All words first; if nothing contains all of them, any of them.
    for any in [false, true] {
        let Some(expr) = fts_query(q, any) else { return Ok(vec![]) };
        let rows: Vec<ClassicRow> = sqlx::query_as(
            "SELECT c.id, c.meeting_id, c.kind, c.start_ms,
                    snippet(search_fts, 0, '**', '**', '…', 24) AS excerpt, bm25(search_fts) AS rank
             FROM search_fts JOIN search_chunks c ON c.id = search_fts.rowid
             WHERE search_fts MATCH ?1 AND (?2 IS NULL OR c.meeting_id = ?2)
             ORDER BY rank LIMIT ?3",
        )
        .bind(&expr)
        .bind(meeting)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        if !rows.is_empty() {
            return Ok(rows
                .into_iter()
                .map(|r| Hit {
                    chunk_id: r.id,
                    meeting_id: r.meeting_id,
                    kind: r.kind,
                    start_ms: r.start_ms,
                    excerpt: r.excerpt,
                    score: -r.rank, // bm25 is lower-is-better
                    matched_by: vec!["classic"],
                })
                .collect());
        }
    }
    Ok(vec![])
}

#[derive(FromRow)]
struct VectorRow {
    id: i64,
    meeting_id: String,
    kind: String,
    start_ms: Option<i64>,
    text: String,
    embedding: Vec<u8>,
}

pub async fn search_smart(pool: &SqlitePool, query_vec: &[f32], limit: usize, meeting: Option<&str>) -> Result<Vec<Hit>> {
    let rows: Vec<VectorRow> = sqlx::query_as(
        "SELECT id, meeting_id, kind, start_ms, text, embedding FROM search_chunks
         WHERE embedding IS NOT NULL AND (?1 IS NULL OR meeting_id = ?1)",
    )
    .bind(meeting)
    .fetch_all(pool)
    .await?;
    // ponytail: brute-force cosine over every chunk (~1 ms per 10k chunks of 768 floats; fine up
    // to a few hundred meetings). Beyond that, move the vectors to sqlite-vec.
    let mut scored: Vec<(f32, VectorRow)> = rows
        .into_iter()
        .map(|r| (embedding::cosine(query_vec, &embedding::from_blob(&r.embedding)), r))
        .filter(|(s, _)| *s >= MIN_COSINE)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(scored
        .into_iter()
        .take(limit)
        .map(|(s, r)| Hit {
            chunk_id: r.id,
            meeting_id: r.meeting_id,
            kind: r.kind,
            start_ms: r.start_ms,
            excerpt: cut(&r.text, EXCERPT_CHARS),
            score: s as f64,
            matched_by: vec!["smart"],
        })
        .collect())
}

/// Reciprocal rank fusion: a chunk's score is the sum of 1/(60+rank) over the lists it is in.
pub fn fuse(lists: Vec<Vec<Hit>>, limit: usize) -> Vec<Hit> {
    let mut merged: HashMap<i64, Hit> = HashMap::new();
    for list in lists {
        for (rank, mut hit) in list.into_iter().enumerate() {
            let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
            match merged.get_mut(&hit.chunk_id) {
                Some(m) => {
                    m.score += rrf;
                    m.matched_by.extend(&hit.matched_by);
                    if hit.matched_by[0] == "classic" {
                        m.excerpt = hit.excerpt; // the highlighted one
                    }
                }
                None => {
                    hit.score = rrf;
                    merged.insert(hit.chunk_id, hit);
                }
            }
        }
    }
    let mut hits: Vec<Hit> = merged.into_values().collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.chunk_id.cmp(&b.chunk_id)));
    hits.truncate(limit);
    hits
}

/// Replaces the meeting's chunks (keeping embeddings of unchanged text) and records its stamp.
pub async fn reindex_meeting(pool: &SqlitePool, m: &Meeting) -> Result<()> {
    let kept: HashMap<String, Vec<u8>> =
        sqlx::query_as::<_, (String, Vec<u8>)>("SELECT text, embedding FROM search_chunks WHERE meeting_id = ? AND embedding IS NOT NULL")
            .bind(&m.brief.id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM search_chunks WHERE meeting_id = ?").bind(&m.brief.id).execute(&mut *tx).await?;
    for c in chunks(m) {
        sqlx::query("INSERT INTO search_chunks (meeting_id, kind, start_ms, text, embedding) VALUES (?, ?, ?, ?, ?)")
            .bind(&m.brief.id)
            .bind(c.kind)
            .bind(c.start_ms)
            .bind(&c.text)
            .bind(kept.get(&c.text))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO search_index_state (meeting_id, stamp) VALUES (?, ?) ON CONFLICT(meeting_id) DO UPDATE SET stamp = excluded.stamp")
        .bind(&m.brief.id)
        .bind(m.brief.stamp())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Brings the index in line with the meetings: reindexes the changed ones, drops the deleted ones.
/// Returns how many meetings were reindexed.
pub async fn sync_text(pool: &SqlitePool) -> Result<usize> {
    let states: HashMap<String, String> =
        sqlx::query_as::<_, (String, String)>("SELECT meeting_id, stamp FROM search_index_state").fetch_all(pool).await?.into_iter().collect();
    let mut done = 0;
    for b in library::briefs(pool).await? {
        if states.get(&b.id) != Some(&b.stamp()) {
            if let Some(m) = library::load(pool, &b.id).await? {
                reindex_meeting(pool, &m).await?;
                done += 1;
            }
        }
    }
    sqlx::query("DELETE FROM search_chunks WHERE meeting_id NOT IN (SELECT id FROM meetings)").execute(pool).await?;
    sqlx::query("DELETE FROM search_index_state WHERE meeting_id NOT IN (SELECT id FROM meetings)").execute(pool).await?;
    Ok(done)
}

/// Embeds the chunks that have no vector yet. Returns how many.
pub async fn embed_missing(pool: &SqlitePool, model_dir: &Path) -> Result<usize> {
    let mut total = 0;
    loop {
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT id, text FROM search_chunks WHERE embedding IS NULL LIMIT 32").fetch_all(pool).await?;
        if rows.is_empty() {
            return Ok(total);
        }
        let dir = model_dir.to_path_buf();
        let texts: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();
        let vectors = tokio::task::spawn_blocking(move || embedding::with_embedder(&dir, |e| e.embed_documents(&texts))).await??;
        for ((id, _), v) in rows.iter().zip(vectors) {
            sqlx::query("UPDATE search_chunks SET embedding = ? WHERE id = ?").bind(embedding::to_blob(&v)).bind(id).execute(pool).await?;
        }
        total += rows.len();
    }
}

// --- background maintenance -------------------------------------------------------------------

static MODEL_ROOTS: Mutex<Option<(PathBuf, Option<PathBuf>)>> = Mutex::new(None);
static KICK: OnceLock<Notify> = OnceLock::new();

/// The embedding model directory, if installed.
pub fn model_dir() -> Option<PathBuf> {
    let roots = MODEL_ROOTS.lock().unwrap().clone()?;
    embedding::find_model(&roots.0, roots.1.as_deref())
}

pub fn set_model_roots(user: PathBuf, bundle: Option<PathBuf>) {
    *MODEL_ROOTS.lock().unwrap() = Some((user, bundle));
}

/// Asks the maintenance task to resync now (after a meeting is saved, a note edited…).
pub fn kick() {
    KICK.get_or_init(Notify::new).notify_one();
}

/// Keeps the index current: at startup (this also indexes meetings that already exist), when
/// kicked, and every two minutes (summaries and renames are picked up by their stamps).
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    let user = app.path().app_data_dir().map(|d| d.join("models"));
    let bundle = app.path().resource_dir().map(|d| d.join("resources").join("models")).ok();
    if let Ok(user) = user {
        set_model_roots(user, bundle);
    }
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(state) = app.try_state::<crate::state::AppState>() {
                let pool = state.db_manager.pool().clone();
                match sync_text(&pool).await {
                    Ok(n) if n > 0 => log::info!("Search index: {n} meetings reindexed"),
                    Ok(_) => {}
                    Err(e) => log::warn!("Search index sync failed: {e:#}"),
                }
                if let Some(dir) = model_dir() {
                    match embed_missing(&pool, &dir).await {
                        Ok(n) if n > 0 => log::info!("Search index: {n} chunks embedded"),
                        Ok(_) => {}
                        Err(e) => log::warn!("Embedding failed: {e:#}"),
                    }
                }
                embedding::unload_if_idle();
            }
            let wake = KICK.get_or_init(Notify::new);
            // Database not open yet (first launch, import): look again soon.
            let wait = if app.try_state::<crate::state::AppState>().is_some() { SYNC_EVERY } else { Duration::from_secs(5) };
            let _ = tokio::time::timeout(wait, wake.notified()).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::fixtures::*;
    use crate::library::{Brief, Line};
    use crate::notes::Note;

    fn line(start: f64, speaker: &str, text: &str) -> Line {
        Line { start, end: start + 2.0, speaker: (!speaker.is_empty()).then(|| speaker.to_string()), text: text.into() }
    }

    fn meeting_with(lines: Vec<Line>, notes: Vec<Note>, summary: Option<&str>) -> Meeting {
        Meeting {
            brief: Brief {
                id: "m".into(),
                title: "t".into(),
                created_at: Default::default(),
                updated_at: Default::default(),
                duration_secs: 0.0,
                participants: vec![],
                line_count: lines.len() as i64,
                notes_count: notes.len() as i64,
                has_summary: summary.is_some(),
            },
            lines,
            notes,
            summary: summary.map(String::from),
        }
    }

    #[test]
    fn chunks_group_lines_name_speakers_and_keep_the_start() {
        let long = "parola ".repeat(40); // 280 chars
        let m = meeting_with(
            vec![
                line(1.0, "Rossi", "ciao"),
                line(3.0, "Rossi", "a tutti"),
                line(6.0, "Bianchi", "buongiorno"),
                line(9.0, "Bianchi", &long),
                line(12.0, "Bianchi", &long),
                line(20.0, "Verdi", "   "),
                line(30.0, "Verdi", "ultimo"),
            ],
            vec![Note::new("chiedere il budget", 7_500)],
            Some("# Riassunto\n\nSi è parlato di budget.\n\nAzioni: Rossi invia il piano."),
        );
        let c = chunks(&m);
        assert_eq!(c[0].start_ms, Some(1000));
        assert!(c[0].text.starts_with("Rossi: ciao a tutti Bianchi: buongiorno parola"), "{}", c[0].text);
        assert!(c[0].text.chars().count() > 300 && c[0].text.ends_with("parola"), "{}", c[0].text);
        // a new chunk once the first holds ≥ 300 chars; the blank line is skipped
        assert_eq!(c[1].start_ms, Some(12_000));
        assert!(c[1].text.starts_with("Bianchi: parola") && c[1].text.ends_with("Verdi: ultimo"), "{}", c[1].text);
        let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
        assert_eq!(kinds, ["transcript", "transcript", "note", "summary"]);
        assert_eq!((c[2].start_ms, c[2].text.as_str()), (Some(7_500), "chiedere il budget"));
        assert_eq!(c[3].start_ms, None);
        assert!(c[3].text.contains("budget") && c[3].text.contains("Azioni"));
    }

    #[test]
    fn fts_query_handles_phrases_prefixes_and_junk() {
        assert_eq!(fts_query("problemi di performance", false).unwrap(), r#""problemi"* AND "di" AND "performance"*"#);
        assert_eq!(fts_query(r#"il "piano di rilascio" luglio"#, false).unwrap(), r#""il" AND "piano di rilascio" AND "luglio"*"#);
        assert_eq!(fts_query("a OR b", true).unwrap(), r#""a" OR "OR" OR "b""#); // operators are just words
        assert_eq!(fts_query(r#"x" NEAR(y"#, false).unwrap(), r#""x" AND "NEAR y""#); // an unclosed quote is a phrase
        assert_eq!(fts_query("  ?! ", false), None);
    }

    #[tokio::test]
    async fn classic_search_folds_accents_finds_phrases_and_prefixes() {
        let p = pool().await;
        meeting(&p, "m1", "Sprint", "2026-10-23T10:00:00+00:00", &[
            (1.0, "Rossi", "Perché il rilascio slitta a luglio"),
            (30.0, "Bianchi", "La performance del sistema è peggiorata dopo l'aggiornamento"),
        ]).await;
        meeting(&p, "m2", "Daily", "2026-10-24T10:00:00+00:00", &[(5.0, "Verdi", "Pranzo insieme domani, il rilascio non c'entra")]).await;
        sqlx::query("INSERT INTO meeting_note_entries VALUES ('n1', 'm2', 'Ricordare il piano di rilascio', 9000, 'a', 'b')").execute(&p).await.unwrap();
        assert_eq!(sync_text(&p).await.unwrap(), 2);
        assert_eq!(sync_text(&p).await.unwrap(), 0); // nothing changed

        // accents: "perche" finds "Perché"; the excerpt highlights it and keeps the instant
        let h = search_classic(&p, "perche", 10, None).await.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].meeting_id.as_str(), h[0].start_ms, h[0].kind.as_str()), ("m1", Some(1000), "transcript"));
        assert!(h[0].excerpt.contains("**Perché**"), "{}", h[0].excerpt);
        // phrase: only the note has "piano di rilascio" in that order
        let h = search_classic(&p, "\"piano di rilascio\"", 10, None).await.unwrap();
        assert_eq!((h.len(), h[0].kind.as_str(), h[0].start_ms), (1, "note", Some(9000)));
        // prefix, and the meeting filter
        assert_eq!(search_classic(&p, "perf", 10, None).await.unwrap().len(), 1);
        assert_eq!(search_classic(&p, "rilascio", 10, Some("m2")).await.unwrap().len(), 2); // transcript line + note
        // all words must match, else any of them does
        assert_eq!(search_classic(&p, "rilascio luglio", 10, None).await.unwrap()[0].meeting_id, "m1");
        assert_eq!(search_classic(&p, "rilascio xyzzy", 10, None).await.unwrap().len(), 3);
        assert!(search_classic(&p, "xyzzy", 10, None).await.unwrap().is_empty());
        assert!(search_classic(&p, "?!", 10, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn sync_follows_edits_and_deletions_and_keeps_embeddings_of_unchanged_text() {
        let p = pool().await;
        meeting(&p, "m1", "Sprint", "2026-10-23T10:00:00+00:00", &[(1.0, "Rossi", "ciao a tutti")]).await;
        sync_text(&p).await.unwrap();
        sqlx::query("UPDATE search_chunks SET embedding = ?").bind(embedding::to_blob(&[1.0, 0.0])).execute(&p).await.unwrap();

        // a note is added: the transcript chunk keeps its vector, the note's chunk has none
        sqlx::query("INSERT INTO meeting_note_entries VALUES ('n1', 'm1', 'nuova nota', 2000, 'a', '2026-10-30T00:00:00+00:00')").execute(&p).await.unwrap();
        assert_eq!(sync_text(&p).await.unwrap(), 1);
        let missing: Vec<String> = sqlx::query_scalar("SELECT text FROM search_chunks WHERE embedding IS NULL").fetch_all(&p).await.unwrap();
        assert_eq!(missing, ["nuova nota"]);

        // a speaker rename changes the stamp too
        sqlx::query("UPDATE transcripts SET speaker = 'Neri'").execute(&p).await.unwrap();
        assert_eq!(sync_text(&p).await.unwrap(), 1);
        assert_eq!(search_classic(&p, "Neri", 10, None).await.unwrap().len(), 1);

        // deleted meeting: chunks and index rows go (FTS included)
        sqlx::query("DELETE FROM meetings").execute(&p).await.unwrap();
        sync_text(&p).await.unwrap();
        assert!(search_classic(&p, "ciao", 10, None).await.unwrap().is_empty());
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM search_fts").fetch_one(&p).await.unwrap();
        assert_eq!(left, 0);
    }

    /// Hand-made 2-d "embeddings": the ranking and the hybrid fusion on a fixture, no model needed.
    #[tokio::test]
    async fn smart_ranks_by_cosine_and_hybrid_fuses_both_lists() {
        let p = pool().await;
        meeting(&p, "m1", "Sprint", "2026-10-23T10:00:00+00:00", &[
            (1.0, "", "il sistema è lento"),         // semantically the query, shares no word with it
            (10.0, "", "la performance è un tema"),  // shares the word, less close in meaning
            (20.0, "", "pranzo domani"),             // unrelated
        ]).await;
        // 3 transcript lines grouped into one chunk: add one chunk per line by hand for the test
        sqlx::query("DELETE FROM search_chunks").execute(&p).await.unwrap();
        for (ms, text, v) in [(1000, "il sistema è lento", [0.99f32, 0.1]), (10_000, "la performance è un tema", [0.7, 0.714]), (20_000, "pranzo domani", [0.0, 1.0])] {
            sqlx::query("INSERT INTO search_chunks (meeting_id, kind, start_ms, text, embedding) VALUES ('m1', 'transcript', ?, ?, ?)")
                .bind(ms).bind(text).bind(embedding::to_blob(&v)).execute(&p).await.unwrap();
        }
        let query_vec = [1.0f32, 0.0]; // "problemi di performance"
        let smart = search_smart(&p, &query_vec, 10, None).await.unwrap();
        assert_eq!(smart.iter().map(|h| h.start_ms.unwrap()).collect::<Vec<_>>(), [1000, 10_000]); // 0.0 is under the floor
        assert!(smart[0].score > smart[1].score);

        let classic = search_classic(&p, "performance", 10, None).await.unwrap();
        assert_eq!(classic.len(), 1);
        let hybrid = fuse(vec![classic, smart], 10);
        // The chunk found by both lists beats the one found by one list only.
        assert_eq!(hybrid.iter().map(|h| h.start_ms.unwrap()).collect::<Vec<_>>(), [10_000, 1000]);
        assert_eq!(hybrid[0].matched_by, ["classic", "smart"]);
        assert!(hybrid[0].excerpt.contains("**performance**"));
        assert_eq!(hybrid[1].matched_by, ["smart"]);
        assert_eq!(fuse(vec![], 5).len(), 0);
    }
}
