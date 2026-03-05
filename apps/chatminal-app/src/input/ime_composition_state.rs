use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum ImeCompositionPhase {
    Idle,
    Composing { preedit_hash: Option<u64> },
    Committed { text: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImeCompositionSnapshot {
    pub phase: &'static str,
    pub preedit_hash: Option<u64>,
    pub committed_text_len: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ImeCompositionState {
    phase: ImeCompositionPhase,
}

impl Default for ImeCompositionState {
    fn default() -> Self {
        Self {
            phase: ImeCompositionPhase::Idle,
        }
    }
}

impl ImeCompositionState {
    pub fn mark_composing(&mut self, preedit: Option<&str>) {
        self.phase = ImeCompositionPhase::Composing {
            preedit_hash: preedit.map(hash_text),
        };
    }

    pub fn mark_commit(&mut self, text: &str) {
        self.phase = ImeCompositionPhase::Committed {
            text: text.to_string(),
        };
    }

    pub fn on_focus_lost(&mut self) {
        self.phase = ImeCompositionPhase::Idle;
    }

    pub fn snapshot(&self) -> ImeCompositionSnapshot {
        match &self.phase {
            ImeCompositionPhase::Idle => ImeCompositionSnapshot {
                phase: "idle",
                preedit_hash: None,
                committed_text_len: None,
            },
            ImeCompositionPhase::Composing { preedit_hash } => ImeCompositionSnapshot {
                phase: "composing",
                preedit_hash: *preedit_hash,
                committed_text_len: None,
            },
            ImeCompositionPhase::Committed { text } => ImeCompositionSnapshot {
                phase: "committed",
                preedit_hash: None,
                committed_text_len: Some(text.len()),
            },
        }
    }
}

fn hash_text(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::ImeCompositionState;

    #[test]
    fn composition_state_tracks_composing_and_commit_transitions() {
        let mut state = ImeCompositionState::default();
        assert_eq!(state.snapshot().phase, "idle");

        state.mark_composing(Some("ti"));
        let composing = state.snapshot();
        assert_eq!(composing.phase, "composing");
        assert!(composing.preedit_hash.is_some());

        state.mark_commit("tí");
        let committed = state.snapshot();
        assert_eq!(committed.phase, "committed");
        assert_eq!(committed.committed_text_len, Some("tí".len()));
    }

    #[test]
    fn composition_state_resets_on_focus_lost() {
        let mut state = ImeCompositionState::default();
        state.mark_composing(None);
        state.on_focus_lost();
        assert_eq!(state.snapshot().phase, "idle");
    }
}
