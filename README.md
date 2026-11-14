# pitchforge

Find the pitch of one note at a time, in Rust.

Give `pitchforge` a short slice of audio samples. It estimates the note's
fundamental frequency in hertz (Hz), says whether a reliable pitch was found,
and returns diagnostic information. It works without a microphone or a UI.

**Early development:** the API can change between 0.x releases. This is a
monophonic detector: play one note at a time.

## Where it fits

| Repository | What it does |
| --- | --- |
| **pitchforge-rs** | Measures frequency from audio. This repository. |
| [naghma-core](https://github.com/MansourAZ0/naghma-core) | Turns frequency into Arabic/Latin solfège, quarter-tones and musical events. |
| [watar-tuner](https://github.com/MansourAZ0/watar-tuner) | A standalone oud tuner. |
| [Ajnas](https://github.com/MansourAZ0/Ajnas) | An oud practice app with teaching, maqam suggestions and optional camera tracking. |

Pitch detection itself does not need a Western or Eastern tuning system.
`pitchforge` keeps the frequency continuous; `naghma-core` adds the Arabic music
interpretation. See [credits and project additions](CREDITS.md).

## Try it

Install Rust, clone this repository, then run a generated example:

```sh
git clone https://github.com/MansourAZ0/pitchforge-rs.git
cd pitchforge-rs
cargo run --example basic --features signals
```

To analyse your own WAV recording:

```sh
cargo run --release --example analyze_wav -- recording.wav 4096 512
```

The last two numbers are the analysis window and hop, in samples. A window is
the short piece of audio being measured; a hop is how far it advances each time.

## Use it in Rust

The existing Git release is `v0.1.0`; it does not include later unreleased fixes.

```toml
[dependencies]
pitchforge = { git = "https://github.com/MansourAZ0/pitchforge-rs", tag = "v0.1.0" }
```

This complete example generates a 220 Hz tone and measures it:

```rust
use pitchforge::{PitchDetector, Yin, YinConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 48_000.0;
    let window: Vec<f32> = (0..4096)
        .map(|n| (std::f64::consts::TAU * 220.0 * n as f64 / sample_rate).sin() as f32)
        .collect();
    let mut detector = Yin::new(sample_rate, window.len(), YinConfig::default())?;
    let estimate = detector.detect(&window)?;
    match estimate.frequency() {
        Some(hz) => println!("{hz:.2} Hz"),
        None => println!("No reliable pitch: {:?}", estimate.voicing),
    }
    Ok(())
}
```

## What is implemented

- YIN pitch estimation, with an FFT path and a direct reference path.
- Frequency range, silence gate, confidence and explicit unvoiced reasons.
- Working buffers allocated once when the detector is created.
- `Framer` for overlapping windows and `OnsetDetector` for note attacks.
- Deterministic test signals behind the `signals` feature.

McLeod Pitch Method is planned; it is **not implemented**.

## Settings and limits

Defaults search 50–1400 Hz. At 48 kHz, a 4096-sample window covers about 85 ms;
a 512-sample hop produces a reading about every 11 ms. Frequent readings do not
remove the delay caused by observing a whole window.

Use `Yin::required_window_size` when changing the lowest frequency. The window
must hold at least two periods plus interpolation space. Silence, unsuitable
input and some out-of-range candidates produce no voiced frequency. A tone
outside the search range can still be mistaken for a harmonic inside it.

Confidence measures periodicity, not the probability that a note is correct.
Overlapping notes and abrupt changes can confuse the detector. Tests cover
quarter-tone sweeps, noise, vibrato and synthetic oud-like plucks; these do not
establish accuracy on every real instrument. Historical measurements and their
conditions are in [benchmarks](docs/benchmarks.md).

## Contribute

```sh
cargo test --all-features --locked
cargo test --no-default-features --locked
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo bench --bench detectors --features signals
```

The library declares Rust 1.85 as its minimum; use current stable Rust for the
full development toolchain. See [contributing](CONTRIBUTING.md)
and [security](SECURITY.md).

## Credits and license

YIN was developed by **Alain de Cheveigné and Hideki Kawahara**.
[Alessandro Genova's pitch-detection](https://github.com/alesgenova/pitch-detection)
is acknowledged as related prior work in Rust pitch detection. The available
history does not establish a fork of that library. [CREDITS.md](CREDITS.md)
explains the distinction and the additions across this ecosystem.

Project code is available under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option. Dependency licenses still apply; see [third-party notices](THIRD_PARTY_NOTICES.md).
