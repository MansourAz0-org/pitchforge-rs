# Repository audit — 2026-10-27

Baseline commit: `d59d6cc712d8` plus the local preparation changes.

**Architecture: 8.5/10. Overall engineering maturity: 8/10.** Scores describe
this working source, not a verified production release. The strongest of the four
repositories for a small, reusable public library.

## Architecture

The dependency direction is clean: `PitchDetector` → `Yin` → preprocessing and
FFT/direct difference computation. `PitchEstimate` keeps a rejected candidate
separate from a voiced frequency. `Framer` and `OnsetDetector` are independent
streaming helpers. Musical names, Tauri and microphone capture stay outside.
Buffers are preallocated for detection, and unsafe project code is forbidden.

Evidence: [public API](../src/lib.rs), [detector](../src/detector.rs),
[YIN](../src/yin.rs), [difference paths](../src/difference.rs),
[synthetic accuracy tests](../tests/yin_accuracy.rs).

## Findings and changes

| Priority | Finding | Result |
| --- | --- | --- |
| High | A positive but extremely small minimum frequency could overflow lag/window arithmetic. | Fixed: reject an unrepresentable period before conversion; regression test added. |
| Medium | Crate metadata advertised McLeod before it existed. | Fixed description and README; McLeod remains explicitly planned. |
| Medium | Original algorithm credit existed in code but broader provenance was unclear. | Added CREDITS and third-party notices; YIN authors are verified, Genova is acknowledged as related prior work without claiming a fork. |
| Medium | Public contribution, security and release guidance were missing. | Added guides, secret/dependency CI, Dependabot and broader ignore rules. |
| Privacy | Old commits contain personal email metadata. | Prepared history-free source export; old Git history remains intact. |

## Remaining work

- Establish detector accuracy on appropriately licensed real recordings and
  difficult attacks, sympathetic strings, low fundamentals and background noise.
- Keep harmonic/subharmonic mistakes and mixed-note windows visible in docs;
  periodicity confidence is not a calibrated correctness probability.
- Broaden extreme-parameter and non-finite-input coverage for public helpers,
  including onset processing; this pass does not establish every input invariant.
- Publish the reviewed fixes under a new tag before updating dependent repos.
  Existing `v0.1.0` does not contain working-tree fixes.

## Validation

On macOS, Rust 1.99.1: all-feature tests **24 unit + 9 integration + 3 doctests**;
no-default-feature tests **14 unit + 3 doctests**; Clippy with warnings denied;
formatting checks. RustSec audit reports no vulnerabilities or informational
warnings in this lockfile. Gitleaks 8.30.1 reports no credential findings in the
scanned reachable history and the release source snapshot.

This pass did not rerun benchmarks, validate Rust 1.85, run physical audio tests,
or execute hosted Linux/Windows CI. Historical benchmark numbers remain labelled
with their original conditions. See [release preparation](OPEN_SOURCE.md).
