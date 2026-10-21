//! Detector output types shared by every algorithm.

/// Why a window was judged unvoiced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnvoicedReason {
    /// The RMS level was below the configured silence threshold. No pitch
    /// search was performed.
    Silence,
    /// A pitch search was performed but no candidate was periodic enough.
    Aperiodic,
    /// The best candidate lies outside the configured frequency range (for
    /// example because the period could not be refined at the edge of the lag
    /// search range).
    OutOfRange,
}

/// Voicing decision for one analysis window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voicing {
    /// A fundamental frequency was found with sufficient confidence.
    Voiced,
    /// No reliable fundamental frequency.
    Unvoiced(UnvoicedReason),
}

/// Result of analysing one window of audio.
///
/// Use [`PitchEstimate::frequency`] to get the fundamental frequency only when
/// the detector considered the window voiced. The raw best candidate is still
/// available through [`PitchEstimate::candidate`] for diagnostics, even when it
/// was rejected — callers must not present it as a confident pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchEstimate {
    /// Voicing decision.
    pub voicing: Voicing,
    /// Best fundamental-frequency candidate in Hz, if the search produced one.
    pub candidate: Option<f64>,
    /// Confidence in `[0, 1]` that `candidate` is the true fundamental.
    ///
    /// The scale is algorithm-specific (for YIN it is `1 − d′(τ)`, for McLeod
    /// it is the NSDF peak value). It is a periodicity measure, **not** a
    /// calibrated probability. `0.0` when there is no candidate.
    pub confidence: f64,
    /// Algorithm diagnostics.
    pub diagnostics: Diagnostics,
}

impl PitchEstimate {
    /// Fundamental frequency in Hz, or `None` if the window was unvoiced.
    #[inline]
    pub fn frequency(&self) -> Option<f64> {
        match self.voicing {
            Voicing::Voiced => self.candidate,
            Voicing::Unvoiced(_) => None,
        }
    }

    /// `true` if the window was judged voiced.
    #[inline]
    pub fn is_voiced(&self) -> bool {
        self.voicing == Voicing::Voiced
    }

    pub(crate) fn silence(rms: f64) -> Self {
        Self {
            voicing: Voicing::Unvoiced(UnvoicedReason::Silence),
            candidate: None,
            confidence: 0.0,
            diagnostics: Diagnostics {
                rms,
                ..Diagnostics::default()
            },
        }
    }
}

/// Algorithm-level diagnostic information for one analysis window.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Diagnostics {
    /// RMS level of the analysed window (after DC removal, if enabled).
    pub rms: f64,
    /// Refined period of the best candidate, in samples.
    pub period: Option<f64>,
    /// Integer lag of the best candidate before refinement.
    pub lag: Option<usize>,
    /// Algorithm-specific periodicity value at the chosen lag (YIN: the
    /// cumulative mean normalised difference `d′(τ)`, lower is more periodic;
    /// McLeod: the NSDF peak, higher is more periodic).
    pub periodicity: Option<f64>,
    /// Whether the candidate was selected by the primary threshold rule
    /// (`true`) or by a fallback such as the global minimum (`false`).
    pub threshold_met: bool,
}
