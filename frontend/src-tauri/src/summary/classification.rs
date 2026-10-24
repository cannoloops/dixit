//! Dixit: classifies a call (SAL, analysis, mix, other) to pick the summary template.

use crate::summary::llm_client::{generate_summary, LLMProvider};
use crate::summary::processor::clean_llm_markdown_detailed;
use reqwest::Client;
use serde::Serialize;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

/// Template id that asks the backend to pick the template from the meeting type.
pub const AUTO_TEMPLATE_ID: &str = "auto";

/// Below this many words there is nothing to classify (tests, aborted recordings).
const MIN_WORDS: usize = 40;
/// Longer transcripts are sampled (head + tail) for the short classification pass.
const MAX_CHARS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeetingKind {
    Sal,
    Analisi,
    Mix,
    Altro,
}

impl MeetingKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Sal => "sal",
            Self::Analisi => "analisi",
            Self::Mix => "mix",
            Self::Altro => "altro",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "sal" => Some(Self::Sal),
            "analisi" | "analysis" => Some(Self::Analisi),
            "mix" => Some(Self::Mix),
            "altro" | "other" => Some(Self::Altro),
            _ => None,
        }
    }

    pub fn template_id(self) -> &'static str {
        match self {
            Self::Sal => "sal",
            Self::Analisi => "analisi",
            Self::Mix => "sal_analisi",
            Self::Altro => "standard_meeting",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub kind: MeetingKind,
    pub reason: String,
}

const SYSTEM_PROMPT: &str = r#"You classify work meetings of an Italian software consultancy from their transcript.
Types:
- "sal": status update (Stato Avanzamento Lavori). The call reviews progress of activities or project phases: what is done, in progress or blocked, deadlines, estimates, planning, next steps.
- "analisi": analysis session. The call explores a domain or a problem: how processes work, requirements, business rules, solution design, technical walkthroughs or knowledge transfer.
- "mix": a substantial part is a status update AND another substantial part is analysis of requirements or solutions.
- "altro": anything else (informal chat, tests, sales pitch, interviews) or too little content to tell.
Reply with ONLY a JSON object, no Markdown: {"tipo": "sal|analisi|mix|altro", "motivazione": "one short sentence, in the transcript's language, citing what in the call justifies the type"}"#;

/// Reads the model's answer; anything unreadable or unknown falls back to `altro`.
pub fn parse_classification(raw: &str) -> Classification {
    let cleaned = clean_llm_markdown_detailed(raw).markdown;
    let json = match (cleaned.find('{'), cleaned.rfind('}')) {
        (Some(start), Some(end)) if start < end => &cleaned[start..=end],
        _ => "",
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Classification {
            kind: MeetingKind::Altro,
            reason: "Classificazione non riuscita: risposta del modello non leggibile.".into(),
        };
    };
    let field = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| value.get(*k).and_then(|v| v.as_str()))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let tipo = field(&["tipo", "type"]);
    let reason = field(&["motivazione", "reason"]);
    match MeetingKind::from_id(&tipo) {
        Some(kind) => Classification { kind, reason },
        None => Classification {
            kind: MeetingKind::Altro,
            reason: format!("Tipo «{tipo}» non riconosciuto. {reason}").trim().to_string(),
        },
    }
}

/// Head and tail of long transcripts: the opening sets the agenda, the end the outcome.
fn sample(text: &str) -> String {
    if text.chars().count() <= MAX_CHARS {
        return text.to_string();
    }
    let head: String = text.chars().take(MAX_CHARS * 3 / 5).collect();
    let tail: Vec<char> = text.chars().rev().take(MAX_CHARS * 2 / 5).collect();
    format!("{head}\n[…]\n{}", tail.into_iter().rev().collect::<String>())
}

/// Short LLM pass that picks the meeting type. Err only when the model call itself fails.
#[allow(clippy::too_many_arguments)]
pub async fn classify_meeting(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    text: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<Classification, String> {
    if text.split_whitespace().count() < MIN_WORDS {
        return Ok(Classification {
            kind: MeetingKind::Altro,
            reason: "Trascrizione troppo breve per riconoscere il tipo di call.".into(),
        });
    }
    let user_prompt = format!("<transcript>\n{}\n</transcript>", sample(text));
    let completion = generate_summary(
        client, provider, model_name, api_key, SYSTEM_PROMPT, &user_prompt, ollama_endpoint,
        custom_openai_endpoint, None, None, None, app_data_dir, cancellation_token,
    )
    .await?;
    Ok(parse_classification(&completion.content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_json_is_read() {
        let c = parse_classification(r#"{"tipo": "sal", "motivazione": "Si rivede lo stato delle fasi."}"#);
        assert_eq!(c.kind, MeetingKind::Sal);
        assert_eq!(c.reason, "Si rivede lo stato delle fasi.");
    }

    #[test]
    fn json_inside_fences_and_reasoning_is_read() {
        let raw = "<think>hmm</think>\n```json\n{\"tipo\": \"Mix\", \"motivazione\": \"Entrambi.\"}\n```";
        assert_eq!(parse_classification(raw).kind, MeetingKind::Mix);
        let english = parse_classification(r#"Sure: {"type": "analysis", "reason": "Requirements."}"#);
        assert_eq!(english.kind, MeetingKind::Analisi);
        assert_eq!(english.reason, "Requirements.");
    }

    #[test]
    fn malformed_json_falls_back_to_altro() {
        for raw in ["", "sal", "{tipo: sal", "{\"tipo\": }"] {
            assert_eq!(parse_classification(raw).kind, MeetingKind::Altro, "{raw}");
        }
    }

    #[test]
    fn unknown_type_falls_back_to_altro() {
        let c = parse_classification(r#"{"tipo": "retrospettiva", "motivazione": "x"}"#);
        assert_eq!(c.kind, MeetingKind::Altro);
        assert!(c.reason.contains("retrospettiva"));
    }

    #[test]
    fn each_type_picks_its_template() {
        assert_eq!(MeetingKind::Sal.template_id(), "sal");
        assert_eq!(MeetingKind::Analisi.template_id(), "analisi");
        assert_eq!(MeetingKind::Mix.template_id(), "sal_analisi");
        assert_eq!(MeetingKind::Altro.template_id(), "standard_meeting");
        for kind in [MeetingKind::Sal, MeetingKind::Analisi, MeetingKind::Mix, MeetingKind::Altro] {
            assert_eq!(MeetingKind::from_id(kind.id()), Some(kind));
            crate::summary::templates::get_template(kind.template_id())
                .unwrap_or_else(|e| panic!("{}: {e}", kind.id()));
        }
    }

    #[test]
    fn long_transcripts_are_sampled_from_both_ends() {
        let text = format!("INIZIO {} FINE", "parola ".repeat(10_000));
        let s = sample(&text);
        assert!(s.starts_with("INIZIO") && s.ends_with("FINE"));
        assert!(s.chars().count() <= MAX_CHARS + 5);
    }
}
