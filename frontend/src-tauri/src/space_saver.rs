//! Salva spazio: in the background, once a day, re-encodes the audio of older meetings recorded at
//! a high bitrate down to `AUDIO_BITRATE`, and (only if the user turned it on) moves to the Trash
//! the audio of meetings older than N months. Transcript, summary, notes and speakers stay.

use crate::audio::encode::AUDIO_BITRATE;
use crate::audio::recording_preferences::{get_default_recordings_folder, load_recording_preferences};
use crate::meeting_folder::{move_to_trash, trashable, InUse};
use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};

/// Files above this are re-encoded (the old recordings are ~160 kb/s, new ones 64).
const COMPACT_ABOVE_KBPS: u32 = 100;

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        // Let startup work (models, interrupted meetings) go first.
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
        loop {
            if app.try_state::<crate::state::AppState>().is_some() {
                run_once(&app).await;
            }
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

async fn run_once<R: Runtime>(app: &AppHandle<R>) {
    let Some(ffmpeg) = crate::audio::ffmpeg::find_ffmpeg_path() else { return };
    let prefs = load_recording_preferences(app).await.unwrap_or_default();
    let roots = vec![get_default_recordings_folder(), prefs.save_folder.clone()];
    let cutoff = (prefs.delete_audio_after_months > 0)
        .then(|| chrono::Utc::now() - chrono::Duration::days(30 * prefs.delete_audio_after_months as i64));
    let pool = app.state::<crate::state::AppState>().db_manager.pool().clone();
    let rows: Vec<(String, String)> =
        match sqlx::query_as("SELECT folder_path, created_at FROM meetings WHERE folder_path IS NOT NULL")
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => return log::warn!("Space saver: cannot list meetings: {e}"),
        };

    let (mut freed, mut compacted, mut trashed) = (0u64, 0, 0);
    for (folder, created_at) in rows {
        // A call started: stop here, the rest waits for tomorrow.
        if crate::audio::recording_commands::is_recording().await {
            break;
        }
        let delete = is_older(&created_at, cutoff);
        let (ffmpeg, roots) = (ffmpeg.clone(), roots.clone());
        match tokio::task::spawn_blocking(move || process(&ffmpeg, &folder, &roots, delete)).await {
            Ok(Done::Compacted(bytes)) => (freed, compacted) = (freed + bytes, compacted + 1),
            Ok(Done::Trashed(bytes)) => (freed, trashed) = (freed + bytes, trashed + 1),
            Ok(Done::Skipped) => {}
            Err(e) => log::warn!("Space saver: {e}"),
        }
    }
    if compacted + trashed > 0 {
        // Trashed audio frees its space once the Trash is emptied.
        log::info!(
            "Space saver: {compacted} audio files compacted, {trashed} moved to the Trash, {} MB saved",
            freed / 1_000_000
        );
    }
}

#[derive(Debug, PartialEq)]
enum Done {
    Compacted(u64),
    Trashed(u64),
    Skipped,
}

/// One meeting folder: its audio to the Trash if `delete`, else compacted when worth it.
fn process(ffmpeg: &Path, folder: &str, roots: &[PathBuf], delete: bool) -> Done {
    // Same checks as deleting a meeting: inside the recordings folder, not being worked on.
    let Ok(dir) = trashable(Some(folder), roots, false) else { return Done::Skipped };
    let Ok(audio) = crate::audio::retranscription::find_audio_file(&dir) else { return Done::Skipped };
    let Ok(before) = audio.metadata().map(|m| m.len()) else { return Done::Skipped };
    let _in_use = InUse::start(Path::new(folder));
    // Across processes (a second Dixit on the same folder, e.g. a dev sandbox): whoever holds the
    // lock on this file works on it, the other skips. A late locker finds it already compacted.
    let Ok(lock) = std::fs::File::open(&audio) else { return Done::Skipped };
    if lock.try_lock().is_err() {
        return Done::Skipped;
    }

    if delete {
        return match move_to_trash(&audio) {
            Ok(()) => Done::Trashed(before),
            Err(e) => {
                log::warn!("Space saver: {} not trashed: {e}", audio.display());
                Done::Skipped
            }
        };
    }
    // ponytail: only AAC containers (what Dixit records and imports); a .wav stays as it is.
    if !matches!(audio.extension().and_then(|e| e.to_str()), Some("mp4" | "m4a")) {
        return Done::Skipped;
    }
    match compact(ffmpeg, &audio) {
        Ok(Some(after)) => Done::Compacted(before.saturating_sub(after)),
        Ok(None) => Done::Skipped,
        Err(e) => {
            log::warn!("Space saver: {} not compacted: {e:#}", audio.display());
            Done::Skipped
        }
    }
}

fn is_older(created_at: &str, cutoff: Option<chrono::DateTime<chrono::Utc>>) -> bool {
    let Some(cutoff) = cutoff else { return false };
    chrono::DateTime::parse_from_rfc3339(created_at).is_ok_and(|t| t < cutoff)
}

/// Re-encodes `audio` to `AUDIO_BITRATE` if it is above the threshold. The original is replaced
/// only after the new file decodes to the same duration. Returns the new size, or None if skipped.
fn compact(ffmpeg: &Path, audio: &Path) -> anyhow::Result<Option<u64>> {
    let (duration, kbps) = probe(ffmpeg, audio)?;
    if kbps <= COMPACT_ABOVE_KBPS {
        return Ok(None);
    }
    let ext = audio.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let tmp = audio.with_file_name(format!("audio.compact-{}.{ext}", std::process::id()));
    let out = Command::new(ffmpeg)
        .args(["-y", "-v", "error", "-i"])
        .arg(audio)
        .args(["-vn", "-c:a", "aac", "-b:a", AUDIO_BITRATE, "-profile:a", "aac_low", "-movflags", "+faststart"])
        .arg(&tmp)
        .output()?;
    let check = || -> anyhow::Result<()> {
        if !out.status.success() {
            bail!("ffmpeg: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        let (new_duration, _) = probe(ffmpeg, &tmp)?;
        if (new_duration - duration).abs() > 1.0 {
            bail!("duration changed: {duration:.1}s → {new_duration:.1}s");
        }
        // Every frame must decode: a header can look right over damaged audio.
        let decode = Command::new(ffmpeg).args(["-v", "error", "-i"]).arg(&tmp).args(["-f", "null", "-"]).output()?;
        if !decode.status.success() || !decode.stderr.is_empty() {
            bail!("new file does not decode cleanly: {}", String::from_utf8_lossy(&decode.stderr).lines().next().unwrap_or(""));
        }
        // Keep the recording date in Finder.
        let mtime = audio.metadata()?.modified()?;
        std::fs::File::options().write(true).open(&tmp)?.set_modified(mtime)?;
        Ok(())
    };
    if let Err(e) = check() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, audio)?;
    Ok(Some(audio.metadata()?.len()))
}

/// Duration (s) and bitrate (kb/s) from `ffmpeg -i` (ffprobe is not bundled).
fn probe(ffmpeg: &Path, file: &Path) -> anyhow::Result<(f64, u32)> {
    let out = Command::new(ffmpeg).arg("-hide_banner").arg("-i").arg(file).output()?;
    parse_probe(&String::from_utf8_lossy(&out.stderr)).context("no duration in ffmpeg output")
}

/// Parses `Duration: 00:29:02.43, start: 0.000000, bitrate: 161 kb/s`.
fn parse_probe(stderr: &str) -> Option<(f64, u32)> {
    let line = stderr.lines().find(|l| l.trim_start().starts_with("Duration:"))?;
    let hms = line.split("Duration:").nth(1)?.split(',').next()?.trim();
    let mut secs = 0.0;
    for part in hms.split(':') {
        secs = secs * 60.0 + part.parse::<f64>().ok()?;
    }
    let kbps = line.split("bitrate:").nth(1)?.trim().split(' ').next()?.parse().ok()?;
    Some((secs, kbps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_duration_line() {
        let s = "Input #0, mov,mp4 from 'a.mp4':\n  Duration: 00:29:02.43, start: 0.000000, bitrate: 161 kb/s\n";
        assert_eq!(parse_probe(s), Some((1742.43, 161)));
        assert_eq!(parse_probe("Duration: N/A, bitrate: N/A"), None);
    }

    #[test]
    fn deletes_only_past_the_cutoff() {
        let cutoff = Some("2026-06-01T00:00:00Z".parse().unwrap());
        assert!(is_older("2026-05-01T10:00:00.1+00:00", cutoff));
        assert!(!is_older("2026-07-01T10:00:00+00:00", cutoff));
        assert!(!is_older("not a date", cutoff));
        assert!(!is_older("2020-01-01T00:00:00Z", None));
    }

    /// Whole path on a generated 192k recording: compacted once, then skipped; outside the
    /// recordings folder or while in use, left alone.
    #[test]
    fn compacts_a_meeting_folder_once() {
        let Some(ffmpeg) = crate::audio::ffmpeg::find_ffmpeg_path() else { return };
        let root = tempfile::tempdir().unwrap();
        let meeting = root.path().join("2026-01-01 10.00 Prova");
        std::fs::create_dir(&meeting).unwrap();
        let audio = meeting.join("audio.mp4");
        let ok = Command::new(&ffmpeg)
            .args(["-v", "error", "-f", "lavfi", "-i", "anoisesrc=d=60:a=0.3:r=48000", "-ac", "1"])
            .args(["-c:a", "aac", "-b:a", "192k"])
            .arg(&audio)
            .status()
            .unwrap();
        assert!(ok.success());
        let folder = meeting.to_string_lossy().to_string();
        let roots = [root.path().to_path_buf()];

        {
            let _busy = InUse::start(&meeting);
            assert_eq!(process(&ffmpeg, &folder, &roots, false), Done::Skipped, "in use");
        }
        let elsewhere = [root.path().join("altro")];
        assert_eq!(process(&ffmpeg, &folder, &elsewhere, false), Done::Skipped, "outside");

        assert!(matches!(process(&ffmpeg, &folder, &roots, false), Done::Compacted(n) if n > 0));
        assert_eq!(process(&ffmpeg, &folder, &roots, false), Done::Skipped, "already small");
        let (secs, kbps) = probe(&ffmpeg, &audio).unwrap();
        assert!((secs - 60.0).abs() < 1.0 && kbps < COMPACT_ABOVE_KBPS, "{secs} {kbps}");
        assert_eq!(std::fs::read_dir(&meeting).unwrap().count(), 1, "no temporary file left");

        // Another process holding the file: skipped.
        let other = std::fs::File::open(&audio).unwrap();
        other.lock().unwrap();
        assert_eq!(process(&ffmpeg, &folder, &roots, true), Done::Skipped, "locked elsewhere");
    }

    /// The incident of 26/11/2026: two workers on the same file at once. Exactly one compacts it,
    /// and the result decodes cleanly.
    #[test]
    fn two_workers_never_mix_one_file() {
        let Some(ffmpeg) = crate::audio::ffmpeg::find_ffmpeg_path() else { return };
        let root = tempfile::tempdir().unwrap();
        let meeting = root.path().join("2026-01-01 10.00 Concorrenza");
        std::fs::create_dir(&meeting).unwrap();
        let audio = meeting.join("audio.mp4");
        let ok = Command::new(&ffmpeg)
            .args(["-v", "error", "-f", "lavfi", "-i", "anoisesrc=d=300:a=0.3:r=48000", "-ac", "1"])
            .args(["-c:a", "aac", "-b:a", "192k"])
            .arg(&audio)
            .status()
            .unwrap();
        assert!(ok.success());
        let folder = meeting.to_string_lossy().to_string();
        let roots = vec![root.path().to_path_buf()];
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let (ffmpeg, folder, roots) = (ffmpeg.clone(), folder.clone(), roots.clone());
                std::thread::spawn(move || process(&ffmpeg, &folder, &roots, false))
            })
            .collect();
        let done: Vec<Done> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(done.iter().filter(|d| matches!(d, Done::Compacted(_))).count(), 1, "{done:?}");
        let decode = Command::new(&ffmpeg).args(["-v", "error", "-i"]).arg(&audio).args(["-f", "null", "-"]).output().unwrap();
        assert!(decode.stderr.is_empty(), "{}", String::from_utf8_lossy(&decode.stderr));
    }

    /// Moves a generated file to the real Trash: `cargo test --lib space_saver_trash -- --ignored`
    #[test]
    #[ignore]
    fn space_saver_trash() {
        let root = tempfile::tempdir().unwrap();
        let meeting = root.path().join("2020-01-01 10.00 Prova cestino Dixit");
        std::fs::create_dir(&meeting).unwrap();
        std::fs::write(meeting.join("audio.m4a"), b"x").unwrap();
        let folder = meeting.to_string_lossy().to_string();
        let done = process(Path::new("ffmpeg"), &folder, &[root.path().to_path_buf()], true);
        assert_eq!(done, Done::Trashed(1));
        assert!(!meeting.join("audio.m4a").exists());
        assert!(meeting.join("..").exists());
    }

    /// Real files: `SPACE_SAVER_FILE=/path/copy/audio.mp4 cargo test --lib space_saver_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn space_saver_real() {
        let file = std::path::PathBuf::from(std::env::var("SPACE_SAVER_FILE").unwrap());
        let ffmpeg = crate::audio::ffmpeg::find_ffmpeg_path().unwrap();
        let before = file.metadata().unwrap().len();
        let after = compact(&ffmpeg, &file).unwrap();
        println!("{before} → {after:?} ({:?})", probe(&ffmpeg, &file).unwrap());
        assert!(after.is_some_and(|a| a < before / 2));
        assert_eq!(compact(&ffmpeg, &file).unwrap(), None, "second run skips");
    }
}
