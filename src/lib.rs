//! # pitchforge
//!
//! Fundamental-frequency (f0) estimation for monophonic audio.
//!
//! `pitchforge` is a general-purpose DSP library. It knows nothing about
//! instruments, note names, or tuning systems, and it never touches audio
//! hardware: it takes a buffer of samples and a sample rate, and reports the
//! most likely fundamental frequency together with a confidence measure, an
//! explicit voicing decision, and diagnostics.
//!
//! ## Detectors
//!
//! | Detector | Status |
//! |----------|--------|
//! | [`Yin`] — de Cheveigné & Kawahara (2002) | implemented |
//! | McLeod Pitch Method | planned |
//!
//! Every detector implements [`PitchDetector`], preallocates its working
//! memory at construction and does not allocate in
//! [`detect`](PitchDetector::detect).
//!
//! ## Streaming
//!
//! [`Framer`] turns audio arriving in arbitrary chunks into overlapping
//! windows, and [`OnsetDetector`] reports note attacks from energy rises.
//!
//! ```
//! use pitchforge::{Framer, PitchDetector, Yin, YinConfig};
//!
//! let sample_rate = 48_000.0;
//! let mut yin = Yin::new(sample_rate, 4096, YinConfig::default())?;
//! let mut framer = Framer::new(4096, 512)?;
//!
//! # let audio_callback_chunk = vec![0.0f32; 1024];
//! // For each chunk of mono samples delivered by your audio source:
//! framer.push(&audio_callback_chunk, |window, start| {
//!     let estimate = yin.detect(window).expect("window size matches");
//!     if let Some(f0) = estimate.frequency() {
//!         println!("sample {start}: {f0:.2} Hz (confidence {:.2})", estimate.confidence);
//!     }
//! });
//! # Ok::<(), pitchforge::ConfigError>(())
//! ```
//!
//! ## Honesty about uncertainty
//!
//! A window is reported as [`Voicing::Voiced`] only when the periodicity
//! criterion is met. Silence, aperiodic input and out-of-range candidates are
//! reported as [`Voicing::Unvoiced`] with a reason;
//! [`PitchEstimate::frequency`] returns `None` for them. The confidence value
//! is a periodicity measure, not a calibrated probability.
//!
//! ## Cargo features
//!
//! - `fft` *(default)* — O(N log N) YIN difference function via `realfft`.
//! - `signals` — deterministic synthetic test signals ([`signals`]).
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "signals")]
#[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
pub mod signals;

mod detector;
mod difference;
mod error;
mod estimate;
mod framer;
mod onset;
mod preprocess;
mod yin;

pub use detector::PitchDetector;
pub use difference::DifferenceMethod;
pub use error::{ConfigError, DetectError};
pub use estimate::{Diagnostics, PitchEstimate, UnvoicedReason, Voicing};
pub use framer::Framer;
pub use onset::{Onset, OnsetConfig, OnsetDetector};
pub use yin::{Yin, YinConfig};
