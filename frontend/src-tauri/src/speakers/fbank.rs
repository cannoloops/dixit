//! Kaldi-compatible 80-bin log-mel filterbank (what WeSpeaker models are trained on).

use once_cell::sync::Lazy;
use realfft::RealFftPlanner;

pub const NUM_BINS: usize = 80;
const SAMPLE_RATE: f32 = 16000.0;
const FRAME_LEN: usize = 400; // 25 ms
const FRAME_SHIFT: usize = 160; // 10 ms
const FFT_LEN: usize = 512;
const LOW_HZ: f32 = 20.0;
const HIGH_HZ: f32 = 8000.0;

fn mel(hz: f32) -> f32 {
    1127.0 * (1.0 + hz / 700.0).ln()
}

fn inv_mel(m: f32) -> f32 {
    700.0 * ((m / 1127.0).exp() - 1.0)
}

fn mel_delta() -> f32 {
    (mel(HIGH_HZ) - mel(LOW_HZ)) / (NUM_BINS as f32 + 1.0)
}

/// Center frequency of mel bin `b`.
pub fn bin_center_hz(b: usize) -> f32 {
    inv_mel(mel(LOW_HZ) + (b as f32 + 1.0) * mel_delta())
}

/// Triangular weights per bin over FFT bins 0..FFT_LEN/2: (first fft bin, weights).
static BANKS: Lazy<Vec<(usize, Vec<f32>)>> = Lazy::new(|| {
    let (low, delta) = (mel(LOW_HZ), mel_delta());
    (0..NUM_BINS)
        .map(|b| {
            let (left, center, right) = (low + b as f32 * delta, low + (b as f32 + 1.0) * delta, low + (b as f32 + 2.0) * delta);
            let mut first = None;
            let mut weights = Vec::new();
            for k in 0..FFT_LEN / 2 {
                let m = mel(k as f32 * SAMPLE_RATE / FFT_LEN as f32);
                if m > left && m < right {
                    first.get_or_insert(k);
                    weights.push(if m <= center { (m - left) / (center - left) } else { (right - m) / (right - center) });
                }
            }
            (first.unwrap_or(0), weights)
        })
        .collect()
});

static POVEY: Lazy<Vec<f32>> = Lazy::new(|| {
    (0..FRAME_LEN)
        .map(|i| {
            let a = 2.0 * std::f32::consts::PI * i as f32 / (FRAME_LEN as f32 - 1.0);
            (0.5 - 0.5 * a.cos()).powf(0.85)
        })
        .collect()
});

/// Log-mel frames without mean normalization (snip edges, no dither).
pub fn fbank_raw(samples_16k: &[f32]) -> Vec<[f32; NUM_BINS]> {
    if samples_16k.len() < FRAME_LEN {
        return Vec::new();
    }
    let frames = 1 + (samples_16k.len() - FRAME_LEN) / FRAME_SHIFT;
    let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT_LEN);
    let mut input = fft.make_input_vec();
    let mut spectrum = fft.make_output_vec();
    let mut out = Vec::with_capacity(frames);
    for f in 0..frames {
        let frame = &samples_16k[f * FRAME_SHIFT..f * FRAME_SHIFT + FRAME_LEN];
        let mut x: Vec<f32> = frame.iter().map(|s| s * 32768.0).collect();
        let mean = x.iter().sum::<f32>() / FRAME_LEN as f32;
        x.iter_mut().for_each(|v| *v -= mean);
        for i in (1..FRAME_LEN).rev() {
            x[i] -= 0.97 * x[i - 1];
        }
        x[0] -= 0.97 * x[0];
        input.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..FRAME_LEN {
            input[i] = x[i] * POVEY[i];
        }
        fft.process(&mut input, &mut spectrum).expect("fft sizes are fixed");
        let mut bins = [0f32; NUM_BINS];
        for (b, (first, weights)) in BANKS.iter().enumerate() {
            let e: f32 = weights.iter().enumerate().map(|(i, w)| w * spectrum[first + i].norm_sqr()).sum();
            bins[b] = e.max(f32::EPSILON).ln();
        }
        out.push(bins);
    }
    out
}

/// Log-mel frames with per-utterance mean subtraction (CMN), as WeSpeaker expects.
pub fn fbank(samples_16k: &[f32]) -> Vec<[f32; NUM_BINS]> {
    let mut feats = fbank_raw(samples_16k);
    if feats.is_empty() {
        return feats;
    }
    let n = feats.len() as f32;
    for b in 0..NUM_BINS {
        let mean = feats.iter().map(|f| f[b]).sum::<f32>() / n;
        feats.iter_mut().for_each(|f| f[b] -= mean);
    }
    feats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, secs: f32) -> Vec<f32> {
        (0..(16000.0 * secs) as usize)
            .map(|i| 0.3 * (2.0 * std::f32::consts::PI * freq * i as f32 / 16000.0).sin())
            .collect()
    }

    #[test]
    fn fbank_shape() {
        let feats = fbank(&tone(440.0, 1.0));
        assert_eq!(feats.len(), 98); // 1 + (16000 - 400) / 160
        assert!(fbank(&[0.0; 399]).is_empty());
    }

    #[test]
    fn fbank_tone_peaks_near_1khz() {
        let raw = fbank_raw(&tone(1000.0, 0.5));
        let mean: Vec<f32> = (0..NUM_BINS)
            .map(|b| raw.iter().map(|f| f[b]).sum::<f32>() / raw.len() as f32)
            .collect();
        let argmax = (0..NUM_BINS).max_by(|&a, &b| mean[a].total_cmp(&mean[b])).unwrap();
        let nearest = (0..NUM_BINS)
            .min_by(|&a, &b| (bin_center_hz(a) - 1000.0).abs().total_cmp(&(bin_center_hz(b) - 1000.0).abs()))
            .unwrap();
        assert!((argmax as i32 - nearest as i32).abs() <= 1, "argmax {argmax} nearest {nearest}");
    }

    #[test]
    fn fbank_is_mean_normalized() {
        let feats = fbank(&tone(300.0, 1.0));
        for b in 0..NUM_BINS {
            let m = feats.iter().map(|f| f[b]).sum::<f32>() / feats.len() as f32;
            assert!(m.abs() < 1e-3, "bin {b} mean {m}");
        }
    }
}
