//! Dixit: summary model comparison on real meetings (docs/confronto-modelli-riassunto.md).
//! Runs the app's own pipeline (`generate_meeting_summary`, template `standard_meeting`, Italian).
//!
//! BENCH_DB=<copy of meeting_minutes.sqlite> BENCH_APP_DATA=<dir with models/summary/*.gguf>
//! BENCH_PROVIDER=builtin-ai BENCH_MODEL=gemma4:e4b BENCH_MEETINGS=id1,id2 BENCH_OUT=<dir>
//! MEETILY_LLAMA_HELPER=<llama-helper> cargo test --lib bench_summary_model -- --ignored --nocapture

use crate::summary::llm_client::LLMProvider;
use crate::summary::processor::generate_meeting_summary;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::test]
#[ignore]
async fn bench_summary_model() {
    let env = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{k} not set"));
    let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=ro", env("BENCH_DB"))).await.unwrap();
    let provider = LLMProvider::from_str(&env("BENCH_PROVIDER")).unwrap();
    let model = env("BENCH_MODEL");
    let app_data = PathBuf::from(env("BENCH_APP_DATA"));
    let out = PathBuf::from(env("BENCH_OUT"));
    std::fs::create_dir_all(&out).unwrap();
    let template = crate::summary::templates::get_template("standard_meeting").unwrap();
    let client = reqwest::Client::new();

    for meeting in env("BENCH_MEETINGS").split(',') {
        // Same text the frontend sends: "[mm:ss] text" per segment.
        let rows: Vec<(String, Option<f64>)> = sqlx::query_as(
            "SELECT transcript, audio_start_time FROM transcripts WHERE meeting_id = ? ORDER BY audio_start_time",
        )
        .bind(meeting)
        .fetch_all(&pool)
        .await
        .unwrap();
        let text: String = rows
            .iter()
            .map(|(t, s)| {
                let s = s.unwrap_or(0.0) as u64;
                format!("[{:02}:{:02}] {t}", s / 60, s % 60)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let start = std::time::Instant::now();
        let result = generate_meeting_summary(
            &client, &provider, &model, "", &text, "", "", "standard_meeting", &template, 32768 - 300,
            None, None, None, None, None, Some(&app_data), None, Some("it"), Some("it"), None,
        )
        .await;
        let secs = start.elapsed().as_secs_f64();
        let body = match result {
            Ok(r) => r.final_markdown,
            Err(e) => format!("ERRORE: {e}"),
        };
        let file = out.join(format!("{}__{}.md", model.replace([':', '/'], "_"), meeting));
        std::fs::write(&file, format!("<!-- {model} | {meeting} | {secs:.1}s | {} chars in -->\n\n{body}\n", text.len())).unwrap();
        println!("BENCH {model} {meeting} {secs:.1}s -> {}", file.display());
    }
    let _ = crate::summary::summary_engine::shutdown_sidecar_gracefully().await;
}
