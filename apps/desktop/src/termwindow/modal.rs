use crate::TermWindow;
use crate::termwindow::box_model::ComputedElement;
use config::keyassignment::KeyAssignment;
use downcast_rs::{Downcast, impl_downcast};
use terminal_emulator::{KeyCode, KeyModifiers, MouseEvent};
use std::cell::Ref;
use window::DeadKeyStatus;

pub trait Modal: Downcast {
    fn perform_assignment(
        &self,
        _assignment: &KeyAssignment,
        _term_window: &mut TermWindow,
    ) -> bool {
        false
    }
    fn mouse_event(&self, event: MouseEvent, term_window: &mut TermWindow) -> anyhow::Result<()>;
    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool>;
    fn text_input(
        &self,
        text: &str,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        let mut handled = false;
        for c in text.chars() {
            if c.is_control() {
                continue;
            }
            handled |= self.key_down(KeyCode::Char(c), mods, term_window)?;
        }
        Ok(handled)
    }
    fn composition_status_changed(
        &self,
        _status: &DeadKeyStatus,
        _term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }
    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>>;
    fn reconfigure(&self, term_window: &mut TermWindow);
}
impl_downcast!(Modal);
