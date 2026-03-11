use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use crate::{LeafId, LeafRuntime, LeafRuntimeEvent, LeafRuntimeSpawn, SessionCoreState, SurfaceId};

#[derive(Default)]
pub struct LeafRuntimeRegistry {
    runtimes: Mutex<HashMap<LeafId, Arc<LeafRuntime>>>,
}

impl std::fmt::Debug for LeafRuntimeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeafRuntimeRegistry")
            .field("runtime_count", &self.runtimes.lock().unwrap().len())
            .finish()
    }
}

impl LeafRuntimeRegistry {
    pub fn spawn_for_surface(
        &self,
        core_state: &Arc<Mutex<SessionCoreState>>,
        session_id: impl Into<String>,
        generation: u64,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        command: CommandBuilder,
        size: TerminalSize,
        events: std_mpsc::SyncSender<LeafRuntimeEvent>,
    ) -> Result<Arc<LeafRuntime>, String> {
        if self.runtimes.lock().unwrap().contains_key(&leaf_id) {
            return Err(format!("leaf runtime {leaf_id} already exists"));
        }
        let spawn = LeafRuntimeSpawn {
            session_id: session_id.into(),
            generation,
            surface_id,
            leaf_id,
            command,
            size,
        };
        let runtime = Arc::new(LeafRuntime::spawn(spawn.clone(), events)?);
        core_state
            .lock()
            .unwrap()
            .surface_mut(surface_id)
            .ok_or_else(|| format!("surface {surface_id} missing in session core state"))?
            .set_leaf_process(leaf_id, runtime.process_state(&spawn));
        self.runtimes
            .lock()
            .unwrap()
            .insert(leaf_id, Arc::clone(&runtime));
        Ok(runtime)
    }

    pub fn runtime(&self, leaf_id: LeafId) -> Option<Arc<LeafRuntime>> {
        self.runtimes.lock().unwrap().get(&leaf_id).cloned()
    }

    pub fn replay_output(&self, leaf_id: LeafId) -> Option<String> {
        self.runtime(leaf_id).map(|runtime| runtime.replay_output())
    }

    pub fn remove_for_surface(
        &self,
        core_state: &Arc<Mutex<SessionCoreState>>,
        surface_id: SurfaceId,
        leaf_id: LeafId,
    ) -> Option<Arc<LeafRuntime>> {
        let runtime = self.runtimes.lock().unwrap().remove(&leaf_id)?;
        runtime.kill();
        if let Some(surface) = core_state.lock().unwrap().surface_mut(surface_id) {
            surface.clear_leaf_process(leaf_id);
        }
        Some(runtime)
    }
}

#[cfg(test)]
#[path = "leaf_runtime_registry_tests.rs"]
mod tests;
