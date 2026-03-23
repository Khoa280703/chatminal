use super::{
    RuntimeId, SessionEventBus, SessionRuntimeEvent, SessionRuntimeLookup, SessionWorkspaceHost,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionBridgeAction {
    Noop,
    FocusSession { session_id: String },
}

pub struct SessionRuntimeBridge<'a, H: SessionWorkspaceHost, B: SessionEventBus> {
    host: &'a H,
    bus: &'a B,
}

impl<'a, H: SessionWorkspaceHost, B: SessionEventBus> SessionRuntimeBridge<'a, H, B> {
    pub fn new(host: &'a H, bus: &'a B) -> Self {
        Self { host, bus }
    }

    pub fn reconcile_session_lookup(
        &self,
        lookup: &SessionRuntimeLookup,
    ) -> Result<SessionBridgeAction, String> {
        let runtime_active = self.host.active_session_id();
        Ok(match (runtime_active, lookup.active_session_id.clone()) {
            (Some(runtime_active), Some(lookup_active)) if runtime_active == lookup_active => {
                SessionBridgeAction::Noop
            }
            (Some(runtime_active), _) => SessionBridgeAction::FocusSession {
                session_id: runtime_active,
            },
            (None, _) => SessionBridgeAction::Noop,
        })
    }

    pub fn on_session_activated(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String> {
        if self.host.active_session_id().as_deref() != Some(session_id) {
            self.host.activate_session(session_id)?;
        }
        self.bus.publish(SessionRuntimeEvent::RuntimeFocused {
            session_id: session_id.to_string(),
            runtime_id,
        });
        Ok(())
    }

    pub fn on_session_closed(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &SessionRuntimeLookup,
    ) -> Result<(), String> {
        self.bus.publish(SessionRuntimeEvent::RuntimeClosed {
            session_id: session_id.to_string(),
            runtime_id,
        });

        let host_active = self.host.active_session_id();
        let should_promote_next = match host_active.as_deref() {
            Some(active_session_id) => active_session_id == session_id,
            None => true,
        };

        if !should_promote_next {
            return Ok(());
        }

        if let Some(next_session_id) = lookup_after_close.active_session_id.as_deref() {
            self.host.activate_session(next_session_id)?;
            self.bus.publish(SessionRuntimeEvent::RuntimeFocused {
                session_id: next_session_id.to_string(),
                runtime_id: lookup_after_close
                    .runtime_ids_by_session
                    .get(next_session_id)
                    .copied()
                    .unwrap_or(runtime_id),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{
        RuntimeId, SessionEventBus, SessionRuntimeEvent, SessionRuntimeLookup, SessionWorkspaceHost,
    };

    use super::{SessionBridgeAction, SessionRuntimeBridge};

    #[derive(Default)]
    struct TestHost {
        active_session_id: Mutex<Option<String>>,
        activated: Mutex<Vec<String>>,
    }

    impl TestHost {
        fn with_active(session_id: Option<&str>) -> Self {
            Self {
                active_session_id: Mutex::new(session_id.map(str::to_owned)),
                activated: Mutex::new(Vec::new()),
            }
        }
    }

    impl SessionWorkspaceHost for TestHost {
        fn active_session_id(&self) -> Option<String> {
            self.active_session_id
                .lock()
                .expect("lock active session")
                .clone()
        }

        fn activate_session(&self, session_id: &str) -> Result<(), String> {
            self.activated
                .lock()
                .expect("lock activated")
                .push(session_id.to_string());
            *self.active_session_id.lock().expect("lock active session") =
                Some(session_id.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingBus {
        events: Mutex<Vec<SessionRuntimeEvent>>,
    }

    impl SessionEventBus for RecordingBus {
        fn publish(&self, event: SessionRuntimeEvent) {
            self.events.lock().expect("lock bus").push(event);
        }
    }

    #[test]
    fn reconcile_prefers_runtime_active_session() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);
        let lookup = SessionRuntimeLookup {
            active_session_id: Some("session-b".to_string()),
            ..SessionRuntimeLookup::default()
        };

        let action = bridge.reconcile_session_lookup(&lookup).expect("reconcile");
        assert_eq!(
            action,
            SessionBridgeAction::FocusSession {
                session_id: "session-a".to_string()
            }
        );
    }

    #[test]
    fn focusing_other_runtime_activates_runtime_then_publishes() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);

        bridge
            .on_session_activated("session-b", RuntimeId::new(22))
            .expect("focus runtime");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [SessionRuntimeEvent::RuntimeFocused {
                session_id: "session-b".to_string(),
                runtime_id: RuntimeId::new(22),
            }]
        );
    }

    #[test]
    fn closing_active_runtime_promotes_lookup_active_session() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);
        let lookup = SessionRuntimeLookup {
            active_session_id: Some("session-b".to_string()),
            runtime_ids_by_session: [("session-b".to_string(), RuntimeId::new(33))].into(),
            ..SessionRuntimeLookup::default()
        };

        bridge
            .on_session_closed("session-a", RuntimeId::new(11), &lookup)
            .expect("close active runtime");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [
                SessionRuntimeEvent::RuntimeClosed {
                    session_id: "session-a".to_string(),
                    runtime_id: RuntimeId::new(11),
                },
                SessionRuntimeEvent::RuntimeFocused {
                    session_id: "session-b".to_string(),
                    runtime_id: RuntimeId::new(33),
                }
            ]
        );
    }

    #[test]
    fn closing_runtime_after_runtime_marker_cleared_still_promotes_next_session() {
        let host = TestHost::with_active(None);
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);
        let lookup = SessionRuntimeLookup {
            active_session_id: Some("session-b".to_string()),
            runtime_ids_by_session: [("session-b".to_string(), RuntimeId::new(44))].into(),
            ..SessionRuntimeLookup::default()
        };

        bridge
            .on_session_closed("session-a", RuntimeId::new(11), &lookup)
            .expect("close runtime after marker cleared");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [
                SessionRuntimeEvent::RuntimeClosed {
                    session_id: "session-a".to_string(),
                    runtime_id: RuntimeId::new(11),
                },
                SessionRuntimeEvent::RuntimeFocused {
                    session_id: "session-b".to_string(),
                    runtime_id: RuntimeId::new(44),
                }
            ]
        );
    }
}
