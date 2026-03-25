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
fn string_state_roundtrip_and_clear() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");

    assert_eq!(
        store
            .get_string_state("workspace_layout:test")
            .expect("get empty"),
        None
    );

    store
        .set_string_state("workspace_layout:test", "{\"ok\":true}")
        .expect("set string state");
    assert_eq!(
        store
            .get_string_state("workspace_layout:test")
            .expect("get stored string")
            .as_deref(),
        Some("{\"ok\":true}")
    );

    store
        .clear_state("workspace_layout:test")
        .expect("clear string state");
    assert_eq!(
        store
            .get_string_state("workspace_layout:test")
            .expect("get cleared"),
        None
    );
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

#[test]
fn profiles_keep_creation_order_after_updates() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");

    let first = store
        .create_profile(Some("Alpha".to_string()))
        .expect("create first profile");
    let second = store
        .create_profile(Some("Beta".to_string()))
        .expect("create second profile");

    store
        .rename_profile(&first.profile_id, "Alpha Renamed")
        .expect("rename first profile");

    let workspace = store.load_workspace().expect("load workspace");
    let created_profiles: Vec<_> = workspace
        .profiles
        .into_iter()
        .filter(|profile| profile.profile_id == first.profile_id || profile.profile_id == second.profile_id)
        .collect();

    assert_eq!(created_profiles.len(), 2);
    assert_eq!(created_profiles[0].profile_id, first.profile_id);
    assert_eq!(created_profiles[1].profile_id, second.profile_id);
}

#[test]
fn sessions_keep_creation_order_after_runtime_updates() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;

    let first = store
        .create_session(
            &active_profile_id,
            Some("First".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create first session");
    let second = store
        .create_session(
            &active_profile_id,
            Some("Second".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create second session");

    store
        .set_session_status(&first.session_id, StoredSessionStatus::Running)
        .expect("update first session status");
    store
        .update_session_seq(&first.session_id, 42)
        .expect("update first session seq");

    let workspace = store.load_workspace().expect("load workspace");
    let created_sessions: Vec<_> = workspace
        .sessions
        .into_iter()
        .filter(|session| session.session_id == first.session_id || session.session_id == second.session_id)
        .collect();

    assert_eq!(created_sessions.len(), 2);
    assert_eq!(created_sessions[0].session_id, first.session_id);
    assert_eq!(created_sessions[1].session_id, second.session_id);
}

#[test]
fn move_session_to_profile_reparents_without_losing_order_contract() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let default_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let target_profile = store
        .create_profile(Some("Work".to_string()))
        .expect("create target profile");

    let first = store
        .create_session(
            &default_profile_id,
            Some("First".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create first");
    let second = store
        .create_session(
            &default_profile_id,
            Some("Second".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create second");
    let target_existing = store
        .create_session(
            &target_profile.profile_id,
            Some("Target".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create target session");

    store
        .set_active_session(&default_profile_id, Some(&second.session_id))
        .expect("set source active session");

    store
        .move_session_to_profile(&second.session_id, &target_profile.profile_id, Some(0))
        .expect("move session");

    let source_sessions = store
        .list_sessions_by_profile(&default_profile_id)
        .expect("list source sessions");
    let target_sessions = store
        .list_sessions_by_profile(&target_profile.profile_id)
        .expect("list target sessions");
    let workspace = store.load_workspace().expect("reload workspace");

    assert_eq!(source_sessions.len(), 1);
    assert_eq!(source_sessions[0].session_id, first.session_id);
    assert_eq!(target_sessions.len(), 2);
    assert_eq!(target_sessions[0].session_id, second.session_id);
    assert_eq!(target_sessions[1].session_id, target_existing.session_id);
    assert_eq!(workspace.active_profile_id, default_profile_id);
    assert_eq!(workspace.active_session_id.as_deref(), Some(first.session_id.as_str()));
}

#[test]
fn move_sessions_to_profile_keeps_relative_order_within_same_profile() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;

    let first = store
        .create_session(
            &profile_id,
            Some("First".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create first");
    let second = store
        .create_session(
            &profile_id,
            Some("Second".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create second");
    let third = store
        .create_session(
            &profile_id,
            Some("Third".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create third");
    let fourth = store
        .create_session(
            &profile_id,
            Some("Fourth".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create fourth");

    store
        .move_sessions_to_profile(
            &[second.session_id.clone(), third.session_id.clone()],
            &profile_id,
            Some(2),
        )
        .expect("reorder grouped sessions");

    let ordered_ids = store
        .list_sessions_by_profile(&profile_id)
        .expect("list sessions")
        .into_iter()
        .map(|session| session.session_id)
        .collect::<Vec<_>>();

    assert_eq!(
        ordered_ids,
        vec![
            first.session_id,
            fourth.session_id,
            second.session_id,
            third.session_id,
        ]
    );
}

#[test]
fn move_sessions_to_profile_moves_group_transactionally_across_profiles() {
    let temp = TempDb::new();
    let store = Store::initialize(&temp.path).expect("initialize store");
    let source_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let target_profile = store
        .create_profile(Some("Target".to_string()))
        .expect("create target profile");

    let first = store
        .create_session(
            &source_profile_id,
            Some("First".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create first");
    let second = store
        .create_session(
            &source_profile_id,
            Some("Second".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create second");
    let third = store
        .create_session(
            &source_profile_id,
            Some("Third".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create third");
    let target_existing = store
        .create_session(
            &target_profile.profile_id,
            Some("Target existing".to_string()),
            "/tmp".to_string(),
            "/bin/bash".to_string(),
            true,
        )
        .expect("create target existing");

    store
        .set_active_session(&source_profile_id, Some(&second.session_id))
        .expect("set source active session");

    store
        .move_sessions_to_profile(
            &[second.session_id.clone(), third.session_id.clone()],
            &target_profile.profile_id,
            Some(0),
        )
        .expect("move grouped sessions");

    let source_ids = store
        .list_sessions_by_profile(&source_profile_id)
        .expect("list source")
        .into_iter()
        .map(|session| session.session_id)
        .collect::<Vec<_>>();
    let target_ids = store
        .list_sessions_by_profile(&target_profile.profile_id)
        .expect("list target")
        .into_iter()
        .map(|session| session.session_id)
        .collect::<Vec<_>>();
    let workspace = store.load_workspace().expect("reload workspace");

    assert_eq!(source_ids, vec![first.session_id.clone()]);
    assert_eq!(
        target_ids,
        vec![
            second.session_id.clone(),
            third.session_id.clone(),
            target_existing.session_id,
        ]
    );
    assert_eq!(workspace.active_profile_id, source_profile_id);
    assert_eq!(workspace.active_session_id.as_deref(), Some(first.session_id.as_str()));
}
