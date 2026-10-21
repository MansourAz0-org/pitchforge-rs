# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/) (0.x: minor versions may
break the API).

## [Unreleased]

## [0.1.0] - 2026-10-21

### Added
- `Yin` detector: difference function (direct and FFT), cumulative mean
  normalised difference, first-dip absolute threshold with a looser voicing
  fallback, parabolic refinement, silence gate, out-of-range detection.
- `PitchDetector` trait, `PitchEstimate` with explicit `Voicing`, `Diagnostics`.
- `Framer` for streaming overlapping windows.
- `OnsetDetector` (energy-rise attack detection).
- `signals` feature: deterministic synthetic test signals.
- Accuracy test matrix, Criterion benchmarks, WAV analysis example.
