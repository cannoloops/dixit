//! Local text embeddings for semantic search: EmbeddingGemma-300M (ONNX, multilingual) through the
//! same ONNX Runtime as Parakeet. No LLM involved. The model is looked up in the user data
//! (`models/embedding/<name>`), then in the app bundle (`resources/models/<name>`).
// ponytail: one model. If EmbeddingGemma ever disappoints on real meetings, multilingual-e5-small
// (input_ids/attention_mask, mean pooling, "query: "/"passage: " prefixes) is the planned swap.

use anyhow::{anyhow, Context, Result};
use ort::inputs;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

pub const MODEL_NAME: &str = "embeddinggemma-300m";
const ONNX_FILE: &str = "model_quantized.onnx"; // int8, ~310 MB with its .onnx_data
const QUERY_PREFIX: &str = "task: search result | query: ";
const DOC_PREFIX: &str = "title: none | text: ";
const MAX_TOKENS: usize = 512; // a chunk is ~500 characters
const BATCH: usize = 8;
/// The model is dropped from memory after this long without use.
pub const IDLE_UNLOAD: Duration = Duration::from_secs(600);

/// Directory with the model files, if present: user data first, then the bundle.
pub fn find_model(user_models: &Path, bundle_models: Option<&Path>) -> Option<PathBuf> {
    [Some(user_models.join("embedding").join(MODEL_NAME)), bundle_models.map(|b| b.join(MODEL_NAME))]
        .into_iter()
        .flatten()
        .find(|d| d.join(ONNX_FILE).exists() && d.join("tokenizer.json").exists())
}

pub struct Embedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl Embedder {
    pub fn new(dir: &Path) -> Result<Self> {
        crate::ensure_onnx_runtime_available().map_err(|e| anyhow!("ONNX Runtime unavailable: {e}"))?;
        let session = Session::builder()?
            .with_intra_threads(4)?
            .commit_from_file(dir.join(ONNX_FILE))
            .with_context(|| format!("loading {}", dir.display()))?;
        let mut tokenizer = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| anyhow!("tokenizer: {e}"))?;
        tokenizer.with_padding(Some(PaddingParams::default()));
        tokenizer
            .with_truncation(Some(TruncationParams { max_length: MAX_TOKENS, ..Default::default() }))
            .map_err(|e| anyhow!("tokenizer: {e}"))?;
        Ok(Self { session, tokenizer })
    }

    fn embed(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let enc = self.tokenizer.encode_batch(texts, true).map_err(|e| anyhow!("tokenizer: {e}"))?;
        let (n, len) = (enc.len(), enc[0].get_ids().len());
        let ids = ndarray::Array2::from_shape_vec((n, len), enc.iter().flat_map(|e| e.get_ids().iter().map(|&i| i as i64)).collect())?;
        let mask = ndarray::Array2::from_shape_vec(
            (n, len),
            enc.iter().flat_map(|e| e.get_attention_mask().iter().map(|&i| i as i64)).collect(),
        )?;
        let out = self.session.run(inputs![
            "input_ids" => TensorRef::from_array_view(ids.view())?,
            "attention_mask" => TensorRef::from_array_view(mask.view())?,
        ])?;
        let emb = out
            .get("sentence_embedding")
            .ok_or_else(|| anyhow!("model has no 'sentence_embedding' output"))?
            .try_extract_array::<f32>()?;
        // Already pooled and L2-normalized by the model: one row per text.
        Ok(emb.iter().copied().collect::<Vec<f32>>().chunks(emb.len() / n).map(<[f32]>::to_vec).collect())
    }

    pub fn embed_query(&mut self, q: &str) -> Result<Vec<f32>> {
        Ok(self.embed(vec![format!("{QUERY_PREFIX}{q}")])?.remove(0))
    }

    pub fn embed_documents(&mut self, docs: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(docs.len());
        for batch in docs.chunks(BATCH) {
            out.extend(self.embed(batch.iter().map(|d| format!("{DOC_PREFIX}{d}")).collect())?);
        }
        Ok(out)
    }
}

pub fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Both sides are unit vectors, so cosine similarity is the dot product.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

// The loaded model stays in memory between calls and is dropped after IDLE_UNLOAD.
static RESIDENT: Mutex<Option<(Embedder, Instant)>> = Mutex::new(None);

/// Runs `f` with the embedder, loading it from `model_dir` on first use. Blocking.
pub fn with_embedder<T>(model_dir: &Path, f: impl FnOnce(&mut Embedder) -> Result<T>) -> Result<T> {
    let mut slot = RESIDENT.lock().unwrap();
    if slot.is_none() {
        log::info!("Loading embedding model from {}", model_dir.display());
        *slot = Some((Embedder::new(model_dir)?, Instant::now()));
    }
    let (embedder, used) = slot.as_mut().unwrap();
    *used = Instant::now();
    f(embedder)
}

/// Frees the model if it has been idle long enough.
pub fn unload_if_idle() {
    let mut slot = RESIDENT.lock().unwrap();
    if slot.as_ref().is_some_and(|(_, used)| used.elapsed() > IDLE_UNLOAD) {
        *slot = None;
        log::info!("Embedding model unloaded (idle)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_roundtrip_and_cosine() {
        let v = vec![0.6, -0.8, 0.0];
        assert_eq!(from_blob(&to_blob(&v)), v);
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&v, &[0.0, 0.0, 1.0]), 0.0);
    }

    #[test]
    fn finds_the_model_user_data_first() {
        let user = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        assert_eq!(find_model(user.path(), Some(bundle.path())), None);
        for root in [bundle.path().join(MODEL_NAME), user.path().join("embedding").join(MODEL_NAME)] {
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join(ONNX_FILE), b"x").unwrap();
            std::fs::write(root.join("tokenizer.json"), b"{}").unwrap();
            let found = find_model(user.path(), Some(bundle.path())).unwrap();
            assert!(found.starts_with(if root.starts_with(user.path()) { user.path() } else { bundle.path() }));
        }
    }

    /// Needs the model: `TRASCRIVI_EMBEDDING_MODEL=<dir> cargo test --lib embedding -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn paraphrase_beats_unrelated_text() {
        let dir = std::env::var("TRASCRIVI_EMBEDDING_MODEL").expect("set TRASCRIVI_EMBEDDING_MODEL");
        let mut e = Embedder::new(Path::new(&dir)).unwrap();
        let docs = [
            "Il sistema è lento e ci mette un sacco a caricare le pagine.".to_string(),
            "Domani pranziamo insieme al ristorante vicino all'ufficio.".to_string(),
            "Il budget per il prossimo trimestre è stato approvato dalla direzione.".to_string(),
        ];
        let docs_v = e.embed_documents(&docs).unwrap();
        let q = e.embed_query("problemi di performance").unwrap();
        let scores: Vec<f32> = docs_v.iter().map(|d| cosine(&q, d)).collect();
        println!("dim {} scores {scores:?}", q.len());
        assert_eq!(q.len(), 768);
        assert!(scores[0] > scores[1] + 0.05 && scores[0] > scores[2] + 0.05, "{scores:?}");
    }
}
