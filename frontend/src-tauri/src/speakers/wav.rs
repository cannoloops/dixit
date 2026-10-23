//! Minimal 16-bit PCM mono WAV writer/reader for the attribution tracks.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct WavWriter {
    out: BufWriter<File>,
    samples: u32,
}

impl WavWriter {
    pub fn create(path: &Path, sample_rate: u32) -> Result<Self> {
        let mut out = BufWriter::new(File::create(path).with_context(|| format!("create {}", path.display()))?);
        out.write_all(b"RIFF\0\0\0\0WAVEfmt ")?;
        out.write_all(&16u32.to_le_bytes())?; // fmt chunk size
        out.write_all(&1u16.to_le_bytes())?; // PCM
        out.write_all(&1u16.to_le_bytes())?; // mono
        out.write_all(&sample_rate.to_le_bytes())?;
        out.write_all(&(sample_rate * 2).to_le_bytes())?; // byte rate
        out.write_all(&2u16.to_le_bytes())?; // block align
        out.write_all(&16u16.to_le_bytes())?; // bits per sample
        out.write_all(b"data\0\0\0\0")?;
        Ok(Self { out, samples: 0 })
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<()> {
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            self.out.write_all(&v.to_le_bytes())?;
        }
        self.samples += samples.len() as u32;
        Ok(())
    }

    /// Patches the RIFF/data sizes; the file is unreadable until this runs.
    pub fn finish(mut self) -> Result<()> {
        let data_bytes = self.samples * 2;
        self.out.seek(SeekFrom::Start(4))?;
        self.out.write_all(&(36 + data_bytes).to_le_bytes())?;
        self.out.seek(SeekFrom::Start(40))?;
        self.out.write_all(&data_bytes.to_le_bytes())?;
        self.out.flush()?;
        Ok(())
    }
}

/// Random access to a 16-bit PCM mono WAV: a 3-hour track is ~350 MB, never load it whole.
pub struct WavReader {
    file: File,
    data_start: u64,
    samples: usize,
}

impl WavReader {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let mut riff = [0u8; 12];
        file.read_exact(&mut riff)?;
        if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
            bail!("{} is not a WAV file", path.display());
        }
        let mut pos = 12u64;
        loop {
            let mut header = [0u8; 8];
            file.seek(SeekFrom::Start(pos))?;
            if file.read_exact(&mut header).is_err() {
                bail!("{} has no data chunk", path.display());
            }
            let len = u32::from_le_bytes(header[4..8].try_into()?) as u64;
            if &header[0..4] == b"fmt " {
                let mut fmt = [0u8; 16];
                file.read_exact(&mut fmt)?;
                let channels = u16::from_le_bytes(fmt[2..4].try_into()?);
                let bits = u16::from_le_bytes(fmt[14..16].try_into()?);
                if channels != 1 || bits != 16 {
                    bail!("unsupported WAV format: {channels} ch, {bits} bit");
                }
            } else if &header[0..4] == b"data" {
                let available = file.metadata()?.len().saturating_sub(pos + 8);
                return Ok(Self { file, data_start: pos + 8, samples: (len.min(available) / 2) as usize });
            }
            pos += 8 + len + (len & 1);
        }
    }

    pub fn len(&self) -> usize {
        self.samples
    }

    /// Samples [start, end) clamped to the file, as [-1, 1] floats.
    pub fn read_range(&mut self, start: usize, end: usize) -> Result<Vec<f32>> {
        let (start, end) = (start.min(self.samples), end.min(self.samples));
        if start >= end {
            return Ok(Vec::new());
        }
        let mut bytes = vec![0u8; (end - start) * 2];
        self.file.seek(SeekFrom::Start(self.data_start + start as u64 * 2))?;
        self.file.read_exact(&mut bytes)?;
        Ok(bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32767.0).collect())
    }
}

/// Reads a whole 16-bit PCM mono WAV into [-1, 1] floats (short files only).
pub fn read_wav_mono_16k(path: &Path) -> Result<Vec<f32>> {
    let mut r = WavReader::open(path)?;
    let n = r.len();
    r.read_range(0, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let samples: Vec<f32> = (0..1600).map(|i| ((i as f32) * 0.01).sin() * 0.5).collect();
        let mut w = WavWriter::create(&path, 16000).unwrap();
        w.write(&samples[..800]).unwrap();
        w.write(&samples[800..]).unwrap();
        w.finish().unwrap();
        let back = read_wav_mono_16k(&path).unwrap();
        assert_eq!(back.len(), samples.len());
        assert!(back.iter().zip(&samples).all(|(a, b)| (a - b).abs() <= 1.0 / 32768.0 + 1e-6));
    }

    #[test]
    fn reader_reads_ranges_without_loading_everything() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.wav");
        let samples: Vec<f32> = (0..16000).map(|i| (i as f32 / 16000.0) - 0.5).collect();
        let mut w = WavWriter::create(&path, 16000).unwrap();
        w.write(&samples).unwrap();
        w.finish().unwrap();
        let mut r = WavReader::open(&path).unwrap();
        assert_eq!(r.len(), 16000);
        let mid = r.read_range(100, 200).unwrap();
        assert_eq!(mid.len(), 100);
        assert!(mid.iter().zip(&samples[100..200]).all(|(a, b)| (a - b).abs() < 1e-4));
        assert_eq!(r.read_range(15990, 99999).unwrap().len(), 10);
        assert!(r.read_range(20000, 30000).unwrap().is_empty());
    }
}
