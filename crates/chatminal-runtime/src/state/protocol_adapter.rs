use std::sync::mpsc as std_mpsc;

use chatminal_protocol::{ClientFrame, PingResponse, Request, Response, ServerFrame};

use super::protocol_clients::ProtocolClientSender;
use super::{DaemonState, StateInner};
use crate::session::SessionEvent;

impl DaemonState {
    pub fn register_client(&self, client_id: u64, tx: ProtocolClientSender) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.protocol_clients.register(client_id, tx);
            inner.broadcast_daemon_health();
        }
    }

    pub fn unregister_client(&self, client_id: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.protocol_clients.unregister(client_id);
            inner.broadcast_daemon_health();
        }
    }

    pub fn handle_request(&self, frame: ClientFrame) -> ServerFrame {
        self.metrics.inc_requests_total();
        let id = frame.id;
        match frame.request {
            Request::WorkspaceLoad => {
                return match self.workspace_load() {
                    Ok(workspace) => ServerFrame::ok(id, Response::Workspace(workspace.into())),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::WorkspaceLoadPassive => {
                return match self.workspace_load_passive() {
                    Ok(workspace) => ServerFrame::ok(id, Response::Workspace(workspace.into())),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionSnapshotGet {
                session_id,
                preview_lines,
            } => {
                return match self.session_snapshot_get(&session_id, preview_lines) {
                    Ok(snapshot) => ServerFrame::ok(id, Response::SessionSnapshot(snapshot.into())),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionInputWrite { session_id, data } => {
                return match self.session_input_write(&session_id, &data) {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionResize {
                session_id,
                cols,
                rows,
            } => {
                return match self.session_resize(&session_id, cols, rows) {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionCreate {
                name,
                cols,
                rows,
                cwd,
                persist_history,
            } => {
                return match self.session_create(name, cols, rows, cwd, persist_history) {
                    Ok(created) => ServerFrame::ok(id, Response::SessionCreate(created.into())),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionActivate {
                session_id,
                cols,
                rows,
            } => {
                return match self.session_activate(&session_id, cols, rows) {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionClose { session_id } => {
                return match self.session_close(&session_id) {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::SessionHistoryClear { session_id } => {
                return match self.session_history_clear(&session_id) {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::WorkspaceHistoryClearAll => {
                return match self.workspace_history_clear_all() {
                    Ok(()) => ServerFrame::ok(id, Response::Empty),
                    Err(err) => {
                        self.metrics.inc_request_errors_total();
                        ServerFrame::err(id, err)
                    }
                };
            }
            Request::AppShutdown => {
                self.app_shutdown();
                return ServerFrame::ok(id, Response::Empty);
            }
            request => self.handle_protocol_request_with_lock(id, request),
        }
    }

    fn handle_protocol_request_with_lock(&self, id: String, request: Request) -> ServerFrame {
        let mut inner = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => {
                self.metrics.inc_request_errors_total();
                return ServerFrame::err(id, "state lock poisoned".to_string());
            }
        };

        let result = inner.handle_request(request, self.events.clone());
        match result {
            Ok(response) => ServerFrame::ok(id, response),
            Err(err) => {
                self.metrics.inc_request_errors_total();
                ServerFrame::err(id, err)
            }
        }
    }
}

impl StateInner {
    pub(super) fn handle_request(
        &mut self,
        request: Request,
        _events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<Response, String> {
        match request {
            Request::Ping => Ok(Response::Ping(PingResponse {
                message: "pong chatminald/1".to_string(),
            })),
            Request::LifecyclePreferencesGet => Ok(Response::LifecyclePreferences(
                self.get_lifecycle_preferences()?.into(),
            )),
            Request::LifecyclePreferencesSet {
                keep_alive_on_close,
                start_in_tray,
            } => Ok(Response::LifecyclePreferences(
                self.set_lifecycle_preferences(keep_alive_on_close, start_in_tray)?
                    .into(),
            )),
            Request::WorkspaceLoad | Request::WorkspaceLoadPassive => Err(
                "workspace load requests must be handled before state lock dispatch".to_string(),
            ),
            Request::ProfileList => Ok(Response::Profiles(
                self.store
                    .list_profiles()?
                    .into_iter()
                    .map(crate::api::RuntimeProfile::from)
                    .map(Into::into)
                    .collect(),
            )),
            Request::ProfileCreate { name } => {
                Ok(Response::Profile(self.profile_create(name)?.into()))
            }
            Request::ProfileRename { profile_id, name } => {
                let renamed = self.store.rename_profile(&profile_id, &name)?;
                self.publish_workspace_updated();
                Ok(Response::Profile(
                    crate::api::RuntimeProfile::from(renamed).into(),
                ))
            }
            Request::ProfileDelete { profile_id } => {
                self.store.delete_profile(&profile_id)?;
                self.close_profile_runtimes(&profile_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::ProfileSwitch { profile_id } => Ok(Response::Workspace(
                self.profile_switch(&profile_id)?.into(),
            )),
            Request::SessionList => {
                let workspace = self.store.load_workspace()?;
                let filtered = workspace
                    .sessions
                    .into_iter()
                    .filter(|value| value.profile_id == workspace.active_profile_id)
                    .map(crate::api::RuntimeSession::from)
                    .map(Into::into)
                    .collect();
                Ok(Response::Sessions(filtered))
            }
            Request::SessionCreate { .. } => Err(
                "session create requests must be handled before state lock dispatch".to_string(),
            ),
            Request::SessionActivate { .. } => Err(
                "session activate requests must be handled before state lock dispatch".to_string(),
            ),
            Request::SessionRename { session_id, name } => {
                self.store.rename_session(&session_id, &name)?;
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    entry.session.name = name.trim().to_string();
                }
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionClose { .. } => {
                Err("session close requests must be handled before state lock dispatch".to_string())
            }
            Request::SessionSetPersist {
                session_id,
                persist_history,
            } => {
                self.session_set_persist(&session_id, persist_history)?;
                Ok(Response::Empty)
            }
            Request::SessionInputWrite { .. } => Err(
                "session input write requests must be handled before state lock dispatch"
                    .to_string(),
            ),
            Request::SessionResize { .. } => Err(
                "session resize requests must be handled before state lock dispatch".to_string(),
            ),
            Request::SessionSnapshotGet {
                session_id,
                preview_lines,
            } => Ok(Response::SessionSnapshot(
                self.session_snapshot_get(&session_id, preview_lines)?
                    .into(),
            )),
            Request::SessionExplorerStateGet { session_id } => Ok(Response::SessionExplorerState(
                self.get_session_explorer_state(&session_id)?.into(),
            )),
            Request::SessionExplorerRootSet {
                session_id,
                root_path,
            } => Ok(Response::SessionExplorerState(
                self.set_session_explorer_root(&session_id, &root_path)?
                    .into(),
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
                )?
                .into(),
            )),
            Request::SessionExplorerList {
                session_id,
                relative_path,
            } => Ok(Response::SessionExplorerEntries(
                self.list_session_explorer_entries(&session_id, relative_path.as_deref())?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )),
            Request::SessionExplorerReadFile {
                session_id,
                relative_path,
                max_bytes,
            } => Ok(Response::SessionExplorerFileContent(
                self.read_session_explorer_file(&session_id, &relative_path, max_bytes)?
                    .into(),
            )),
            Request::SessionHistoryClear { .. } => Err(
                "session history clear requests must be handled before state lock dispatch"
                    .to_string(),
            ),
            Request::WorkspaceHistoryClearAll => Err(
                "workspace history clear requests must be handled before state lock dispatch"
                    .to_string(),
            ),
            Request::AppShutdown => {
                Err("app shutdown requests must be handled before state lock dispatch".to_string())
            }
        }
    }
}
