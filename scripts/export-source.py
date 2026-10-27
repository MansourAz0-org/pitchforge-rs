#!/usr/bin/env python3
"""Export current source without Git history, ignored data, or owner metadata."""
import argparse
import gzip
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="Archive path outside the repository")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.expanduser().resolve()
    if output == root or root in output.parents:
        parser.error("Write the archive outside this repository")
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root
    ).split(b"\0")
    entries = []
    for raw in sorted(set(names)):
        if not raw:
            continue
        relative = Path(raw.decode("utf-8"))
        path = root / relative
        if relative.is_absolute() or ".." in relative.parts or ".git" in relative.parts:
            raise SystemExit("Unsafe source path; export stopped")
        if path.is_symlink():
            raise SystemExit(f"Review symlink before export: {relative}")
        if not path.exists():  # A tracked file deleted in the current working tree.
            continue
        if not path.is_file():
            raise SystemExit(f"Review non-file source entry: {relative}")
        if (relative.name == ".env" or
            relative.name.startswith(".env.") and relative.name != ".env.example" or
            relative.suffix.lower() in {".pem", ".key", ".p12", ".pfx", ".keystore", ".jks"} or
            relative.as_posix() == ".cargo/config.toml"):
            raise SystemExit(f"Private/local file is tracked; export stopped: {relative}")
        entries.append((relative, path))
    output.parent.mkdir(parents=True, exist_ok=True)
    # Zero timestamps and IDs; do not embed a local username or original filename.
    with output.open("wb") as destination:
        with gzip.GzipFile(filename="", mode="wb", fileobj=destination, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for relative, path in entries:
                    data = path.read_bytes()
                    info = tarfile.TarInfo(f"{root.name}/{relative.as_posix()}")
                    info.size = len(data)
                    info.mode = 0o755 if path.stat().st_mode & 0o111 else 0o644
                    info.mtime = info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    archive.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix(output.suffix + ".sha256").write_text(f"{digest}  {output.name}\n")
    print(f"Exported {len(entries)} source files: {output}")
    print(f"SHA-256: {digest}")


if __name__ == "__main__":
    main()
