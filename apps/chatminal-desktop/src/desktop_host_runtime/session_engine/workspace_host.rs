/// Bridge implemented by the business runtime.
///
/// `chatminal-runtime` remains the owner of workspace metadata and the
/// authoritative `active_session_id`. The live session runtime consumes this
/// host interface instead of reading persistence state directly.
pub trait SessionWorkspaceHost: Send + Sync {
    fn active_session_id(&self) -> Option<String>;
    fn activate_session(&self, session_id: &str) -> Result<(), String>;
    fn close_session(&self, session_id: &str) -> Result<(), String>;
}
