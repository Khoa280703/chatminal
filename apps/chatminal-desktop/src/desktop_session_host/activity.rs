use std::sync::atomic::{AtomicUsize, Ordering};

static COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Activity;

impl Activity {
    pub(crate) fn new() -> Self {
        COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }

    pub(crate) fn count() -> usize {
        COUNT.load(Ordering::SeqCst)
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}
