//! Deterministic synthetic test signals.
//!
//! These generators exist so that pitch detectors can be validated against
//! signals whose fundamental frequency is known exactly. Every generator is
//! deterministic: the same arguments (and, for noise, the same seed) always
//! produce bit-identical output, which keeps tests and benchmarks reproducible.
//!
//! All generators return `Vec<f32>` with samples nominally in `[-1.0, 1.0]`
//! (sums of partials are normalised so that the peak stays in range).
//!
//! Enabled with the `signals` cargo feature.

use core::f64::consts::TAU;

/// Pure sine wave.
///
/// `phase` is the initial phase in radians.
pub fn sine(frequency: f64, sample_rate: f64, len: usize, amplitude: f32, phase: f64) -> Vec<f32> {
    let w = TAU * frequency / sample_rate;
    (0..len)
        .map(|n| amplitude * (w * n as f64 + phase).sin() as f32)
        .collect()
}

/// Sum of harmonic partials `k · frequency` with the given relative amplitudes.
///
/// `partials[0]` is the fundamental, `partials[1]` the second harmonic, and so
/// on. A zero entry produces a "missing" harmonic, which makes it easy to build
/// signals with a weak or absent fundamental. The output is normalised so that
/// its peak absolute value equals `amplitude`.
pub fn harmonic(
    frequency: f64,
    sample_rate: f64,
    len: usize,
    amplitude: f32,
    partials: &[f32],
) -> Vec<f32> {
    let w = TAU * frequency / sample_rate;
    let mut out: Vec<f32> = (0..len)
        .map(|n| {
            partials
                .iter()
                .enumerate()
                .map(|(k, &a)| {
                    let h = (k + 1) as f64;
                    // Offset each partial's phase so peaks don't all align at n = 0.
                    a as f64 * (w * h * n as f64 + 0.37 * h).sin()
                })
                .sum::<f64>() as f32
        })
        .collect();
    normalize(&mut out, amplitude);
    out
}

/// Parameters for [`pluck`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PluckParams {
    /// Number of partials to synthesise.
    pub partials: usize,
    /// Time in seconds for the fundamental to decay by 60 dB.
    pub t60: f64,
    /// Stiff-string inharmonicity coefficient `B`: partial `k` is at
    /// `k · f0 · sqrt(1 + B·k²)`. Real plucked gut/nylon strings are around
    /// `1e-5 .. 1e-4`. Zero gives a perfectly harmonic tone.
    pub inharmonicity: f64,
    /// Spectral tilt: partial `k` starts with amplitude `1 / k^tilt`.
    pub tilt: f64,
    /// Attack time in seconds (linear ramp) to avoid a click at onset.
    pub attack: f64,
    /// Peak level of a short (≈3 ms) decaying white-noise burst added at the
    /// start, relative to the fundamental's initial amplitude — a crude model
    /// of the broadband transient from a plectrum. `0.0` disables it.
    pub pick_noise: f64,
}

impl Default for PluckParams {
    fn default() -> Self {
        Self {
            partials: 12,
            t60: 2.0,
            inharmonicity: 3e-5,
            tilt: 1.0,
            attack: 0.002,
            pick_noise: 0.0,
        }
    }
}

/// A simplified plucked-string tone: decaying, slightly inharmonic partials
/// where higher partials decay faster.
///
/// This is not a physical model of any particular instrument; it only
/// reproduces the features that make plucked strings harder than sine waves
/// for a pitch detector (rich harmonics, decay, mild inharmonicity).
pub fn pluck(
    frequency: f64,
    sample_rate: f64,
    len: usize,
    amplitude: f32,
    params: PluckParams,
) -> Vec<f32> {
    // ln(1000) ≈ 6.9078: amplitude falls by 60 dB over t60.
    let base_decay = 6.907_755 / params.t60;
    let attack_samples = (params.attack * sample_rate).max(1.0);
    let noise_decay = 1.0 / (0.001 * sample_rate);
    let mut rng = SplitMix64::new(frequency.to_bits());
    let mut out: Vec<f32> = (0..len)
        .map(|n| {
            let t = n as f64 / sample_rate;
            let env_attack = (n as f64 / attack_samples).min(1.0);
            let s: f64 = (1..=params.partials)
                .map(|k| {
                    let k = k as f64;
                    let fk = k * frequency * (1.0 + params.inharmonicity * k * k).sqrt();
                    if fk >= sample_rate / 2.0 {
                        return 0.0;
                    }
                    let a = k.powf(-params.tilt);
                    // Higher partials decay faster, roughly as on real strings.
                    let decay = base_decay * (1.0 + 0.15 * (k - 1.0));
                    a * (-decay * t).exp() * (TAU * fk * t + 0.29 * k).sin()
                })
                .sum();
            let burst = if params.pick_noise > 0.0 {
                params.pick_noise * (-(n as f64) * noise_decay).exp() * rng.next_bipolar() as f64
            } else {
                0.0
            };
            env_attack * s + burst
        })
        .map(|x| x as f32)
        .collect();
    normalize(&mut out, amplitude);
    out
}

/// Sine wave with sinusoidal frequency modulation (vibrato).
///
/// `depth_cents` is the peak deviation from `frequency`; `rate` is in Hz.
/// The instantaneous frequency at sample `n` is returned by
/// [`vibrato_frequency_at`].
pub fn vibrato(
    frequency: f64,
    sample_rate: f64,
    len: usize,
    amplitude: f32,
    depth_cents: f64,
    rate: f64,
) -> Vec<f32> {
    let mut phase = 0.0f64;
    (0..len)
        .map(|n| {
            let f = vibrato_frequency_at(frequency, sample_rate, depth_cents, rate, n);
            let s = amplitude * phase.sin() as f32;
            phase = (phase + TAU * f / sample_rate) % TAU;
            s
        })
        .collect()
}

/// Instantaneous frequency of [`vibrato`] at sample index `n`.
pub fn vibrato_frequency_at(
    frequency: f64,
    sample_rate: f64,
    depth_cents: f64,
    rate: f64,
    n: usize,
) -> f64 {
    let t = n as f64 / sample_rate;
    frequency * 2f64.powf(depth_cents * (TAU * rate * t).sin() / 1200.0)
}

/// Sine sweep whose frequency moves exponentially (linearly in cents) from
/// `start` to `end` over `len` samples.
///
/// The instantaneous frequency at sample `n` is returned by
/// [`glide_frequency_at`].
pub fn glide(start: f64, end: f64, sample_rate: f64, len: usize, amplitude: f32) -> Vec<f32> {
    let mut phase = 0.0f64;
    (0..len)
        .map(|n| {
            let f = glide_frequency_at(start, end, len, n);
            let s = amplitude * phase.sin() as f32;
            phase = (phase + TAU * f / sample_rate) % TAU;
            s
        })
        .collect()
}

/// Instantaneous frequency of [`glide`] at sample index `n`.
pub fn glide_frequency_at(start: f64, end: f64, len: usize, n: usize) -> f64 {
    let x = if len <= 1 {
        0.0
    } else {
        n as f64 / (len - 1) as f64
    };
    start * (end / start).powf(x)
}

/// Uniform white noise in `[-amplitude, amplitude]` from a seeded PRNG.
pub fn white_noise(len: usize, amplitude: f32, seed: u64) -> Vec<f32> {
    let mut rng = SplitMix64::new(seed);
    (0..len).map(|_| amplitude * rng.next_bipolar()).collect()
}

/// Add seeded white noise to `signal` so that the result has the requested
/// signal-to-noise ratio in dB (computed from RMS values).
pub fn add_noise(signal: &mut [f32], snr_db: f64, seed: u64) {
    let signal_rms = rms(signal);
    if signal_rms == 0.0 {
        return;
    }
    // Uniform noise in [-a, a] has RMS a/√3.
    let noise_rms = signal_rms / 10f64.powf(snr_db / 20.0);
    let a = (noise_rms * 3f64.sqrt()) as f32;
    let mut rng = SplitMix64::new(seed);
    for s in signal.iter_mut() {
        *s += a * rng.next_bipolar();
    }
}

/// Root-mean-square value of `signal`.
pub fn rms(signal: &[f32]) -> f64 {
    if signal.is_empty() {
        return 0.0;
    }
    (signal.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / signal.len() as f64).sqrt()
}

/// Concatenate signals end to end.
pub fn concat(parts: &[&[f32]]) -> Vec<f32> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

fn normalize(buf: &mut [f32], amplitude: f32) {
    let peak = buf.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    if peak > 0.0 {
        let g = amplitude / peak;
        buf.iter_mut().for_each(|x| *x *= g);
    }
}

/// Small, fast, deterministic PRNG (SplitMix64). Not cryptographic.
#[derive(Debug, Clone)]
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[-1, 1)`.
    fn next_bipolar(&mut self) -> f32 {
        // Top 24 bits → exactly representable f32 in [0, 1).
        let u = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        2.0 * u - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_has_expected_period() {
        // 1 kHz at 48 kHz → exactly 48 samples per period.
        let s = sine(1000.0, 48_000.0, 480, 1.0, 0.0);
        for n in 0..(480 - 48) {
            assert!((s[n] - s[n + 48]).abs() < 1e-4);
        }
    }

    #[test]
    fn generators_are_deterministic() {
        assert_eq!(white_noise(64, 1.0, 7), white_noise(64, 1.0, 7));
        assert_ne!(white_noise(64, 1.0, 7), white_noise(64, 1.0, 8));
        let p = PluckParams::default();
        assert_eq!(
            pluck(110.0, 48_000.0, 1024, 0.8, p),
            pluck(110.0, 48_000.0, 1024, 0.8, p)
        );
    }

    #[test]
    fn normalised_generators_respect_amplitude() {
        let h = harmonic(220.0, 48_000.0, 4096, 0.5, &[1.0, 0.8, 0.6, 0.4]);
        let peak = h.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!((peak - 0.5).abs() < 1e-6);
        let p = pluck(98.0, 48_000.0, 4096, 0.9, PluckParams::default());
        let peak = p.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!((peak - 0.9).abs() < 1e-6);
    }

    #[test]
    fn noise_is_bounded_and_snr_is_close() {
        let n = white_noise(10_000, 0.25, 1);
        assert!(n.iter().all(|x| x.abs() <= 0.25));

        let clean = sine(440.0, 48_000.0, 48_000, 0.5, 0.0);
        let mut noisy = clean.clone();
        add_noise(&mut noisy, 20.0, 3);
        let noise: Vec<f32> = noisy.iter().zip(&clean).map(|(a, b)| a - b).collect();
        let snr = 20.0 * (rms(&clean) / rms(&noise)).log10();
        assert!((snr - 20.0).abs() < 0.5, "snr = {snr}");
    }

    #[test]
    fn glide_endpoints() {
        assert!((glide_frequency_at(100.0, 400.0, 101, 0) - 100.0).abs() < 1e-9);
        assert!((glide_frequency_at(100.0, 400.0, 101, 100) - 400.0).abs() < 1e-9);
        assert!((glide_frequency_at(100.0, 400.0, 101, 50) - 200.0).abs() < 1e-9);
    }
}
