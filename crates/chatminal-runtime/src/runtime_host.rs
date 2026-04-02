use crate::{RuntimeId, RuntimeSessionLaunchSpec, SessionTerminalHandle, TerminalInstanceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeTerminalSize {
    pub rows: usize,
    pub cols: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub dpi: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeHostSessionState {
    pub session_id: String,
    pub runtime_id: RuntimeId,
    pub active_terminal_instance_id: Option<TerminalInstanceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeHostTerminalBinding {
    pub session_id: String,
    pub runtime_id: RuntimeId,
    pub terminal_instance_id: TerminalInstanceId,
    pub terminal_handle: SessionTerminalHandle,
}

pub trait RuntimeHost: Send + Sync + std::fmt::Debug {
    fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId>;

    fn ensure_session_runtime(
        &self,
        launch: &RuntimeSessionLaunchSpec,
        generation: u64,
        size: RuntimeTerminalSize,
    ) -> Option<RuntimeHostSessionState>;

    fn focus_session_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<RuntimeHostSessionState>;

    fn hydrate_session_runtime(&self, runtime_id: RuntimeId) -> Option<RuntimeHostSessionState>;

    fn remember_runtime_terminal_size(&self, runtime_id: RuntimeId, size: RuntimeTerminalSize);

    fn terminal_binding_for_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<RuntimeHostTerminalBinding>;

    fn focus_terminal_instance(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<RuntimeHostSessionState>;

    fn focus_terminal_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<RuntimeHostSessionState> {
        let binding = self.terminal_binding_for_handle(terminal_handle)?;
        self.focus_terminal_instance(
            &binding.session_id,
            binding.runtime_id,
            binding.terminal_instance_id,
        )
    }

    fn resize_session_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        size: RuntimeTerminalSize,
    ) -> Option<RuntimeHostSessionState>;

    fn close_session_runtime(&self, session_id: &str, runtime_id: RuntimeId);
}
