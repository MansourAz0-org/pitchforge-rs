//! Shared helpers for accuracy tests.
#![allow(dead_code)]

use pitchforge::{PitchDetector, PitchEstimate};

/// Signed error in cents of `got` relative to `expected`.
pub fn cents(got: f64, expected: f64) -> f64 {
    1200.0 * (got / expected).log2()
}

/// Frequencies from `lo` to `hi` (inclusive-ish) in 24-EDO (50-cent) steps.
pub fn quarter_tone_sweep(lo: f64, hi: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut f = lo;
    while f <= hi * 1.000_001 {
        out.push(f);
        f *= 2f64.powf(1.0 / 24.0);
    }
    out
}

/// Outcome of one detection compared against ground truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Voiced, error in cents.
    Ok(f64),
    /// Voiced, but off by (close to) a whole number of octaves.
    Octave(f64),
    /// Voiced, more than 50 cents off but not an octave error.
    Gross(f64),
    /// Not voiced.
    Unvoiced,
}

pub fn classify(est: &PitchEstimate, expected: f64) -> Outcome {
    let Some(f) = est.frequency() else {
        return Outcome::Unvoiced;
    };
    let c = cents(f, expected);
    if c.abs() <= 50.0 {
        Outcome::Ok(c)
    } else {
        let octaves = (c / 1200.0).round();
        if octaves != 0.0 && (c - octaves * 1200.0).abs() <= 50.0 {
            Outcome::Octave(c)
        } else {
            Outcome::Gross(c)
        }
    }
}

/// Aggregated statistics over a set of detections.
#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub n: usize,
    pub ok: usize,
    pub octave: usize,
    pub gross: usize,
    pub unvoiced: usize,
    pub max_abs_cents: f64,
    sum_abs_cents: f64,
}

impl Stats {
    pub fn push(&mut self, o: Outcome) {
        self.n += 1;
        match o {
            Outcome::Ok(c) => {
                self.ok += 1;
                self.max_abs_cents = self.max_abs_cents.max(c.abs());
                self.sum_abs_cents += c.abs();
            }
            Outcome::Octave(_) => self.octave += 1,
            Outcome::Gross(_) => self.gross += 1,
            Outcome::Unvoiced => self.unvoiced += 1,
        }
    }

    pub fn merge(&mut self, other: &Stats) {
        self.n += other.n;
        self.ok += other.ok;
        self.octave += other.octave;
        self.gross += other.gross;
        self.unvoiced += other.unvoiced;
        self.max_abs_cents = self.max_abs_cents.max(other.max_abs_cents);
        self.sum_abs_cents += other.sum_abs_cents;
    }

    pub fn mean_abs_cents(&self) -> f64 {
        if self.ok == 0 {
            0.0
        } else {
            self.sum_abs_cents / self.ok as f64
        }
    }

    pub fn report(&self, label: &str) {
        println!(
            "{label:<44} n={:>4} ok={:>4} oct={:>3} gross={:>3} unvoiced={:>3}  \
             mean|c|={:>6.3} max|c|={:>6.3}",
            self.n,
            self.ok,
            self.octave,
            self.gross,
            self.unvoiced,
            self.mean_abs_cents(),
            self.max_abs_cents
        );
    }
}

/// Run a detector over `signal` with a hop, comparing each window against
/// `truth(center_sample_index)`.
pub fn track<D: PitchDetector>(
    det: &mut D,
    signal: &[f32],
    hop: usize,
    truth: impl Fn(usize) -> f64,
) -> Stats {
    let w = det.window_size();
    let mut stats = Stats::default();
    let mut start = 0;
    while start + w <= signal.len() {
        let est = det.detect(&signal[start..start + w]).unwrap();
        stats.push(classify(&est, truth(start + w / 2)));
        start += hop;
    }
    stats
}
