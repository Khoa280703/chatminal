use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaneBinding {
    pub session_id: String,
    pub terminal_id: String,
}

#[derive(Debug, Default)]
pub struct SessionPaneRegistry {
    bindings: Vec<SessionPaneBinding>,
    active_session_id: Option<String>,
    next_terminal_index: u64,
}

#[allow(dead_code)]
impl SessionPaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_terminal_for_session(&mut self, session_id: &str) -> String {
        if let Some(binding) = self
            .bindings
            .iter()
            .find(|value| value.session_id == session_id)
        {
            return binding.terminal_id.clone();
        }

        self.next_terminal_index = self.next_terminal_index.saturating_add(1);
        let terminal_id = format!("terminal-{}", self.next_terminal_index);
        self.bindings.push(SessionPaneBinding {
            session_id: session_id.to_string(),
            terminal_id: terminal_id.clone(),
        });
        terminal_id
    }

    pub fn terminal_for_session(&self, session_id: &str) -> Option<&str> {
        self.bindings
            .iter()
            .find(|value| value.session_id == session_id)
            .map(|value| value.terminal_id.as_str())
    }

    pub fn activate_session(&mut self, session_id: &str) -> String {
        let terminal_id = self.ensure_terminal_for_session(session_id);
        self.active_session_id = Some(session_id.to_string());
        terminal_id
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn active_terminal_id(&self) -> Option<&str> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_for_session(session_id))
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
            return Some(binding.terminal_id);
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
    fn ensure_terminal_for_session_is_stable() {
        let mut registry = SessionPaneRegistry::new();
        let first = registry.ensure_terminal_for_session("s-1");
        let second = registry.ensure_terminal_for_session("s-1");
        assert_eq!(first, second);
        assert_eq!(registry.bindings().len(), 1);
    }

    #[test]
    fn activate_and_remove_session_updates_active_state() {
        let mut registry = SessionPaneRegistry::new();
        let terminal_a = registry.activate_session("s-a");
        let terminal_b = registry.activate_session("s-b");
        assert_ne!(terminal_a, terminal_b);
        assert_eq!(registry.active_session_id(), Some("s-b"));
        assert_eq!(registry.active_terminal_id(), Some(terminal_b.as_str()));

        let removed = registry.remove_session("s-b");
        assert_eq!(removed.as_deref(), Some(terminal_b.as_str()));
        assert_eq!(registry.active_session_id(), None);
        assert_eq!(registry.active_terminal_id(), None);
        assert_eq!(registry.terminal_for_session("s-a"), Some(terminal_a.as_str()));
    }

    #[test]
    fn prune_to_sessions_removes_stale_bindings_and_active_id() {
        let mut registry = SessionPaneRegistry::new();
        registry.activate_session("s-a");
        registry.ensure_terminal_for_session("s-b");
        registry.prune_to_sessions(&["s-b".to_string()]);

        assert_eq!(registry.bindings().len(), 1);
        assert_eq!(registry.terminal_for_session("s-a"), None);
        assert_eq!(registry.active_session_id(), None);
        assert_eq!(registry.terminal_for_session("s-b"), Some("terminal-2"));
    }
}
