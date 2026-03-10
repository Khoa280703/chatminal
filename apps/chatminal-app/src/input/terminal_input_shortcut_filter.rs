use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_attach_exit_key(key: KeyEvent) -> bool {
    key.code == KeyCode::F(10) && key.modifiers == KeyModifiers::NONE
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::is_attach_exit_key;

    #[test]
    fn attach_exit_key_is_f10() {
        assert!(is_attach_exit_key(KeyEvent::new(
            KeyCode::F(10),
            KeyModifiers::NONE
        )));
        assert!(!is_attach_exit_key(KeyEvent::new(
            KeyCode::F(9),
            KeyModifiers::NONE
        )));
        assert!(!is_attach_exit_key(KeyEvent::new(
            KeyCode::F(10),
            KeyModifiers::SHIFT
        )));
    }
}
