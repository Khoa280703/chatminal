use config::keyassignment::SessionDirection;
use config::keyassignment::SpawnSessionDomain;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;
use window::Window;

use super::{TerminalInstanceId, SessionRuntimeState, RuntimeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineRuntimeRef {
    pub runtime_id: RuntimeId,
    pub session_id: String,
}

#[derive(Clone, Debug)]
pub struct SpawnSessionRuntimeRequest {
    pub session_id: String,
    pub terminal_size: TerminalSize,
    pub current_host_handle: Option<u64>,
    pub workspace: String,
    pub domain: SpawnSessionDomain,
    pub command: CommandBuilder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTerminalInstanceTarget {
    NewWindow,
    NewRuntimeInWindow,
}

pub trait EngineRuntimeAdapter: Send + Sync {
    type Error;

    fn attach_runtime(&self, session_id: &str) -> Result<EngineRuntimeRef, Self::Error>;
    fn focus_runtime(&self, runtime_id: RuntimeId) -> Result<(), Self::Error>;
    fn focus_terminal_instance(&self, runtime_id: RuntimeId, terminal_instance_id: TerminalInstanceId) -> Result<(), Self::Error>;
    fn adjacent_active_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        direction: SessionDirection,
    ) -> Result<Option<TerminalInstanceId>, Self::Error>;
    fn swap_active_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<(), Self::Error>;
    fn move_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        target: MoveTerminalInstanceTarget,
    ) -> Result<(), Self::Error>;
    fn close_runtime(&self, runtime_id: RuntimeId) -> Result<(), Self::Error>;
    fn spawn_runtime(
        &self,
        request: SpawnSessionRuntimeRequest,
        window: Option<Window>,
    ) -> Result<(), Self::Error>;
    fn snapshot_runtime(&self, runtime_id: RuntimeId) -> Result<SessionRuntimeState, Self::Error>;
}
