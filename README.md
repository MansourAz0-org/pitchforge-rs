# pitchforge

Fundamental-frequency (f0) estimation for monophonic audio, written in pure Rust.

`pitchforge` takes a buffer of audio samples and a sample rate and returns the
estimated fundamental frequency, a confidence measure, a voicing decision, and
optional diagnostics. It is a general-purpose DSP library: it has **no**
knowledge of instruments, note names, or tuning systems, and it does not touch
audio hardware.

> Status: early development (0.x). The API may change between minor versions.

## Features

- Own implementation of **YIN** (de Cheveigné & Kawahara, 2002).
- **McLeod Pitch Method** (planned).
- Allocation-free detection after construction (buffers are preallocated).
- Explicit voicing decision: silence / aperiodic / out-of-range are reported,
  never guessed.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
