//! The YIN fundamental-frequency estimator.
//!
//! Implementation of A. de Cheveigné and H. Kawahara, *"YIN, a fundamental
//! frequency estimator for speech and music"*, J. Acoust. Soc. Am. 111(4),
//! 2002. Steps, using the paper's numbering:
//!
//! 1. *(step 2)* Difference function
//!    `d(τ) = Σ_{j=0}^{W−1} (x_j − x_{j+τ})²` over a fixed integration window
//!    `W = window_size − τ_max`.
//! 2. *(step 3)* Cumulative mean normalised difference
//!    `d′(0) = 1`, `d′(τ) = d(τ) · τ / Σ_{j=1}^{τ} d(j)`.
//! 3. *(step 4)* Absolute threshold: the first local minimum of `d′` below
//!    the threshold, searched from the shortest admissible lag. Choosing the
//!    *first* qualifying dip, rather than the global minimum, is what
//!    suppresses YIN's octave-down (sub-harmonic) errors.
//! 4. *(step 5)* Parabolic interpolation of `d` around the chosen lag for
//!    sub-sample period resolution.
//! 5. `f0 = sample_rate / period`.
//!
//! The paper's step 6 ("best local estimate") is not implemented; temporal
//! smoothing is left to the caller.
//!
//! Additions beyond the paper:
//!
//! * an RMS silence gate that skips the search entirely;
//! * a second, looser *voicing threshold*: if no dip falls below
//!   [`YinConfig::threshold`] but one falls below
//!   [`YinConfig::voicing_threshold`], it is still reported as voiced, with
//!   [`Diagnostics::threshold_met`](crate::Diagnostics::threshold_met)
//!   cleared so callers can treat it with suspicion;
//! * explicit out-of-range detection when the best dip sits on the boundary
//!   of the lag search range.

use crate::difference::{self, DifferenceMethod};
use crate::preprocess::{parabolic_offset, prepare, rms_to_dbfs};
use crate::{
    ConfigError, DetectError, Diagnostics, PitchDetector, PitchEstimate, UnvoicedReason, Voicing,
};

/// Configuration for [`Yin`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YinConfig {
    /// Lowest fundamental frequency to search for, in Hz.
    pub min_frequency: f64,
    /// Highest fundamental frequency to search for, in Hz.
    pub max_frequency: f64,
    /// Absolute threshold on `d′(τ)` (paper step 4), in `(0, 1)`.
    /// Lower is stricter. The paper uses `0.1`.
    pub threshold: f64,
    /// Looser fallback threshold, `>= threshold`. Dips between the two
    /// thresholds are reported as voiced with `threshold_met == false`. Set
    /// equal to `threshold` to disable the fallback.
    pub voicing_threshold: f64,
    /// Windows whose RMS level is below this many dBFS are reported as
    /// [`UnvoicedReason::Silence`] without searching.
    pub silence_threshold_db: f64,
    /// Subtract the window mean before analysis.
    pub remove_dc: bool,
    /// How to compute the difference function (speed only; results agree
    /// to rounding error).
    pub difference: DifferenceMethod,
}

impl Default for YinConfig {
    /// A general-purpose musical range: 50 Hz – 1400 Hz.
    fn default() -> Self {
        Self {
            min_frequency: 50.0,
            max_frequency: 1400.0,
            threshold: 0.15,
            voicing_threshold: 0.25,
            silence_threshold_db: -60.0,
            remove_dc: true,
            difference: DifferenceMethod::default(),
        }
    }
}

/// YIN pitch detector with preallocated working buffers.
///
/// ```
/// use pitchforge::{PitchDetector, Yin, YinConfig};
///
/// let sample_rate = 48_000.0;
/// let mut yin = Yin::new(sample_rate, 4096, YinConfig::default()).unwrap();
///
/// // A 220 Hz sine wave.
/// let window: Vec<f32> = (0..4096)
///     .map(|n| (2.0 * std::f64::consts::PI * 220.0 * n as f64 / sample_rate).sin() as f32)
///     .collect();
///
/// let estimate = yin.detect(&window).unwrap();
/// let f0 = estimate.frequency().expect("voiced");
/// assert!((f0 - 220.0).abs() < 0.1);
/// ```
#[derive(Debug, Clone)]
pub struct Yin {
    config: YinConfig,
    sample_rate: f64,
    window_size: usize,
    tau_min: usize,
    tau_max: usize,
    /// Integration window length `W` in the paper.
    integration: usize,
    samples: Vec<f64>,
    /// `d(τ)` for `τ = 0..=tau_max`.
    diff: Vec<f64>,
    /// `d′(τ)` for `τ = 0..=tau_max`.
    cmndf: Vec<f64>,
    #[cfg(feature = "fft")]
    fft: Option<difference::FftDifference>,
}

impl Yin {
    /// Build a detector for windows of `window_size` samples at `sample_rate`.
    ///
    /// The window must hold at least two periods of
    /// [`YinConfig::min_frequency`] (plus one sample for interpolation); see
    /// [`Yin::required_window_size`].
    pub fn new(
        sample_rate: f64,
        window_size: usize,
        config: YinConfig,
    ) -> Result<Self, ConfigError> {
        let (tau_min, tau_max) = validate(sample_rate, &config)?;
        let required = 2 * tau_max;
        if window_size < required {
            return Err(ConfigError::WindowTooShort {
                required,
                actual: window_size,
            });
        }
        Ok(Self {
            config,
            sample_rate,
            window_size,
            tau_min,
            tau_max,
            integration: window_size - tau_max,
            samples: vec![0.0; window_size],
            diff: vec![0.0; tau_max + 1],
            cmndf: vec![1.0; tau_max + 1],
            #[cfg(feature = "fft")]
            fft: (config.difference == DifferenceMethod::Fft)
                .then(|| difference::FftDifference::new(window_size)),
        })
    }

    /// Smallest window size accepted by [`Yin::new`] for this configuration.
    pub fn required_window_size(
        sample_rate: f64,
        config: &YinConfig,
    ) -> Result<usize, ConfigError> {
        validate(sample_rate, config).map(|(_, tau_max)| 2 * tau_max)
    }

    /// The configuration this detector was built with.
    pub fn config(&self) -> &YinConfig {
        &self.config
    }

    /// Inclusive lag search range `(τ_min, τ_max)` in samples.
    pub fn lag_range(&self) -> (usize, usize) {
        (self.tau_min, self.tau_max)
    }

    /// The cumulative mean normalised difference function `d′(τ)` from the
    /// most recent call to [`detect`](PitchDetector::detect), indexed by lag
    /// `τ = 0..=τ_max`.
    ///
    /// Useful for visual diagnostics. Not updated for windows rejected as
    /// silence.
    pub fn cmndf(&self) -> &[f64] {
        &self.cmndf
    }

    fn difference(&mut self) {
        #[cfg(feature = "fft")]
        if let Some(fft) = &mut self.fft {
            fft.compute(&self.samples, self.integration, &mut self.diff);
            return;
        }
        difference::direct(&self.samples, self.integration, &mut self.diff);
    }

    fn cumulative_mean_normalize(&mut self) {
        self.cmndf[0] = 1.0;
        let mut running = 0.0;
        for tau in 1..=self.tau_max {
            running += self.diff[tau];
            self.cmndf[tau] = if running > 0.0 {
                self.diff[tau] * tau as f64 / running
            } else {
                1.0
            };
        }
    }

    /// Bottom of the first dip of `d′` that falls below `threshold` in
    /// `[τ_min, τ_max]`.
    ///
    /// The paper walks down from the first sub-threshold lag to the nearest
    /// local minimum. With broadband noise the dip around a low-frequency
    /// period is wide and rippled, so that walk can stop on a ripple on the
    /// falling slope and bias the estimate sharp by tens of cents. Instead we
    /// take the minimum over the whole contiguous run of lags below the
    /// threshold, which is the same dip but immune to ripples.
    fn first_dip_below(&self, threshold: f64) -> Option<usize> {
        let d = &self.cmndf;
        let start = (self.tau_min..=self.tau_max).find(|&tau| d[tau] < threshold)?;
        let mut best = start;
        let mut tau = start + 1;
        while tau <= self.tau_max && d[tau] < threshold {
            if d[tau] < d[best] {
                best = tau;
            }
            tau += 1;
        }
        Some(best)
    }

    fn global_min(&self) -> usize {
        let d = &self.cmndf;
        (self.tau_min..=self.tau_max)
            .min_by(|&a, &b| d[a].total_cmp(&d[b]))
            .unwrap_or(self.tau_min)
    }

    /// The chosen lag is the bottom of a dip that actually lies outside the
    /// search range (the function is still falling past the boundary).
    fn on_open_boundary(&self, tau: usize) -> bool {
        let d = &self.cmndf;
        (tau == self.tau_min && tau > 1 && d[tau - 1] < d[tau])
            || (tau == self.tau_max && d[tau - 1] > d[tau])
    }

    fn refine(&self, tau: usize) -> f64 {
        if tau == 0 || tau >= self.tau_max {
            return tau as f64;
        }
        let d = &self.diff;
        tau as f64 + parabolic_offset(d[tau - 1], d[tau], d[tau + 1])
    }
}

impl PitchDetector for Yin {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn window_size(&self) -> usize {
        self.window_size
    }

    fn detect(&mut self, window: &[f32]) -> Result<PitchEstimate, DetectError> {
        if window.len() != self.window_size {
            return Err(DetectError::WindowLength {
                expected: self.window_size,
                actual: window.len(),
            });
        }
        let rms = prepare(window, &mut self.samples, self.config.remove_dc)?;
        if rms_to_dbfs(rms) < self.config.silence_threshold_db {
            return Ok(PitchEstimate::silence(rms));
        }

        self.difference();
        self.cumulative_mean_normalize();

        let (tau, threshold_met, voiced) =
            if let Some(tau) = self.first_dip_below(self.config.threshold) {
                (tau, true, true)
            } else if let Some(tau) = self.first_dip_below(self.config.voicing_threshold) {
                (tau, false, true)
            } else {
                (self.global_min(), false, false)
            };

        let periodicity = self.cmndf[tau];
        let period = self.refine(tau);
        let candidate = (period > 0.0).then(|| self.sample_rate / period);
        let voicing = if self.on_open_boundary(tau) {
            Voicing::Unvoiced(UnvoicedReason::OutOfRange)
        } else if voiced {
            Voicing::Voiced
        } else {
            Voicing::Unvoiced(UnvoicedReason::Aperiodic)
        };

        Ok(PitchEstimate {
            voicing,
            candidate,
            confidence: (1.0 - periodicity).clamp(0.0, 1.0),
            diagnostics: Diagnostics {
                rms,
                period: Some(period),
                lag: Some(tau),
                periodicity: Some(periodicity),
                threshold_met,
            },
        })
    }
}

fn validate(sample_rate: f64, c: &YinConfig) -> Result<(usize, usize), ConfigError> {
    if !(sample_rate.is_finite() && sample_rate > 0.0) {
        return Err(ConfigError::InvalidSampleRate(sample_rate));
    }
    let nyquist = sample_rate / 2.0;
    let range_ok = c.min_frequency.is_finite()
        && c.max_frequency.is_finite()
        && c.min_frequency > 0.0
        && c.min_frequency < c.max_frequency
        && c.max_frequency < nyquist;
    if !range_ok {
        return Err(ConfigError::InvalidFrequencyRange {
            min: c.min_frequency,
            max: c.max_frequency,
        });
    }
    if !(c.threshold > 0.0 && c.threshold < 1.0) {
        return Err(ConfigError::InvalidParameter {
            name: "threshold",
            value: c.threshold,
        });
    }
    if !(c.voicing_threshold >= c.threshold && c.voicing_threshold < 1.0) {
        return Err(ConfigError::InvalidParameter {
            name: "voicing_threshold",
            value: c.voicing_threshold,
        });
    }
    if c.silence_threshold_db.is_nan() {
        return Err(ConfigError::InvalidParameter {
            name: "silence_threshold_db",
            value: c.silence_threshold_db,
        });
    }
    // τ_min ≥ 2 so the parabolic fit always has a left neighbour with τ ≥ 1.
    let tau_min = ((sample_rate / c.max_frequency).floor() as usize).max(2);
    // +1 so a candidate exactly at min_frequency still has a right neighbour.
    let tau_max = (sample_rate / c.min_frequency).ceil() as usize + 1;
    Ok((tau_min, tau_max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::TAU;

    const SR: f64 = 48_000.0;

    fn sine(f: f64, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| (0.5 * (TAU * f * n as f64 / SR).sin()) as f32)
            .collect()
    }

    fn cents(a: f64, b: f64) -> f64 {
        1200.0 * (a / b).log2()
    }

    #[test]
    fn detects_sine_frequencies_within_a_cent() {
        let mut yin = Yin::new(SR, 4096, YinConfig::default()).unwrap();
        for f in [55.0, 98.0, 146.83, 261.63, 440.0, 880.0, 1318.5] {
            let est = yin.detect(&sine(f, 4096)).unwrap();
            let got = est
                .frequency()
                .unwrap_or_else(|| panic!("{f} Hz unvoiced: {est:?}"));
            assert!(cents(got, f).abs() < 1.0, "{f} Hz → {got} Hz");
            assert!(est.diagnostics.threshold_met);
            assert!(est.confidence > 0.9);
        }
    }

    #[test]
    fn silence_is_unvoiced_without_candidate() {
        let mut yin = Yin::new(SR, 4096, YinConfig::default()).unwrap();
        let est = yin.detect(&[0.0; 4096]).unwrap();
        assert_eq!(est.voicing, Voicing::Unvoiced(UnvoicedReason::Silence));
        assert_eq!(est.frequency(), None);
        assert_eq!(est.candidate, None);
    }

    #[test]
    fn rejects_wrong_window_length_and_nan() {
        let mut yin = Yin::new(SR, 4096, YinConfig::default()).unwrap();
        assert_eq!(
            yin.detect(&[0.0; 100]),
            Err(DetectError::WindowLength {
                expected: 4096,
                actual: 100
            })
        );
        let mut w = sine(220.0, 4096);
        w[17] = f32::INFINITY;
        assert_eq!(yin.detect(&w), Err(DetectError::NonFiniteInput));
    }

    #[test]
    fn validates_configuration() {
        let c = YinConfig::default();
        assert!(matches!(
            Yin::new(0.0, 4096, c),
            Err(ConfigError::InvalidSampleRate(_))
        ));
        assert!(matches!(
            Yin::new(
                SR,
                4096,
                YinConfig {
                    min_frequency: 500.0,
                    max_frequency: 400.0,
                    ..c
                }
            ),
            Err(ConfigError::InvalidFrequencyRange { .. })
        ));
        assert!(matches!(
            Yin::new(
                SR,
                4096,
                YinConfig {
                    max_frequency: 30_000.0,
                    ..c
                }
            ),
            Err(ConfigError::InvalidFrequencyRange { .. })
        ));
        assert!(matches!(
            Yin::new(
                SR,
                4096,
                YinConfig {
                    voicing_threshold: 0.1,
                    ..c
                }
            ),
            Err(ConfigError::InvalidParameter {
                name: "voicing_threshold",
                ..
            })
        ));
        // 50 Hz at 48 kHz → τ_max = 961 → needs 1922 samples.
        assert_eq!(Yin::required_window_size(SR, &c), Ok(1922));
        assert_eq!(
            Yin::new(SR, 1024, c).unwrap_err(),
            ConfigError::WindowTooShort {
                required: 1922,
                actual: 1024
            }
        );
        assert!(Yin::new(SR, 1922, c).is_ok());
    }

    #[test]
    fn frequency_above_range_is_not_reported_as_voiced_in_range() {
        // 2 kHz with max 1400 Hz: the true dip (τ = 24) is below τ_min (34).
        // YIN may land on the 2nd sub-harmonic dip (1 kHz), which is a
        // genuine periodicity of the signal; it must never report 2 kHz.
        let mut yin = Yin::new(SR, 4096, YinConfig::default()).unwrap();
        let est = yin.detect(&sine(2000.0, 4096)).unwrap();
        if let Some(f) = est.frequency() {
            assert!(f <= 1400.0);
        }
    }

    #[test]
    fn frequency_below_range_is_out_of_range() {
        let c = YinConfig {
            min_frequency: 100.0,
            ..YinConfig::default()
        };
        let mut yin = Yin::new(SR, 4096, c).unwrap();
        let est = yin.detect(&sine(60.0, 4096)).unwrap();
        assert_ne!(est.voicing, Voicing::Voiced, "{est:?}");
    }

    #[cfg(feature = "fft")]
    #[test]
    fn fft_and_direct_difference_agree() {
        let direct_cfg = YinConfig {
            difference: DifferenceMethod::Direct,
            ..YinConfig::default()
        };
        let fft_cfg = YinConfig {
            difference: DifferenceMethod::Fft,
            ..YinConfig::default()
        };
        for window in [2048, 3000, 4096] {
            let mut direct = Yin::new(SR, window, direct_cfg).unwrap();
            let mut fft = Yin::new(SR, window, fft_cfg).unwrap();
            for (i, f) in [61.0, 133.3, 440.0, 1111.0].into_iter().enumerate() {
                // Sine plus a deterministic pseudo-random component.
                let w: Vec<f32> = sine(f, window)
                    .iter()
                    .enumerate()
                    .map(|(n, &x)| {
                        x + 0.05 * (((n * 7919 + i * 104_729) % 1000) as f32 / 500.0 - 1.0)
                    })
                    .collect();
                let a = direct.detect(&w).unwrap();
                let b = fft.detect(&w).unwrap();
                let scale = direct.diff.iter().fold(0.0f64, |m, &v| m.max(v));
                for (x, y) in direct.diff.iter().zip(&fft.diff) {
                    assert!((x - y).abs() <= 1e-9 * scale, "{x} vs {y}");
                }
                assert_eq!(a.voicing, b.voicing);
                assert_eq!(a.diagnostics.lag, b.diagnostics.lag);
                let (fa, fb) = (a.candidate.unwrap(), b.candidate.unwrap());
                assert!(cents(fa, fb).abs() < 1e-3, "{fa} vs {fb}");
            }
        }
    }

    #[test]
    fn cmndf_is_exposed_for_diagnostics() {
        let mut yin = Yin::new(SR, 4096, YinConfig::default()).unwrap();
        yin.detect(&sine(480.0, 4096)).unwrap();
        let (_, tau_max) = yin.lag_range();
        assert_eq!(yin.cmndf().len(), tau_max + 1);
        // 480 Hz at 48 kHz → period of exactly 100 samples.
        assert!(yin.cmndf()[100] < 0.01);
    }
}
