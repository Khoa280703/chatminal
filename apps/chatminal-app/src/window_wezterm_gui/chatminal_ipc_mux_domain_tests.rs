use chatminal_protocol::{
    Event, PtyExitedEvent, PtyOutputEvent, SessionStatus, SessionUpdatedEvent,
};

use super::{
    ChatminalIpcMuxDomain, DomainEventAction, INPUT_WRITE_BATCH_MAX_BYTES, clamp_preview_lines,
    decode_input_payload_chunks,
};

#[test]
fn clamp_preview_lines_enforces_bounds() {
    assert_eq!(clamp_preview_lines(1), 50);
    assert_eq!(clamp_preview_lines(2_000), 2_000);
    assert_eq!(clamp_preview_lines(50_000), 20_000);
}

#[test]
fn decode_input_payload_chunks_handles_utf8_split_boundaries() {
    let mut pending = Vec::<u8>::new();
    assert!(decode_input_payload_chunks(&mut pending, &[0xe1, 0xba]).is_empty());
    assert_eq!(pending, vec![0xe1, 0xba]);
    assert_eq!(
        decode_input_payload_chunks(&mut pending, &[0xbf]),
        vec!["ế".to_string()]
    );
    assert!(pending.is_empty());
}

#[test]
fn domain_ignores_stale_or_foreign_output_events() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 10);

    let stale = domain.consume_event(Event::PtyOutput(PtyOutputEvent {
        session_id: "s1".to_string(),
        chunk: "old".to_string(),
        seq: 9,
        ts: 100,
    }));
    assert_eq!(stale, DomainEventAction::Ignore);

    let foreign = domain.consume_event(Event::PtyOutput(PtyOutputEvent {
        session_id: "s2".to_string(),
        chunk: "other".to_string(),
        seq: 99,
        ts: 100,
    }));
    assert_eq!(foreign, DomainEventAction::Ignore);
}

#[test]
fn domain_emits_output_and_exit_for_active_session() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 10);

    let output = domain.consume_event(Event::PtyOutput(PtyOutputEvent {
        session_id: "s1".to_string(),
        chunk: "hello".to_string(),
        seq: 11,
        ts: 100,
    }));
    assert_eq!(output, DomainEventAction::Output("hello".to_string()));

    let exit = domain.consume_event(Event::PtyExited(PtyExitedEvent {
        session_id: "s1".to_string(),
        exit_code: Some(0),
        reason: "done".to_string(),
    }));
    assert_eq!(exit, DomainEventAction::ExitRequested);
}

#[test]
fn disconnected_session_updated_requests_exit() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 10);

    let action = domain.consume_event(Event::SessionUpdated(SessionUpdatedEvent {
        session_id: "s1".to_string(),
        status: SessionStatus::Disconnected,
        seq: 12,
        persist_history: false,
        ts: 100,
    }));
    assert_eq!(action, DomainEventAction::ExitRequested);
}

#[test]
fn stale_session_updated_by_timestamp_is_ignored() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 10);
    let first = domain.consume_event(Event::SessionUpdated(SessionUpdatedEvent {
        session_id: "s1".to_string(),
        status: SessionStatus::Running,
        seq: 11,
        persist_history: false,
        ts: 200,
    }));
    assert_eq!(first, DomainEventAction::Ignore);

    let stale = domain.consume_event(Event::SessionUpdated(SessionUpdatedEvent {
        session_id: "s1".to_string(),
        status: SessionStatus::Disconnected,
        seq: 9,
        persist_history: false,
        ts: 100,
    }));
    assert_eq!(stale, DomainEventAction::Ignore);
}

#[test]
fn disconnected_with_older_seq_is_ignored() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 10);
    let _ = domain.consume_event(Event::PtyOutput(PtyOutputEvent {
        session_id: "s1".to_string(),
        chunk: "new".to_string(),
        seq: 12,
        ts: 120,
    }));

    let stale_disconnect = domain.consume_event(Event::SessionUpdated(SessionUpdatedEvent {
        session_id: "s1".to_string(),
        status: SessionStatus::Disconnected,
        seq: 11,
        persist_history: false,
        ts: 999,
    }));
    assert_eq!(stale_disconnect, DomainEventAction::Ignore);
}

#[test]
fn input_queue_batches_and_flushes() {
    let mut domain = ChatminalIpcMuxDomain::new("s1".to_string(), 0);
    domain.queue_input_payload("abc".as_bytes());
    assert!(!domain.should_flush_input_batch());
    assert_eq!(domain.take_input_batch().as_deref(), Some("abc"));
    assert!(domain.take_input_batch().is_none());

    let big = vec![b'x'; INPUT_WRITE_BATCH_MAX_BYTES];
    domain.queue_input_payload(&big);
    assert!(domain.should_flush_input_batch());
}
