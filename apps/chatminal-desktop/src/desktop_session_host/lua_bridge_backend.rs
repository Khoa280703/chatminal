use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use super::{
    HostTerminal, active_frontend_client, active_workspace_for_client,
    focus_host_root_runtime_entry, host_iter_all_panes, host_root_active_runtime_id,
    host_root_runtime_entry_infos, host_root_window_spawn_context, host_root_window_title,
    host_root_window_workspace_name_value, host_runtime_available,
    host_runtime_entry_exists_for_session, host_runtime_entry_info_by_session_id,
    host_runtime_entry_terminal_handle_in_direction_by_session_id,
    host_runtime_entry_terminal_handles_by_session_id,
    host_runtime_entry_terminal_infos_by_session_id, host_spawn_tab_raw, primary_host_window_id,
    rename_host_workspace, resolve_host_runtime_id_for_terminal_handle,
    session_id_from_pane_metadata, set_active_workspace_for_client, set_host_active_workspace_name,
    set_host_root_window_title, set_host_root_window_workspace_name,
    set_host_runtime_entry_active_terminal, set_host_runtime_entry_title_by_session_id,
    terminal_handle_arc, workspace_names,
};
use chatminal_lua_bridge::backend::{
    BackendFuture, LuaBridgeBackend, LuaPane, LuaSessionRecord, LuaSessionTerminalRecord,
    LuaSpawnContext, LuaSpawnResult, LuaSplitSource,
};
use chatminal_runtime::{
    CachePolicy, LogicalLine, RenderableDimensions, SessionTerminalHandle, SplitRequest,
    StableCursorPosition,
};
use config::keyassignment::SessionDirection;
use engine_dynamic::Value;
use engine_term::{Progress, SemanticZone, StableRowIndex, TerminalSize};
use procinfo::LocalProcessInfo;
use termwiz::surface::Line;
use url::Url;

struct DesktopLuaPane(Arc<dyn HostTerminal>);

impl LuaPane for DesktopLuaPane {
    fn terminal_handle(&self) -> SessionTerminalHandle {
        self.0.terminal_handle()
    }
    fn get_metadata(&self) -> Value {
        self.0.get_metadata()
    }
    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        self.0.send_paste(text)
    }
    fn send_text(&self, text: &str) -> anyhow::Result<()> {
        self.0
            .writer()
            .write_all(text.as_bytes())
            .map_err(anyhow::Error::from)
    }
    fn get_title(&self) -> String {
        self.0.get_title()
    }
    fn get_progress(&self) -> Progress {
        self.0.get_progress()
    }
    fn get_current_working_dir(&self, policy: CachePolicy) -> Option<Url> {
        self.0.get_current_working_dir(policy)
    }
    fn get_foreground_process_name(&self, policy: CachePolicy) -> Option<String> {
        self.0.get_foreground_process_name(policy)
    }
    fn get_foreground_process_info(&self, policy: CachePolicy) -> Option<LocalProcessInfo> {
        self.0.get_foreground_process_info(policy)
    }
    fn get_cursor_position(&self) -> StableCursorPosition {
        self.0.get_cursor_position()
    }
    fn get_dimensions(&self) -> RenderableDimensions {
        self.0.get_dimensions()
    }
    fn copy_user_vars(&self) -> HashMap<String, String> {
        self.0.copy_user_vars()
    }
    fn has_unseen_output(&self) -> bool {
        self.0.has_unseen_output()
    }
    fn is_alt_screen_active(&self) -> bool {
        self.0.is_alt_screen_active()
    }
    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        self.0.get_lines(lines)
    }
    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        self.0.get_logical_lines(lines)
    }
    fn perform_actions(&self, actions: Vec<termwiz::escape::Action>) {
        self.0.perform_actions(actions)
    }
    fn get_semantic_zones(&self) -> anyhow::Result<Vec<SemanticZone>> {
        self.0.get_semantic_zones()
    }
    fn tty_name(&self) -> Option<String> {
        self.0.tty_name()
    }
}

pub(crate) struct DesktopLuaBridgeBackend;

impl DesktopLuaBridgeBackend {
    fn pane_adapter(pane: Arc<dyn HostTerminal>) -> Arc<dyn LuaPane> {
        Arc::new(DesktopLuaPane(pane))
    }
}

impl LuaBridgeBackend for DesktopLuaBridgeBackend {
    fn is_runtime_available(&self) -> bool {
        host_runtime_available()
    }
    fn root_window_id(&self) -> u64 {
        primary_host_window_id()
    }
    fn pane(&self, terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn LuaPane>> {
        terminal_handle_arc(terminal_handle).map(Self::pane_adapter)
    }
    fn all_panes(&self) -> Vec<Arc<dyn LuaPane>> {
        host_iter_all_panes()
            .into_iter()
            .map(Self::pane_adapter)
            .collect()
    }
    fn root_sessions(&self) -> Vec<LuaSessionRecord> {
        let active_runtime_id = host_root_active_runtime_id();
        host_root_runtime_entry_infos()
            .into_iter()
            .map(|info| LuaSessionRecord {
                session_id: info.session_id,
                title: info.title,
                size: info.size,
                active_terminal: info.active_terminal_handle,
                active_terminal_instance_id: info.active_terminal_instance_id,
                is_active: active_runtime_id == Some(info.runtime_id),
            })
            .collect()
    }
    fn session(&self, session_id: &str) -> Option<LuaSessionRecord> {
        let active_runtime_id = host_root_active_runtime_id();
        let info = host_runtime_entry_info_by_session_id(session_id)?;
        Some(LuaSessionRecord {
            session_id: info.session_id,
            title: info.title,
            size: info.size,
            active_terminal: info.active_terminal_handle,
            active_terminal_instance_id: info.active_terminal_instance_id,
            is_active: active_runtime_id == Some(info.runtime_id),
        })
    }
    fn spawn_root_session(
        &self,
        cmd_builder: Option<portable_pty::CommandBuilder>,
        cwd: Option<String>,
        size: TerminalSize,
        pane: Option<SessionTerminalHandle>,
    ) -> BackendFuture<anyhow::Result<LuaSpawnResult>> {
        Box::pin(async move {
            let pane = host_spawn_tab_raw(cmd_builder, cwd, size, pane).await?.pane;
            let session_id = session_id_from_pane_metadata(pane.as_ref())
                .ok_or_else(|| anyhow::anyhow!("spawned session has no chatminal session_id"))?;
            Ok(LuaSpawnResult {
                session_id,
                terminal: pane.terminal_handle(),
            })
        })
    }
    fn split_terminal(
        &self,
        _terminal_handle: SessionTerminalHandle,
        _request: SplitRequest,
        _source: LuaSplitSource,
    ) -> BackendFuture<anyhow::Result<SessionTerminalHandle>> {
        Box::pin(async move { anyhow::bail!("terminal splitting is not supported in this build") })
    }
    fn active_workspace(&self) -> anyhow::Result<String> {
        active_frontend_client()
            .map(|client_id| active_workspace_for_client(&client_id))
            .or_else(host_root_window_workspace_name_value)
            .ok_or_else(|| anyhow::anyhow!("root window not available"))
    }
    fn root_workspace(&self) -> anyhow::Result<String> {
        host_root_window_workspace_name_value()
            .ok_or_else(|| anyhow::anyhow!("root window not available"))
    }
    fn set_root_workspace(&self, workspace: &str) -> anyhow::Result<()> {
        set_host_root_window_workspace_name(workspace)
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("root window not available"))
    }
    fn workspace_names(&self) -> anyhow::Result<Vec<String>> {
        Ok(workspace_names())
    }
    fn set_active_workspace(&self, workspace: &str) -> anyhow::Result<()> {
        if let Some(client_id) = active_frontend_client() {
            set_active_workspace_for_client(&client_id, workspace);
            return Ok(());
        }
        set_host_active_workspace_name(workspace)
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("host runtime is not available"))
    }
    fn rename_workspace(&self, old_workspace: &str, new_workspace: &str) -> anyhow::Result<()> {
        rename_host_workspace(old_workspace, new_workspace)
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("host runtime is not available"))
    }
    fn root_title(&self) -> anyhow::Result<String> {
        host_root_window_title().ok_or_else(|| anyhow::anyhow!("root window not available"))
    }
    fn set_root_title(&self, title: &str) -> anyhow::Result<()> {
        set_host_root_window_title(title)
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("root window not available"))
    }
    fn root_spawn_context(&self) -> anyhow::Result<LuaSpawnContext> {
        let (size, sibling) = host_root_window_spawn_context();
        Ok(LuaSpawnContext { size, sibling })
    }
    fn set_session_title(&self, session_id: &str, title: &str) -> anyhow::Result<()> {
        set_host_runtime_entry_title_by_session_id(session_id, title)
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("session '{session_id}' not found in runtime"))
    }
    fn session_terminals(&self, session_id: &str) -> anyhow::Result<Vec<SessionTerminalHandle>> {
        let handles = host_runtime_entry_terminal_handles_by_session_id(session_id);
        if handles.is_empty() && !host_runtime_entry_exists_for_session(session_id) {
            return Err(anyhow::anyhow!(
                "session '{session_id}' not found in runtime"
            ));
        }
        Ok(handles)
    }
    fn terminal_in_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> anyhow::Result<Option<SessionTerminalHandle>> {
        if !host_runtime_entry_exists_for_session(session_id) {
            return Err(anyhow::anyhow!(
                "session '{session_id}' not found in runtime"
            ));
        }
        Ok(host_runtime_entry_terminal_handle_in_direction_by_session_id(session_id, direction))
    }
    fn set_session_zoomed(&self, session_id: &str, zoomed: bool) -> anyhow::Result<bool> {
        let _ = (session_id, zoomed);
        Err(anyhow::anyhow!(
            "session zoom is not supported in single-runtime Chatminal"
        ))
    }
    fn session_terminal_infos(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<LuaSessionTerminalRecord>> {
        let infos = host_runtime_entry_terminal_infos_by_session_id(session_id);
        if infos.is_empty() && !host_runtime_entry_exists_for_session(session_id) {
            return Err(anyhow::anyhow!(
                "session '{session_id}' not found in runtime"
            ));
        }
        Ok(infos
            .into_iter()
            .map(|info| LuaSessionTerminalRecord {
                index: info.index,
                is_active: info.is_active,
                is_zoomed: info.is_zoomed,
                left: info.left,
                top: info.top,
                width: info.width,
                pixel_width: info.pixel_width,
                height: info.height,
                pixel_height: info.pixel_height,
                terminal: info.terminal_handle,
                session_id: info.session_id,
                terminal_instance_id: info.terminal_instance_id,
            })
            .collect())
    }
    fn rotate_session_counter_clockwise(&self, session_id: &str) -> anyhow::Result<()> {
        let _ = session_id;
        Err(anyhow::anyhow!(
            "session rotation is not supported in single-runtime Chatminal"
        ))
    }
    fn rotate_session_clockwise(&self, session_id: &str) -> anyhow::Result<()> {
        let _ = session_id;
        Err(anyhow::anyhow!(
            "session rotation is not supported in single-runtime Chatminal"
        ))
    }
    fn activate_terminal(&self, terminal_handle: SessionTerminalHandle) -> anyhow::Result<()> {
        let runtime_id =
            resolve_host_runtime_id_for_terminal_handle(terminal_handle).ok_or_else(|| {
                anyhow::anyhow!(
                    "terminal handle {} not found in runtime",
                    terminal_handle.as_u64()
                )
            })?;
        if !focus_host_root_runtime_entry(runtime_id) {
            return Err(anyhow::anyhow!(
                "session handle {} is not attached to root window",
                runtime_id.as_u64()
            ));
        }
        if set_host_runtime_entry_active_terminal(runtime_id, terminal_handle) {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "terminal handle {} not found in runtime",
                terminal_handle.as_u64()
            ))
        }
    }
    fn activate_session(&self, session_id: &str) -> anyhow::Result<()> {
        let info = host_runtime_entry_info_by_session_id(session_id)
            .ok_or_else(|| anyhow::anyhow!("session '{session_id}' not found in runtime"))?;
        let terminal = info
            .active_terminal_handle
            .ok_or_else(|| anyhow::anyhow!("session '{session_id}' has no active terminal"))?;
        self.activate_terminal(terminal)
    }
}
