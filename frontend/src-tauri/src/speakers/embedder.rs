//! WeSpeaker ResNet34 speaker embeddings (256-d) through ONNX Runtime.

use super::fbank::{fbank, NUM_BINS};
use anyhow::{anyhow, bail, Context, Result};
use ort::inputs;
use ort::session::Session;
use ort::value::TensorRef;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const MODEL_FILE: &str = "wespeaker_en_voxceleb_resnet34_LM.onnx";
const MODEL_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx";
const MODEL_SHA256: &str = "e9848563da86f263117134dfd7ad63c92355b37de492b55e325400c9d9c39012";

fn sha256_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// Path of the verified model under `<models_dir>/speaker/`, downloading it (26 MB) if missing.
pub async fn ensure_model(models_dir: &Path) -> Result<PathBuf> {
    let dir = models_dir.join("speaker");
    let path = dir.join(MODEL_FILE);
    if path.exists() && sha256_hex(&path)? == MODEL_SHA256 {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir)?;
    log::info!("Downloading speaker model to {}", path.display());
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let bytes = client.get(MODEL_URL).send().await?.error_for_status()?.bytes().await?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != MODEL_SHA256 {
        bail!("speaker model checksum mismatch ({digest})");
    }
    let tmp = dir.join(format!("{MODEL_FILE}.part"));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

pub struct Embedder {
    session: Session,
}

impl Embedder {
    pub fn new(model: &Path) -> Result<Self> {
        let session = Session::builder()?
            .with_intra_threads(2)?
            .commit_from_file(model)
            .with_context(|| format!("loading {}", model.display()))?;
        Ok(Self { session })
    }

    /// L2-normalized 256-d embedding of a 16 kHz mono clip (≥ 25 ms).
    pub fn embed(&mut self, samples_16k: &[f32]) -> Result<Vec<f32>> {
        let feats = fbank(samples_16k);
        if feats.is_empty() {
            bail!("clip too short for an embedding");
        }
        let flat: Vec<f32> = feats.iter().flat_map(|f| f.iter().copied()).collect();
        let arr = ndarray::Array3::from_shape_vec((1, feats.len(), NUM_BINS), flat)?;
        let outputs = self.session.run(inputs!["feats" => TensorRef::from_array_view(arr.view())?])?;
        let embs = outputs
            .get("embs")
            .ok_or_else(|| anyhow!("model has no 'embs' output"))?
            .try_extract_array::<f32>()?;
        let v: Vec<f32> = embs.iter().copied().collect();
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
        Ok(v.into_iter().map(|x| x / norm).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the model: `TRASCRIVI_SPEAKER_MODEL=/path/wespeaker.onnx cargo test --lib embedder -- --ignored`.
    #[test]
    #[ignore]
    fn embedder_same_audio_is_similar() {
        let model = std::env::var("TRASCRIVI_SPEAKER_MODEL").expect("set TRASCRIVI_SPEAKER_MODEL");
        let mut e = Embedder::new(std::path::Path::new(&model)).unwrap();
        let audio: Vec<f32> = (0..48000).map(|i| ((i * 7919 % 1000) as f32 / 1000.0 - 0.5) * 0.2).collect();
        let a = e.embed(&audio).unwrap();
        let b = e.embed(&audio).unwrap();
        assert_eq!(a.len(), 256);
        let cos: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        assert!(cos > 0.99, "cosine {cos}");
    }

    /// `TRASCRIVI_SPEAKER_MODEL=… TRASCRIVI_VOICES_DIR=<dir of <voice>_<n>.wav> cargo test --lib clusters_real_voices -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn clusters_real_voices() {
        let model = std::env::var("TRASCRIVI_SPEAKER_MODEL").expect("set TRASCRIVI_SPEAKER_MODEL");
        let dir = std::env::var("TRASCRIVI_VOICES_DIR").expect("set TRASCRIVI_VOICES_DIR");
        // Alice / Eddy: two clearly different voices (Eddy and Grandpa are the same TTS voice
        // pitch-shifted, ~0.55 apart, which real meetings don't produce).
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| !p.to_string_lossy().contains("Grandpa"))
            .collect();
        files.sort();
        let mut e = Embedder::new(std::path::Path::new(&model)).unwrap();
        let embs: Vec<Vec<f32>> = files
            .iter()
            .map(|f| e.embed(&crate::speakers::wav::read_wav_mono_16k(f).unwrap()).unwrap())
            .collect();
        for (i, a) in embs.iter().enumerate() {
            let row: Vec<String> = embs.iter().map(|b| format!("{:.2}", a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>())).collect();
            println!("{:>14} {}", files[i].file_stem().unwrap().to_string_lossy(), row.join(" "));
        }
        let labels = crate::speakers::cluster::cluster(&embs, crate::speakers::cluster::SAME_SPEAKER_THRESHOLD);
        println!("labels {labels:?}");
        let voice = |i: usize| files[i].file_stem().unwrap().to_string_lossy().split('_').next().unwrap().to_string();
        for i in 0..files.len() {
            for j in 0..files.len() {
                assert_eq!(voice(i) == voice(j), labels[i] == labels[j], "{:?} vs {:?}", files[i], files[j]);
            }
        }
    }
}
