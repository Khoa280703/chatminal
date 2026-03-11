use crate::{
    LeafId, SessionEventBus, SessionRuntimeEvent, SessionSurfaceLookup, SessionWorkspaceHost,
    SurfaceId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionBridgeAction {
    Noop,
    FocusSurface { session_id: String },
}

pub struct SessionRuntimeBridge<'a, H: SessionWorkspaceHost, B: SessionEventBus> {
    host: &'a H,
    bus: &'a B,
}

impl<'a, H: SessionWorkspaceHost, B: SessionEventBus> SessionRuntimeBridge<'a, H, B> {
    pub fn new(host: &'a H, bus: &'a B) -> Self {
        Self { host, bus }
    }

    pub fn reconcile_lookup(
        &self,
        lookup: &SessionSurfaceLookup,
    ) -> Result<SessionBridgeAction, String> {
        let runtime_active = self.host.active_session_id();
        Ok(match (runtime_active, lookup.active_session_id.clone()) {
            (Some(runtime_active), Some(surface_active)) if runtime_active == surface_active => {
                SessionBridgeAction::Noop
            }
            (Some(runtime_active), _) => SessionBridgeAction::FocusSurface {
                session_id: runtime_active,
            },
            (None, _) => SessionBridgeAction::Noop,
        })
    }

    pub fn on_surface_focused(
        &self,
        session_id: &str,
        surface_id: SurfaceId,
    ) -> Result<(), String> {
        if self.host.active_session_id().as_deref() != Some(session_id) {
            self.host.activate_session(session_id)?;
        }
        self.bus.publish(SessionRuntimeEvent::SurfaceFocused {
            session_id: session_id.to_string(),
            surface_id,
        });
        Ok(())
    }

    pub fn on_leaf_focused(
        &self,
        session_id: &str,
        surface_id: SurfaceId,
        leaf_id: LeafId,
    ) -> Result<(), String> {
        if self.host.active_session_id().as_deref() != Some(session_id) {
            self.host.activate_session(session_id)?;
        }
        self.bus.publish(SessionRuntimeEvent::LeafFocused {
            session_id: session_id.to_string(),
            surface_id,
            leaf_id,
        });
        Ok(())
    }

    pub fn on_surface_closed(
        &self,
        session_id: &str,
        surface_id: SurfaceId,
        lookup_after_close: &SessionSurfaceLookup,
    ) -> Result<(), String> {
        self.bus.publish(SessionRuntimeEvent::SurfaceClosed {
            session_id: session_id.to_string(),
            surface_id,
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
            self.bus.publish(SessionRuntimeEvent::SurfaceFocused {
                session_id: next_session_id.to_string(),
                surface_id: lookup_after_close
                    .surface_ids_by_session
                    .get(next_session_id)
                    .copied()
                    .unwrap_or(surface_id),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::{
        LeafId, SessionEventBus, SessionRuntimeEvent, SessionSurfaceLookup, SessionWorkspaceHost,
        SurfaceId,
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

        fn close_session(&self, _session_id: &str) -> Result<(), String> {
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
        let lookup = SessionSurfaceLookup {
            active_session_id: Some("session-b".to_string()),
            ..SessionSurfaceLookup::default()
        };

        let action = bridge.reconcile_lookup(&lookup).expect("reconcile");
        assert_eq!(
            action,
            SessionBridgeAction::FocusSurface {
                session_id: "session-a".to_string()
            }
        );
    }

    #[test]
    fn focusing_other_surface_activates_runtime_then_publishes() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);

        bridge
            .on_surface_focused("session-b", SurfaceId::new(22))
            .expect("focus surface");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [SessionRuntimeEvent::SurfaceFocused {
                session_id: "session-b".to_string(),
                surface_id: SurfaceId::new(22),
            }]
        );
    }

    #[test]
    fn closing_active_surface_promotes_lookup_active_session() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);
        let lookup = SessionSurfaceLookup {
            active_session_id: Some("session-b".to_string()),
            surface_ids_by_session: [("session-b".to_string(), SurfaceId::new(33))].into(),
            ..SessionSurfaceLookup::default()
        };

        bridge
            .on_surface_closed("session-a", SurfaceId::new(11), &lookup)
            .expect("close active surface");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [
                SessionRuntimeEvent::SurfaceClosed {
                    session_id: "session-a".to_string(),
                    surface_id: SurfaceId::new(11),
                },
                SessionRuntimeEvent::SurfaceFocused {
                    session_id: "session-b".to_string(),
                    surface_id: SurfaceId::new(33),
                }
            ]
        );
    }

    #[test]
    fn closing_surface_after_runtime_marker_cleared_still_promotes_next_session() {
        let host = TestHost::with_active(None);
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);
        let lookup = SessionSurfaceLookup {
            active_session_id: Some("session-b".to_string()),
            surface_ids_by_session: [("session-b".to_string(), SurfaceId::new(44))].into(),
            ..SessionSurfaceLookup::default()
        };

        bridge
            .on_surface_closed("session-a", SurfaceId::new(11), &lookup)
            .expect("close surface after marker cleared");

        assert_eq!(
            host.activated.lock().expect("lock activated").as_slice(),
            ["session-b"]
        );
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [
                SessionRuntimeEvent::SurfaceClosed {
                    session_id: "session-a".to_string(),
                    surface_id: SurfaceId::new(11),
                },
                SessionRuntimeEvent::SurfaceFocused {
                    session_id: "session-b".to_string(),
                    surface_id: SurfaceId::new(44),
                }
            ]
        );
    }

    #[test]
    fn focusing_leaf_on_same_session_only_publishes_leaf_event() {
        let host = TestHost::with_active(Some("session-a"));
        let bus = RecordingBus::default();
        let bridge = SessionRuntimeBridge::new(&host, &bus);

        bridge
            .on_leaf_focused("session-a", SurfaceId::new(1), LeafId::new(2))
            .expect("focus leaf");

        assert!(host.activated.lock().expect("lock activated").is_empty());
        assert_eq!(
            bus.events.lock().expect("lock bus").as_slice(),
            [SessionRuntimeEvent::LeafFocused {
                session_id: "session-a".to_string(),
                surface_id: SurfaceId::new(1),
                leaf_id: LeafId::new(2),
            }]
        );
    }
}
