// Async persist pipeline — offloads SQLite writes from the output event processor.
//
// The persist worker runs on a dedicated background thread. It receives `PersistJob`
// messages via a bounded channel and flushes them to SQLite. This decouples the
// hot-path output event processing from disk I/O.
//
// Coalescing: multiple `UpdateSeq` jobs for the same session are coalesced to
// persist only the highest seq. `MarkRunning` is deduped per-session.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;
use std::thread;

use chatminal_store::{Store, StoredScrollbackRecordInput, StoredSessionStatus};

#[allow(dead_code)]
pub(super) enum PersistJob {
    OutputChunk {
        session_id: Arc<str>,
        seq: u64,
        records: Vec<StoredScrollbackRecordInput>,
        replay_chunk: String,
        ts: u64,
    },
    UpdateSeq {
        session_id: Arc<str>,
        seq: u64,
    },
    MarkRunning {
        session_id: Arc<str>,
    },
    EnforceLimit {
        session_id: Arc<str>,
        max_lines: usize,
    },
    /// Barrier: flushes all pending work, then responds via the ack channel.
    /// Used by tests to synchronize with the persist worker.
    Flush {
        ack: std_mpsc::SyncSender<()>,
    },
    Shutdown,
}

/// Spawn the persist worker thread. Returns `(sender, join_handle)`.
///
/// The sender is `Clone` and should be stored on `RuntimeState`.
/// The join handle should be stored on `StateInner` for orderly shutdown.
pub(super) fn spawn_persist_worker(
    store: Store,
) -> (std_mpsc::SyncSender<PersistJob>, thread::JoinHandle<()>) {
    let (tx, rx) = std_mpsc::sync_channel::<PersistJob>(4096);
    let handle = thread::Builder::new()
        .name("persist-worker".to_string())
        .spawn(move || run_persist_loop(store, rx))
        .expect("spawn persist worker thread");
    (tx, handle)
}

fn run_persist_loop(store: Store, rx: std_mpsc::Receiver<PersistJob>) {
    let mut pending_seqs: HashMap<Arc<str>, u64> = HashMap::new();
    let mut pending_running: HashMap<Arc<str>, bool> = HashMap::new();

    while let Ok(job) = rx.recv() {
        if process_job(&store, job, &mut pending_seqs, &mut pending_running) {
            break;
        }

        // Drain remaining queued jobs without blocking (coalesce burst).
        while let Ok(job) = rx.try_recv() {
            if process_job(&store, job, &mut pending_seqs, &mut pending_running) {
                return;
            }
        }

        // After draining burst, flush coalesced seq/status updates.
        flush_coalesced(&store, &mut pending_seqs, &mut pending_running);
    }
}

/// Returns `true` if shutdown was requested.
fn process_job(
    store: &Store,
    job: PersistJob,
    pending_seqs: &mut HashMap<Arc<str>, u64>,
    pending_running: &mut HashMap<Arc<str>, bool>,
) -> bool {
    match job {
        PersistJob::Shutdown => {
            flush_coalesced(store, pending_seqs, pending_running);
            true
        }
        PersistJob::UpdateSeq { session_id, seq } => {
            let entry = pending_seqs.entry(session_id).or_insert(0);
            *entry = (*entry).max(seq);
            false
        }
        PersistJob::MarkRunning { session_id } => {
            pending_running.entry(session_id).or_insert(true);
            false
        }
        PersistJob::OutputChunk {
            session_id,
            seq,
            records,
            replay_chunk,
            ts,
        } => {
            // Flush coalesced state before processing chunk to keep ordering.
            flush_coalesced(store, pending_seqs, pending_running);

            if let Err(err) =
                store.append_terminal_replay_chunk(&session_id, seq, &replay_chunk, ts)
            {
                log::warn!("persist replay chunk failed: {err}");
            }
            if let Err(err) = store.append_scrollback_records(&session_id, seq, &records, ts) {
                log::warn!("persist scrollback records failed: {err}");
            }
            false
        }
        PersistJob::EnforceLimit {
            session_id,
            max_lines,
        } => {
            if let Err(err) = store.enforce_session_scrollback_record_limit(&session_id, max_lines)
            {
                log::warn!("enforce scrollback limit failed: {err}");
            }
            false
        }
        PersistJob::Flush { ack } => {
            flush_coalesced(store, pending_seqs, pending_running);
            let _ = ack.send(());
            false
        }
    }
}

fn flush_coalesced(
    store: &Store,
    pending_seqs: &mut HashMap<Arc<str>, u64>,
    pending_running: &mut HashMap<Arc<str>, bool>,
) {
    for (session_id, seq) in pending_seqs.drain() {
        if let Err(err) = store.update_session_seq(&session_id, seq) {
            log::warn!("flush coalesced seq failed for {session_id}: {err}");
        }
    }
    for (session_id, _) in pending_running.drain() {
        if let Err(err) = store.set_session_status(&session_id, StoredSessionStatus::Running) {
            log::warn!("flush coalesced status failed for {session_id}: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_store() -> (Store, PathBuf) {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "chatminal-persist-worker-test-{}-{}.db",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
        ));
        let store = Store::initialize(&path).expect("init test store");
        (store, path)
    }

    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn coalesces_seq_updates_to_highest() {
        let (store, path) = temp_store();
        let profile_id = store
            .load_workspace()
            .expect("load workspace")
            .active_profile_id;
        let session = store
            .create_session(
                &profile_id,
                Some("Test".into()),
                "/tmp".into(),
                "/bin/sh".into(),
                false,
            )
            .expect("create session");

        let (tx, handle) = spawn_persist_worker(store.clone());
        for seq in 1..=10 {
            let _ = tx.send(PersistJob::UpdateSeq {
                session_id: session.session_id.clone().into(),
                seq,
            });
        }
        let _ = tx.send(PersistJob::Shutdown);
        let _ = handle.join();

        let loaded = store
            .get_session(&session.session_id)
            .expect("get session")
            .expect("session exists");
        assert_eq!(loaded.seq, 10);
        cleanup(&path);
    }

    #[test]
    fn flushes_all_on_shutdown() {
        let (store, path) = temp_store();
        let profile_id = store
            .load_workspace()
            .expect("load workspace")
            .active_profile_id;
        let session = store
            .create_session(
                &profile_id,
                Some("Flush".into()),
                "/tmp".into(),
                "/bin/sh".into(),
                false,
            )
            .expect("create session");

        let (tx, handle) = spawn_persist_worker(store.clone());
        let _ = tx.send(PersistJob::UpdateSeq {
            session_id: session.session_id.clone().into(),
            seq: 42,
        });
        let _ = tx.send(PersistJob::MarkRunning {
            session_id: session.session_id.clone().into(),
        });
        let _ = tx.send(PersistJob::Shutdown);
        let _ = handle.join();

        let loaded = store
            .get_session(&session.session_id)
            .expect("get session")
            .expect("session exists");
        assert_eq!(loaded.seq, 42);
        assert_eq!(loaded.status, StoredSessionStatus::Running);
        cleanup(&path);
    }
}
