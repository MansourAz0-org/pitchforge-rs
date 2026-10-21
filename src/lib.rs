//! # pitchforge
//!
//! Fundamental-frequency (f0) estimation for monophonic audio.
//!
//! `pitchforge` is a general-purpose DSP library. It knows nothing about
//! instruments, note names, or tuning systems: it takes a buffer of samples
//! and a sample rate, and reports the most likely fundamental frequency
//! together with a confidence measure and diagnostics.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "signals")]
#[cfg_attr(docsrs, doc(cfg(feature = "signals")))]
pub mod signals;

mod detector;
mod error;
mod estimate;

pub use detector::PitchDetector;
pub use error::{ConfigError, DetectError};
pub use estimate::{Diagnostics, PitchEstimate, UnvoicedReason, Voicing};
