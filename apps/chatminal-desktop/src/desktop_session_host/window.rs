use chatminal_runtime::{HostRuntimeNotification, RuntimeId};
use config::GuiPosition;

use super::publish_runtime_notification;

pub(crate) type WindowId = usize;
pub(crate) const ROOT_WINDOW_ID: WindowId = 0;

#[derive(Clone, Debug)]
pub(crate) struct WindowRuntimeEntry {
    runtime_id: RuntimeId,
    title: String,
}

impl WindowRuntimeEntry {
    pub(crate) fn new(runtime_id: RuntimeId) -> Self {
        Self {
            runtime_id,
            title: String::new(),
        }
    }

    pub(crate) fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn set_title(&mut self, title: &str) {
        self.title.clear();
        self.title.push_str(title);
    }
}

pub(crate) struct Window {
    runtime_entries: Vec<WindowRuntimeEntry>,
    active: usize,
    last_active: Option<RuntimeId>,
    workspace: String,
    title: String,
    initial_position: Option<GuiPosition>,
}

impl Window {
    pub(crate) fn new(workspace: String, initial_position: Option<GuiPosition>) -> Self {
        Self {
            runtime_entries: Vec::new(),
            active: 0,
            last_active: None,
            workspace,
            title: String::new(),
            initial_position,
        }
    }

    pub(crate) fn get_initial_position(&self) -> &Option<GuiPosition> {
        &self.initial_position
    }

    pub(crate) fn get_workspace(&self) -> &str {
        &self.workspace
    }

    pub(crate) fn set_title(&mut self, title: &str) {
        if self.title == title {
            return;
        }
        self.title = title.to_string();
        publish_runtime_notification(HostRuntimeNotification::WindowTitleChanged {
            title: self.title.clone(),
        });
    }

    pub(crate) fn get_title(&self) -> &str {
        &self.title
    }

    pub(crate) fn set_workspace(&mut self, workspace: &str) {
        if self.workspace == workspace {
            return;
        }
        self.workspace = workspace.to_string();
        publish_runtime_notification(HostRuntimeNotification::WindowWorkspaceChanged);
    }

    fn invalidate(&self) {
        publish_runtime_notification(HostRuntimeNotification::WindowInvalidated);
    }

    fn idx_by_id(&self, id: RuntimeId) -> Option<usize> {
        self.runtime_entries
            .iter()
            .position(|entry| entry.runtime_id() == id)
    }

    fn check_runtime_not_present(&self, runtime_id: RuntimeId) {
        assert!(
            self.idx_by_id(runtime_id).is_none(),
            "runtime entry already added to this window"
        );
    }

    pub(crate) fn push_runtime(&mut self, runtime_id: RuntimeId) {
        self.check_runtime_not_present(runtime_id);
        self.runtime_entries
            .push(WindowRuntimeEntry::new(runtime_id));
        self.invalidate();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.runtime_entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.runtime_entries.len()
    }

    pub(crate) fn get_by_idx(&self, idx: usize) -> Option<&WindowRuntimeEntry> {
        self.runtime_entries.get(idx)
    }

    pub(crate) fn get_by_idx_mut(&mut self, idx: usize) -> Option<&mut WindowRuntimeEntry> {
        self.runtime_entries.get_mut(idx)
    }

    pub(crate) fn entry_by_runtime_id_mut(
        &mut self,
        runtime_id: RuntimeId,
    ) -> Option<&mut WindowRuntimeEntry> {
        let idx = self.idx_by_id(runtime_id)?;
        self.get_by_idx_mut(idx)
    }

    pub(crate) fn remove_by_id(&mut self, id: RuntimeId) -> bool {
        let Some(idx) = self.idx_by_id(id) else {
            return false;
        };
        self.runtime_entries.remove(idx);
        if self.runtime_entries.is_empty() {
            self.active = 0;
            self.last_active = None;
            self.invalidate();
            return true;
        }
        if self.active >= self.runtime_entries.len() {
            self.active = self.runtime_entries.len() - 1;
        }
        self.invalidate();
        true
    }

    pub(crate) fn get_active(&self) -> Option<&WindowRuntimeEntry> {
        self.get_by_idx(self.active)
    }

    fn save_last_active(&mut self) {
        self.last_active = self.get_active().map(|tab| tab.runtime_id());
    }

    pub(crate) fn save_and_then_set_active(&mut self, idx: usize) {
        if idx == self.active || idx >= self.runtime_entries.len() {
            return;
        }
        self.save_last_active();
        self.set_active_without_saving(idx);
    }

    pub(crate) fn set_active_without_saving(&mut self, idx: usize) {
        if idx >= self.runtime_entries.len() {
            return;
        }
        self.active = idx;
        self.invalidate();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &WindowRuntimeEntry> {
        self.runtime_entries.iter()
    }
}
