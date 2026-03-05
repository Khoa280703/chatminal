use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaneBinding {
    pub session_id: String,
    pub pane_id: String,
}

#[derive(Debug, Default)]
pub struct SessionPaneRegistry {
    bindings: Vec<SessionPaneBinding>,
    active_session_id: Option<String>,
    next_pane_index: u64,
}

#[allow(dead_code)]
impl SessionPaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_pane_for_session(&mut self, session_id: &str) -> String {
        if let Some(binding) = self
            .bindings
            .iter()
            .find(|value| value.session_id == session_id)
        {
            return binding.pane_id.clone();
        }

        self.next_pane_index = self.next_pane_index.saturating_add(1);
        let pane_id = format!("pane-{}", self.next_pane_index);
        self.bindings.push(SessionPaneBinding {
            session_id: session_id.to_string(),
            pane_id: pane_id.clone(),
        });
        pane_id
    }

    pub fn pane_for_session(&self, session_id: &str) -> Option<&str> {
        self.bindings
            .iter()
            .find(|value| value.session_id == session_id)
            .map(|value| value.pane_id.as_str())
    }

    pub fn activate_session(&mut self, session_id: &str) -> String {
        let pane_id = self.ensure_pane_for_session(session_id);
        self.active_session_id = Some(session_id.to_string());
        pane_id
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn active_pane_id(&self) -> Option<&str> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.pane_for_session(session_id))
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<String> {
        if let Some(index) = self
            .bindings
            .iter()
            .position(|value| value.session_id == session_id)
        {
            let binding = self.bindings.remove(index);
            if self.active_session_id.as_deref() == Some(session_id) {
                self.active_session_id = None;
            }
            return Some(binding.pane_id);
        }
        None
    }

    pub fn bindings(&self) -> &[SessionPaneBinding] {
        &self.bindings
    }

    pub fn prune_to_sessions(&mut self, session_ids: &[String]) {
        let keep = session_ids
            .iter()
            .map(|value| value.as_str())
            .collect::<HashSet<&str>>();
        self.bindings
            .retain(|binding| keep.contains(binding.session_id.as_str()));
        if let Some(active) = self.active_session_id.as_deref()
            && !keep.contains(active)
        {
            self.active_session_id = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SessionPaneRegistry;

    #[test]
    fn ensure_pane_for_session_is_stable() {
        let mut registry = SessionPaneRegistry::new();
        let first = registry.ensure_pane_for_session("s-1");
        let second = registry.ensure_pane_for_session("s-1");
        assert_eq!(first, second);
        assert_eq!(registry.bindings().len(), 1);
    }

    #[test]
    fn activate_and_remove_session_updates_active_state() {
        let mut registry = SessionPaneRegistry::new();
        let pane_a = registry.activate_session("s-a");
        let pane_b = registry.activate_session("s-b");
        assert_ne!(pane_a, pane_b);
        assert_eq!(registry.active_session_id(), Some("s-b"));
        assert_eq!(registry.active_pane_id(), Some(pane_b.as_str()));

        let removed = registry.remove_session("s-b");
        assert_eq!(removed.as_deref(), Some(pane_b.as_str()));
        assert_eq!(registry.active_session_id(), None);
        assert_eq!(registry.active_pane_id(), None);
        assert_eq!(registry.pane_for_session("s-a"), Some(pane_a.as_str()));
    }

    #[test]
    fn prune_to_sessions_removes_stale_bindings_and_active_id() {
        let mut registry = SessionPaneRegistry::new();
        registry.activate_session("s-a");
        registry.ensure_pane_for_session("s-b");
        registry.prune_to_sessions(&["s-b".to_string()]);

        assert_eq!(registry.bindings().len(), 1);
        assert_eq!(registry.pane_for_session("s-a"), None);
        assert_eq!(registry.active_session_id(), None);
        assert_eq!(registry.pane_for_session("s-b"), Some("pane-2"));
    }
}
