//! YIN accuracy matrix on deterministic synthetic signals.
//!
//! Run with `cargo test --features signals --test yin_accuracy -- --nocapture`
//! to print the per-scenario report.
//!
//! These tests validate behaviour on *synthetic* signals only. They say
//! nothing about accuracy on real instrument recordings, which must be
//! evaluated separately.

mod common;

use common::*;
use pitchforge::signals::{self, PluckParams};
use pitchforge::{PitchDetector, UnvoicedReason, Voicing, Yin, YinConfig};

const LO: f64 = 50.0;
const HI: f64 = 1400.0;

fn yin(sr: f64, window: usize) -> Yin {
    Yin::new(sr, window, YinConfig::default()).unwrap()
}

fn one_window<D: PitchDetector>(det: &mut D, sig: &[f32], expected: f64) -> Outcome {
    // Skip the first window's worth of the signal so plucks have settled
    // their attack ramp but are still near full level.
    let w = det.window_size();
    classify(&det.detect(&sig[..w]).unwrap(), expected)
}

#[test]
fn clean_sines_quarter_tone_sweep() {
    for (sr, window) in [(48_000.0, 4096), (44_100.0, 4096), (48_000.0, 2048)] {
        let mut det = yin(sr, window);
        let mut stats = Stats::default();
        for f in quarter_tone_sweep(LO + 1.0, HI - 1.0) {
            let sig = signals::sine(f, sr, window, 0.5, 0.3);
            stats.push(one_window(&mut det, &sig, f));
        }
        stats.report(&format!("sine sweep sr={sr} W={window}"));
        assert_eq!(stats.ok, stats.n, "every clean sine must be detected");
        assert!(
            stats.max_abs_cents < 5.0,
            "max error {}",
            stats.max_abs_cents
        );
    }
}

#[test]
fn quarter_tones_are_resolved() {
    // Neighbouring 24-EDO steps are 50 cents apart; to place a note on the
    // right step with a meaningful cents read-out we need errors far below
    // 25 cents. Check that ±7-cent detunings are measured correctly.
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    for base in [65.41, 110.0, 146.83, 293.66, 587.33, 1174.66] {
        for detune in [-7.0, 0.0, 7.0, 50.0] {
            let f = base * 2f64.powf(detune / 1200.0);
            let sig = signals::sine(f, sr, 4096, 0.5, 0.0);
            let got = det.detect(&sig).unwrap().frequency().unwrap();
            let measured = cents(got, base);
            assert!(
                (measured - detune).abs() < 1.0,
                "{base} Hz {detune:+} c → measured {measured:+.2} c"
            );
        }
    }
}

#[test]
fn harmonic_rich_and_missing_fundamental() {
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    let cases: &[(&str, &[f32])] = &[
        (
            "sawtooth-like 1/k (8 partials)",
            &[1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.125],
        ),
        ("strong 2nd harmonic", &[0.3, 1.0, 0.5, 0.3]),
        ("missing fundamental", &[0.0, 1.0, 0.8, 0.6, 0.4]),
        ("strong 3rd harmonic", &[0.4, 0.3, 1.0, 0.2]),
    ];
    for (label, partials) in cases {
        let mut stats = Stats::default();
        // Keep the highest partial below Nyquist and the fundamental in range.
        for f in quarter_tone_sweep(LO + 1.0, 700.0) {
            let sig = signals::harmonic(f, sr, 4096, 0.5, partials);
            stats.push(one_window(&mut det, &sig, f));
        }
        stats.report(label);
        assert_eq!(
            stats.octave + stats.gross,
            0,
            "{label}: wrong-pitch detections"
        );
        assert!(stats.max_abs_cents < 5.0, "{label}");
    }
}

#[test]
fn oud_like_plucks() {
    // Open-course frequencies of a common oud tuning and the same tuning
    // lowered by 2 semitones, plus fingered notes up the neck. These are
    // only generic plucked-string signals at those frequencies.
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    let mut freqs: Vec<f64> = vec![65.41, 87.31, 110.0, 146.83, 196.0, 261.63];
    let lowered: Vec<f64> = freqs.iter().map(|f| f * 2f64.powf(-2.0 / 12.0)).collect();
    freqs.extend(lowered);
    freqs.extend([349.23, 523.25, 698.46]);

    let mut stats = Stats::default();
    for &f in &freqs {
        let sig = signals::pluck(f, sr, 48_000, 0.8, PluckParams::default());
        // Analyse several windows across the decay (after a 20 ms attack).
        stats.merge(&track(&mut det, &sig[960..], 2048, |_| f));
    }
    stats.report("plucks (12 partials, B=3e-5)");
    assert_eq!(stats.octave + stats.gross, 0);
    // Inharmonicity pulls the perceived period slightly sharp; stay well
    // inside a quarter-tone.
    assert!(stats.max_abs_cents < 10.0, "max {}", stats.max_abs_cents);
    assert!(stats.unvoiced * 10 < stats.n, "too many unvoiced windows");
}

#[test]
fn low_level_signals() {
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    // -40 dBFS peak sine (RMS ≈ -43 dBFS) is above the -60 dBFS gate.
    let sig = signals::sine(110.0, sr, 4096, 0.01, 0.0);
    let est = det.detect(&sig).unwrap();
    assert!(cents(est.frequency().unwrap(), 110.0).abs() < 1.0);
    // -70 dBFS peak is gated as silence.
    let sig = signals::sine(110.0, sr, 4096, 0.000_316, 0.0);
    assert_eq!(
        det.detect(&sig).unwrap().voicing,
        Voicing::Unvoiced(UnvoicedReason::Silence)
    );
}

#[test]
fn noisy_signals() {
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    for snr in [30.0, 20.0, 10.0] {
        let mut stats = Stats::default();
        for (i, f) in quarter_tone_sweep(LO + 5.0, 1000.0).into_iter().enumerate() {
            let mut sig = signals::harmonic(f, sr, 4096, 0.5, &[1.0, 0.5, 0.3, 0.2]);
            signals::add_noise(&mut sig, snr, i as u64);
            stats.push(one_window(&mut det, &sig, f));
        }
        stats.report(&format!("harmonic + white noise, SNR {snr} dB"));
        assert_eq!(stats.octave + stats.gross, 0, "SNR {snr}: wrong pitch");
        if snr >= 20.0 {
            assert_eq!(stats.ok, stats.n, "SNR {snr}: unvoiced windows");
            assert!(stats.max_abs_cents < 5.0);
        }
    }
}

#[test]
fn pure_noise_is_not_reported_as_pitch() {
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    let mut voiced = 0;
    for seed in 0..50 {
        let sig = signals::white_noise(4096, 0.3, seed);
        if det.detect(&sig).unwrap().is_voiced() {
            voiced += 1;
        }
    }
    println!("white noise: {voiced}/50 windows falsely voiced");
    assert_eq!(voiced, 0);
}

#[test]
fn vibrato_and_glides_are_tracked() {
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);

    // Oud-like vibrato: ±20 cents at 5.5 Hz.
    for f in [110.0, 293.66, 587.33] {
        let sig = signals::vibrato(f, sr, 48_000, 0.5, 20.0, 5.5);
        let stats = track(&mut det, &sig, 512, |n| {
            signals::vibrato_frequency_at(f, sr, 20.0, 5.5, n)
        });
        stats.report(&format!("vibrato ±20c 5.5Hz @ {f}"));
        assert_eq!(stats.ok, stats.n);
        // The window spans ~85 ms (almost half a vibrato cycle), so the
        // estimate is a smoothed version of the instantaneous frequency.
        assert!(stats.max_abs_cents < 15.0);
    }

    // One-octave glide over one second.
    let sig = signals::glide(110.0, 220.0, sr, 48_000, 0.5);
    let stats = track(&mut det, &sig, 512, |n| {
        signals::glide_frequency_at(110.0, 220.0, 48_000, n)
    });
    stats.report("glide 110→220 Hz in 1 s");
    assert_eq!(stats.ok, stats.n);
    assert!(stats.max_abs_cents < 10.0);
}

#[test]
fn note_change_does_not_produce_octave_errors() {
    // Two consecutive plucks: windows straddling the change may be unvoiced
    // or match either note, but must never be an octave or gross error
    // relative to *both* notes.
    let sr = 48_000.0;
    let mut det = yin(sr, 4096);
    let p = PluckParams::default();
    let a = signals::pluck(146.83, sr, 24_000, 0.8, p);
    let b = signals::pluck(196.0, sr, 24_000, 0.8, p);
    let sig = signals::concat(&[&a, &b]);
    let w = det.window_size();
    let mut start = 0;
    while start + w <= sig.len() {
        let est = det.detect(&sig[start..start + w]).unwrap();
        if let Some(f) = est.frequency() {
            let near = cents(f, 146.83).abs() < 50.0 || cents(f, 196.0).abs() < 50.0;
            assert!(near, "window at {start}: {f} Hz");
        }
        start += 512;
    }
}
