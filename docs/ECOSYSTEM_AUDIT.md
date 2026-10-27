# Four-repository audit — 2026-10-27

The four-repository split is sound. Keep generic DSP independent, keep Arabic
music interpretation in the shared library, and let Watar/Ajnas remain standalone
products. The main weakness is coordinated releases and validation across repos,
not the number of repositories.

## Ratings

These are engineering judgments of the working source after this preparation,
not user-study scores or certifications. Architecture considers boundaries,
dependency direction and reuse. Maturity also considers tests, failure handling,
reproducibility, documentation, platform evidence and distribution gaps.

| Repository | Architecture | Overall maturity | Main strength | Main remaining weakness |
| --- | --- | --- | --- | --- |
| pitchforge-rs | 8.5/10 | 8/10 | Small, hardware-free DSP API with synthetic accuracy tests. | Real-recording coverage and a new fix release. |
| naghma-core | 8.5/10 | 7.5/10 | Continuous pitch and clear Arabic-music domain boundaries. | Lifecycle/configuration hardening and musician validation. |
| watar-tuner | 8/10 | 7/10 | A thin independent tuner built on shared libraries. | Sparse UI tests, old library tags, Linux dependency warnings. |
| Ajnas | 8/10 | 7/10 | Well-separated vision modules and useful shared-core reuse. | Product/platform validation, orchestration complexity, model distribution terms. |

## Dependency map

```text
pitchforge-rs
    └── naghma-core repository
          ├── naghma-core: theory and note tracking
          ├── naghma-audio: capture, worker, analysis
          └── @naghma/ui: shared React rendering
                ├── watar-tuner
                └── Ajnas
```

More precisely, naghma-audio depends on pitchforge and the pure naghma-core crate;
@naghma/ui is a separate JavaScript package in the same repository. Watar and
Ajnas depend directly on the shared crates and UI, not on each other. Keep this
acyclic direction. The source checkouts are independent; there is no shared
four-repository Cargo workspace.

## Important fixes in this pass

- Reject unrepresentable YIN lag/window calculations.
- Validate equal-temperament JSON instead of bypassing constructor rules.
- Recover from non-finite audio windows without panicking the analyser.
- Upgrade Watar's vulnerable Tokio 1.54.0 lockfile entry to patched 1.54.1
  ([RUSTSEC-2026-0384](https://rustsec.org/advisories/RUSTSEC-2026-0384.html)).
- Disable Watar's development autostart in release binaries.
- Fix clean builds by explicitly installing each app's sibling shared-UI dependencies.
- Align Ajnas UI/CI to naghma v0.5.0 and Watar UI/CI to v0.3.0.
- Remove private-token CI setup and internal account information from current docs.
- Add simple READMEs, credits, privacy/security guidance, contribution guides,
  third-party notices, release instructions, ignore rules, dependency updates,
  secret scans and dependency audits to all four repos.
- Include license and notice files beside Ajnas's generated runtime assets.
- Provide source archives with no Git history, ignored assets or tar owner metadata.

## Attribution outcome

The verified algorithm source is YIN by Alain de Cheveigné and Hideki Kawahara
([paper](https://doi.org/10.1121/1.1458024)). Alessandro Genova's
[pitch-detection](https://github.com/alesgenova/pitch-detection) is the closest
identified Rust reference. Neither source history nor the original project
specification establishes a code fork or identifies a particular upstream
commit. All four repos credit it as related prior work, with the uncertainty
stated. It would be inaccurate to publish a verified fork claim.

Eastern-music additions are documented in every CREDITS.md: continuous pitch,
quarter-tones, Arabic/Latin solfège, custom intervals, player/sounding names, oud
geometry, sourced maqam variants, tolerance and tuning correction. Most are in
naghma-core and the applications, not changes that make generic YIN exclusively
Eastern. Current scope is Arabic maqam/oud, not every Eastern tradition.

## Privacy outcome

Gitleaks 8.30.1 found no credential patterns in scanned reachable history.
Tracked-file review found no home-directory paths, private email addresses,
credential files or recorded media in the current source. Public author names,
repository URLs, research citations and copyright attribution were retained.

Old Git histories still contain personal author/committer emails, and Ajnas's
old documentation retains an internal account note. A clean current tree does
not erase either. History was not rewritten or pushed. Use the generated
history-free source exports for a clean publication, or explicitly migrate and
review all history/refs before changing existing repository visibility.

## Validation and its limits

Local macOS validation used Rust 1.99.1 and Node 26.12.1. Across the primary
suites, **216 Rust/TypeScript tests including Rust doctests** pass (36 pitchforge,
90 naghma Rust, 11 shared UI, 13 Watar, 66 Ajnas). The no-default-feature detector
suite also passes. TypeScript checks, frontend production builds, Clippy with
warnings denied, formatting, vision asset integrity and source-export checks
were performed. Isolated frontend builds use the exact shared-UI tags specified
by each app. Package audits report no npm vulnerabilities.

After the Tokio fix, all four Cargo audits have zero vulnerability-class
findings. Both apps still produce GLib unsoundness and proc-macro-error maintenance
warnings. GLib comes through Tauri's Linux stack; it was not forced across
incompatible GTK versions. See the app reports before any Linux release.

This pass does not claim successful hosted CI, MSRV validation, physical-device
or packaged-app testing, new performance/corpus measurements, or binary signing.
The source preparation is reviewable; repository publication and public installer
release are separate outstanding actions.

## Release order and remaining work

Publish a new pitchforge fix tag, update/release naghma-core against it, then
update the apps' Rust tags, lockfiles and shared-UI CI tags together. Do not move
old tags. If publishing clean histories, reconstruct required source tags or use
new ones. Current app tags still use the old library implementations.

Confirm Ajnas model redistribution terms, resolve/review Linux dependency risk,
validate supported platforms, and collect complete binary dependency notices
before shipping installers. Per-repository reports:

- [pitchforge](AUDIT.md)
- [naghma-core](https://github.com/MansourAZ0/naghma-core/blob/main/docs/AUDIT.md)
- [Watar](https://github.com/MansourAZ0/watar-tuner/blob/main/docs/AUDIT.md)
- [Ajnas](https://github.com/MansourAZ0/Ajnas/blob/main/docs/AUDIT.md)
