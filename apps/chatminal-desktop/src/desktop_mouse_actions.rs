use config::keyassignment::{KeyAssignment, SpawnSessionDomain};

pub fn primary_runtime_entry_button_action() -> KeyAssignment {
    KeyAssignment::SpawnSession(SpawnSessionDomain::CurrentSessionDomain)
}
