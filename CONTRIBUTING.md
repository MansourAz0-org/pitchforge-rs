# Contributing

Start with the README and [credits](CREDITS.md). Small bug fixes, clearer writing,
reproducible audio cases and musician-reviewed theory corrections are welcome.

1. Describe the problem and the expected behaviour in an issue or pull request.
2. Keep changes focused. Put general DSP in pitchforge, shared musical rules in
   naghma-core, and application-specific behaviour in the relevant app.
3. Add regression coverage for a behavioural fix. Prefer deterministic generated
   audio over private recordings. For musical data, include a source and explain
   whether it describes notation, a performance measurement, or a teaching choice.
4. Run the checks below. Include the commands and their results in the PR.

```sh
cargo fmt --all --check
cargo test --all-features --locked
cargo test --no-default-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

Use current stable Rust and, for JavaScript packages, Node 24+. Install native
Tauri/ALSA prerequisites where applicable. Do not replace labelled simulation
with something presented as live audio. Preserve continuous pitch, exact custom
intervals, provenance, and the distinction between player and sounding names.

## Contributions and licensing

By submitting a contribution, you agree to offer it under the repository's
MIT OR Apache-2.0 terms, unless the contribution explicitly documents different
existing terms. Retain third-party notices. Please be respectful, describe
technical disagreements clearly, and avoid personal attacks. Report security
issues through [SECURITY.md](SECURITY.md).
