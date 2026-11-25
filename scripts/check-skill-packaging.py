#!/usr/bin/env python3
"""Exercise skill packaging and postinstall routing without building/installing an App."""
import argparse
import importlib.util
import json
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path
from unittest.mock import patch

REPO = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, default=REPO / "src-tauri/target/debug/overstory-cli")
    args = parser.parse_args()
    cli = args.cli.resolve()
    version = json.loads((REPO / "package.json").read_text())["version"]
    with tempfile.TemporaryDirectory(prefix="overstory-skill-package-check-") as temporary:
        work = Path(temporary)
        repo = work / "repo"
        for name in ("src-tauri/tauri.conf.json", "skills/overstory/SKILL.md",
                     "skills/overstory-entry/SKILL.md", "scripts/installer/preinstall",
                     "scripts/installer/postinstall", "scripts/cli-manager.sh"):
            target = repo / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO / name, target)
        app = work / "Overstory.app"
        resources = app / "Contents/Resources"
        resources.mkdir(parents=True)
        (resources / "cli").mkdir()
        shutil.copy2(cli, resources / "cli/overstory-cli")
        entry = resources / "overstory-skill/SKILL.md"
        entry.parent.mkdir()
        entry.write_bytes((repo / "skills/overstory-entry/SKILL.md").read_bytes())
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
            "CFBundleIdentifier": "app.overstory.desktop", "CFBundleShortVersionString": version,
        }))
        spec = importlib.util.spec_from_file_location("overstory_package_check", REPO / "scripts/build-pkg.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.__file__ = str(repo / "scripts/build-pkg.py")

        def simulated_build(*arguments):
            command, *rest = map(str, arguments)
            if command == str((resources / "cli/overstory-cli").resolve()):
                subprocess.run([command, *rest], check=True, capture_output=True)
            elif command == "ditto":
                shutil.copytree(rest[0], rest[1])
            elif command == "codesign":
                pass  # Signatures are covered by check-pkg.py against a real built App.
            elif command == "pkgbuild" and "--analyze" in rest:
                Path(rest[-1]).write_bytes(plistlib.dumps([{"RootRelativeBundlePath": "Applications/Overstory.app"}]))
            elif command == "pkgbuild":
                components = plistlib.loads(Path(rest[rest.index("--component-plist") + 1]).read_bytes())
                assert not components[0]["BundleIsRelocatable"]
                scripts = Path(rest[rest.index("--scripts") + 1])
                assert "@VERSION@" not in (scripts / "postinstall").read_text()
                assert "--json skill install" in (scripts / "postinstall").read_text()
                Path(rest[-1]).write_text("simulated package")
            else:
                raise AssertionError(arguments)

        with patch.object(module, "run", simulated_build), patch.object(sys, "argv", ["build-pkg.py", "--app", str(app)]):
            module.main()
        with zipfile.ZipFile(repo / "release/overstory-skill.zip") as archive:
            assert archive.namelist() == ["overstory/SKILL.md"]
            assert archive.read("overstory/SKILL.md") == entry.read_bytes()
        entry.write_text("stale entry")
        with patch.object(module, "run", simulated_build), patch.object(sys, "argv", ["build-pkg.py", "--app", str(app)]):
            try:
                module.main()
                raise AssertionError("Stale App skill resource was accepted")
            except ValueError as error:
                assert "skill entry does not match source" in str(error)
        print("PASS: package assembly simulation, entry zip and stale bundled resource rejection")

        # Replace only OS commands in a disposable copy of the real postinstall template.
        # The installed CLI runs for real, against a temporary user's HOME.
        mock_bin = work / "mock commands"
        mock_bin.mkdir()
        user_home = work / "user's home"
        user_home.mkdir()
        installer_home = work / "root-home"
        installer_home.mkdir()
        calls = work / "sudo-calls.json"
        runner = mock_bin / "runner"
        runner.write_text(
            f"#!{sys.executable}\n"
            "import json, os, subprocess, sys\n"
            "from pathlib import Path\n"
            "name = Path(sys.argv[0]).name\n"
            "if name == 'stat': print(os.environ['QA_CONSOLE_USER'])\n"
            "elif name == 'dscl': print('mock directory service output')\n"
            "elif name == 'plutil': print(os.environ['QA_USER_HOME'])\n"
            "elif name == 'sudo':\n"
            "    args = sys.argv[1:]\n"
            "    assert args[:4] == ['-H', '-u', 'qa-user', '--'], args\n"
            "    Path(os.environ['QA_CALLS']).write_text(json.dumps(args))\n"
            "    env = {**os.environ, 'HOME': os.environ['QA_USER_HOME']}\n"
            "    sys.exit(subprocess.run(args[4:], env=env).returncode)\n"
        )
        runner.chmod(0o755)
        template = (REPO / "scripts/installer/postinstall").read_text().replace("@VERSION@", version)
        for command in ("stat", "dscl", "plutil", "sudo"):
            mock = mock_bin / command
            mock.symlink_to(runner)
            template = template.replace(f"/usr/bin/{command}", "'" + str(mock).replace("'", "'\\''") + "'")
        template = template.replace("/usr/local/bin/overstory", "'" + str(cli).replace("'", "'\\''") + "'")
        postinstall = work / "postinstall"
        postinstall.write_text(template)
        (work / "cli-manager.sh").write_text("#!/bin/sh\nexit 0\n")
        env = {**os.environ, "HOME": str(installer_home), "QA_CONSOLE_USER": "qa-user",
               "QA_USER_HOME": str(user_home), "QA_CALLS": str(calls)}

        def install(volume="/"):
            result = subprocess.run(["sh", str(postinstall), "package", "/", volume],
                                    capture_output=True, text=True, env=env)
            assert result.returncode == 0, result.stdout + result.stderr
            return result

        directory = user_home / ".agents/skills/overstory"
        install()
        assert (directory / "SKILL.md").read_bytes() == (REPO / "skills/overstory-entry/SKILL.md").read_bytes()
        assert not (installer_home / ".agents").exists()
        assert json.loads(calls.read_text())[:4] == ["-H", "-u", "qa-user", "--"]
        marker = directory / ".managed-by-overstory.json"
        receipt = json.loads(marker.read_text())
        receipt["version"] = "0.0.1"
        marker.write_text(json.dumps(receipt))
        install()
        assert json.loads(marker.read_text())["version"] == version
        (directory / "SKILL.md").write_text("user customization")
        assert "SKILL_CONFLICT" in install().stderr
        assert (directory / "SKILL.md").read_text() == "user customization"
        calls.unlink()
        env["QA_CONSOLE_USER"] = "root"
        install()
        assert not calls.exists()
        env["QA_CONSOLE_USER"] = "qa-user"
        env["QA_USER_HOME"] = "/var/root"
        install()
        assert not calls.exists()
        env["QA_USER_HOME"] = str(user_home)
        install(str(work / "offline volume"))
        assert not calls.exists()
        print("PASS: postinstall user routing, upgrade, root/offline skips and nonfatal skill conflict")


if __name__ == "__main__":
    main()
