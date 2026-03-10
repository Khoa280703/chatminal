use chatminal_store::StoredSessionSnapshot;

use super::{
    DEFAULT_KEEP_ALIVE_ON_CLOSE, DEFAULT_START_IN_TRAY, KEEP_ALIVE_ON_CLOSE_KEY, START_IN_TRAY_KEY,
    StateInner, now_millis,
};
use crate::api::{
    RuntimeLifecyclePreferences, RuntimeProfile, RuntimeSessionSnapshot, RuntimeWorkspace,
};

impl StateInner {
    pub(super) fn load_workspace_snapshot(&self) -> Result<RuntimeWorkspace, String> {
        let mut workspace = self.store.load_workspace()?;
        for session in &mut workspace.sessions {
            if let Some(entry) = self.sessions.get(&session.session_id) {
                session.status = entry.session.status.clone().into();
                session.seq = entry.session.seq;
                session.persist_history = entry.session.persist_history;
                session.cwd = entry.session.cwd.clone();
                session.name = entry.session.name.clone();
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

    pub(super) fn session_snapshot_get(
        &self,
        session_id: &str,
        preview_lines: Option<usize>,
    ) -> Result<RuntimeSessionSnapshot, String> {
        if !self.sessions.contains_key(session_id) {
            return Err("session not found".to_string());
        }

        let from_store = self.store.session_snapshot(
            session_id,
            preview_lines.unwrap_or(self.config.default_preview_lines),
        )?;
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

    pub(super) fn session_set_persist(
        &mut self,
        session_id: &str,
        persist_history: bool,
    ) -> Result<(), String> {
        let mut flush_seq: Option<u64> = None;
        let mut flush_chunk: Option<String> = None;
        if let Some(entry) = self.sessions.get(session_id)
            && entry.session.persist_history != persist_history
            && persist_history
            && !entry.live_output.is_empty()
        {
            flush_seq = Some(entry.session.seq.saturating_add(1));
            flush_chunk = Some(entry.live_output.clone());
        }

        self.store
            .set_session_persist(session_id, persist_history)?;
        if let (Some(seq), Some(chunk)) = (flush_seq, flush_chunk.as_ref()) {
            let ts = now_millis();
            self.store.update_session_seq(session_id, seq)?;
            self.store
                .append_scrollback_chunk(session_id, seq, chunk, ts)?;
            self.store.enforce_session_scrollback_line_limit(
                session_id,
                self.config.max_scrollback_lines_per_session,
            )?;
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
