//! Window preparation shared by the detectors.

use crate::DetectError;

/// Copy `input` into `out` as `f64`, optionally removing the mean (DC
/// offset), and return the RMS of the result.
///
/// Rejects non-finite input: a single NaN would otherwise silently poison
/// every downstream sum.
pub(crate) fn prepare(input: &[f32], out: &mut [f64], remove_dc: bool) -> Result<f64, DetectError> {
    debug_assert_eq!(input.len(), out.len());
    let mut sum = 0.0f64;
    for (o, &x) in out.iter_mut().zip(input) {
        let x = x as f64;
        if !x.is_finite() {
            return Err(DetectError::NonFiniteInput);
        }
        *o = x;
        sum += x;
    }
    if remove_dc && !out.is_empty() {
        let mean = sum / out.len() as f64;
        out.iter_mut().for_each(|x| *x -= mean);
    }
    let energy: f64 = out.iter().map(|x| x * x).sum();
    Ok(if out.is_empty() {
        0.0
    } else {
        (energy / out.len() as f64).sqrt()
    })
}

/// Convert an RMS value to dBFS (0 dBFS = RMS of 1.0). Returns `-inf` for 0.
#[inline]
pub(crate) fn rms_to_dbfs(rms: f64) -> f64 {
    20.0 * rms.log10()
}

/// Vertex offset of the parabola through `(-1, y0)`, `(0, y1)`, `(1, y2)`.
///
/// Returns a value in `[-1, 1]`; `0` when the three points are collinear.
#[inline]
pub(crate) fn parabolic_offset(y0: f64, y1: f64, y2: f64) -> f64 {
    let denom = y0 - 2.0 * y1 + y2;
    if denom.abs() <= f64::EPSILON * (y0.abs() + y1.abs() + y2.abs()).max(f64::MIN_POSITIVE) {
        return 0.0;
    }
    (0.5 * (y0 - y2) / denom).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_dc_and_measures_rms() {
        let input = [1.5f32, 0.5, 1.5, 0.5];
        let mut out = [0.0; 4];
        let rms = prepare(&input, &mut out, true).unwrap();
        assert_eq!(out, [0.5, -0.5, 0.5, -0.5]);
        assert!((rms - 0.5).abs() < 1e-12);
    }

    #[test]
    fn rejects_nan() {
        let mut out = [0.0; 3];
        assert_eq!(
            prepare(&[0.0, f32::NAN, 0.0], &mut out, false),
            Err(DetectError::NonFiniteInput)
        );
    }

    #[test]
    fn parabola_vertex() {
        // y = (x - 0.25)^2 sampled at -1, 0, 1.
        let f = |x: f64| (x - 0.25) * (x - 0.25);
        assert!((parabolic_offset(f(-1.0), f(0.0), f(1.0)) - 0.25).abs() < 1e-12);
        assert_eq!(parabolic_offset(1.0, 1.0, 1.0), 0.0);
    }
}
