# Credits and project additions

## Pitch detection

YIN comes from **Alain de Cheveigné and Hideki Kawahara**,
[*YIN, a fundamental frequency estimator for speech and music* (2002)](https://doi.org/10.1121/1.1458024).
The algorithm predates this project; it is not an invention of this repository.

We also acknowledge **Alessandro Genova** and
[`alesgenova/pitch-detection`](https://github.com/alesgenova/pitch-detection)
as related prior work in Rust pitch detection.

The available source history describes `pitchforge` as a separate YIN
implementation. It contains no documented upstream commit, fork relationship,
or import from Genova's library. This acknowledgement credits prior work; it
does not assert an unverified source-code derivation. If reused code is later
identified, preserve its original copyright and license notices and document
the exact source and modifications.

## What this repository contributes

`pitchforge` measures frequency. Its additions include explicit voiced/unvoiced results, diagnostics, configurable frequency bounds, an RMS silence gate, a secondary voicing threshold, minimum selection across a contiguous below-threshold dip, FFT/direct paths, streaming windows, onset detection, and deterministic test signals. It does not name notes or encode maqam theory.

## Adapting the ecosystem for Arabic music

A frequency such as 300 Hz is independent of a musical tradition. The detector
therefore stays general-purpose. The Eastern-music work is chiefly in
`naghma-core` and the apps built around it:

- Keep the measured pitch continuous instead of forcing it onto a semitone grid.
- Name quarter-tone accidentals in Arabic and Latin solfège.
- Allow non-equal-tempered targets and keep their original cents values.
- Keep an oud player's note names separate from the pitches that actually sound.
- Model fretless fingerboard positions and sourced maqam/jins alternatives.
- Allow intonation tolerance and overall tuning correction in maqam ranking.

The current scope is Arabic maqam and oud practice. A 24-step grid is a useful
approximation of written notation, not a universal description of Eastern music
or of Arabic performance intonation. These are this ecosystem's features, not a
verified list of patches applied to Genova's code.

## Musical and software sources

Maqam illustrations are documented in
[naghma-core's research record](https://github.com/MansourAZ0/naghma-core/blob/main/docs/MAQAMAT_RESEARCH.md),
including MaqamWorld and Sami Asmar's transposition reference. Preserve the
source URLs and provisional status when redistributing the JSON. Source score
images, page prose and third-party recordings are not bundled here.

Other foundations include RealFFT/RustFFT, Serde, CPAL, rtrb, React and Tauri
where used. Ajnas uses Google's MediaPipe Tasks Vision for hand landmarks.
See this repository's [third-party notices](THIRD_PARTY_NOTICES.md) and lockfiles
for the dependencies it actually uses. Attribution does not imply endorsement.
