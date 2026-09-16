#!/usr/bin/env python3
"""Package an already-built native binary; no downloads, signing or publishing."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin",
           "x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu")


def version():
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version = "([^"]+)"$', text, re.MULTILINE).group(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    release_version = version()
    if os.environ.get("GITHUB_REF_TYPE") == "tag":
        expected = "v" + release_version
        if os.environ.get("GITHUB_REF_NAME") != expected:
            raise SystemExit("tag must match Cargo.toml: " + expected)
    binary = args.binary.resolve(strict=True)
    demo = json.loads(subprocess.check_output([str(binary), "--dump-demo", "--lang=en"]))
    assert demo["type"] == "show"
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / ("hyper-palette-" + release_version + "-" + args.target + ".zip")
    # Refuse to overwrite release assets; reruns must use a fresh output folder.
    if archive.exists():
        raise SystemExit("asset already exists: " + str(archive))
    with tempfile.TemporaryDirectory(prefix="hyper-palette-package-") as temporary:
        stage = Path(temporary)
        for name in ("README.md", "RELEASING.md", "THIRD_PARTY.md"):
            shutil.copy2(ROOT / name, stage / name)
        if (ROOT / "LICENSE").exists():
            shutil.copy2(ROOT / "LICENSE", stage / "LICENSE")
        executable = "hyper-palette.exe" if "windows" in args.target else "hyper-palette"
        shutil.copy2(binary, stage / executable)
        (stage / executable).chmod(0o755)
        if "apple" in args.target:
            contents = stage / "Hyper Palette Prototype.app" / "Contents"
            macos = contents / "MacOS"
            macos.mkdir(parents=True)
            shutil.copy2(binary, macos / "hyper-palette")
            shutil.copy2(ROOT / "macos" / "demo", macos / "demo")
            (macos / "demo").chmod(0o755)
            info = plistlib.loads((ROOT / "macos" / "Info.plist").read_bytes())
            info["CFBundleVersion"] = release_version
            info["CFBundleShortVersionString"] = release_version
            (contents / "Info.plist").write_bytes(plistlib.dumps(info))
        manifest = {"version": release_version, "target": args.target,
                    "commit": os.environ.get("GITHUB_SHA", "local"), "protocol": 1,
                    "signed": False, "prototype": True}
        (stage / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        with zipfile.ZipFile(archive, "x", zipfile.ZIP_DEFLATED) as output:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    output.write(path, path.relative_to(stage))
    print(archive)


if __name__ == "__main__":
    main()
