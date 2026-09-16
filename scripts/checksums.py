#!/usr/bin/env python3
"""Fail closed unless all four platform archives exist, then hash them."""
import hashlib
from pathlib import Path
import sys
from package import TARGETS, version

directory = Path(sys.argv[1])
expected = {"hyper-palette-{}-{}.zip".format(version(), target) for target in TARGETS}
actual = {path.name for path in directory.glob("*.zip")}
if actual != expected:
    raise SystemExit("wrong release asset set: " + repr(actual ^ expected))
lines = []
for name in sorted(expected):
    digest = hashlib.sha256()
    with (directory / name).open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    lines.append(digest.hexdigest() + "  " + name)
with (directory / "SHA256SUMS").open("x", encoding="utf-8") as output:
    output.write("\n".join(lines) + "\n")
