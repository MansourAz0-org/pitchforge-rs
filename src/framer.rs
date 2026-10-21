//! Turning a continuous sample stream into overlapping analysis windows.

use crate::ConfigError;

/// Collects samples arriving in arbitrary-sized chunks and yields
/// overlapping windows of `window` samples every `hop` samples.
///
/// Memory is allocated once in [`Framer::new`]; [`Framer::push`] does not
/// allocate.
///
/// ```
/// use pitchforge::Framer;
///
/// let mut framer = Framer::new(4, 2).unwrap();
/// let mut windows = Vec::new();
/// framer.push(&[1.0, 2.0, 3.0], |w, start| windows.push((start, w.to_vec())));
/// framer.push(&[4.0, 5.0, 6.0, 7.0], |w, start| windows.push((start, w.to_vec())));
/// assert_eq!(
///     windows,
///     vec![
///         (0, vec![1.0, 2.0, 3.0, 4.0]),
///         (2, vec![3.0, 4.0, 5.0, 6.0]),
///     ]
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Framer {
    window: usize,
    hop: usize,
    buf: Vec<f32>,
    fill: usize,
    /// Absolute index (in the pushed stream) of `buf[0]`.
    start: u64,
}

impl Framer {
    /// Create a framer. `hop` must be in `1..=window`.
    pub fn new(window: usize, hop: usize) -> Result<Self, ConfigError> {
        if window == 0 || hop == 0 || hop > window {
            return Err(ConfigError::InvalidParameter {
                name: "hop",
                value: hop as f64,
            });
        }
        Ok(Self {
            window,
            hop,
            buf: vec![0.0; window],
            fill: 0,
            start: 0,
        })
    }

    /// Window length in samples.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Hop length in samples.
    pub fn hop(&self) -> usize {
        self.hop
    }

    /// Append samples, calling `on_window(window, start_index)` for every
    /// complete window. `start_index` is the position of the window's first
    /// sample in the stream of all samples pushed since creation or the last
    /// [`reset`](Framer::reset).
    pub fn push(&mut self, mut samples: &[f32], mut on_window: impl FnMut(&[f32], u64)) {
        while !samples.is_empty() {
            let n = (self.window - self.fill).min(samples.len());
            self.buf[self.fill..self.fill + n].copy_from_slice(&samples[..n]);
            self.fill += n;
            samples = &samples[n..];
            if self.fill == self.window {
                on_window(&self.buf, self.start);
                // Drop the oldest `hop` samples. Copying `window − hop`
                // samples per hop is cheaper than the analysis that follows
                // and keeps every window contiguous for the detectors.
                self.buf.copy_within(self.hop.., 0);
                self.fill = self.window - self.hop;
                self.start += self.hop as u64;
            }
        }
    }

    /// Discard buffered samples and restart stream indexing at zero.
    pub fn reset(&mut self) {
        self.fill = 0;
        self.start = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_does_not_change_output() {
        let stream: Vec<f32> = (0..1000).map(|x| x as f32).collect();
        let collect = |chunk: usize| {
            let mut f = Framer::new(64, 16).unwrap();
            let mut out = Vec::new();
            for c in stream.chunks(chunk) {
                f.push(c, |w, s| out.push((s, w[0], w[63])));
            }
            out
        };
        let reference = collect(1000);
        assert_eq!(reference.len(), (1000 - 64) / 16 + 1);
        for (i, &(start, first, last)) in reference.iter().enumerate() {
            assert_eq!(start, (i * 16) as u64);
            assert_eq!(first, start as f32);
            assert_eq!(last, (start + 63) as f32);
        }
        for chunk in [1, 7, 16, 63, 64, 65, 333] {
            assert_eq!(collect(chunk), reference, "chunk size {chunk}");
        }
    }

    #[test]
    fn hop_equal_to_window_is_non_overlapping() {
        let mut f = Framer::new(4, 4).unwrap();
        let mut starts = Vec::new();
        f.push(&[0.0; 12], |_, s| starts.push(s));
        assert_eq!(starts, vec![0, 4, 8]);
    }

    #[test]
    fn rejects_invalid_hop() {
        assert!(Framer::new(4, 0).is_err());
        assert!(Framer::new(4, 5).is_err());
        assert!(Framer::new(0, 0).is_err());
    }
}
