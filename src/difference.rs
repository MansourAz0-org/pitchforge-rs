//! Computation of the YIN difference function
//! `d(τ) = Σ_{j=0}^{W−1} (x_j − x_{j+τ})²`.

/// How the YIN difference function is computed.
///
/// Both methods compute the same quantity up to floating-point rounding; the
/// choice only affects speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DifferenceMethod {
    /// Direct summation, `O(W · τ_max)`. Simple reference implementation.
    #[cfg_attr(not(feature = "fft"), default)]
    Direct,
    /// Expansion into energy terms plus an FFT cross-correlation,
    /// `O(N log N)`. Requires the `fft` feature.
    #[cfg(feature = "fft")]
    #[cfg_attr(docsrs, doc(cfg(feature = "fft")))]
    #[default]
    Fft,
}

/// Direct summation into `diff[0..=tau_max]`.
pub(crate) fn direct(x: &[f64], integration: usize, diff: &mut [f64]) {
    diff[0] = 0.0;
    for (tau, d) in diff.iter_mut().enumerate().skip(1) {
        let mut acc = 0.0;
        for j in 0..integration {
            let delta = x[j] - x[j + tau];
            acc += delta * delta;
        }
        *d = acc;
    }
}

#[cfg(feature = "fft")]
pub(crate) use fft::FftDifference;

#[cfg(feature = "fft")]
mod fft {
    use realfft::num_complex::Complex;
    use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
    use std::sync::Arc;

    /// FFT-based difference function with preallocated buffers.
    ///
    /// Expanding the square gives
    ///
    /// `d(τ) = Σ_{j<W} x_j²  +  Σ_{j<W} x_{j+τ}²  −  2 Σ_{j<W} x_j x_{j+τ}`
    ///
    /// The first two terms come from a prefix sum of squares; the third is the
    /// cross-correlation of the first `W` samples with the whole window, which
    /// is computed as `IFFT(conj(FFT(a)) · FFT(x))`. Because `j + τ < N` for
    /// every term we need, an FFT of length `N` (the window size) has no
    /// circular wrap-around in the lags we read.
    pub(crate) struct FftDifference {
        forward: Arc<dyn RealToComplex<f64>>,
        inverse: Arc<dyn ComplexToReal<f64>>,
        len: usize,
        time_a: Vec<f64>,
        time_x: Vec<f64>,
        spec_a: Vec<Complex<f64>>,
        spec_x: Vec<Complex<f64>>,
        scratch_fwd: Vec<Complex<f64>>,
        scratch_inv: Vec<Complex<f64>>,
        prefix: Vec<f64>,
    }

    impl Clone for FftDifference {
        fn clone(&self) -> Self {
            Self::new(self.len)
        }
    }

    impl core::fmt::Debug for FftDifference {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("FftDifference")
                .field("len", &self.len)
                .finish()
        }
    }

    impl FftDifference {
        pub(crate) fn new(len: usize) -> Self {
            // realfft handles odd lengths too, but even lengths are faster.
            let mut planner = RealFftPlanner::<f64>::new();
            let forward = planner.plan_fft_forward(len);
            let inverse = planner.plan_fft_inverse(len);
            Self {
                time_a: forward.make_input_vec(),
                time_x: forward.make_input_vec(),
                spec_a: forward.make_output_vec(),
                spec_x: forward.make_output_vec(),
                scratch_fwd: forward.make_scratch_vec(),
                scratch_inv: inverse.make_scratch_vec(),
                prefix: vec![0.0; len + 1],
                forward,
                inverse,
                len,
            }
        }

        /// Fill `diff[0..=tau_max]` for window `x` (length `len`) with
        /// integration window `w`.
        pub(crate) fn compute(&mut self, x: &[f64], w: usize, diff: &mut [f64]) {
            debug_assert_eq!(x.len(), self.len);
            let tau_max = diff.len() - 1;
            debug_assert!(w + tau_max <= self.len);

            // Prefix sum of squares: prefix[k] = Σ_{j<k} x_j².
            self.prefix[0] = 0.0;
            for (k, &v) in x.iter().enumerate() {
                self.prefix[k + 1] = self.prefix[k] + v * v;
            }

            self.time_a[..w].copy_from_slice(&x[..w]);
            self.time_a[w..].fill(0.0);
            self.time_x.copy_from_slice(x);

            // These calls only fail on buffer-length mismatches, which the
            // constructor rules out.
            self.forward
                .process_with_scratch(&mut self.time_a, &mut self.spec_a, &mut self.scratch_fwd)
                .expect("forward FFT buffer sizes");
            self.forward
                .process_with_scratch(&mut self.time_x, &mut self.spec_x, &mut self.scratch_fwd)
                .expect("forward FFT buffer sizes");

            for (a, x) in self.spec_a.iter_mut().zip(&self.spec_x) {
                *a = a.conj() * x;
            }
            // The DC and Nyquist bins of a real signal's spectrum are real;
            // clear rounding residue so the inverse transform accepts them.
            if let Some(first) = self.spec_a.first_mut() {
                first.im = 0.0;
            }
            if self.len % 2 == 0 {
                if let Some(last) = self.spec_a.last_mut() {
                    last.im = 0.0;
                }
            }
            // Reuse time_a for the correlation output.
            self.inverse
                .process_with_scratch(&mut self.spec_a, &mut self.time_a, &mut self.scratch_inv)
                .expect("inverse FFT buffer sizes");

            let scale = 1.0 / self.len as f64;
            let energy_head = self.prefix[w];
            diff[0] = 0.0;
            for (tau, d) in diff.iter_mut().enumerate().skip(1) {
                let energy_lag = self.prefix[tau + w] - self.prefix[tau];
                let corr = self.time_a[tau] * scale;
                // Clamp: rounding can make a near-zero result slightly negative.
                *d = (energy_head + energy_lag - 2.0 * corr).max(0.0);
            }
        }
    }
}
