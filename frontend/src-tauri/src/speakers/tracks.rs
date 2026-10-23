//! Two 16 kHz mono tracks (microphone, system) written next to the meeting during recording.
//! Post-call attribution compares their energy and embeds the system track.

use super::wav::WavWriter;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread::JoinHandle;
use tokio::sync::mpsc::UnboundedReceiver;

/// Writer threads by meeting folder, so attribution can wait for finalized files.
static WRITERS: Lazy<Mutex<HashMap<PathBuf, JoinHandle<()>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub const MIC_TRACK: &str = "mic16k.wav";
pub const SYS_TRACK: &str = "sys16k.wav";

/// Mixer windows as (mic, system) at 48 kHz, on the same timeline as the VAD segments.
pub type TrackWindow = (Vec<f32>, Vec<f32>);

/// 48 kHz → 16 kHz by averaging groups of three (the mixer windows are anti-alias-free enough
/// for energy and speaker embeddings); an incomplete tail group is dropped.
pub fn downsample_48k_to_16k(x: &[f32]) -> Vec<f32> {
    x.chunks_exact(3).map(|c| (c[0] + c[1] + c[2]) / 3.0).collect()
}

/// Writes both tracks on a dedicated thread until the pipeline drops the sender.
pub fn run_writer(folder: PathBuf, mut rx: UnboundedReceiver<TrackWindow>) {
    let key = folder.clone();
    let handle = std::thread::spawn(move || {
        let open = |name: &str| WavWriter::create(&folder.join(name), 16000);
        let (mut mic, mut sys) = match (open(MIC_TRACK), open(SYS_TRACK)) {
            (Ok(m), Ok(s)) => (m, s),
            (Err(e), _) | (_, Err(e)) => {
                log::error!("Speaker tracks disabled: {e:#}");
                while rx.blocking_recv().is_some() {}
                return;
            }
        };
        // Carry-over so windows not divisible by 3 keep both tracks aligned.
        let (mut mic_rest, mut sys_rest) = (Vec::new(), Vec::new());
        while let Some((m, s)) = rx.blocking_recv() {
            for (rest, window, writer) in [(&mut mic_rest, m, &mut mic), (&mut sys_rest, s, &mut sys)] {
                rest.extend(window);
                let usable = rest.len() / 3 * 3;
                if let Err(e) = writer.write(&downsample_48k_to_16k(&rest[..usable])) {
                    log::error!("Speaker track write failed: {e:#}");
                }
                rest.drain(..usable);
            }
        }
        for (name, w) in [(MIC_TRACK, mic), (SYS_TRACK, sys)] {
            if let Err(e) = w.finish() {
                log::error!("Speaker track {name} not finalized: {e:#}");
            }
        }
    });
    WRITERS.lock().unwrap().insert(key, handle);
}

/// Blocks until the folder's writer has finalized both files (no-op if none is running).
pub fn wait_finished(folder: &Path) {
    let handle = WRITERS.lock().unwrap().remove(folder);
    if let Some(h) = handle {
        let _ = h.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_by_three() {
        assert_eq!(downsample_48k_to_16k(&[1.0, 1.0, 1.0, 2.0, 2.0, 2.0]), vec![1.0, 2.0]);
        assert_eq!(downsample_48k_to_16k(&[3.0, 0.0, 0.0, 9.0]), vec![1.0]); // tail dropped
    }

    #[test]
    fn writer_keeps_tracks_aligned_and_finalized() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        run_writer(dir.path().to_path_buf(), rx);
        for _ in 0..10 {
            tx.send((vec![0.5; 2401], vec![-0.25; 2401])).unwrap(); // odd size exercises the carry
        }
        drop(tx);
        wait_finished(dir.path());
        let mic = crate::speakers::wav::read_wav_mono_16k(&dir.path().join(MIC_TRACK)).unwrap();
        let sys = crate::speakers::wav::read_wav_mono_16k(&dir.path().join(SYS_TRACK)).unwrap();
        assert_eq!(mic.len(), 24010 / 3);
        assert_eq!(mic.len(), sys.len());
        assert!((mic[0] - 0.5).abs() < 1e-3 && (sys[0] + 0.25).abs() < 1e-3);
    }
}
