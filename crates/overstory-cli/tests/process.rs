use serde_json::{json, Value};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
};
use tempfile::TempDir;

fn output(dir: &Path, args: &[&str], input: Option<&str>, expected: i32) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_overstory-cli"))
        .arg("--json")
        .arg("--data-dir")
        .arg(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    let result = child.wait_with_output().unwrap();
    assert_eq!(
        result.status.code(),
        Some(expected),
        "args={args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    if expected == 0 {
        assert!(
            result.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&result.stdout).unwrap()
    } else {
        assert!(result.stdout.is_empty());
        serde_json::from_slice(&result.stderr).unwrap()
    }
}
fn ok(dir: &Path, args: &[&str]) -> Value {
    output(dir, args, None, 0)
}
fn id(value: &Value) -> &str {
    value["data"]["id"].as_str().unwrap()
}

#[test]
fn standalone_crud_groups_search_order_and_backup() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("data");
    let status = ok(&dir, &["status"]);
    assert_eq!(status["data"]["appRequired"], false);
    let a = ok(&dir, &["todos", "add", "事项 A", "--favorite"]);
    let b = ok(&dir, &["todos", "add", "事项 B"]);
    let a = id(&a);
    let b = id(&b);
    ok(&dir, &["todos", "complete", a]);
    assert_eq!(
        ok(&dir, &["todos", "list", "--completed", "true"])["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    ok(&dir, &["todos", "reopen", a]);
    ok(&dir, &["todos", "favorite", b]);
    ok(&dir, &["todos", "move-favorite", b, "--before", a]);
    assert_eq!(
        ok(&dir, &["todos", "list", "--favorited"])["data"][0]["id"],
        b
    );
    ok(&dir, &["todos", "move", b, "--before", a]);
    assert_eq!(ok(&dir, &["todos", "list"])["data"][0]["id"], b);
    ok(&dir, &["todos", "unfavorite", a]);
    let g = ok(&dir, &["bookmarks", "groups", "add", "参考"]);
    let g = id(&g);
    let web = ok(
        &dir,
        &[
            "bookmarks",
            "add",
            "Rust",
            "--url",
            "rust-lang.org",
            "--group-id",
            g,
        ],
    );
    let web = id(&web);
    assert_eq!(
        ok(&dir, &["bookmarks", "get", web])["data"]["url"],
        "https://rust-lang.org/"
    );
    ok(&dir, &["bookmarks", "update", web, "--title", "语言"]);
    assert_eq!(
        ok(
            &dir,
            &["bookmarks", "list", "--search", "语言", "--group-id", g]
        )["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    ok(&dir, &["bookmarks", "groups", "rename", g, "编程"]);
    let h = ok(&dir, &["bookmarks", "groups", "add", "资料"]);
    let h = id(&h);
    ok(&dir, &["bookmarks", "groups", "move", h, "--before", g]);
    assert_eq!(
        ok(&dir, &["bookmarks", "groups", "list"])["data"][1]["id"],
        h
    );
    ok(&dir, &["bookmarks", "move", web, "--group-id", h]);
    ok(&dir, &["bookmarks", "groups", "delete", h]);
    assert_eq!(
        ok(&dir, &["bookmarks", "get", web])["data"]["groupId"],
        "ungrouped"
    );
    let note = output(
        &dir,
        &["notes", "add", "正文", "--content-stdin"],
        Some("第一行\n第二行🙂"),
        0,
    );
    let note = id(&note);
    output(
        &dir,
        &["notes", "update", note, "--content-stdin"],
        Some("更新正文\n"),
        0,
    );
    assert_eq!(
        ok(&dir, &["notes", "get", note])["data"]["content"],
        "更新正文\n"
    );
    let backup = temp.path().join("backup.json");
    ok(&dir, &["backup", "export", backup.to_str().unwrap()]);
    ok(&dir, &["todos", "delete", a]);
    ok(&dir, &["bookmarks", "delete", web]);
    ok(&dir, &["notes", "delete", note]);
    output(
        &dir,
        &["backup", "import", backup.to_str().unwrap()],
        None,
        2,
    );
    ok(
        &dir,
        &["backup", "import", backup.to_str().unwrap(), "--yes"],
    );
    assert_eq!(
        ok(&dir, &["notes", "get", note])["data"]["content"],
        "更新正文\n"
    );
    assert!(std::fs::read_dir(&dir).unwrap().any(|p| p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("before-restore-")));
}

#[test]
fn simultaneous_processes_keep_every_write_and_replay_once() {
    let temp = TempDir::new().unwrap();
    let handles: Vec<_> = (0..24)
        .map(|n| {
            let dir = temp.path().to_path_buf();
            thread::spawn(move || ok(&dir, &["todos", "add", &format!("并发 {n}")]))
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let handles: Vec<_> = (0..12)
        .map(|_| {
            let dir = temp.path().to_path_buf();
            thread::spawn(move || {
                ok(
                    &dir,
                    &["--request-id", "same-create", "notes", "add", "只添加一次"],
                )
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.iter().all(|r| id(r) == id(&results[0])));
    assert_eq!(
        results
            .iter()
            .filter(|r| r["data"]["replayed"] == false)
            .count(),
        1
    );
    assert_eq!(
        ok(temp.path(), &["todos", "list"])["data"]
            .as_array()
            .unwrap()
            .len(),
        24
    );
    assert_eq!(
        ok(temp.path(), &["notes", "list"])["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn conflicts_strict_json_protocol_and_recovery() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    let a = ok(dir, &["--request-id", "create", "notes", "add", "原始"]);
    let a = id(&a);
    output(
        dir,
        &["--request-id", "create", "notes", "add", "不同内容"],
        None,
        3,
    );
    ok(
        dir,
        &[
            "--request-id",
            "update",
            "notes",
            "update",
            a,
            "--content",
            "CLI",
        ],
    );
    let replay = ok(
        dir,
        &[
            "--request-id",
            "update",
            "notes",
            "update",
            a,
            "--content",
            "CLI",
        ],
    );
    assert_eq!(replay["data"]["replayed"], true);
    output(
        dir,
        &[
            "--request-id",
            "update",
            "notes",
            "update",
            a,
            "--content",
            "CLI",
            "--expected-item-version",
            "2",
        ],
        None,
        3,
    );
    let error = output(
        dir,
        &[
            "notes",
            "update",
            a,
            "--content",
            "旧草稿",
            "--expected-item-version",
            "1",
        ],
        None,
        3,
    );
    assert_eq!(error["error"]["code"], "ITEM_CONFLICT");
    output(
        dir,
        &["--expected-revision", "0", "todos", "add", "不写入"],
        None,
        3,
    );
    output(
        dir,
        &["todos", "add", "错误", "--url", "example.com"],
        None,
        2,
    );
    output(dir, &["notes", "complete", a], None, 2);
    output(dir, &["todos", "groups", "list"], None, 2);
    output(dir, &["missing-command"], None, 2);
    for action in [
        json!({"type":"create","title":"wrong","content":4}),
        json!({"type":"update","id":a,"contnet":"typo"}),
        json!({"type":"update","id":a,"expectedItemVersion":"1"}),
    ] {
        output(
            dir,
            &["action", "notes", "--stdin"],
            Some(&action.to_string()),
            2,
        );
    }
    output(
        dir,
        &["action", "todos", "--stdin"],
        Some(r#"{"type":"create","title":"x","url":"https://example.com"}"#),
        2,
    );
    output(
        dir,
        &["action", "todos", "--stdin"],
        Some("invalid JSON"),
        2,
    );
    let snapshot = ok(dir, &["snapshot"]);
    assert_eq!(
        snapshot["data"]["modules"]["notes"]["data"]["items"][0]["content"],
        "CLI"
    );
    let state = dir.join("state.json");
    output(dir, &["backup", "export", state.to_str().unwrap()], None, 2);
    let backup = dir.join("valid-backup.json");
    ok(dir, &["backup", "export", backup.to_str().unwrap()]);
    // Missing primary must recover the prior valid snapshot, not initialize empty state.
    std::fs::remove_file(&state).unwrap();
    assert_eq!(
        ok(dir, &["notes", "list"])["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    std::fs::write(&state, "{bad").unwrap();
    std::fs::write(dir.join("state.last-good.json"), "{bad").unwrap();
    let error = output(dir, &["todos", "add", "不能覆盖"], None, 1);
    assert_eq!(error["error"]["code"], "RECOVERY_REQUIRED");
    assert_eq!(std::fs::read_to_string(&state).unwrap(), "{bad");
    ok(
        dir,
        &["backup", "import", backup.to_str().unwrap(), "--yes"],
    );
    assert_eq!(ok(dir, &["notes", "get", a])["data"]["content"], "CLI");
    assert_eq!(ok(dir, &["schema"])["data"]["protocolVersion"], 1);
}

#[test]
fn todo_descriptions_survive_edits_and_reject_stale_updates() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    let added = ok(dir, &["todos", "add", "事项", "--content", "第一段说明"]);
    let item = id(&added);
    let old_version = added["data"]["item"]["version"]
        .as_u64()
        .unwrap()
        .to_string();
    output(
        dir,
        &["todos", "update", item, "--content-stdin"],
        Some("补充说明\n第二行"),
        0,
    );
    let stale = output(
        dir,
        &[
            "todos",
            "update",
            item,
            "--expected-item-version",
            &old_version,
            "--content",
            "旧草稿",
        ],
        None,
        3,
    );
    assert_eq!(stale["error"]["code"], "ITEM_CONFLICT");
    ok(dir, &["todos", "complete", item]);
    ok(dir, &["todos", "update", item, "--title", "新标题"]);
    let saved = ok(dir, &["todos", "get", item]);
    assert_eq!(saved["data"]["content"], "补充说明\n第二行");
    assert_eq!(saved["data"]["title"], "新标题");
    assert_eq!(saved["data"]["completed"], true);
    assert_eq!(
        ok(dir, &["todos", "list", "--search", "第二行"])["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
