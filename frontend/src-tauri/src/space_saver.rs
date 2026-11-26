//! Salva spazio: in the background, once a day, re-encodes the audio of older meetings recorded at
//! a high bitrate down to `AUDIO_BITRATE`, and (only if the user turned it on) moves to the Trash
//! the audio of meetings older than N months. Transcript, summary, notes and speakers stay.

use crate::audio::encode::AUDIO_BITRATE;
use crate::audio::recording_preferences::{get_default_recordings_folder, load_recording_preferences};
use crate::meeting_folder::{move_to_trash, trashable, InUse};
use anyhow::{bail, Context};
use std::path::Path;
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
        // Same checks as deleting a meeting: inside the recordings folder, not being worked on.
        let Ok(dir) = trashable(Some(&folder), &roots, false) else { continue };
        let audio = dir.join("audio.mp4");
        let Ok(before) = audio.metadata().map(|m| m.len()) else { continue };
        let _in_use = InUse::start(Path::new(&folder));

        if is_older(&created_at, cutoff) {
            match move_to_trash(&audio) {
                Ok(()) => {
                    freed += before;
                    trashed += 1;
                }
                Err(e) => log::warn!("Space saver: {} not trashed: {e}", audio.display()),
            }
            continue;
        }
        let (ffmpeg, path) = (ffmpeg.clone(), audio.clone());
        match tokio::task::spawn_blocking(move || compact(&ffmpeg, &path)).await {
            Ok(Ok(Some(after))) => {
                freed += before.saturating_sub(after);
                compacted += 1;
            }
            Ok(Ok(None)) => {}
            Ok(Err(e)) => log::warn!("Space saver: {} not compacted: {e:#}", audio.display()),
            Err(e) => log::warn!("Space saver: {e}"),
        }
    }
    if compacted + trashed > 0 {
        log::info!(
            "Space saver: {compacted} audio files compacted, {trashed} moved to the Trash, {} MB freed",
            freed / 1_000_000
        );
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
    let tmp = audio.with_file_name("audio.compact.mp4");
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
