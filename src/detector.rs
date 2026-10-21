//! The common interface implemented by every pitch detector.

use crate::{DetectError, PitchEstimate};

/// A monophonic fundamental-frequency estimator operating on fixed-size
/// windows.
///
/// Implementations preallocate all working memory at construction, so
/// [`detect`](PitchDetector::detect) does not allocate and is suitable for
/// running on a dedicated (non-real-time-callback) DSP thread.
pub trait PitchDetector {
    /// Sample rate in Hz the detector was configured for.
    fn sample_rate(&self) -> f64;

    /// Number of samples [`detect`](PitchDetector::detect) expects.
    fn window_size(&self) -> usize;

    /// Analyse one window of mono samples.
    ///
    /// `window.len()` must equal [`window_size`](PitchDetector::window_size).
    fn detect(&mut self, window: &[f32]) -> Result<PitchEstimate, DetectError>;
}
