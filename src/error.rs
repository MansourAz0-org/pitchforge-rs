//! Error types.

use core::fmt;

/// A detector configuration that cannot work.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConfigError {
    /// The sample rate is not a positive finite number.
    InvalidSampleRate(f64),
    /// The frequency range is empty, non-finite, or outside `(0, Nyquist)`.
    InvalidFrequencyRange {
        /// Requested lower bound in Hz.
        min: f64,
        /// Requested upper bound in Hz.
        max: f64,
    },
    /// A threshold parameter is outside its valid range.
    InvalidThreshold {
        /// Parameter name.
        name: &'static str,
        /// Rejected value.
        value: f64,
    },
    /// The analysis window is too short to observe the lowest requested
    /// frequency.
    WindowTooShort {
        /// Minimum window length in samples for this configuration.
        required: usize,
        /// Requested window length.
        actual: usize,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate(sr) => write!(f, "invalid sample rate: {sr}"),
            Self::InvalidFrequencyRange { min, max } => {
                write!(f, "invalid frequency range: {min} Hz .. {max} Hz")
            }
            Self::InvalidThreshold { name, value } => {
                write!(f, "invalid value for {name}: {value}")
            }
            Self::WindowTooShort { required, actual } => write!(
                f,
                "window of {actual} samples is too short; at least {required} are required \
                 for the configured minimum frequency"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// An error while analysing a window.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DetectError {
    /// The input slice length does not match the detector's window size.
    WindowLength {
        /// Window size the detector was built for.
        expected: usize,
        /// Length of the slice passed in.
        actual: usize,
    },
    /// The input contains NaN or infinite samples.
    NonFiniteInput,
}

impl fmt::Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowLength { expected, actual } => {
                write!(f, "expected a window of {expected} samples, got {actual}")
            }
            Self::NonFiniteInput => write!(f, "input contains NaN or infinite samples"),
        }
    }
}

impl std::error::Error for DetectError {}
