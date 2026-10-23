//! Post-call attribution: who said each transcript segment, persisted to transcripts.json,
//! the database and a Teams-style .docx.

use super::cluster::{cluster, SAME_SPEAKER_THRESHOLD};
use super::embedder::Embedder;
use super::naming::{assign_names, classify_source, SegmentInfo, Source, Turn};
pub use super::tracks::{MIC_TRACK, SYS_TRACK};
use super::transcript_io::{parse_teams_docx, parse_vtt, write_teams_docx};
use super::wav::WavReader;
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const DIARIZATION_FILE: &str = "diarization.json";
/// Manual renames: `{"renames": {"Parlante A": "Rossi Mario"}}`.
pub const SPEAKERS_FILE: &str = "speakers.json";
pub const NATIVE_DOCX: &str = "teams-transcript.docx";
pub const NATIVE_VTT: &str = "teams-transcript.vtt";
/// Segments shorter than this borrow the cluster of the nearest embedded segment.
const MIN_EMBED_SECS: f64 = 1.0;

pub struct Outcome {
    pub title: String,
    /// (audio_start_time, speaker) per segment, in transcript order.
    pub speakers: Vec<(f64, String)>,
    pub docx: PathBuf,
}

struct Segment {
    seq: u64,
    start: f64,
    end: f64,
    text: String,
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn rms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

fn read_secs(track: &mut WavReader, start: f64, end: f64) -> Vec<f32> {
    let (a, b) = ((start.max(0.0) * 16000.0) as usize, (end.max(0.0) * 16000.0) as usize);
    track.read_range(a, b).unwrap_or_default()
}

/// Me/others by track energy, clusters for others (when an embedder is available).
/// Reads only each segment's slice: a 3-hour meeting's tracks would not fit comfortably in RAM.
/// Returns the segment infos and whether they are final (voices clustered, or nothing to cluster).
fn diarize(folder: &Path, segs: &[Segment], embedder: Option<&mut Embedder>) -> Result<(Vec<SegmentInfo>, bool)> {
    let mut mic = WavReader::open(&folder.join(MIC_TRACK))?;
    let mut sys = WavReader::open(&folder.join(SYS_TRACK))?;
    let mut infos: Vec<SegmentInfo> = segs
        .iter()
        .map(|s| SegmentInfo {
            seq: s.seq,
            start: s.start,
            end: s.end,
            source: classify_source(rms(&read_secs(&mut mic, s.start, s.end)), rms(&read_secs(&mut sys, s.start, s.end))),
            cluster: None,
        })
        .collect();
    let needs_embedding = infos.iter().any(|i| i.source == Source::Others && i.end - i.start >= MIN_EMBED_SECS);
    let complete = embedder.is_some() || !needs_embedding;
    if let Some(embedder) = embedder {
        let mut embedded = Vec::new();
        let mut embs = Vec::new();
        for (i, info) in infos.iter().enumerate() {
            if info.source == Source::Others && info.end - info.start >= MIN_EMBED_SECS {
                match embedder.embed(&read_secs(&mut sys, info.start, info.end)) {
                    Ok(e) => {
                        embedded.push(i);
                        embs.push(e);
                    }
                    Err(e) => log::warn!("No embedding for segment {}: {e:#}", info.seq),
                }
            }
        }
        for (i, label) in embedded.iter().zip(cluster(&embs, SAME_SPEAKER_THRESHOLD)) {
            infos[*i].cluster = Some(label);
        }
        let anchors: Vec<(f64, u32)> = infos.iter().filter_map(|s| s.cluster.map(|c| ((s.start + s.end) / 2.0, c))).collect();
        for info in infos.iter_mut().filter(|s| s.source == Source::Others && s.cluster.is_none()) {
            let mid = (info.start + info.end) / 2.0;
            info.cluster = anchors
                .iter()
                .min_by(|a, b| (a.0 - mid).abs().total_cmp(&(b.0 - mid).abs()))
                .map(|a| a.1);
        }
    }
    Ok((infos, complete))
}

fn words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .map(str::to_string)
        .collect()
}

/// Shift that aligns a Teams transcript to the recording: median of (segment start − turn start)
/// over turns whose wording matches a transcribed segment. None without enough evidence.
pub fn estimate_offset(native: &[Turn], segs: &[(f64, &str)]) -> Option<f64> {
    const MIN_SHARED_WORDS: usize = 3;
    const MIN_MATCHES: usize = 3;
    let seg_words: Vec<(f64, HashSet<String>)> = segs.iter().map(|(start, text)| (*start, words(text))).collect();
    let mut deltas: Vec<f64> = native
        .iter()
        .filter_map(|turn| {
            let w = words(&turn.text);
            seg_words
                .iter()
                .map(|(start, sw)| (*start, w.intersection(sw).count()))
                .filter(|(_, shared)| *shared >= MIN_SHARED_WORDS)
                .max_by_key(|(_, shared)| *shared)
                .map(|(start, _)| start - turn.start)
        })
        .collect();
    if deltas.len() < MIN_MATCHES {
        return None;
    }
    deltas.sort_by(f64::total_cmp);
    Some(deltas[deltas.len() / 2])
}

/// Native Teams turns on the recording timeline. Offset: the one Trascrivi recorded when it
/// started the transcription, else estimated from the text; when it can't be aligned and captions
/// exist, the native transcript is ignored (a wrong shift would rename the whole meeting).
fn native_turns(folder: &Path, segs: &[Segment], have_captions: bool) -> Vec<Turn> {
    let turns = if folder.join(NATIVE_DOCX).exists() {
        parse_teams_docx(&folder.join(NATIVE_DOCX)).map(|(_, t)| t).unwrap_or_else(|e| {
            log::warn!("Unreadable Teams transcript: {e:#}");
            Vec::new()
        })
    } else {
        std::fs::read_to_string(folder.join(NATIVE_VTT)).map(|t| parse_vtt(&t)).unwrap_or_default()
    };
    if turns.is_empty() {
        return turns;
    }
    let texts: Vec<(f64, &str)> = segs.iter().map(|s| (s.start, s.text.as_str())).collect();
    let offset = match crate::teams::native_offset(folder).or_else(|| estimate_offset(&turns, &texts)) {
        Some(offset) => offset,
        None if have_captions => {
            log::info!("Teams transcript not aligned to the recording; using live captions instead");
            return Vec::new();
        }
        None => 0.0,
    };
    turns.into_iter().map(|t| Turn { start: t.start + offset, end: t.end + offset, ..t }).collect()
}

/// Blocking: attributes every segment of the meeting in `folder` and writes the results.
/// Builds `diarization.json` from the tracks once (then deletes them); reruns reuse it.
pub fn attribute_folder(folder: &Path, me: &str, embedder: Option<&mut Embedder>) -> Result<Outcome> {
    let transcripts_path = folder.join("transcripts.json");
    let mut transcripts = read_json(&transcripts_path).context("transcripts.json missing or invalid")?;
    let segs: Vec<Segment> = transcripts["segments"]
        .as_array()
        .context("transcripts.json has no segments")?
        .iter()
        .map(|s| Segment {
            seq: s["sequence_id"].as_u64().unwrap_or(0),
            start: s["audio_start_time"].as_f64().unwrap_or(0.0),
            end: s["audio_end_time"].as_f64().unwrap_or(0.0),
            text: s["text"].as_str().unwrap_or_default().to_string(),
        })
        .collect();

    let diarization_path = folder.join(DIARIZATION_FILE);
    let saved: Option<Vec<SegmentInfo>> =
        read_json(&diarization_path).and_then(|v| serde_json::from_value(v["segments"].clone()).ok());
    let infos = match saved {
        Some(saved) => {
            let by_seq: HashMap<u64, SegmentInfo> = saved.into_iter().map(|s| (s.seq, s)).collect();
            segs.iter()
                .map(|s| by_seq.get(&s.seq).cloned().unwrap_or(SegmentInfo { seq: s.seq, start: s.start, end: s.end, source: Source::Others, cluster: None }))
                .collect()
        }
        None => match diarize(folder, &segs, embedder) {
            Ok((infos, complete)) => {
                // Persist and drop the tracks only once voices are clustered; otherwise keep them
                // so a later rerun (with the speaker model available) can still cluster.
                if complete {
                    std::fs::write(&diarization_path, serde_json::to_string_pretty(&serde_json::json!({ "segments": infos }))?)?;
                    for track in [MIC_TRACK, SYS_TRACK] {
                        let _ = std::fs::remove_file(folder.join(track));
                    }
                }
                infos
            }
            Err(e) => {
                log::warn!("No speaker tracks for {} ({e:#}); names from Teams only", folder.display());
                segs.iter()
                    .map(|s| SegmentInfo { seq: s.seq, start: s.start, end: s.end, source: Source::Others, cluster: None })
                    .collect()
            }
        },
    };

    let renames: HashMap<String, String> = read_json(&folder.join(SPEAKERS_FILE))
        .and_then(|v| serde_json::from_value(v["renames"].clone()).ok())
        .unwrap_or_default();
    let captions = crate::teams::read_captions(folder);
    let native = native_turns(folder, &segs, !captions.is_empty());
    let names = assign_names(&infos, &native, &captions, &renames, me);

    if let Some(arr) = transcripts["segments"].as_array_mut() {
        for (seg, name) in arr.iter_mut().zip(&names) {
            seg["speaker"] = serde_json::Value::String(name.clone());
        }
    }
    let tmp = folder.join(".transcripts.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&transcripts)?)?;
    std::fs::rename(&tmp, &transcripts_path)?;

    let metadata = read_json(&folder.join("metadata.json")).unwrap_or_default();
    let title = metadata["meeting_name"].as_str().unwrap_or("Riunione").to_string();
    let started = metadata["created_at"]
        .as_str()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Local))
        .unwrap_or_else(Local::now);
    let duration = metadata["duration_seconds"].as_f64().unwrap_or_else(|| segs.last().map_or(0.0, |s| s.end));
    let lines: Vec<(String, f64, String)> =
        segs.iter().zip(&names).map(|(s, n)| (n.clone(), s.start, s.text.clone())).collect();
    let docx = folder.join(format!("{}-Trascrizione della riunione.docx", super::sanitize_title_for_file(&title)));
    write_teams_docx(&docx, &title, started, duration, &lines)?;

    Ok(Outcome {
        title,
        speakers: segs.iter().zip(names).map(|(s, n)| (s.start, n)).collect(),
        docx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speakers::wav::WavWriter;

    fn tone(freq: f32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(16000.0 * secs) as usize)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / 16000.0).sin())
            .collect()
    }

    fn write_track(path: &Path, samples: &[f32]) {
        let mut w = WavWriter::create(path, 16000).unwrap();
        w.write(samples).unwrap();
        w.finish().unwrap();
    }

    /// Meeting folder with 3 segments: 0–2 s me (mic), 3–5 s and 6–8 s remote (system).
    fn fixture(dir: &Path) {
        let seg = |seq: u64, start: f64, end: f64, text: &str| {
            serde_json::json!({"id": format!("seg_{seq}"), "text": text, "audio_start_time": start, "audio_end_time": end,
                "duration": end - start, "display_time": "[00:00]", "confidence": 0.9, "sequence_id": seq})
        };
        let transcripts = serde_json::json!({"version": "1.0", "segments": [
            seg(1, 0.0, 2.0, "buongiorno"), seg(2, 3.0, 5.0, "il deploy è pronto"), seg(3, 6.0, 8.0, "confermo")]});
        std::fs::write(dir.join("transcripts.json"), transcripts.to_string()).unwrap();
        let metadata = serde_json::json!({"version": "1.0", "meeting_id": null, "meeting_name": "Sprint: review",
            "created_at": "2026-10-23T08:02:15+00:00", "completed_at": null, "duration_seconds": 9.0,
            "devices": {"microphone": null, "system_audio": null}, "audio_file": "", "transcript_file": "transcripts.json",
            "sample_rate": 48000, "status": "completed", "platform": "teams"});
        std::fs::write(dir.join("metadata.json"), metadata.to_string()).unwrap();
        let mut mic = vec![0.0; 16000 * 9];
        mic[..32000].copy_from_slice(&tone(220.0, 2.0, 0.3));
        let mut sys = vec![0.0; 16000 * 9];
        sys[48000..80000].copy_from_slice(&tone(330.0, 2.0, 0.3));
        sys[96000..128000].copy_from_slice(&tone(500.0, 2.0, 0.3));
        write_track(&dir.join(MIC_TRACK), &mic);
        write_track(&dir.join(SYS_TRACK), &sys);
    }

    #[test]
    fn attributes_folder_with_captions() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        let captions = serde_json::json!([
            {"name": "Rossi Mario", "text": "il deploy", "start": 2.8, "end": 5.2},
            {"name": "Bianchi Anna", "text": "confermo", "start": 5.9, "end": 8.1}]);
        std::fs::write(dir.path().join(crate::teams::CAPTIONS_FILE), captions.to_string()).unwrap();

        let out = attribute_folder(dir.path(), "Manuguerra Vito", None).unwrap();

        assert_eq!(out.speakers, vec![(0.0, "Manuguerra Vito".into()), (3.0, "Rossi Mario".into()), (6.0, "Bianchi Anna".into())]);
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("transcripts.json")).unwrap()).unwrap();
        assert_eq!(saved["segments"][1]["speaker"], "Rossi Mario");
        // No embedder: voices not clustered yet, so the tracks stay for a later rerun.
        assert!(!dir.path().join(DIARIZATION_FILE).exists());
        assert!(dir.path().join(MIC_TRACK).exists() && dir.path().join(SYS_TRACK).exists());
        assert_eq!(out.docx.file_name().unwrap(), "Sprint- review-Trascrizione della riunione.docx");
        let (_, turns) = crate::speakers::transcript_io::parse_teams_docx(&out.docx).unwrap();
        let names: Vec<_> = turns.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["Manuguerra Vito", "Rossi Mario", "Bianchi Anna"]);
    }

    #[test]
    fn rerun_applies_renames() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        attribute_folder(dir.path(), "Manuguerra Vito", None).unwrap();
        std::fs::write(dir.path().join(SPEAKERS_FILE), r#"{"renames": {"Altri": "Ospiti"}}"#).unwrap();
        let out = attribute_folder(dir.path(), "Manuguerra Vito", None).unwrap();
        assert_eq!(out.speakers[0].1, "Manuguerra Vito");
        assert_eq!(out.speakers[1].1, "Ospiti");
    }

    #[test]
    fn native_transcript_is_shifted_by_offset() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        std::fs::write(dir.path().join(crate::teams::SESSION_FILE), r#"{"native_transcription_offset": 2.0}"#).unwrap();
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\n<v Verdi Luca>il deploy</v>\n";
        std::fs::write(dir.path().join("teams-transcript.vtt"), vtt).unwrap();
        let out = attribute_folder(dir.path(), "Manuguerra Vito", None).unwrap();
        assert_eq!(out.speakers[1].1, "Verdi Luca"); // 1–3 s + 2 s offset covers 3–5 s
    }

    /// Two synthetic voices on the system track, no captions: distinct "Parlante" labels.
    /// `TRASCRIVI_SPEAKER_MODEL=… TRASCRIVI_VOICES_DIR=… cargo test --lib clusters_voices_in_folder -- --ignored`
    #[test]
    #[ignore]
    fn clusters_voices_in_folder() {
        let model = std::env::var("TRASCRIVI_SPEAKER_MODEL").unwrap();
        let voices = std::path::PathBuf::from(std::env::var("TRASCRIVI_VOICES_DIR").unwrap());
        let clips: Vec<Vec<f32>> = ["Alice_1", "Grandpa_7", "Alice_2", "Grandpa_8"]
            .iter()
            .map(|n| crate::speakers::wav::read_wav_mono_16k(&voices.join(format!("{n}.wav"))).unwrap())
            .collect();
        let dir = tempfile::tempdir().unwrap();
        let mut sys = Vec::new();
        let mut segs = Vec::new();
        for (i, c) in clips.iter().enumerate() {
            let start = sys.len() as f64 / 16000.0;
            sys.extend_from_slice(c);
            segs.push(serde_json::json!({"id": format!("seg_{i}"), "text": "x", "audio_start_time": start,
                "audio_end_time": sys.len() as f64 / 16000.0, "duration": 1.0, "display_time": "", "confidence": 1.0, "sequence_id": i}));
            sys.extend(std::iter::repeat(0.0).take(8000));
        }
        std::fs::write(dir.path().join("transcripts.json"), serde_json::json!({"segments": segs}).to_string()).unwrap();
        write_track(&dir.path().join(MIC_TRACK), &vec![0.0; sys.len()]);
        write_track(&dir.path().join(SYS_TRACK), &sys);
        let mut e = crate::speakers::embedder::Embedder::new(Path::new(&model)).unwrap();
        let out = attribute_folder(dir.path(), "Io", Some(&mut e)).unwrap();
        let labels: Vec<_> = out.speakers.iter().map(|s| s.1.as_str()).collect();
        assert_eq!(labels, vec!["Parlante A", "Parlante B", "Parlante A", "Parlante B"]);
        assert!(dir.path().join(DIARIZATION_FILE).exists());
        assert!(!dir.path().join(SYS_TRACK).exists());
    }

    #[test]
    fn estimates_offset_from_text() {
        let turn = |start: f64, text: &str| Turn { name: "X".into(), text: text.into(), start, end: start + 5.0 };
        let native = [
            turn(0.0, "buongiorno partiamo subito con il rilascio"),
            turn(10.0, "il deploy della versione nuova è pronto"),
            turn(20.0, "manca soltanto il collaudo del cliente"),
        ];
        let segs = [
            (30.5, "buongiorno partiamo subito con il rilascio"),
            (40.0, "il deploy della versione nuova è pronto"),
            (50.0, "manca soltanto il collaudo del cliente finale"),
        ];
        let off = estimate_offset(&native, &segs).unwrap();
        assert!((off - 30.0).abs() < 0.6, "offset {off}");
        assert_eq!(estimate_offset(&native[..1], &segs[..1]), None); // too little evidence
    }

    #[test]
    fn unaligned_native_is_ignored_when_captions_exist() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        let captions = serde_json::json!([{"name": "Rossi Mario", "text": "x", "start": 3.0, "end": 5.0}]);
        std::fs::write(dir.path().join(crate::teams::CAPTIONS_FILE), captions.to_string()).unwrap();
        // Native transcript Trascrivi didn't start (no offset) whose text matches nothing.
        let vtt = "WEBVTT\n\n00:00:03.000 --> 00:00:05.000\n<v Verdi Luca>tutt'altro discorso</v>\n";
        std::fs::write(dir.path().join(NATIVE_VTT), vtt).unwrap();
        let out = attribute_folder(dir.path(), "Io", None).unwrap();
        assert_eq!(out.speakers[1].1, "Rossi Mario");
    }

    /// Calibrates SAME_SPEAKER_THRESHOLD on a real meeting (aggregate numbers only):
    /// `TRASCRIVI_SPEAKER_MODEL=… TRASCRIVI_CAL_WAV=<16k mono wav> TRASCRIVI_CAL_DIAR=<diarization.json>
    ///  cargo test --lib calibrate_threshold -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn calibrate_threshold() {
        let model = std::env::var("TRASCRIVI_SPEAKER_MODEL").unwrap();
        let mut wav = WavReader::open(Path::new(&std::env::var("TRASCRIVI_CAL_WAV").unwrap())).unwrap();
        let diar: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(std::env::var("TRASCRIVI_CAL_DIAR").unwrap()).unwrap()).unwrap();
        let infos: Vec<SegmentInfo> = serde_json::from_value(diar["segments"].clone()).unwrap();
        let mut e = crate::speakers::embedder::Embedder::new(Path::new(&model)).unwrap();
        let embs: Vec<Vec<f32>> = infos
            .iter()
            .filter(|i| i.source == Source::Others && i.end - i.start >= MIN_EMBED_SECS)
            .filter_map(|i| e.embed(&read_secs(&mut wav, i.start, i.end.min(i.start + 6.0))).ok())
            .collect();
        println!("embedded others segments: {}", embs.len());
        for t in [0.3f32, 0.35, 0.4, 0.45] {
            let labels = crate::speakers::cluster::cluster(&embs, t);
            let mut sizes: HashMap<u32, usize> = HashMap::new();
            for l in &labels {
                *sizes.entry(*l).or_default() += 1;
            }
            let mut v: Vec<usize> = sizes.values().copied().collect();
            v.sort_unstable_by(|a, b| b.cmp(a));
            let big = v.iter().filter(|n| **n >= 5).count();
            let top8: usize = v.iter().take(8).sum();
            println!(
                "threshold {t:.2}: clusters {:3}, with ≥5 segs {:2}, top-8 cover {:3.0}%, sizes {:?}",
                v.len(), big, 100.0 * top8 as f32 / labels.len() as f32, &v[..v.len().min(10)]
            );
        }
    }
}
