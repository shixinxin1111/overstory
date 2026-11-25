#!/usr/bin/env python3
"""Inspect/exercise an actual PKG without installing it on the host."""
import argparse
import json
import os
import plistlib
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def run(*args, **kwargs):
    return subprocess.run([str(arg) for arg in args], check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--launch-qa", action="store_true", help="Launch a separate native QA App with isolated data")
    args = parser.parse_args()
    version = json.loads((REPO / "src-tauri/tauri.conf.json").read_text())["version"]
    work = Path(tempfile.mkdtemp(prefix="overstory-pkg-qa-"))
    expanded = work / "expanded"
    run("pkgutil", "--expand-full", REPO / "release" / f"Overstory-{version}.pkg", expanded)
    payload = expanded / "Payload"
    app = payload / "Applications/Overstory.app"
    run("codesign", "--verify", "--deep", "--strict", app)
    package_info = (expanded / "PackageInfo").read_text()
    assert f'version="{version}"' in package_info and 'relocatable="false"' in package_info
    assert sorted(p.name for p in payload.iterdir()) == ["Applications"]
    scripts = expanded / "Scripts"
    for name in ("preinstall", "postinstall"):
        assert "@VERSION@" not in (scripts / name).read_text()
    volume = work / "volume"
    volume.mkdir()
    data = work / "isolated-data"
    data.mkdir()
    # Seed a full real snapshot via the packaged CLI, before any App is running.
    source_cli = app / "Contents/Resources/cli/overstory-cli"
    created = json.loads(subprocess.check_output([str(source_cli), "--json", "--data-dir", str(data),
                                                 "notes", "add", "CLI 同步验证", "--content", "CLI 在 App 关闭时创建的正文"]))
    note_id = created["data"]["id"]
    state = data / "state.json"
    before = state.read_bytes()
    for _ in range(2):
        run("sh", scripts / "preinstall", "package", "/", volume)
        target = volume / "Applications/Overstory.app"
        target.parent.mkdir(parents=True, exist_ok=True)
        run("ditto", app, target)
        run("sh", scripts / "postinstall", "package", "/", volume)
        cli = volume / "usr/local/lib/overstory/overstory-cli"
        assert subprocess.check_output([str(cli), "--version"], text=True).strip() == f"overstory {version}"
        assert state.read_bytes() == before
    run("sh", scripts / "cli-manager.sh", "uninstall", "", version, volume)
    assert not (volume / "usr/local/bin/overstory").is_symlink()
    assert not (volume / "usr/local/lib/overstory/overstory-cli").exists()
    assert state.read_bytes() == before
    print("PASS: actual PKG payload, signature, fixed location, repeat install, bundled CLI and uninstall")
    if args.launch_qa:
        info_path = app / "Contents/Info.plist"
        info = plistlib.loads(info_path.read_bytes())
        info["CFBundleIdentifier"] = "app.overstory.cliqa"
        info["CFBundleName"] = "Overstory CLI QA"
        info_path.write_bytes(plistlib.dumps(info))
        run("codesign", "--force", "--deep", "--sign", "-", app)
        snapshot = json.loads(state.read_text())
        snapshot["windowState"]["mode"] = "main"
        snapshot["session"]["activeModuleId"] = "notes"
        snapshot["session"]["selectedNoteId"] = note_id
        state.write_text(json.dumps(snapshot, ensure_ascii=False))
        metadata = {"work": str(work), "app": str(app), "dataDir": str(data),
                    "cli": str(source_cli), "noteId": note_id, "pid": os.getpid(), "bundleId": info["CFBundleIdentifier"]}
        (REPO / "verification/cli-qa-runtime.json").write_text(json.dumps(metadata, indent=2))
        print(json.dumps(metadata, indent=2), flush=True)
        executable = str(app / "Contents/MacOS/overstory")
        os.execve(executable, [executable], {**os.environ, "OVERSTORY_DATA_DIR": str(data)})
    else:
        import shutil
        shutil.rmtree(work)


if __name__ == "__main__":
    main()
