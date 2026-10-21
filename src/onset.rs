//! Signal-level onset (attack) detection.
//!
//! [`OnsetDetector`] looks for sudden rises in short-term energy, which is
//! how plucked and struck notes begin. It works on the signal alone — it
//! knows nothing about pitch — so it detects re-plucks of the same note
//! (tremolo) but not legato pitch changes without a new attack; combine it
//! with pitch tracking to catch those.
//!
//! Energy is measured on the first difference of the signal
//! (`x[n] − x[n−1]`), a cheap high-frequency emphasis: pluck transients are
//! broadband while the sustained part of a low note is not, so this makes
//! attacks stand out more clearly than raw energy would.

use crate::ConfigError;

/// Configuration for [`OnsetDetector`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnsetConfig {
    /// Minimum rise, in dB, of the block's emphasised energy over the mean of
    /// the preceding blocks.
    pub rise_db: f64,
    /// Blocks quieter than this (dBFS, emphasised RMS) never trigger.
    pub min_level_db: f64,
    /// Minimum time in seconds between two reported onsets.
    pub refractory: f64,
    /// How many preceding blocks form the reference level.
    pub history: usize,
}

impl Default for OnsetConfig {
    fn default() -> Self {
        Self {
            rise_db: 6.0,
            min_level_db: -55.0,
            refractory: 0.06,
            history: 4,
        }
    }
}

/// A detected onset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Onset {
    /// Index (in the stream of samples passed to the detector) of the first
    /// sample of the block in which the onset was detected.
    pub sample_index: u64,
    /// Rise over the reference level, in dB.
    pub strength_db: f64,
    /// Emphasised RMS level of the triggering block, in dBFS.
    pub level_db: f64,
}

/// Streaming energy-rise onset detector.
///
/// Feed consecutive, non-overlapping blocks of samples (any length; around
/// 5–12 ms is a good resolution) to [`OnsetDetector::process`].
#[derive(Debug, Clone)]
pub struct OnsetDetector {
    config: OnsetConfig,
    sample_rate: f64,
    /// Emphasised levels (dB) of the most recent blocks, ring buffer.
    levels: Vec<f64>,
    next: usize,
    seen: usize,
    prev_sample: f32,
    position: u64,
    last_onset: Option<u64>,
}

impl OnsetDetector {
    /// Create a detector for a stream at `sample_rate`.
    pub fn new(sample_rate: f64, config: OnsetConfig) -> Result<Self, ConfigError> {
        if !(sample_rate.is_finite() && sample_rate > 0.0) {
            return Err(ConfigError::InvalidSampleRate(sample_rate));
        }
        if !(config.rise_db.is_finite() && config.rise_db > 0.0) {
            return Err(ConfigError::InvalidParameter {
                name: "rise_db",
                value: config.rise_db,
            });
        }
        if !(config.refractory.is_finite() && config.refractory >= 0.0) {
            return Err(ConfigError::InvalidParameter {
                name: "refractory",
                value: config.refractory,
            });
        }
        if config.history == 0 {
            return Err(ConfigError::InvalidParameter {
                name: "history",
                value: 0.0,
            });
        }
        Ok(Self {
            config,
            sample_rate,
            levels: vec![f64::NEG_INFINITY; config.history],
            next: 0,
            seen: 0,
            prev_sample: 0.0,
            position: 0,
            last_onset: None,
        })
    }

    /// Analyse the next block of samples. Returns an onset if one starts in
    /// this block.
    pub fn process(&mut self, block: &[f32]) -> Option<Onset> {
        if block.is_empty() {
            return None;
        }
        let mut energy = 0.0f64;
        let mut prev = self.prev_sample;
        for &x in block {
            let d = (x - prev) as f64;
            energy += d * d;
            prev = x;
        }
        self.prev_sample = prev;
        let level_db = 10.0 * (energy / block.len() as f64).log10();

        let block_start = self.position;
        self.position += block.len() as u64;

        // Reference: mean energy (in the linear domain) of preceding blocks.
        let reference_db = if self.seen == 0 {
            f64::NEG_INFINITY
        } else {
            let n = self.seen.min(self.levels.len());
            let mean = self.levels[..n]
                .iter()
                .map(|db| 10f64.powf(db / 10.0))
                .sum::<f64>()
                / n as f64;
            10.0 * mean.log10()
        };

        self.levels[self.next] = level_db;
        self.next = (self.next + 1) % self.levels.len();
        self.seen += 1;

        if !level_db.is_finite() || level_db < self.config.min_level_db {
            return None;
        }
        let rise = level_db - reference_db;
        if rise < self.config.rise_db {
            return None;
        }
        let refractory = (self.config.refractory * self.sample_rate) as u64;
        if let Some(last) = self.last_onset {
            if block_start.saturating_sub(last) < refractory {
                return None;
            }
        }
        self.last_onset = Some(block_start);
        Some(Onset {
            sample_index: block_start,
            strength_db: rise.min(120.0),
            level_db,
        })
    }

    /// Forget all history.
    pub fn reset(&mut self) {
        self.levels.fill(f64::NEG_INFINITY);
        self.next = 0;
        self.seen = 0;
        self.prev_sample = 0.0;
        self.position = 0;
        self.last_onset = None;
    }
}

#[cfg(all(test, feature = "signals"))]
mod tests {
    use super::*;
    use crate::signals::{self, PluckParams};

    const SR: f64 = 48_000.0;

    fn onsets(sig: &[f32], block: usize) -> Vec<u64> {
        let mut det = OnsetDetector::new(SR, OnsetConfig::default()).unwrap();
        sig.chunks(block)
            .filter_map(|b| det.process(b))
            .map(|o| o.sample_index)
            .collect()
    }

    #[test]
    fn detects_each_pluck_once() {
        let p = PluckParams::default();
        let silence = vec![0.0f32; 4800];
        let a = signals::pluck(110.0, SR, 24_000, 0.8, p);
        let b = signals::pluck(146.83, SR, 24_000, 0.8, p);
        let sig = signals::concat(&[&silence, &a, &b]);
        let found = onsets(&sig, 256);
        assert_eq!(found.len(), 2, "{found:?}");
        // Within one block of the true onsets.
        assert!(found[0].abs_diff(4800) <= 256);
        assert!(found[1].abs_diff(4800 + 24_000) <= 256);
    }

    #[test]
    fn detects_tremolo_replucks_of_same_note() {
        // 8 plucks of the same note, 125 ms apart (≈ 8 notes/s). The string
        // barely decays between plucks, so detection relies on the plectrum
        // transient.
        let p = PluckParams {
            pick_noise: 0.5,
            ..PluckParams::default()
        };
        let one = signals::pluck(196.0, SR, 6000, 0.7, p);
        let parts: Vec<&[f32]> = (0..8).map(|_| one.as_slice()).collect();
        let sig = signals::concat(&parts);
        let found = onsets(&sig, 256);
        assert_eq!(found.len(), 8, "{found:?}");
    }

    #[test]
    fn steady_tone_and_noise_do_not_trigger_after_start() {
        let tone = signals::sine(220.0, SR, 48_000, 0.5, 0.0);
        let found = onsets(&tone, 256);
        assert!(
            found.len() <= 1 && found.iter().all(|&i| i == 0),
            "{found:?}"
        );

        let mut noise = signals::white_noise(48_000, 0.1, 9);
        // Start mid-stream so the initial rise from nothing is excluded.
        noise.drain(..256);
        let mut det = OnsetDetector::new(SR, OnsetConfig::default()).unwrap();
        let hits = noise
            .chunks(256)
            .skip(1)
            .filter_map(|b| det.process(b))
            .count();
        assert!(hits <= 1, "{hits}");
    }

    #[test]
    fn quiet_blocks_never_trigger() {
        let quiet = signals::pluck(110.0, SR, 24_000, 0.0005, PluckParams::default());
        assert!(onsets(&quiet, 256).is_empty());
    }
}
