use crate::{backup::Backup, models::*, modules, storage::Store};
use serde_json::json;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().into()).unwrap();
    (dir, store)
}
fn action(s: &mut Store, module: &str, a: serde_json::Value) -> Snapshot {
    s.transact(|s| modules::apply(s, module, &a)).unwrap()
}
#[test]
fn favorites_are_persistent_and_independent_from_groups_and_content() {
    let (dir, mut s) = store();
    action(
        &mut s,
        "notes",
        json!({"type":"create_group","name":"工作"}),
    );
    let group = modules::data(&s.snapshot, "notes").unwrap().groups[1]
        .id
        .clone();
    for (title, group_id) in [("a", UNGROUPED), ("b", group.as_str()), ("c", UNGROUPED)] {
        action(
            &mut s,
            "notes",
            json!({"type":"create","title":title,"groupId":group_id,"content":"正文"}),
        );
    }
    let original = modules::data(&s.snapshot, "notes").unwrap().items;
    assert!(original
        .iter()
        .all(|i| !i.favorited && i.favorite_index.is_none()));
    for item in &original {
        action(
            &mut s,
            "notes",
            json!({"type":"set_favorite","id":item.id,"favorited":true}),
        );
    }
    let revision = s.snapshot.revision;
    action(
        &mut s,
        "notes",
        json!({"type":"move_favorite","itemId":original[2].id,"beforeItemId":original[0].id,"expectedRevision":revision}),
    );
    let ordered = modules::data(&s.snapshot, "notes").unwrap().items;
    assert_eq!(
        ordered.iter().map(|i| i.favorite_index).collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(0)]
    );
    for (item, old) in ordered.iter().zip(&original) {
        assert_eq!(
            (
                &item.id,
                &item.group_id,
                item.sort_index,
                &item.content,
                &item.updated_at
            ),
            (
                &old.id,
                &old.group_id,
                old.sort_index,
                &old.content,
                &old.updated_at
            )
        );
    }
    let revision = s.snapshot.revision;
    action(
        &mut s,
        "notes",
        json!({"type":"set_favorite","id":original[0].id,"favorited":true}),
    );
    assert_eq!(s.snapshot.revision, revision, "收藏幂等");
    action(
        &mut s,
        "notes",
        json!({"type":"set_favorite","id":original[0].id,"favorited":false}),
    );
    let items = modules::data(&s.snapshot, "notes").unwrap().items;
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].favorite_index, None);
    assert_eq!(items[1].favorite_index, Some(1));
    assert_eq!(items[2].favorite_index, Some(0));
    let reopened = Store::open(dir.path().into()).unwrap();
    assert_eq!(
        modules::data(&reopened.snapshot, "notes").unwrap().items,
        items
    );
    let backup =
        Backup::parse(&serde_json::to_vec(&Backup::from_snapshot(&s.snapshot)).unwrap()).unwrap();
    let (_other_dir, mut other) = store();
    backup.restore(&mut other).unwrap();
    assert_eq!(
        modules::data(&other.snapshot, "notes").unwrap().items,
        items
    );
}

#[test]
fn favorites_reject_stale_or_nonfavorite_targets_and_compact_after_delete() {
    let (_dir, mut s) = store();
    for (title, favorited) in [("a", true), ("b", false), ("c", true)] {
        action(
            &mut s,
            "todos",
            json!({"type":"create","title":title,"favorited":favorited}),
        );
    }
    let items = modules::data(&s.snapshot, "todos").unwrap().items;
    let unchanged = serde_json::to_value(&s.snapshot).unwrap();
    let revision = s.snapshot.revision;
    for (id, before, expected) in [
        (&items[0].id, Some(&items[2].id), 0),
        (&items[0].id, Some(&items[1].id), revision),
        (&items[1].id, None, revision),
    ] {
        assert!(s.transact(|s| modules::apply(s, "todos", &json!({"type":"move_favorite","itemId":id,"beforeItemId":before,"expectedRevision":expected}))).is_err());
        assert_eq!(serde_json::to_value(&s.snapshot).unwrap(), unchanged);
    }
    action(
        &mut s,
        "todos",
        json!({"type":"update","id":items[0].id,"completed":true}),
    );
    assert_eq!(
        modules::data(&s.snapshot, "todos").unwrap().items[0].favorite_index,
        Some(0)
    );
    action(&mut s, "todos", json!({"type":"delete","id":items[0].id}));
    let remaining = modules::data(&s.snapshot, "todos").unwrap().items;
    assert_eq!(remaining[0].favorite_index, None);
    assert_eq!(remaining[1].favorite_index, Some(0));
}

#[test]
fn legacy_backups_remain_valid_and_invalid_favorite_indices_are_rejected() {
    let (_dir, mut s) = store();
    action(
        &mut s,
        "bookmarks",
        json!({"type":"create","title":"Apple","url":"apple.com","favorited":true}),
    );
    let mut legacy = serde_json::to_value(Backup::from_snapshot(&s.snapshot)).unwrap();
    let item = &mut legacy["modules"]["bookmarks"]["data"]["items"][0];
    item.as_object_mut().unwrap().remove("favorited");
    item.as_object_mut().unwrap().remove("favoriteIndex");
    let backup = Backup::parse(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    let candidate = Snapshot {
        modules: backup.modules,
        ..Snapshot::default()
    };
    assert!(!modules::data(&candidate, "bookmarks").unwrap().items[0].favorited);
    for (favorited, index) in [(true, json!(null)), (true, json!(5)), (false, json!(0))] {
        let mut bad = s.snapshot.clone();
        bad.modules.get_mut("bookmarks").unwrap().data["items"][0]["favorited"] = json!(favorited);
        bad.modules.get_mut("bookmarks").unwrap().data["items"][0]["favoriteIndex"] = index;
        assert!(modules::validate(&bad).is_err());
    }
}
#[test]
fn serialized_writes_completion_and_manual_order() {
    let (dir, mut s) = store();
    for title in ["a", "hidden", "b", "c"] {
        action(&mut s, "todos", json!({"type":"create","title":title}));
    }
    let d = modules::data(&s.snapshot, "todos").unwrap();
    let ids: Vec<_> = d.items.iter().map(|i| i.id.clone()).collect();
    action(
        &mut s,
        "todos",
        json!({"type":"update","id":ids[1],"completed":true}),
    );
    let revision = s.snapshot.revision;
    action(
        &mut s,
        "todos",
        json!({"type":"move_item","itemId":ids[3],"beforeItemId":ids[0],"expectedRevision":revision}),
    );
    let d = modules::data(&s.snapshot, "todos").unwrap();
    assert_eq!(
        d.items.iter().map(|i| &i.title).collect::<Vec<_>>(),
        vec!["c", "a", "hidden", "b"]
    );
    for id in &ids {
        action(
            &mut s,
            "todos",
            json!({"type":"update","id":id,"completed":true}),
        );
    }
    assert!(modules::data(&s.snapshot, "todos")
        .unwrap()
        .items
        .iter()
        .all(|i| i.completed == Some(true)));
    assert_eq!(
        Store::open(dir.path().into()).unwrap().snapshot.revision,
        s.snapshot.revision
    );
    assert!(s
        .transact(|s| modules::apply(
            s,
            "todos",
            &json!({"type":"move_item","itemId":ids[0],"expectedRevision":0})
        ))
        .is_err());
}
#[test]
fn cross_group_deletion_and_backup_preserve_order() {
    let (_dir, mut s) = store();
    action(
        &mut s,
        "bookmarks",
        json!({"type":"create_group","name":"工作"}),
    );
    let group = modules::data(&s.snapshot, "bookmarks").unwrap().groups[1]
        .id
        .clone();
    for title in ["a", "b"] {
        action(
            &mut s,
            "bookmarks",
            json!({"type":"create","title":title,"url":"example.com","groupId":group}),
        );
    }
    let id = modules::data(&s.snapshot, "bookmarks").unwrap().items[0]
        .id
        .clone();
    let revision = s.snapshot.revision;
    action(
        &mut s,
        "bookmarks",
        json!({"type":"move_item","itemId":id,"toGroupId":"ungrouped","expectedRevision":revision}),
    );
    let revision = s.snapshot.revision;
    action(
        &mut s,
        "bookmarks",
        json!({"type":"move_group","groupId":group,"beforeGroupId":"ungrouped","expectedRevision":revision}),
    );
    action(
        &mut s,
        "bookmarks",
        json!({"type":"delete_group","id":group}),
    );
    let d = modules::data(&s.snapshot, "bookmarks").unwrap();
    assert_eq!(d.items.len(), 2);
    assert_eq!(d.items[1].title, "b");
    assert!(d
        .items
        .iter()
        .all(|i| i.group_id.as_deref() == Some(UNGROUPED)));
    let bytes = serde_json::to_vec(&Backup::from_snapshot(&s.snapshot)).unwrap();
    let backup = Backup::parse(&bytes).unwrap();
    let (dir, mut other) = store();
    backup.restore(&mut other).unwrap();
    assert_eq!(
        serde_json::to_value(&s.snapshot.modules).unwrap(),
        serde_json::to_value(&other.snapshot.modules).unwrap()
    );
    assert!(dir.path().read_dir().unwrap().count() >= 3);
}
#[test]
fn failed_write_leaves_authority_unchanged() {
    let (dir, mut s) = store();
    action(
        &mut s,
        "notes",
        json!({"type":"create","title":"note","content":"keep"}),
    );
    let before = serde_json::to_value(&s.snapshot).unwrap();
    std::fs::remove_file(dir.path().join("state.json")).unwrap();
    std::fs::create_dir(dir.path().join("state.json")).unwrap();
    assert!(s
        .transact(|s| modules::apply(s, "notes", &json!({"type":"create","title":"never"})))
        .is_err());
    assert_eq!(before, serde_json::to_value(&s.snapshot).unwrap());
}
#[test]
fn corruption_is_recovered_or_blocked_never_reset() {
    let (dir, mut s) = store();
    action(&mut s, "todos", json!({"type":"create","title":"safe"}));
    action(&mut s, "todos", json!({"type":"create","title":"new"}));
    std::fs::write(dir.path().join("state.json"), "{broken").unwrap();
    let recovered = Store::open(dir.path().into()).unwrap();
    assert!(recovered.snapshot.recovery_notice.is_some());
    assert_eq!(
        modules::data(&recovered.snapshot, "todos")
            .unwrap()
            .items
            .len(),
        1
    );
    std::fs::write(dir.path().join("state.json"), "{broken-again").unwrap();
    std::fs::write(dir.path().join("state.last-good.json"), "invalid").unwrap();
    let mut blocked = Store::open(dir.path().into()).unwrap();
    assert!(blocked.load_error.is_some());
    assert!(blocked.transact(|_| Ok(true)).is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("state.json")).unwrap(),
        "{broken-again"
    );
}
#[test]
fn invalid_restore_does_not_touch_data_and_unknown_namespace_survives() {
    let (_dir, mut s) = store();
    let mut backup = Backup::from_snapshot(&s.snapshot);
    backup.modules.insert(
        "future".into(),
        Namespace {
            version: 42,
            data: json!({"private":"kept"}),
        },
    );
    let parsed = Backup::parse(&serde_json::to_vec(&backup).unwrap()).unwrap();
    parsed.restore(&mut s).unwrap();
    assert_eq!(s.snapshot.modules["future"].version, 42);
    let before = s.snapshot.revision;
    backup.modules.get_mut("todos").unwrap().version = 2;
    assert!(Backup::parse(&serde_json::to_vec(&backup).unwrap()).is_err());
    assert_eq!(before, s.snapshot.revision);
    for url in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "https://",
        "a b.com",
    ] {
        assert!(modules::normalize_url(url).is_err(), "{url}");
    }
}
#[test]
fn independent_stores_reload_before_writes_and_window_changes_preserve_cli_data() {
    let (dir, mut app) = store();
    let mut cli = Store::open(dir.path().into()).unwrap();
    action(&mut cli, "todos", json!({"type":"create","title":"CLI"}));
    app.transact(|s| {
        s.window_state.mode = Presentation::Main;
        Ok(true)
    })
    .unwrap();
    action(
        &mut cli,
        "notes",
        json!({"type":"create","title":"independent"}),
    );
    app.read().unwrap();
    assert_eq!(
        modules::data(&app.snapshot, "todos").unwrap().items[0].title,
        "CLI"
    );
    assert_eq!(
        modules::data(&app.snapshot, "notes").unwrap().items.len(),
        1
    );
    assert_eq!(app.snapshot.window_state.mode, Presentation::Main);
    assert_eq!(app.snapshot.revision, 3);
}

#[test]
fn stale_note_edits_keep_external_content_and_unrelated_updates_do_not_conflict() {
    let (dir, mut app) = store();
    action(
        &mut app,
        "notes",
        json!({"type":"create","title":"note","content":"original"}),
    );
    let item = modules::data(&app.snapshot, "notes").unwrap().items[0].clone();
    let mut cli = Store::open(dir.path().into()).unwrap();
    action(
        &mut cli,
        "notes",
        json!({"type":"update","id":item.id,"content":"CLI","expectedItemVersion":item.version}),
    );
    let error = app.transact(|s| modules::apply(s, "notes", &json!({"type":"update","id":item.id,"content":"stale draft","expectedItemVersion":item.version}))).unwrap_err();
    assert_eq!(error.code, "ITEM_CONFLICT");
    assert_eq!(
        modules::data(&app.snapshot, "notes").unwrap().items[0]
            .content
            .as_deref(),
        Some("CLI")
    );
    action(
        &mut cli,
        "todos",
        json!({"type":"create","title":"unrelated"}),
    );
    let version = modules::data(&app.snapshot, "notes").unwrap().items[0].version;
    action(
        &mut app,
        "notes",
        json!({"type":"update","id":item.id,"content":"merged","expectedItemVersion":version}),
    );
    assert_eq!(
        modules::data(&app.snapshot, "todos").unwrap().items.len(),
        1
    );
}

#[test]
fn restore_backs_up_the_latest_external_data_inside_the_lock() {
    let (dir, mut app) = store();
    let backup = Backup::from_snapshot(&app.snapshot);
    let mut cli = Store::open(dir.path().into()).unwrap();
    action(
        &mut cli,
        "todos",
        json!({"type":"create","title":"latest CLI"}),
    );
    backup.restore(&mut app).unwrap();
    let saved = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-restore-")
        })
        .unwrap();
    let saved = Backup::parse(&std::fs::read(saved).unwrap()).unwrap();
    assert_eq!(
        saved.modules["todos"].data["items"][0]["title"],
        "latest CLI"
    );
}
