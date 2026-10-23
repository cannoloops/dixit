//! Minimal 16-bit PCM mono WAV writer/reader for the attribution tracks.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
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

/// Reads a 16-bit PCM mono WAV (as written by `WavWriter`) into [-1, 1] floats.
pub fn read_wav_mono_16k(path: &Path) -> Result<Vec<f32>> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{} is not a WAV file", path.display());
    }
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into()?) as usize;
        let body = pos + 8;
        if id == b"fmt " {
            let channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into()?);
            let bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into()?);
            if channels != 1 || bits != 16 {
                bail!("unsupported WAV format: {channels} ch, {bits} bit");
            }
        } else if id == b"data" {
            let end = (body + len).min(bytes.len());
            return Ok(bytes[body..end]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32767.0)
                .collect());
        }
        pos = body + len + (len & 1);
    }
    bail!("{} has no data chunk", path.display())
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
}
