use std::collections::HashMap;

const DEDUPE_WINDOW_EPOCHS: u64 = 2;
const DEDUPE_COMPACT_INTERVAL_EPOCHS: u64 = 16;
const DEDUPE_MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeDeduperKind {
    TextCommit,
    ImeCommit,
}

#[derive(Debug, Clone)]
pub struct ImeCommitDeduper {
    seen_text_commits: HashMap<String, u64>,
    seen_ime_commits: HashMap<String, u64>,
    epoch: u64,
}

impl Default for ImeCommitDeduper {
    fn default() -> Self {
        Self {
            seen_text_commits: HashMap::new(),
            seen_ime_commits: HashMap::new(),
            epoch: 0,
        }
    }
}

impl ImeCommitDeduper {
    pub fn start_frame(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        if self.epoch % DEDUPE_COMPACT_INTERVAL_EPOCHS == 0 {
            self.compact_recent();
        }
    }

    pub fn clear(&mut self) {
        self.seen_text_commits.clear();
        self.seen_ime_commits.clear();
    }

    pub fn should_skip(&self, kind: ImeDeduperKind, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let min_epoch = self.epoch.saturating_sub(DEDUPE_WINDOW_EPOCHS);
        match kind {
            ImeDeduperKind::TextCommit => self
                .seen_ime_commits
                .get(text)
                .is_some_and(|seen_epoch| *seen_epoch >= min_epoch),
            ImeDeduperKind::ImeCommit => self
                .seen_text_commits
                .get(text)
                .is_some_and(|seen_epoch| *seen_epoch >= min_epoch),
        }
    }

    pub fn mark_sent(&mut self, kind: ImeDeduperKind, text: &str) {
        if text.is_empty() {
            return;
        }
        match kind {
            ImeDeduperKind::TextCommit => {
                self.seen_text_commits.insert(text.to_string(), self.epoch);
            }
            ImeDeduperKind::ImeCommit => {
                self.seen_ime_commits.insert(text.to_string(), self.epoch);
            }
        }

        if self.seen_text_commits.len() > DEDUPE_MAX_ENTRIES
            || self.seen_ime_commits.len() > DEDUPE_MAX_ENTRIES
        {
            self.compact_recent();
        }
    }

    fn compact_recent(&mut self) {
        let min_epoch = self.epoch.saturating_sub(DEDUPE_WINDOW_EPOCHS);
        self.seen_text_commits
            .retain(|_, seen_epoch| *seen_epoch >= min_epoch);
        self.seen_ime_commits
            .retain(|_, seen_epoch| *seen_epoch >= min_epoch);
    }
}

#[cfg(test)]
mod tests {
    use super::{ImeCommitDeduper, ImeDeduperKind};

    #[test]
    fn preserves_order_text_then_ime_skip_second() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        assert!(!deduper.should_skip(ImeDeduperKind::TextCommit, "abc"));
        deduper.mark_sent(ImeDeduperKind::TextCommit, "abc");
        deduper.start_frame();
        assert!(deduper.should_skip(ImeDeduperKind::ImeCommit, "abc"));
    }

    #[test]
    fn preserves_order_ime_then_text_skip_second() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        assert!(!deduper.should_skip(ImeDeduperKind::ImeCommit, "x"));
        deduper.mark_sent(ImeDeduperKind::ImeCommit, "x");
        deduper.start_frame();
        assert!(deduper.should_skip(ImeDeduperKind::TextCommit, "x"));
    }

    #[test]
    fn repeated_text_without_ime_is_not_suppressed() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        assert!(!deduper.should_skip(ImeDeduperKind::TextCommit, "a"));
        assert!(!deduper.should_skip(ImeDeduperKind::TextCommit, "a"));
    }

    #[test]
    fn commit_not_marked_when_send_failed_keeps_text_unsuppressed() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        assert!(!deduper.should_skip(ImeDeduperKind::ImeCommit, "abc"));
        assert!(!deduper.should_skip(ImeDeduperKind::TextCommit, "abc"));
    }

    #[test]
    fn old_entries_expire_after_window() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        deduper.mark_sent(ImeDeduperKind::TextCommit, "abc");
        deduper.start_frame();
        deduper.start_frame();
        deduper.start_frame();
        assert!(!deduper.should_skip(ImeDeduperKind::ImeCommit, "abc"));
    }

    #[test]
    fn clear_resets_seen_entries() {
        let mut deduper = ImeCommitDeduper::default();
        deduper.start_frame();
        deduper.mark_sent(ImeDeduperKind::TextCommit, "abc");
        deduper.clear();
        assert!(!deduper.should_skip(ImeDeduperKind::ImeCommit, "abc"));
    }
}
