use config::keyassignment::SessionDirection;

use super::{RuntimeId, SessionRuntimeState, TerminalInstanceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineRuntimeRef {
    pub runtime_id: RuntimeId,
    pub session_id: String,
}

pub trait EngineRuntimeAdapter: Send + Sync {
    type Error;

    fn attach_runtime(&self, session_id: &str) -> Result<EngineRuntimeRef, Self::Error>;
    fn focus_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Result<(), Self::Error>;
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
    fn snapshot_runtime(&self, runtime_id: RuntimeId) -> Result<SessionRuntimeState, Self::Error>;
}
