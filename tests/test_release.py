import hashlib
import json
import os
import stat
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import package


class ReleaseTests(unittest.TestCase):
    def test_archive_manifest_permissions_and_no_overwrite(self):
        for target in package.TARGETS:
            with self.subTest(target=target):
                self.check_archive(target)

    def check_archive(self, target):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            binary = base / "binary"
            binary.write_bytes(b"fixture, not an executable")
            args = ["package", "--target", target,
                    "--binary", str(binary), "--output", str(base / "dist")]
            with patch.object(sys, "argv", args), patch.dict(os.environ, {}, clear=True), \
                    patch.object(package.subprocess, "check_output", return_value=b'{"type":"show"}'), \
                    patch.object(Path, "chmod", return_value=None):
                # Simulate a host that cannot set Unix execute bits, even on Mac/Linux.
                package.main()
                with self.assertRaises(SystemExit):
                    package.main()
            with zipfile.ZipFile(next((base / "dist").glob("*.zip"))) as archive:
                manifest = json.loads(archive.read("manifest.json"))
                self.assertEqual(manifest["target"], target)
                self.assertEqual(manifest["version"], package.version())
                executables = {"hyper-palette.exe" if "windows" in target else "hyper-palette"}
                if "apple" in target:
                    executables.update({
                        "Hyper Palette Prototype.app/Contents/MacOS/demo",
                        "Hyper Palette Prototype.app/Contents/MacOS/hyper-palette",
                    })
                self.assertTrue(executables.issubset(archive.namelist()))
                for entry in archive.infolist():
                    self.assertEqual(entry.create_system, 3)
                    mode = entry.external_attr >> 16
                    self.assertTrue(stat.S_ISREG(mode))
                    self.assertEqual(stat.S_IMODE(mode),
                                     0o755 if entry.filename in executables else 0o644)
                    self.assertEqual(entry.compress_type, zipfile.ZIP_DEFLATED)

    def test_tag_mismatch_rejected_before_packaging(self):
        with patch.object(sys, "argv", ["package", "--target", "aarch64-apple-darwin",
                                      "--binary", "does-not-exist"]), \
                patch.dict(os.environ, {"GITHUB_REF_TYPE": "tag", "GITHUB_REF_NAME": "v999.0.0"}):
            with self.assertRaisesRegex(SystemExit, "tag must match"):
                package.main()

    def test_checksums_require_exact_asset_set(self):
        with tempfile.TemporaryDirectory() as folder:
            command = [sys.executable, str(ROOT / "scripts/checksums.py"), folder]
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
            for target in package.TARGETS:
                name = "hyper-palette-{}-{}.zip".format(package.version(), target)
                (Path(folder) / name).write_bytes(target.encode())
            subprocess.run(command, check=True)
            lines = (Path(folder) / "SHA256SUMS").read_text().splitlines()
            self.assertEqual(len(lines), 4)
            for line in lines:
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((Path(folder) / name).read_bytes()).hexdigest())
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)


if __name__ == "__main__":
    unittest.main()
