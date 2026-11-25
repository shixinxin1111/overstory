#!/usr/bin/env python3
"""Verify the skill/CLI workflow using isolated data, without installing or launching apps."""
import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, default=REPO / "src-tauri/target/debug/overstory-cli")
    args = parser.parse_args()
    cli = args.cli.resolve()
    version = json.loads((REPO / "package.json").read_text())["version"]
    full = (REPO / "skills/overstory/SKILL.md").read_text()
    entry = (REPO / "skills/overstory-entry/SKILL.md").read_bytes()
    with tempfile.TemporaryDirectory(prefix="overstory-skill-check-") as temporary:
        work = Path(temporary)
        data_dir = work / "isolated data"

        def command(*arguments, payload=None, expected=0, structured=True, env=None):
            result = subprocess.run(
                [str(cli), "--data-dir", str(data_dir), *arguments],
                input=json.dumps(payload) if payload is not None else None,
                text=True, capture_output=True, env=env,
            )
            assert result.returncode == expected, result.stdout + result.stderr
            if not structured:
                return result
            if expected:
                assert not result.stdout
                value = json.loads(result.stderr)
                assert value["ok"] is False
            else:
                assert not result.stderr, result.stderr
                value = json.loads(result.stdout)
                assert value["ok"] is True
            return value

        assert command("skill", "show", structured=False).stdout == full
        shown = command("--json", "skill", "show")
        assert shown["data"] == {"name": "overstory", "version": version, "content": full}
        assert shown["revision"] is None
        command("--json", "schema")
        destination = work / "upload 'quoted' folder"
        exported = command("--json", "skill", "export", str(destination))
        assert Path(exported["data"]["path"]) == (destination / "SKILL.md").resolve()
        assert (destination / "SKILL.md").read_bytes() == entry
        collision = command("--json", "skill", "export", str(destination), expected=2)
        assert collision["error"]["code"] == "VALIDATION"
        assert (destination / "SKILL.md").read_bytes() == entry
        assert not data_dir.exists(), "Skill/schema commands must not open the data store"
        print("PASS: raw/JSON skill, embedded sources, export, collision, no data side effects")

        # Exercise the real manager in a temporary shared skills directory.
        directory = work / "user home/.agents/skills/overstory"
        marker = directory / ".managed-by-overstory.json"
        installed_entry = directory / "SKILL.md"
        other_skill = directory.parent / "another-skill/SKILL.md"
        other_skill.parent.mkdir(parents=True)
        other_skill.write_text("another user's skill")

        def manage(operation, expected=0, target=directory):
            return command("--json", "skill", operation, "--directory", str(target),
                           expected=expected)["data" if expected == 0 else "error"]

        absent = manage("status")
        assert absent["canInstall"] and not absent["installed"] and not directory.exists()
        installed = manage("install")
        assert installed["installed"] and installed["version"] == version
        assert installed["canUninstall"] and not installed["canUpgrade"]
        assert installed_entry.read_bytes() == entry
        manage("install")  # Repeat installs are harmless.
        receipt = json.loads(marker.read_text())
        receipt["version"] = "0.0.1"
        marker.write_text(json.dumps(receipt))
        assert manage("status")["canUpgrade"]
        assert manage("upgrade")["version"] == version
        # Also update an entry changed by a same-version release.
        receipt = json.loads(marker.read_text())
        receipt["content"] = "old official entry"
        marker.write_text(json.dumps(receipt))
        installed_entry.write_text(receipt["content"])
        assert manage("status")["canUpgrade"]
        manage("install")
        assert installed_entry.read_bytes() == entry
        # Repair an interrupted install/upgrade while preserving ownership.
        installed_entry.unlink()
        assert manage("status")["canUpgrade"]
        manage("upgrade")
        assert installed_entry.read_bytes() == entry
        receipt = json.loads(marker.read_text())
        receipt["version"] = "999.0.0"
        marker.write_text(json.dumps(receipt))
        assert not manage("status")["canUpgrade"]
        assert manage("install", expected=3)["code"] == "SKILL_CONFLICT"
        assert json.loads(marker.read_text())["version"] == "999.0.0"
        receipt["version"] = version
        marker.write_text(json.dumps(receipt))
        installed_entry.write_text("my customized skill")
        modified = manage("status")
        assert modified["installed"] and not modified["canUpgrade"] and not modified["canUninstall"]
        assert manage("install", expected=3)["code"] == "SKILL_CONFLICT"
        assert manage("uninstall", expected=3)["code"] == "SKILL_CONFLICT"
        assert installed_entry.read_text() == "my customized skill"
        installed_entry.unlink()
        external = work / "external.md"
        external.write_text("external content")
        installed_entry.symlink_to(external)
        assert manage("uninstall", expected=3)["code"] == "SKILL_CONFLICT"
        assert external.read_text() == "external content"
        installed_entry.unlink()
        installed_entry.write_bytes(entry)
        retained = directory / "my-extra-file.txt"
        retained.write_text("keep")
        assert not manage("uninstall")["installed"]
        assert retained.read_text() == "keep" and not installed_entry.exists() and not marker.exists()
        assert manage("install", expected=3)["code"] == "SKILL_CONFLICT"
        retained.unlink()
        directory.rmdir()
        manage("install")
        manage("uninstall")
        assert not directory.exists()
        foreign = work / "foreign skill"
        foreign.mkdir()
        (foreign / "SKILL.md").write_text("foreign")
        assert manage("install", expected=3, target=foreign)["code"] == "SKILL_CONFLICT"
        assert (foreign / "SKILL.md").read_text() == "foreign"
        link = work / "linked skill"
        link.symlink_to(foreign, target_is_directory=True)
        assert manage("install", expected=3, target=link)["code"] == "SKILL_CONFLICT"
        assert other_skill.read_text() == "another user's skill"
        isolated_home = work / "default home"
        default_status = command("--json", "skill", "status", env={**os.environ, "HOME": str(isolated_home)})
        assert Path(default_status["data"]["path"]) == isolated_home / ".agents/skills/overstory"
        assert not isolated_home.exists(), "Reading status must not create a skill directory"
        # Separate CLI processes must serialize installation through the shared lock.
        processes = [subprocess.Popen(
            [str(cli), "--json", "--data-dir", str(data_dir), "skill", "install", "--directory", str(directory)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        ) for _ in range(6)]
        for process in processes:
            stdout, stderr = process.communicate(timeout=15)
            assert process.returncode == 0, stdout + stderr
            assert json.loads(stdout)["data"]["installed"]
        assert installed_entry.read_bytes() == entry
        assert json.loads(marker.read_text())["content"].encode() == entry
        manage("uninstall")
        assert not data_dir.exists(), "Skill management must not open the data store"
        print("PASS: install/upgrade/repair/uninstall, versions, edits, symlinks, HOME and concurrent installers")

        # Eval 1: create a favorited todo with description, then retry the same request.
        command("--json", "status")
        create = ("--json", "--request-id", "skill-check-todo", "todos", "add",
                  "准备会议", "--content", "核对议程", "--favorite")
        todo = command(*create)
        assert todo["data"]["item"]["content"] == "核对议程"
        assert todo["data"]["item"]["favorited"] is True
        replay = command(*create)
        assert replay["data"]["id"] == todo["data"]["id"]
        assert replay["data"]["replayed"] is True
        assert len(command("--json", "todos", "list")["data"]) == 1
        print("PASS: favorited todo, description and idempotent retry")

        # Eval 2: resolve a real group ID and preserve shell-like text as data.
        group = command("--json", "notes", "groups", "add", "工作")["data"]["id"]
        found = command("--json", "notes", "groups", "list")["data"]
        assert any(g["id"] == group and g["name"] == "工作" for g in found)
        sentinel = work / "should-not-exist"
        content = f"`echo hello`\n$(touch {sentinel})"
        note = command("--json", "--request-id", "skill-check-note", "action", "notes", "--stdin",
                       payload={"type": "create", "title": "会议记录",
                                "content": content, "groupId": group})
        note_id = note["data"]["id"]
        assert note["data"]["item"]["content"] == content
        assert note["data"]["item"]["groupId"] == group
        assert not sentinel.exists()
        print("PASS: group resolution and literal multiline JSON stdin")

        # Eval 3: preserve body on a title-only edit and reject a stale version.
        matches = command("--json", "notes", "list", "--search", "会议记录")["data"]
        assert len(matches) == 1 and matches[0]["id"] == note_id
        original = command("--json", "notes", "get", note_id)["data"]
        changed = command("--json", "notes", "update", note_id, "--title", "本周会议",
                          "--expected-item-version", str(original["version"]))["data"]["item"]
        assert changed["content"] == content
        conflict = command("--json", "notes", "update", note_id, "--title", "过期修改",
                           "--expected-item-version", str(original["version"]), expected=3)
        assert conflict["error"]["code"] == "ITEM_CONFLICT"
        latest = command("--json", "notes", "get", note_id)["data"]
        assert latest["title"] == "本周会议" and latest["content"] == content
        print("PASS: title-only edit and concurrent modification conflict")

        # Eval 4: website metadata is shared by App actions and ordinary CLI commands.
        description = "项目接口说明\n第二行介绍"
        web = command("--json", "bookmarks", "add", "项目文档",
                      "--url", "example.com/docs", "--icon", "example.com/favicon.ico",
                      "--description", description, "--favorite")["data"]["item"]
        web_id = web["id"]
        assert web["icon"] == "https://example.com/favicon.ico"
        assert web["description"] == description and web["favorited"]
        matches = command("--json", "bookmarks", "list", "--search", "第二行介绍")["data"]
        assert len(matches) == 1 and matches[0]["id"] == web_id
        renamed = command("--json", "bookmarks", "update", web_id, "--title", "开发文档",
                          "--expected-item-version", str(web["version"]))["data"]["item"]
        assert renamed["icon"] == web["icon"] and renamed["description"] == description
        conflict = command("--json", "bookmarks", "update", web_id, "--description", "过期修改",
                           "--expected-item-version", str(web["version"]), expected=3)
        assert conflict["error"]["code"] == "ITEM_CONFLICT"
        for invalid_icon in ("javascript:alert(1)", "file:///tmp/icon.svg",
                             "data:image/png;base64,aGVsbG8=", "https://user:pass@example.com/favicon.ico"):
            command("--json", "bookmarks", "update", web_id, "--icon", invalid_icon, expected=2)
        for module in ("todos", "notes"):
            command("--json", module, "add", "错误", "--description", "仅网页字段", expected=2)
            command("--json", module, "add", "错误", "--icon", web["icon"], expected=2)
            command("--json", "action", module, "--stdin",
                    payload={"type": "create", "title": "错误", "description": "仅网页字段"}, expected=2)
        for payload in ({"icon": None}, {"description": 1}):
            command("--json", "action", "bookmarks", "--stdin",
                    payload={"type": "update", "id": web_id, **payload}, expected=2)
        assert command("--json", "bookmarks", "get", web_id)["data"] == renamed
        backup = work / "website-backup.json"
        command("--json", "backup", "export", str(backup))
        saved = json.loads(backup.read_text())
        saved_web = next(item for item in saved["modules"]["bookmarks"]["data"]["items"]
                         if item["id"] == web_id)
        assert saved_web["icon"] == web["icon"] and saved_web["description"] == description
        command("--json", "bookmarks", "update", web_id, "--description", "临时修改")
        command("--json", "backup", "import", str(backup), "--yes")
        restored = command("--json", "bookmarks", "get", web_id)["data"]
        assert restored["icon"] == web["icon"] and restored["description"] == description
        # Invalid icon data in a backup must leave the whole store untouched.
        bad_backup = work / "invalid-website-backup.json"
        saved_web["icon"] = "javascript:alert(1)"
        bad_backup.write_text(json.dumps(saved))
        before = (data_dir / "state.json").read_bytes()
        command("--json", "backup", "import", str(bad_backup), "--yes", expected=2)
        assert (data_dir / "state.json").read_bytes() == before
        cleared = command("--json", "bookmarks", "update", web_id, "--icon", "",
                          "--description", "")["data"]["item"]
        assert "icon" not in cleared and cleared["description"] == ""
        assert cleared["title"] == restored["title"] and cleared["url"] == restored["url"]
        legacy = command("--json", "bookmarks", "add", "旧格式网页", "--url", "example.com")["data"]["item"]
        assert "icon" not in legacy and "description" not in legacy
        assert command("--json", "bookmarks", "get", legacy["id"])["data"] == legacy
        schema = command("--json", "schema")["data"]
        assert "icon" in schema["actions"]["create"]["optional"]
        assert "description" in schema["actions"]["update"]["optional"]
        print("PASS: website icon/description CRUD, search, conflict, clearing, backups and legacy fields")


if __name__ == "__main__":
    main()
