use std::path::PathBuf;

use chatminal_store::{Store, StoredSessionStatus};
use uuid::Uuid;

struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("chatminal-store-{}.db", Uuid::new_v4()));
        Self { path }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn initialize_creates_default_profile() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");

    let workspace = store.load_workspace().expect("load workspace");

    assert_eq!(workspace.profiles.len(), 1);
    assert_eq!(workspace.profiles[0].name, "Default");
    assert_eq!(
        workspace.active_profile_id,
        workspace.profiles[0].profile_id
    );
    assert!(workspace.sessions.is_empty());
    assert!(workspace.active_session_id.is_none());
}

#[test]
fn session_history_roundtrip_and_clear() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;

    let session = store
        .create_session(
            &active_profile_id,
            Some("Dev".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create session");

    store
        .set_active_session(&active_profile_id, Some(&session.session_id))
        .expect("set active session");
    store
        .set_session_status(&session.session_id, StoredSessionStatus::Running)
        .expect("set running status");

    store
        .append_scrollback_chunk(&session.session_id, 1, "line1\nline2\n", 100)
        .expect("append chunk 1");
    store
        .append_scrollback_chunk(&session.session_id, 2, "line3\n", 101)
        .expect("append chunk 2");
    store
        .update_session_seq(&session.session_id, 2)
        .expect("update seq");

    let snapshot = store
        .session_snapshot(&session.session_id, 2)
        .expect("load snapshot");
    assert_eq!(snapshot.seq, 2);
    assert_eq!(snapshot.content, "line1\nline2\nline3\n");

    store
        .clear_session_history(&session.session_id)
        .expect("clear history");
    let snapshot_after_clear = store
        .session_snapshot(&session.session_id, 100)
        .expect("load snapshot after clear");
    assert_eq!(snapshot_after_clear.seq, 0);
    assert!(snapshot_after_clear.content.is_empty());

    let loaded = store
        .get_session(&session.session_id)
        .expect("get session")
        .expect("session exists");
    assert_eq!(loaded.seq, 0);
}

#[test]
fn session_history_retention_keeps_newest_chunks_by_line_budget() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;

    let session = store
        .create_session(
            &active_profile_id,
            Some("Retain".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create session");

    store
        .append_scrollback_chunk(&session.session_id, 1, "l1\nl2\n", 100)
        .expect("append chunk 1");
    store
        .append_scrollback_chunk(&session.session_id, 2, "l3\n", 101)
        .expect("append chunk 2");
    store
        .append_scrollback_chunk(&session.session_id, 3, "l4\n", 102)
        .expect("append chunk 3");

    store
        .enforce_session_scrollback_line_limit(&session.session_id, 2)
        .expect("enforce retention");
    let snapshot = store
        .session_snapshot(&session.session_id, 100)
        .expect("load retained snapshot");

    assert_eq!(snapshot.seq, 3);
    assert_eq!(snapshot.content, "l3\nl4\n");
}

#[test]
fn session_history_retention_counts_non_newline_chunks() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session = store
        .create_session(
            &active_profile_id,
            Some("RetainNoNewline".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create session");

    store
        .append_scrollback_chunk(&session.session_id, 1, "hello", 100)
        .expect("append chunk 1");
    store
        .append_scrollback_chunk(&session.session_id, 2, " world", 101)
        .expect("append chunk 2");

    store
        .enforce_session_scrollback_line_limit(&session.session_id, 1)
        .expect("enforce retention");
    let snapshot = store
        .session_snapshot(&session.session_id, 100)
        .expect("load retained snapshot");

    assert_eq!(snapshot.seq, 2);
    assert_eq!(snapshot.content, " world");
}

#[test]
fn delete_profile_cascades_sessions_and_history() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let default_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;

    let profile = store
        .create_profile(Some("Work".to_string()))
        .expect("create profile");
    store
        .set_active_profile(&profile.profile_id)
        .expect("set active profile");

    let session = store
        .create_session(
            &profile.profile_id,
            Some("Build".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create session");
    store
        .append_scrollback_chunk(&session.session_id, 1, "hello\n", 200)
        .expect("append history");
    store
        .set_active_session(&profile.profile_id, Some(&session.session_id))
        .expect("set active session");

    store
        .delete_profile(&profile.profile_id)
        .expect("delete profile");

    let workspace_after_delete = store.load_workspace().expect("load workspace after delete");
    assert_eq!(workspace_after_delete.profiles.len(), 1);
    assert_eq!(
        workspace_after_delete.profiles[0].profile_id,
        default_profile_id
    );
    assert_eq!(workspace_after_delete.active_profile_id, default_profile_id);
    assert!(
        store
            .get_session(&session.session_id)
            .expect("get session after profile delete")
            .is_none()
    );

    let err = store
        .delete_profile(&default_profile_id)
        .expect_err("delete last profile should fail");
    assert!(err.contains("cannot delete last profile"));
}

#[test]
fn bool_state_roundtrip_with_default() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");

    let keep_alive_key = "keep_alive_on_close";
    let start_in_tray_key = "start_in_tray";

    let keep_alive = store
        .get_bool_state(keep_alive_key, true)
        .expect("get default keep alive");
    let start_in_tray = store
        .get_bool_state(start_in_tray_key, false)
        .expect("get default start in tray");
    assert!(keep_alive);
    assert!(!start_in_tray);

    store
        .set_bool_state(keep_alive_key, false)
        .expect("set keep alive false");
    store
        .set_bool_state(start_in_tray_key, true)
        .expect("set start in tray true");

    let keep_alive_after = store
        .get_bool_state(keep_alive_key, true)
        .expect("get keep alive after set");
    let start_in_tray_after = store
        .get_bool_state(start_in_tray_key, false)
        .expect("get start in tray after set");
    assert!(!keep_alive_after);
    assert!(start_in_tray_after);
}

#[test]
fn session_explorer_state_roundtrip_and_session_delete_cleanup() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session = store
        .create_session(
            &active_profile_id,
            Some("Explorer".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            false,
        )
        .expect("create session");

    let root = std::env::temp_dir().join(format!("chatminal-explorer-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create explorer root");
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).expect("create explorer child dir");
    std::fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write explorer file");

    let set_root = store
        .set_session_explorer_root(&session.session_id, &root.to_string_lossy())
        .expect("set explorer root");
    assert_eq!(set_root.session_id, session.session_id);
    assert_eq!(set_root.root_path, root.to_string_lossy());
    assert_eq!(set_root.current_dir, "");
    assert!(set_root.selected_path.is_none());

    let updated = store
        .update_session_explorer_state(
            &session.session_id,
            "src",
            Some("src/main.rs"),
            Some("src/main.rs"),
        )
        .expect("update explorer state");
    assert_eq!(updated.current_dir, "src");
    assert_eq!(updated.selected_path.as_deref(), Some("src/main.rs"));
    assert_eq!(updated.open_file_path.as_deref(), Some("src/main.rs"));

    let loaded = store
        .get_session_explorer_state(&session.session_id)
        .expect("load explorer state")
        .expect("explorer state exists");
    assert_eq!(loaded.current_dir, "src");
    assert_eq!(loaded.open_file_path.as_deref(), Some("src/main.rs"));

    store
        .delete_session(&session.session_id)
        .expect("delete session");
    let loaded_after_delete = store
        .get_session_explorer_state(&session.session_id)
        .expect("load explorer state after session delete");
    assert!(loaded_after_delete.is_none());

    let _ = std::fs::remove_dir_all(&root);
}
