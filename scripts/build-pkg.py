#!/usr/bin/env python3
"""Build a repeatable App+CLI PKG; staging stays outside the source tree."""
import argparse
import json
import plistlib
import re
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def main():
    repo = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", type=Path, default=repo / "src-tauri/target/release/bundle/macos/Overstory.app")
    args = parser.parse_args()
    app = args.app.resolve()
    version = json.loads((repo / "src-tauri/tauri.conf.json").read_text())["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("PKG requires a numeric x.y.z version")
    if not app.is_dir():
        raise FileNotFoundError(f"Build the App first: {app}")
    with (app / "Contents/Info.plist").open("rb") as file:
        info = plistlib.load(file)
    if info["CFBundleIdentifier"] != "app.overstory.desktop" or info["CFBundleShortVersionString"] != version:
        raise ValueError("App identity/version does not match this release")
    cli = app / "Contents/Resources/cli/overstory-cli"
    if subprocess.check_output([str(cli), "--version"], text=True).strip() != f"overstory {version}":
        raise ValueError("Bundled CLI version does not match App")
    skill = json.loads(subprocess.check_output([str(cli), "--json", "skill", "show"], text=True))
    if not skill["ok"] or skill["data"]["content"] != (repo / "skills/overstory/SKILL.md").read_text():
        raise ValueError("Bundled CLI skill does not match source; rebuild the App")
    entry = app / "Contents/Resources/overstory-skill/SKILL.md"
    if entry.read_bytes() != (repo / "skills/overstory-entry/SKILL.md").read_bytes():
        raise ValueError("Bundled skill entry does not match source; rebuild the App")
    output = repo / "release" / f"Overstory-{version}.pkg"
    output.parent.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="overstory-pkg-") as temporary:
        work = Path(temporary)
        status = json.loads(subprocess.check_output(
            [str(cli), "--json", "skill", "status", "--directory", str(work / "skill-status-check")],
            text=True))
        if not status["ok"] or status["data"]["bundledVersion"] != version or not status["data"]["canInstall"]:
            raise ValueError("Bundled CLI skill manager does not match this release; rebuild the App")
        run(cli, "skill", "export", work / "overstory")
        if (work / "overstory/SKILL.md").read_bytes() != entry.read_bytes():
            raise ValueError("CLI skill entry does not match App resource; rebuild the App")
        payload = work / "payload"
        packaged = payload / "Applications/Overstory.app"
        packaged.parent.mkdir(parents=True)
        run("ditto", app, packaged)
        run("codesign", "--force", "--deep", "--sign", "-", packaged)
        run("codesign", "--verify", "--deep", "--strict", packaged)
        scripts = work / "scripts"
        scripts.mkdir()
        for name in ("preinstall", "postinstall"):
            content = (repo / "scripts/installer" / name).read_text().replace("@VERSION@", version)
            (scripts / name).write_text(content)
            (scripts / name).chmod(0o755)
        shutil.copy2(repo / "scripts/cli-manager.sh", scripts / "cli-manager.sh")
        (scripts / "cli-manager.sh").chmod(0o755)
        components = work / "components.plist"
        run("pkgbuild", "--analyze", "--root", payload, components)
        entries = plistlib.loads(components.read_bytes())
        for component in entries:
            component["BundleIsRelocatable"] = False
            component["BundleOverwriteAction"] = "upgrade"
            component["BundleHasStrictIdentifier"] = True
        components.write_bytes(plistlib.dumps(entries))
        run("pkgbuild", "--root", payload, "--component-plist", components,
            "--scripts", scripts, "--identifier", "app.overstory.desktop.pkg",
            "--version", version, "--install-location", "/", "--ownership", "recommended", output)
    skill_package = output.parent / "overstory-skill.zip"
    with zipfile.ZipFile(skill_package, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(entry, "overstory/SKILL.md")
    print(output)
    print(skill_package)


if __name__ == "__main__":
    main()
