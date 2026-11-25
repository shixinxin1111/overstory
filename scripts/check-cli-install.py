#!/usr/bin/env python3
"""Exercise installer operations on disposable volumes, without sudo or system changes."""
import plistlib
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="overstory-install-test-")
        self.addCleanup(self.temp.cleanup)
        self.work = Path(self.temp.name)
        self.volume = self.work / "volume with spaces"
        self.volume.mkdir()
        self.source = self.work / "source 'quoted' cli"
        self.fake_cli("0.1.0")
        self.command = self.volume / "usr/local/bin/overstory"
        self.binary = self.volume / "usr/local/lib/overstory/overstory-cli"
        self.data = self.volume / "Users/test/Library/Application Support/app.overstory.desktop/state.json"
        self.data.parent.mkdir(parents=True)
        self.data.write_text("user data must survive")

    def fake_cli(self, version):
        self.source.write_text(f"#!/bin/sh\nprintf 'overstory {version}\\n'\n")
        self.source.chmod(0o755)

    def manage(self, operation, version="0.1.0", expected=0):
        result = subprocess.run(["sh", str(REPO / "scripts/cli-manager.sh"), operation,
                                 str(self.source), version, str(self.volume)], capture_output=True, text=True)
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return result

    def test_repeat_upgrade_uninstall_preserves_data(self):
        self.manage("install")
        self.assertEqual(self.command.readlink(), Path("/usr/local/lib/overstory/overstory-cli"))
        self.manage("install")
        self.fake_cli("0.2.0")
        self.manage("install", "0.2.0")
        self.assertIn("0.2.0", self.binary.read_text())
        self.fake_cli("0.1.0")
        self.manage("check", expected=1)
        self.manage("install", expected=1)
        self.assertIn("0.2.0", self.binary.read_text())
        self.manage("uninstall")
        self.assertFalse(self.command.is_symlink())
        self.assertFalse(self.binary.exists())
        self.assertEqual(self.data.read_text(), "user data must survive")
        self.manage("install")

    def test_foreign_path_is_never_overwritten_or_uninstalled(self):
        self.command.parent.mkdir(parents=True)
        self.command.write_text("foreign command")
        for op in ("check", "install", "uninstall"):
            self.manage(op, expected=1)
        self.assertEqual(self.command.read_text(), "foreign command")

    def test_symlink_parent_is_rejected(self):
        (self.volume / "usr").symlink_to(self.work, target_is_directory=True)
        self.manage("install", expected=1)

    def test_pkg_preflight_and_postinstall(self):
        scripts = self.work / "scripts"
        scripts.mkdir()
        for name in ("preinstall", "postinstall"):
            (scripts / name).write_text((REPO / "scripts/installer" / name).read_text().replace("@VERSION@", "0.1.0"))
        (scripts / "cli-manager.sh").write_text((REPO / "scripts/cli-manager.sh").read_text())
        app = self.volume / "Applications/Overstory.app/Contents"
        app.mkdir(parents=True)
        (app / "Info.plist").write_bytes(plistlib.dumps({"CFBundleShortVersionString": "0.1.0"}))
        (app / "Resources/cli").mkdir(parents=True)
        bundled = app / "Resources/cli/overstory-cli"
        bundled.write_bytes(self.source.read_bytes())
        bundled.chmod(0o755)
        for _ in range(2):
            for name in ("preinstall", "postinstall"):
                result = subprocess.run(["sh", str(scripts / name), "package", "/Applications", str(self.volume)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
        (app / "Info.plist").write_bytes(plistlib.dumps({"CFBundleShortVersionString": "0.2.0"}))
        result = subprocess.run(["sh", str(scripts / "preinstall"), "package", "/", str(self.volume)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.data.read_text(), "user data must survive")


if __name__ == "__main__":
    unittest.main(verbosity=2)
