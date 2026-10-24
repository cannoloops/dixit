//! Dixit: classification and summary of every meeting in a copy of the DB, with the local LLM.
//!
//! DIXIT_DB_COPY=/path/copy.sqlite DIXIT_OUT=/path/out \
//!   cargo test --lib -p meetily verifica_riassunti -- --ignored --nocapture
//! Optional: DIXIT_MODEL (default gemma4:latest, via Ollama).

use crate::summary::auto::transcript_text;
use crate::summary::classification::classify_meeting;
use crate::summary::llm_client::LLMProvider;
use crate::summary::processor::generate_meeting_summary;
use crate::summary::templates;
use sqlx::SqlitePool;

/// Numbers in the summary that never appear in the transcript: candidates for invented facts.
fn numbers_not_in(summary: &str, transcript: &str) -> Vec<String> {
    let numbers = |s: &str| {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let known = numbers(transcript);
    let mut missing: Vec<String> = numbers(summary).into_iter().filter(|n| !known.contains(n)).collect();
    missing.dedup();
    missing
}

#[tokio::test]
#[ignore]
async fn verifica_riassunti() {
    let Ok(db) = std::env::var("DIXIT_DB_COPY") else {
        return eprintln!("DIXIT_DB_COPY not set, skipping");
    };
    let out = std::path::PathBuf::from(std::env::var("DIXIT_OUT").expect("DIXIT_OUT"));
    std::fs::create_dir_all(&out).unwrap();
    let model = std::env::var("DIXIT_MODEL").unwrap_or_else(|_| "gemma4:latest".into());
    let pool = SqlitePool::connect(&format!("sqlite://{db}?mode=ro")).await.unwrap();
    let meetings: Vec<(String, String)> =
        sqlx::query_as("SELECT id, title FROM meetings ORDER BY created_at").fetch_all(&pool).await.unwrap();
    let client = reqwest::Client::new();
    let provider = LLMProvider::Ollama;
    let mut report = Vec::new();

    for (id, title) in meetings {
        let text = transcript_text(&pool, &id).await.unwrap();
        let class = classify_meeting(&client, &provider, &model, "", &text, None, None, None, None)
            .await
            .unwrap();
        let template = templates::get_template(class.kind.template_id()).unwrap();
        let started = std::time::Instant::now();
        let summary = generate_meeting_summary(
            &client, &provider, &model, "", &text, "", "", class.kind.template_id(), &template,
            130_000, None, None, None, None, None, None, None, Some("it"), Some("it"), None,
        )
        .await
        .unwrap();
        let missing_sections: Vec<&str> = template
            .sections
            .iter()
            .map(|s| s.title.as_str())
            .filter(|t| !summary.english_markdown.contains(t))
            .collect();
        let suspicious = numbers_not_in(&summary.final_markdown, &text);
        eprintln!("{id} {title}: {} ({:?}) missing sections {missing_sections:?}", class.kind.id(), class.reason);
        std::fs::write(out.join(format!("{id}.md")), &summary.final_markdown).unwrap();
        std::fs::write(out.join(format!("{id}.en.md")), &summary.english_markdown).unwrap();
        std::fs::write(out.join(format!("{id}.transcript.txt")), &text).unwrap();
        report.push(serde_json::json!({
            "id": id, "title": title, "chars": text.chars().count(), "type": class.kind.id(),
            "reason": class.reason, "template": class.kind.template_id(),
            "missing_sections": missing_sections, "numbers_not_in_transcript": suspicious,
            "seconds": started.elapsed().as_secs(),
        }));
        assert!(missing_sections.is_empty(), "{id}: missing sections {missing_sections:?}");
    }
    std::fs::write(out.join("report.json"), serde_json::to_string_pretty(&report).unwrap()).unwrap();
}

#[test]
fn numbers_not_in_transcript_are_flagged() {
    assert_eq!(numbers_not_in("Stima 3350 giorni, 12 persone", "secondo me 3350 ci potrebbe stare"), vec!["12"]);
}
