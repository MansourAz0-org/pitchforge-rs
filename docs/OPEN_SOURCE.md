# Open-source release preparation

## What is prepared

The README, credits, license links, contribution and security guidance, ignore
rules, dependency update configuration, and public CI have been prepared.
No repository visibility has been changed by this work. Local checks are not a
substitute for successful CI on the published source.

## Publish source without personal Git history

A source tree and its Git history are different things. History may contain
personal author/committer emails, old internal documentation, deleted files and
old release tags, even after the current files are cleaned. `.gitignore` does
not remove any of that. Existing histories have not been rewritten.

From the repository root, export the **current** source, including uncommitted
changes and new non-ignored files:

```sh
python3 scripts/export-source.py ../release-source/REPOSITORY-source.tar.gz
```

Replace `REPOSITORY` with this repository's name. The exporter uses Git's file
list, refuses local Cargo overrides and common key files, and omits `.git`,
ignored build outputs, local recordings, downloaded model assets and dependency
folders. Tar ownership fields and timestamps are cleared. It also writes a
SHA-256 file. This is an export helper, not a general secret detector: review
and scan the archive before publication.

Extract it into a new directory and initialise a fresh public repository if
you want to keep the old commit metadata private. Configure your GitHub noreply
commit email there before making the first commit. Keep the original repository
and remotes as a private backup. Do not force-push or remove the old history
without an explicit migration decision.

## Dependency release order

1. Publish the reviewed pitchforge source and its release tag.
2. Update naghma-core's pitchforge tag/lockfile, validate and release it.
3. Update each app's Rust dependency tags, lockfile and shared-UI CI tag.
4. Re-run each app's frontend and Rust checks, then publish its source.

If you preserve old public tags, new safety fixes still need **new tags**.
If you start with fresh histories, recreate the source releases required by the
chosen dependency tags or update dependencies to new tags before building.
The supplied clean archives are source snapshots, not drop-in replacements for
all historical tagged releases. Never move existing dependency tags.

No crates.io or npm publication is claimed. The applications' npm
`private: true` prevents accidental registry publication; it does not prevent
open-sourcing the applications on GitHub. For crates.io, Git dependencies need a
published registry version before dependent crates can be published.

## Maintainer actions on GitHub

- Publish the reviewed source and required dependency releases.
- Enable private vulnerability reporting, secret scanning and push protection
  where available; the repository's SECURITY.md links to private reporting.
- Require CI on the default branch and review who can publish releases.
- Verify a clean anonymous checkout can build without personal tokens, local
  Cargo patches or private fixture directories. Private CI retains conditional dependency authentication; public runs use
  anonymous fetches. DEPS_TOKEN needs read access across the private repositories.
- Run Gitleaks against the exact refs you will publish and against the exported
  files. Inspect author/committer metadata separately if publishing old history.

## Before distributing binaries

Retain all applicable third-party notices for the resolved dependencies. Test
physical audio permissions and device removal on supported platforms, and carry
out signing/notarization or installer checks where applicable. Synthetic tests
and a development bundle alone do not establish a supported public release.
