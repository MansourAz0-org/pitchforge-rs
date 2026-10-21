# pitchforge

Fundamental-frequency (f0) estimation for monophonic audio, in pure Rust.

`pitchforge` takes a buffer of audio samples and a sample rate and returns the
estimated fundamental frequency, a confidence measure, an explicit voicing
decision and diagnostics. It is a general-purpose DSP library: it has **no**
knowledge of instruments, note names or tuning systems, and it does not touch
audio hardware.

> **Status:** 0.x, early development. Minor versions may change the API.

## Features

- **YIN** (de Cheveigné & Kawahara, 2002), our own implementation, with an
  O(N log N) FFT difference function (default) and a direct reference path.
- **McLeod Pitch Method** — planned for 0.2.
- Allocation-free detection after construction.
- Honest voicing: silence, aperiodic and out-of-range windows are reported as
  unvoiced with a reason, never guessed.
- `Framer` for streaming windows and `OnsetDetector` for note attacks.
- Deterministic synthetic signals (`signals` feature) for testing.

## Usage

```toml
[dependencies]
pitchforge = { git = "https://github.com/MansourAZ0/pitchforge-rs", tag = "v0.1.0" }
```

```rust
use pitchforge::{PitchDetector, Yin, YinConfig};

let mut yin = Yin::new(48_000.0, 4096, YinConfig::default())?;
let estimate = yin.detect(&window)?;          // window: &[f32], 4096 samples
match estimate.frequency() {
    Some(f0) => println!("{f0:.2} Hz, confidence {:.2}", estimate.confidence),
    None => println!("no pitch: {:?}", estimate.voicing),
}
```

See [`examples/`](examples) — `analyze_wav` tracks the pitch of a WAV file:

```sh
cargo run --release --example analyze_wav -- recording.wav 4096 512
```

## Choosing parameters

| Parameter | Default | Notes |
|-----------|---------|-------|
| `min_frequency` / `max_frequency` | 50 / 1400 Hz | The window must hold ≥ 2 periods of `min_frequency` (`Yin::required_window_size`). |
| window | — | 4096 at 48 kHz is a good default for low notes (≈ 85 ms); 2048 halves latency but needs `min_frequency` ≥ ~47 Hz. |
| `threshold` | 0.15 | First CMNDF dip below this is taken. Lower = stricter. |
| `voicing_threshold` | 0.25 | Dips between the two thresholds are voiced with `threshold_met = false`. |
| `silence_threshold_db` | −60 dBFS | RMS gate. |

## Accuracy and performance

Measured numbers and conditions are in [docs/benchmarks.md](docs/benchmarks.md).
Summary (synthetic signals, 48 kHz, window 4096): < 0.1 cent error on clean
periodic signals 50–1400 Hz, zero octave errors across the test matrix,
≈ 38 µs per window on an Apple M2.

### Known limitations

- Windows that straddle an abrupt note change report a frequency between the
  two notes, often with high confidence. Use onset detection and temporal
  stability checks downstream.
- Synthetic accuracy does not imply accuracy on real recordings.
- Monophonic only.

## Development

```sh
cargo test --all-features
cargo test --release --features signals --test yin_accuracy -- --nocapture
cargo bench --bench detectors --features signals
```

MSRV: Rust 1.85.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
