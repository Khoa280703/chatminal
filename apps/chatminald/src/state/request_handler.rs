use super::*;
use crate::session::WriteInputError;

impl StateInner {
    pub(super) fn handle_request(
        &mut self,
        request: Request,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<Response, String> {
        match request {
            Request::Ping => Ok(Response::Ping(PingResponse {
                message: "pong chatminald/1".to_string(),
            })),
            Request::LifecyclePreferencesGet => Ok(Response::LifecyclePreferences(
                self.get_lifecycle_preferences()?,
            )),
            Request::LifecyclePreferencesSet {
                keep_alive_on_close,
                start_in_tray,
            } => Ok(Response::LifecyclePreferences(
                self.set_lifecycle_preferences(keep_alive_on_close, start_in_tray)?,
            )),
            Request::WorkspaceLoad => {
                self.ensure_active_session_runtime(events.clone())?;
                Ok(Response::Workspace(self.load_workspace()?))
            }
            Request::ProfileList => Ok(Response::Profiles(self.store.list_profiles()?)),
            Request::ProfileCreate { name } => {
                let created = self.store.create_profile(name)?;
                self.publish_workspace_updated();
                Ok(Response::Profile(created))
            }
            Request::ProfileRename { profile_id, name } => {
                let renamed = self.store.rename_profile(&profile_id, &name)?;
                self.publish_workspace_updated();
                Ok(Response::Profile(renamed))
            }
            Request::ProfileDelete { profile_id } => {
                self.store.delete_profile(&profile_id)?;
                self.close_profile_runtimes(&profile_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::ProfileSwitch { profile_id } => {
                let exists = self
                    .store
                    .list_profiles()?
                    .iter()
                    .any(|value| value.profile_id == profile_id);
                if !exists {
                    return Err("profile not found".to_string());
                }
                self.store.set_active_profile(&profile_id)?;
                self.publish_workspace_updated();
                Ok(Response::Workspace(self.load_workspace()?))
            }
            Request::SessionList => {
                let (_, active_profile_id, sessions, _) = self.store.load_workspace()?;
                let filtered: Vec<SessionInfo> = sessions
                    .into_iter()
                    .filter(|value| value.profile_id == active_profile_id)
                    .collect();
                Ok(Response::Sessions(filtered))
            }
            Request::SessionCreate {
                name,
                cols,
                rows,
                cwd,
                persist_history,
            } => {
                let (_, active_profile_id, _, _) = self.store.load_workspace()?;
                let created = self.store.create_session(
                    &active_profile_id,
                    name,
                    resolve_session_cwd(cwd),
                    self.config.default_shell.clone(),
                    persist_history.unwrap_or(false),
                )?;

                self.store
                    .set_active_session(&active_profile_id, Some(&created.session_id))?;

                let runtime = SessionRuntime::spawn(
                    created.session_id.clone(),
                    0,
                    created.shell.clone(),
                    created.cwd.clone(),
                    cols,
                    rows,
                    events,
                )
                .map_err(|err| {
                    let _ = self.store.delete_session(&created.session_id);
                    err
                })?;
                self.store
                    .set_session_status(&created.session_id, SessionStatus::Running)?;

                let mut entry = SessionEntry {
                    session: created.clone(),
                    runtime: Some(runtime),
                    live_output: String::new(),
                    generation: 0,
                };
                entry.session.status = SessionStatus::Running;
                self.sessions.insert(created.session_id.clone(), entry);
                self.publish_session_updated_for(&created.session_id);
                self.publish_workspace_updated();

                Ok(Response::SessionCreate(CreateSessionResponse {
                    session_id: created.session_id,
                    name: created.name,
                }))
            }
            Request::SessionActivate {
                session_id,
                cols,
                rows,
            } => {
                let profile_id = if let Some(entry) = self.sessions.get(&session_id) {
                    entry.session.profile_id.clone()
                } else {
                    return Err("session not found".to_string());
                };

                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };

                if entry.runtime.is_none() {
                    entry.generation = entry.generation.saturating_add(1);
                    let runtime = SessionRuntime::spawn(
                        entry.session.session_id.clone(),
                        entry.generation,
                        entry.session.shell.clone(),
                        entry.session.cwd.clone(),
                        cols,
                        rows,
                        events,
                    )?;
                    entry.runtime = Some(runtime);
                    entry.session.status = SessionStatus::Running;
                    self.store
                        .set_session_status(&entry.session.session_id, SessionStatus::Running)?;
                }
                self.store
                    .set_active_session(&profile_id, Some(&session_id))?;
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionRename { session_id, name } => {
                self.store.rename_session(&session_id, &name)?;
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    entry.session.name = name.trim().to_string();
                }
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionClose { session_id } => {
                self.store.delete_session(&session_id)?;
                if let Some(mut entry) = self.sessions.remove(&session_id) {
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionSetPersist {
                session_id,
                persist_history,
            } => {
                let mut flush_seq: Option<u64> = None;
                let mut flush_chunk: Option<String> = None;
                if let Some(entry) = self.sessions.get(&session_id) {
                    if entry.session.persist_history != persist_history
                        && persist_history
                        && !entry.live_output.is_empty()
                    {
                        flush_seq = Some(entry.session.seq.saturating_add(1));
                        flush_chunk = Some(entry.live_output.clone());
                    }
                }

                self.store
                    .set_session_persist(&session_id, persist_history)?;
                if let (Some(seq), Some(chunk)) = (flush_seq, flush_chunk.as_ref()) {
                    let ts = now_millis();
                    self.store.update_session_seq(&session_id, seq)?;
                    self.store
                        .append_scrollback_chunk(&session_id, seq, chunk, ts)?;
                    self.store.enforce_session_scrollback_line_limit(
                        &session_id,
                        self.config.max_scrollback_lines_per_session,
                    )?;
                }
                if let Some(entry) = self.sessions.get_mut(&session_id) {
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
                self.publish_session_updated_for(&session_id);
                Ok(Response::Empty)
            }
            Request::SessionInputWrite { session_id, data } => {
                if data.len() > MAX_INPUT_BYTES {
                    return Err(format!(
                        "input payload too large ({} bytes > {} bytes)",
                        data.len(),
                        MAX_INPUT_BYTES
                    ));
                }
                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };
                let Some(runtime) = entry.runtime.as_ref() else {
                    return Err("session is not running".to_string());
                };
                match runtime.write_input(&data) {
                    Ok(stats) => {
                        self.metrics
                            .add_input_queue_full_total(stats.queue_full_hits);
                        self.metrics.add_input_retry_total(stats.retries);
                        self.metrics.add_input_drop_total(stats.drops);
                    }
                    Err(WriteInputError::QueueFullDropped(stats)) => {
                        self.metrics
                            .add_input_queue_full_total(stats.queue_full_hits);
                        self.metrics.add_input_retry_total(stats.retries);
                        self.metrics.add_input_drop_total(stats.drops);
                        return Err(WriteInputError::QueueFullDropped(stats).to_string());
                    }
                    Err(err) => return Err(err.to_string()),
                }
                Ok(Response::Empty)
            }
            Request::SessionResize {
                session_id,
                cols,
                rows,
            } => {
                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };
                let Some(runtime) = entry.runtime.as_ref() else {
                    return Err("session is not running".to_string());
                };
                runtime.resize(cols, rows)?;
                Ok(Response::Empty)
            }
            Request::SessionSnapshotGet {
                session_id,
                preview_lines,
            } => {
                if !self.sessions.contains_key(&session_id) {
                    return Err("session not found".to_string());
                }

                let from_store = self.store.session_snapshot(
                    &session_id,
                    preview_lines.unwrap_or(self.config.default_preview_lines),
                )?;
                let merged = if let Some(entry) = self.sessions.get(&session_id) {
                    if entry.live_output.is_empty() || entry.session.persist_history {
                        from_store
                    } else {
                        SessionSnapshot {
                            content: format!("{}{}", from_store.content, entry.live_output),
                            seq: entry.session.seq.max(from_store.seq),
                        }
                    }
                } else {
                    from_store
                };
                Ok(Response::SessionSnapshot(merged))
            }
            Request::SessionExplorerStateGet { session_id } => Ok(Response::SessionExplorerState(
                self.get_session_explorer_state(&session_id)?,
            )),
            Request::SessionExplorerRootSet {
                session_id,
                root_path,
            } => Ok(Response::SessionExplorerState(
                self.set_session_explorer_root(&session_id, &root_path)?,
            )),
            Request::SessionExplorerStateUpdate {
                session_id,
                current_dir,
                selected_path,
                open_file_path,
            } => Ok(Response::SessionExplorerState(
                self.update_session_explorer_state(
                    &session_id,
                    &current_dir,
                    selected_path.as_deref(),
                    open_file_path.as_deref(),
                )?,
            )),
            Request::SessionExplorerList {
                session_id,
                relative_path,
            } => Ok(Response::SessionExplorerEntries(
                self.list_session_explorer_entries(&session_id, relative_path.as_deref())?,
            )),
            Request::SessionExplorerReadFile {
                session_id,
                relative_path,
                max_bytes,
            } => Ok(Response::SessionExplorerFileContent(
                self.read_session_explorer_file(&session_id, &relative_path, max_bytes)?,
            )),
            Request::SessionHistoryClear { session_id } => {
                self.store.clear_session_history(&session_id)?;
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    entry.generation = entry.generation.saturating_add(1);
                    entry.session.seq = 0;
                    entry.live_output.clear();
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    self.store
                        .set_session_status(&session_id, SessionStatus::Disconnected)?;
                }
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::WorkspaceHistoryClearAll => {
                self.store.clear_all_history()?;
                let mut updated_ids = Vec::new();
                for entry in self.sessions.values_mut() {
                    entry.generation = entry.generation.saturating_add(1);
                    entry.session.seq = 0;
                    entry.live_output.clear();
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = self
                        .store
                        .set_session_status(&entry.session.session_id, SessionStatus::Disconnected);
                    updated_ids.push(entry.session.session_id.clone());
                }
                for session_id in updated_ids {
                    self.publish_session_updated_for(&session_id);
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::AppShutdown => {
                self.shutdown_requested = true;
                let mut updated_ids = Vec::new();
                for entry in self.sessions.values_mut() {
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = self
                        .store
                        .set_session_status(&entry.session.session_id, SessionStatus::Disconnected);
                    updated_ids.push(entry.session.session_id.clone());
                }
                for session_id in updated_ids {
                    self.publish_session_updated_for(&session_id);
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
        }
    }

    pub(super) fn load_workspace(&self) -> Result<WorkspaceState, String> {
        let (profiles, active_profile_id, mut sessions, active_session_id) =
            self.store.load_workspace()?;
        for session in &mut sessions {
            if let Some(entry) = self.sessions.get(&session.session_id) {
                session.status = entry.session.status.clone();
                session.seq = entry.session.seq;
                session.persist_history = entry.session.persist_history;
                session.cwd = entry.session.cwd.clone();
                session.name = entry.session.name.clone();
            }
        }

        Ok(WorkspaceState {
            profiles,
            active_profile_id: Some(active_profile_id),
            sessions,
            active_session_id,
        })
    }

    pub(super) fn get_lifecycle_preferences(&self) -> Result<LifecyclePreferences, String> {
        Ok(LifecyclePreferences {
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
    ) -> Result<LifecyclePreferences, String> {
        if let Some(next) = keep_alive_on_close {
            self.store.set_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, next)?;
        }
        if let Some(next) = start_in_tray {
            self.store.set_bool_state(START_IN_TRAY_KEY, next)?;
        }
        self.get_lifecycle_preferences()
    }
}
