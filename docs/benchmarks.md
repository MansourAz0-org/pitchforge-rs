# Benchmarks

Per-window processing time of `Yin::detect`, measured with Criterion
(`cargo bench --bench detectors --features signals`).

## 2026-10-21 — pitchforge 0.1.0

Conditions: Apple M2 (MacBook), macOS, Rust 1.99.1, `--release`, f64
internals, 48 kHz, default search range 50–1400 Hz (τ_max = 961), input is a
synthetic pluck at 58.27 Hz. Criterion `--warm-up-time 1 --measurement-time 3`.

| Window | Direct | FFT (`realfft`) |
|-------:|-------:|----------------:|
| 2048   | 885 µs | 18.5 µs |
| 4096   | 2.66 ms | 37.7 µs |
| 8192   | 6.30 ms | 79.2 µs |

Silence-gated window (4096): 8.1 µs (input conversion + RMS only).

At a 512-sample hop (≈ 94 windows/s at 48 kHz), FFT YIN with a 4096 window
uses about 0.35 % of one core; direct summation would use about 25 %.

## Accuracy on synthetic signals

From `cargo test --release --features signals --test yin_accuracy -- --nocapture`,
48 kHz, window 4096, default configuration:

| Scenario | Windows | Octave / gross errors | Mean \|error\| | Max \|error\| |
|----------|--------:|----------------------:|---------------:|--------------:|
| Sine, 50–1400 Hz in quarter-tone steps | 115 | 0 / 0 | 0.002 c | 0.017 c |
| Sawtooth-like, 8 partials | 91 | 0 / 0 | 0.007 c | 0.063 c |
| Strong 2nd harmonic | 91 | 0 / 0 | 0.002 c | 0.018 c |
| Missing fundamental | 91 | 0 / 0 | 0.003 c | 0.035 c |
| Plucks (12 partials, B = 3e‑5), oud course frequencies | 315 | 0 / 0 | 0.41 c | 1.23 c |
| Harmonic + white noise, SNR 20 dB | 101 | 0 / 0 | 0.60 c | 3.2 c |
| Harmonic + white noise, SNR 10 dB | 101 | 0 / 0 | 3.4 c | 13.8 c |
| Vibrato ±20 c at 5.5 Hz (vs. instantaneous f) | 258 | 0 / 0 | ≈4 c | 7.0 c |
| Glide 110→220 Hz in 1 s (vs. instantaneous f) | 86 | 0 / 0 | 7.6 c | 8.9 c |
| Pure white noise, falsely voiced | 50 | — | — | 0 / 50 |

These are **synthetic** results. They do not establish accuracy on real
instrument recordings, which must be measured separately (see
`examples/analyze_wav.rs`). Vibrato/glide errors are dominated by the
~85 ms window averaging a moving frequency, not by estimator bias.
