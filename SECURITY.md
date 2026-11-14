# Security and privacy

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/MansourAZ0/pitchforge-rs/security/advisories/new)
if it is enabled. Include the affected revision, reproduction steps and impact.
Do not put credentials, private recordings, personal file paths or device IDs in
a public issue. If private reporting is unavailable, ask the maintainer to enable
it without posting exploit details. There is no guaranteed response time or LTS
support; development targets the current branch and upcoming 0.x releases.

## Data handling

The library performs no network, filesystem, microphone or camera I/O. Callers control any audio they supply. WAV examples read the file you select.

## Before sharing a bug report

Use synthetic input when possible. Remove home-directory paths, account details,
device identifiers and private dataset content from logs. Share recordings or
camera frames only when you have permission from the people and rights holders
involved. Build-time package downloads are separate from runtime processing.

## Keeping secrets out of source

Do not commit `.env` files, access tokens, signing certificates, private keys,
local settings, calibration files or recordings. Ignore rules reduce accidents;
they do not remove a file already committed. A leaked credential must be revoked
and replaced, including if it only exists in old Git history.

History scans and dependency checks help find known patterns; they do not prove
the absence of vulnerabilities.
