//! Dixit: the base models are not in the .app (GitHub release assets stop at 2 GB). At first launch
//! they are downloaded into the user's models dir (`<app_data>/models`), pinned by revision and
//! checked by SHA-256. A copy already in the bundle (dev builds, old full packages) counts as present.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy)]
struct Model {
    rel: &'static str,
    url: &'static str,
    size: u64,
    sha256: &'static str,
}

macro_rules! hf {
    ($repo:literal, $rev:literal, $file:literal) => {
        concat!("https://huggingface.co/", $repo, "/resolve/", $rev, "/", $file)
    };
}

const MODELS: &[Model] = &[
    Model {
        rel: "parakeet/parakeet-tdt-0.6b-v3-int8/encoder-model.int8.onnx",
        url: hf!("istupakov/parakeet-tdt-0.6b-v3-onnx", "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce", "encoder-model.int8.onnx"),
        size: 652_183_999,
        sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
    },
    Model {
        rel: "parakeet/parakeet-tdt-0.6b-v3-int8/decoder_joint-model.int8.onnx",
        url: hf!("istupakov/parakeet-tdt-0.6b-v3-onnx", "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce", "decoder_joint-model.int8.onnx"),
        size: 18_202_004,
        sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
    },
    Model {
        rel: "parakeet/parakeet-tdt-0.6b-v3-int8/nemo128.onnx",
        url: hf!("istupakov/parakeet-tdt-0.6b-v3-onnx", "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce", "nemo128.onnx"),
        size: 139_764,
        sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
    },
    Model {
        rel: "parakeet/parakeet-tdt-0.6b-v3-int8/vocab.txt",
        url: hf!("istupakov/parakeet-tdt-0.6b-v3-onnx", "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce", "vocab.txt"),
        size: 93_939,
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    },
    Model {
        rel: "speaker/wespeaker_en_voxceleb_resnet34_LM.onnx",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx",
        size: 26_530_550,
        sha256: "e9848563da86f263117134dfd7ad63c92355b37de492b55e325400c9d9c39012",
    },
    Model {
        rel: "embedding/embeddinggemma-300m/model_quantized.onnx",
        url: hf!("onnx-community/embeddinggemma-300m-ONNX", "5090578d9565bb06545b4552f76e6bc2c93e4a66", "onnx/model_quantized.onnx"),
        size: 567_874,
        sha256: "172efde319fe1542dc41f31be6154910b05b78f7a861c265c4600eec906bd6d8",
    },
    Model {
        rel: "embedding/embeddinggemma-300m/model_quantized.onnx_data",
        url: hf!("onnx-community/embeddinggemma-300m-ONNX", "5090578d9565bb06545b4552f76e6bc2c93e4a66", "onnx/model_quantized.onnx_data"),
        size: 308_890_624,
        sha256: "705626e28e4c23c82ade34566b4197d97f534c12275fa406dfb71e9937d388c0",
    },
    Model {
        rel: "embedding/embeddinggemma-300m/tokenizer.json",
        url: hf!("onnx-community/embeddinggemma-300m-ONNX", "5090578d9565bb06545b4552f76e6bc2c93e4a66", "tokenizer.json"),
        size: 20_323_312,
        sha256: "4dda02faaf32bc91031dc8c88457ac272b00c1016cc679757d1c441b248b9c47",
    },
    Model {
        rel: "ggml-large-v3.bin",
        url: hf!("ggerganov/whisper.cpp", "5359861c739e955e79d9a303bcbc70fb988958b1", "ggml-large-v3.bin"),
        size: 3_095_033_483,
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
    },
    Model {
        rel: "summary/gemma-4-E4B-it-Q4_K_M.gguf",
        url: hf!("unsloth/gemma-4-E4B-it-GGUF", "bfc15c382204943c3a8fff0c750b94ae2364d7a3", "gemma-4-E4B-it-Q4_K_M.gguf"),
        size: 4_977_171_584,
        sha256: "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87",
    },
];

static DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    downloaded_bytes: u64,
    total_bytes: u64,
}

fn models_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|d| d.join("models")).map_err(|e| e.to_string())
}

/// Present = the right size in the user dir, or in the bundle at the same place (the embedding
/// model sits at the bundle root, so look there too).
fn present(root: &Path, m: &Model) -> bool {
    let user = root.join(m.rel);
    let bundled = crate::bundled_models::resolve(&user);
    let legacy_bundle = m
        .rel
        .strip_prefix("embedding/")
        .map(|rel| crate::bundled_models::resolve(&root.join(rel)));
    [Some(bundled), legacy_bundle]
        .into_iter()
        .flatten()
        .any(|p| p.metadata().is_ok_and(|md| md.len() == m.size))
}

fn missing(root: &Path) -> Vec<&'static Model> {
    MODELS.iter().filter(|m| !present(root, m)).collect()
}

/// Bytes still to download (0 = ready).
#[tauri::command]
pub fn base_models_missing_bytes<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<u64, String> {
    let root = models_root(&app)?;
    Ok(missing(&root).iter().map(|m| m.size).sum())
}

/// Downloads every missing model, resuming `.part` files. Emits `base-models-progress`.
#[tauri::command]
pub async fn base_models_download<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("Download already in progress".into());
    }
    let result = download_all(&app).await.map_err(|e| format!("{e:#}"));
    DOWNLOADING.store(false, Ordering::SeqCst);
    result
}

async fn download_all<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> anyhow::Result<()> {
    let root = models_root(app).map_err(anyhow::Error::msg)?;
    let todo = missing(&root);
    let total: u64 = todo.iter().map(|m| m.size).sum();
    let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(30)).build()?;
    let mut done = 0u64;
    let mut last_emit = Instant::now();
    for m in todo {
        log::info!("Downloading base model {}", m.rel);
        download_one(&client, &root.join(m.rel), m, |n| {
            if last_emit.elapsed() >= Duration::from_millis(250) {
                last_emit = Instant::now();
                let _ = app.emit("base-models-progress", Progress { downloaded_bytes: done + n, total_bytes: total });
            }
        })
        .await?;
        done += m.size;
    }
    let _ = app.emit("base-models-progress", Progress { downloaded_bytes: total, total_bytes: total });
    Ok(())
}

async fn download_one(client: &reqwest::Client, dest: &Path, m: &Model, mut on_bytes: impl FnMut(u64)) -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    tokio::fs::create_dir_all(dest.parent().context("no parent")?).await?;
    let part = dest.with_file_name(format!("{}.part", dest.file_name().context("no file name")?.to_string_lossy()));

    // Resume: hash what is already there, then ask for the rest.
    let mut hasher = Sha256::new();
    let mut have = 0u64;
    if let Ok(mut f) = tokio::fs::File::open(&part).await {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            have += n as u64;
        }
    }
    if have >= m.size {
        hasher = Sha256::new();
        have = 0;
    }
    let mut req = client.get(m.url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await?.error_for_status()?;
    let mut file = if have > 0 && resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        tokio::fs::OpenOptions::new().append(true).open(&part).await?
    } else {
        hasher = Sha256::new();
        have = 0;
        tokio::fs::File::create(&part).await?
    };

    let mut resp = resp;
    while let Some(chunk) = resp.chunk().await? {
        have += chunk.len() as u64;
        if have > m.size {
            bail!("{}: more data than expected", m.rel);
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        on_bytes(have);
    }
    file.flush().await?;
    drop(file);

    let digest = format!("{:x}", hasher.finalize());
    if have != m.size || digest != m.sha256 {
        let _ = tokio::fs::remove_file(&part).await;
        bail!("{}: download corrupted (size {have}, sha256 {digest}), try again", m.rel);
    }
    tokio::fs::rename(&part, dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_are_unique_and_pinned() {
        let mut rels: Vec<_> = MODELS.iter().map(|m| m.rel).collect();
        rels.sort();
        rels.dedup();
        assert_eq!(rels.len(), MODELS.len());
        for m in MODELS {
            assert_eq!(m.sha256.len(), 64, "{}", m.rel);
            assert!(!m.url.contains("/resolve/main/"), "{} not pinned", m.rel);
            assert!(m.url.starts_with("https://huggingface.co/") || m.url.starts_with("https://github.com/"), "{}", m.rel);
        }
    }

    /// Network: real download of the small models, resuming a partial file, then a corrupted one.
    /// `cargo test --lib base_models_download_real -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn base_models_download_real() {
        let tmp = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let speaker = MODELS.iter().find(|m| m.rel.starts_with("speaker/")).unwrap();
        let dest = tmp.path().join(speaker.rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();

        // Resume from the first MB of the real file.
        let mut first = reqwest::get(speaker.url).await.unwrap().bytes().await.unwrap().to_vec();
        first.truncate(1 << 20);
        std::fs::write(dest.with_file_name(format!("{}.part", dest.file_name().unwrap().to_string_lossy())), &first).unwrap();
        let mut calls = 0;
        download_one(&client, &dest, speaker, |_| calls += 1).await.unwrap();
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), speaker.size);
        assert!(calls > 0);

        for m in MODELS.iter().filter(|m| m.size < 1_000_000) {
            download_one(&client, &tmp.path().join(m.rel), m, |_| {}).await.unwrap();
        }

        // A wrong hash is refused and leaves nothing behind.
        let bad = Model { sha256: "00", ..*MODELS.iter().find(|m| m.rel.ends_with("vocab.txt")).unwrap() };
        let out = tmp.path().join("bad.txt");
        assert!(download_one(&client, &out, &bad, |_| {}).await.is_err());
        assert!(!out.exists() && !tmp.path().join("bad.txt.part").exists());
    }
}
