use crate::workspace_layout::WorkspaceLayoutState;
use store::StoredSessionSnapshot;

use super::{
    DEFAULT_KEEP_ALIVE_ON_CLOSE, DEFAULT_START_IN_TRAY, KEEP_ALIVE_ON_CLOSE_KEY, START_IN_TRAY_KEY,
    StateInner, WORKSPACE_LAYOUT_PREFIX, append_disconnected_restore_cleanup,
    canonical_scrollback::{
        build_logical_snapshot, materialize_output_chunk, render_snapshot,
        render_snapshot_for_terminal,
    },
    normalize_restore_replay_snapshot, now_millis, strip_zsh_prompt_spacer_artifact,
};
use crate::api::{
    RuntimeLifecyclePreferences, RuntimeProfile, RuntimeSessionSnapshot, RuntimeWorkspace,
};
use store::StoredSessionStatus;

impl StateInner {
    fn workspace_layout_key(&self, workspace_id: &str) -> String {
        format!("{WORKSPACE_LAYOUT_PREFIX}{workspace_id}")
    }

    pub(super) fn load_workspace_snapshot(&self) -> Result<RuntimeWorkspace, String> {
        let mut workspace = self.store.load_workspace()?;
        for session in &mut workspace.sessions {
            if let Some(entry) = self.sessions.get(&session.session_id) {
                session.status = entry.session.status.clone().into();
                session.seq = entry.session.seq;
                session.persist_history = entry.session.persist_history;
                session.cwd = entry.session.cwd.clone();
                session.name = entry.session.name.clone();
                session.startup_command = entry.session.startup_command.clone();
            }
        }

        Ok(RuntimeWorkspace {
            profiles: workspace.profiles.into_iter().map(Into::into).collect(),
            active_profile_id: Some(workspace.active_profile_id),
            sessions: workspace.sessions.into_iter().map(Into::into).collect(),
            active_session_id: workspace.active_session_id,
        })
    }

    pub(super) fn profile_create(
        &mut self,
        name: Option<String>,
    ) -> Result<RuntimeProfile, String> {
        let created = self.store.create_profile(name)?;
        self.publish_workspace_updated();
        Ok(created.into())
    }

    pub(super) fn profile_rename(
        &mut self,
        profile_id: &str,
        name: &str,
    ) -> Result<RuntimeWorkspace, String> {
        self.store.rename_profile(profile_id, name)?;
        self.publish_workspace_updated();
        self.load_workspace_snapshot()
    }

    pub(super) fn profile_switch(&mut self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        let exists = self
            .store
            .list_profiles()?
            .iter()
            .any(|value| value.profile_id == profile_id);
        if !exists {
            return Err("profile not found".to_string());
        }
        self.store.set_active_profile(profile_id)?;
        self.publish_workspace_updated();
        self.load_workspace_snapshot()
    }

    pub(super) fn session_move_to_profile(
        &mut self,
        session_id: &str,
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        self.store
            .move_session_to_profile(session_id, profile_id, target_index)?;
        if let Some(entry) = self.sessions.get_mut(session_id) {
            entry.session.profile_id = profile_id.to_string();
        }
        self.publish_session_and_workspace_updated(session_id);
        self.load_workspace_snapshot()
    }

    pub(super) fn sessions_move_to_profile(
        &mut self,
        session_ids: &[String],
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        self.store
            .move_sessions_to_profile(session_ids, profile_id, target_index)?;
        for session_id in session_ids {
            if let Some(entry) = self.sessions.get_mut(session_id) {
                entry.session.profile_id = profile_id.to_string();
            }
            self.publish_session_updated_for(session_id);
        }
        self.publish_workspace_updated();
        self.load_workspace_snapshot()
    }

    pub(super) fn session_rename(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<RuntimeWorkspace, String> {
        self.store.rename_session(session_id, name)?;
        if let Some(entry) = self.sessions.get_mut(session_id) {
            entry.session.name = name.trim().to_string();
        }
        self.publish_session_and_workspace_updated(session_id);
        self.load_workspace_snapshot()
    }

    pub(super) fn session_set_startup_command(
        &mut self,
        session_id: &str,
        startup_command: Option<&str>,
    ) -> Result<RuntimeWorkspace, String> {
        self.store
            .set_session_startup_command(session_id, startup_command)?;
        let Some(entry) = self.sessions.get_mut(session_id) else {
            return Err("session not found".to_string());
        };
        entry.session.startup_command = startup_command
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        self.publish_session_and_workspace_updated(session_id);
        self.load_workspace_snapshot()
    }

    pub(super) fn session_snapshot_get(
        &self,
        session_id: &str,
        preview_lines: Option<usize>,
    ) -> Result<RuntimeSessionSnapshot, String> {
        if !self.sessions.contains_key(session_id) {
            return Err("session not found".to_string());
        }

        let from_store = render_snapshot(
            &build_logical_snapshot(&self.store, session_id)?,
            Some(preview_lines.unwrap_or(self.config.default_preview_lines)),
        );
        let merged = if let Some(entry) = self.sessions.get(session_id) {
            if entry.live_output.is_empty() || entry.session.persist_history {
                from_store
            } else {
                StoredSessionSnapshot {
                    content: format!("{}{}", from_store.content, entry.live_output),
                    seq: entry.session.seq.max(from_store.seq),
                }
            }
        } else {
            from_store
        };
        Ok(merged.into())
    }

    pub(super) fn session_restore_snapshot_get(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionSnapshot, String> {
        if !self.sessions.contains_key(session_id) {
            return Err("session not found".to_string());
        }

        let snapshot = normalize_restore_replay_snapshot(
            self.store.session_terminal_replay_snapshot(session_id)?,
        );
        let snapshot = if snapshot.content.is_empty() {
            render_snapshot_for_terminal(&build_logical_snapshot(&self.store, session_id)?)
        } else {
            let is_disconnected = self
                .sessions
                .get(session_id)
                .is_none_or(|entry| entry.session.status == StoredSessionStatus::Disconnected);
            if is_disconnected {
                StoredSessionSnapshot {
                    content: append_disconnected_restore_cleanup(&snapshot.content),
                    seq: snapshot.seq,
                }
            } else {
                snapshot
            }
        };
        Ok(snapshot.into())
    }

    pub(super) fn session_set_persist(
        &mut self,
        session_id: &str,
        persist_history: bool,
    ) -> Result<(), String> {
        let mut flush_seq: Option<u64> = None;
        let mut flush_chunk: Option<String> = None;
        if let Some(entry) = self.sessions.get(session_id) {
            if entry.session.persist_history != persist_history
                && persist_history
                && !entry.live_output.is_empty()
            {
                flush_seq = Some(entry.session.seq.saturating_add(1));
                flush_chunk = Some(entry.live_output.clone());
            }
        }

        self.store
            .set_session_persist(session_id, persist_history)?;
        if let (Some(seq), Some(chunk)) = (flush_seq, flush_chunk.as_ref()) {
            let ts = now_millis();
            let sanitized_chunk = strip_zsh_prompt_spacer_artifact(chunk);
            let current_fragment = self
                .sessions
                .get(session_id)
                .map(|entry| {
                    (
                        entry.canonical_open_fragment.clone(),
                        entry.canonical_cursor_col,
                        entry.canonical_pending_carriage_return,
                    )
                })
                .unwrap_or_default();
            let materialized = materialize_output_chunk(
                &current_fragment.0,
                current_fragment.1,
                current_fragment.2,
                &sanitized_chunk,
            );
            self.store.update_session_seq(session_id, seq)?;
            self.store
                .append_scrollback_records(session_id, seq, &materialized.records, ts)?;
            self.store.enforce_session_scrollback_record_limit(
                session_id,
                self.config.max_scrollback_lines_per_session,
            )?;
            if let Some(entry) = self.sessions.get_mut(session_id) {
                entry.canonical_open_fragment = materialized.open_fragment;
                entry.canonical_cursor_col = materialized.cursor_col;
                entry.canonical_pending_carriage_return = materialized.pending_carriage_return;
            }
        }
        if let Some(entry) = self.sessions.get_mut(session_id) {
            if entry.session.persist_history != persist_history {
                if persist_history {
                    if let Some(seq) = flush_seq {
                        entry.session.seq = seq;
                        entry.live_output.clear();
                    }
                } else {
                    entry.live_output.clear();
                }
            }
            entry.session.persist_history = persist_history;
        }
        self.publish_session_updated_for(session_id);
        Ok(())
    }

    pub(super) fn workspace_layout_load(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let key = self.workspace_layout_key(workspace_id);
        let Some(raw) = self.store.get_string_state(&key)? else {
            return Ok(None);
        };

        let mut layout = serde_json::from_str::<WorkspaceLayoutState>(&raw)
            .map_err(|err| format!("parse workspace layout failed: {err}"))?;
        let valid_sessions = self.sessions.keys().map(|session_id| session_id.as_str());
        if !layout.retain_sessions(valid_sessions) {
            self.store.clear_state(&key)?;
            return Ok(None);
        }

        let normalized = serde_json::to_string(&layout)
            .map_err(|err| format!("serialize workspace layout failed: {err}"))?;
        if normalized != raw {
            self.store.set_string_state(&key, &normalized)?;
        }

        Ok(Some(layout))
    }

    pub(super) fn workspace_layout_save(
        &self,
        workspace_id: &str,
        layout: &WorkspaceLayoutState,
    ) -> Result<(), String> {
        let key = self.workspace_layout_key(workspace_id);
        let value = serde_json::to_string(layout)
            .map_err(|err| format!("serialize workspace layout failed: {err}"))?;
        self.store.set_string_state(&key, &value)
    }

    pub(super) fn workspace_layout_clear(&self, workspace_id: &str) -> Result<(), String> {
        let key = self.workspace_layout_key(workspace_id);
        self.store.clear_state(&key)
    }

    pub(super) fn get_lifecycle_preferences(&self) -> Result<RuntimeLifecyclePreferences, String> {
        Ok(RuntimeLifecyclePreferences {
            keep_alive_on_close: self
                .store
                .get_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, DEFAULT_KEEP_ALIVE_ON_CLOSE)?,
            start_in_tray: self
                .store
                .get_bool_state(START_IN_TRAY_KEY, DEFAULT_START_IN_TRAY)?,
        })
    }

    pub(super) fn set_lifecycle_preferences(
        &self,
        keep_alive_on_close: Option<bool>,
        start_in_tray: Option<bool>,
    ) -> Result<RuntimeLifecyclePreferences, String> {
        if let Some(next) = keep_alive_on_close {
            self.store.set_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, next)?;
        }
        if let Some(next) = start_in_tray {
            self.store.set_bool_state(START_IN_TRAY_KEY, next)?;
        }
        self.get_lifecycle_preferences()
    }
}
