use std::collections::VecDeque;

/// Bounded output history buffer with byte tracking.
/// Caps total memory at MAX_BYTES by evicting oldest chunks.
const MAX_BYTES: usize = 512 * 1024; // 512 KB

pub struct OutputHistory {
    chunks: VecDeque<String>,
    total_bytes: usize,
}

impl OutputHistory {
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            total_bytes: 0,
        }
    }

    pub fn push(&mut self, chunk: String) {
        self.total_bytes += chunk.len();
        self.chunks.push_back(chunk);
        while self.total_bytes > MAX_BYTES {
            if let Some(old) = self.chunks.pop_front() {
                self.total_bytes -= old.len();
            } else {
                break;
            }
        }
    }

    pub fn replay(&self) -> String {
        let mut buf = String::with_capacity(self.total_bytes);
        for chunk in &self.chunks {
            buf.push_str(chunk);
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_at_max_bytes() {
        let mut h = OutputHistory::new();
        // Push 3MB of data in 1KB chunks
        let chunk = "x".repeat(1024);
        for _ in 0..3072 {
            h.push(chunk.clone());
        }
        assert!(h.total_bytes <= MAX_BYTES);
        assert!(!h.chunks.is_empty());
    }

    #[test]
    fn replay_joins_all_chunks() {
        let mut h = OutputHistory::new();
        h.push("hello ".into());
        h.push("world".into());
        assert_eq!(h.replay(), "hello world");
    }

    #[test]
    fn evicts_oldest_first() {
        let mut h = OutputHistory::new();
        h.push("old".into());
        // Fill past limit
        let big = "x".repeat(MAX_BYTES);
        h.push(big);
        // "old" should be evicted
        assert!(!h.replay().starts_with("old"));
    }
}
