use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use super::leaf_runtime::{
    TerminalInstanceRuntime, TerminalInstanceRuntimeEvent, TerminalInstanceRuntimeSpawn,
};
use super::{RuntimeId, SessionCoreState, TerminalInstanceId};

#[derive(Default)]
pub struct TerminalInstanceRuntimeRegistry {
    runtimes: Mutex<HashMap<TerminalInstanceId, Arc<TerminalInstanceRuntime>>>,
}

impl std::fmt::Debug for TerminalInstanceRuntimeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalInstanceRuntimeRegistry")
            .field("runtime_count", &self.runtimes.lock().unwrap().len())
            .finish()
    }
}

impl TerminalInstanceRuntimeRegistry {
    pub fn spawn_for_runtime(
        &self,
        core_state: &Arc<Mutex<SessionCoreState>>,
        session_id: impl Into<String>,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        command: CommandBuilder,
        size: TerminalSize,
        initial_scrollback: Option<String>,
        events: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
    ) -> Result<Arc<TerminalInstanceRuntime>, String> {
        if self
            .runtimes
            .lock()
            .unwrap()
            .contains_key(&terminal_instance_id)
        {
            return Err(format!(
                "terminal instance runtime {terminal_instance_id} already exists"
            ));
        }
        let spawn = TerminalInstanceRuntimeSpawn {
            session_id: session_id.into(),
            generation,
            runtime_id,
            terminal_instance_id,
            command,
            size,
            initial_scrollback,
        };
        let runtime = Arc::new(TerminalInstanceRuntime::spawn(spawn.clone(), events)?);
        core_state
            .lock()
            .unwrap()
            .runtime_mut(runtime_id)
            .ok_or_else(|| format!("runtime {runtime_id} missing in session core state"))?
            .set_leaf_process(terminal_instance_id, runtime.process_state(&spawn));
        self.runtimes
            .lock()
            .unwrap()
            .insert(terminal_instance_id, Arc::clone(&runtime));
        Ok(runtime)
    }

    pub fn runtime(
        &self,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<Arc<TerminalInstanceRuntime>> {
        self.runtimes
            .lock()
            .unwrap()
            .get(&terminal_instance_id)
            .cloned()
    }

    pub fn replay_output(&self, terminal_instance_id: TerminalInstanceId) -> Option<String> {
        self.runtime(terminal_instance_id)
            .map(|runtime| runtime.replay_output())
    }

    pub fn remove_for_runtime(
        &self,
        core_state: &Arc<Mutex<SessionCoreState>>,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<Arc<TerminalInstanceRuntime>> {
        let runtime = self
            .runtimes
            .lock()
            .unwrap()
            .remove(&terminal_instance_id)?;
        runtime.kill();
        if let Some(runtime) = core_state.lock().unwrap().runtime_mut(runtime_id) {
            runtime.leaves.remove(&terminal_instance_id);
        }
        Some(runtime)
    }
}

#[cfg(test)]
#[path = "leaf_runtime_registry_tests.rs"]
mod tests;
