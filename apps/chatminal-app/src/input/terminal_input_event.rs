use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub command: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Esc,
    Function(u8),
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalKeyChord {
    pub key: TerminalKey,
    pub modifiers: TerminalModifiers,
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalInputEvent {
    KeyChord(TerminalKeyChord),
}

pub fn map_crossterm_key_event(key: KeyEvent) -> Option<TerminalInputEvent> {
    let mut normalized_modifiers = key.modifiers;
    let mapped = match key.code {
        KeyCode::Backspace => TerminalKey::Backspace,
        KeyCode::Enter => TerminalKey::Enter,
        KeyCode::Left => TerminalKey::Left,
        KeyCode::Right => TerminalKey::Right,
        KeyCode::Up => TerminalKey::Up,
        KeyCode::Down => TerminalKey::Down,
        KeyCode::Home => TerminalKey::Home,
        KeyCode::End => TerminalKey::End,
        KeyCode::PageUp => TerminalKey::PageUp,
        KeyCode::PageDown => TerminalKey::PageDown,
        KeyCode::Tab => TerminalKey::Tab,
        KeyCode::BackTab => TerminalKey::BackTab,
        KeyCode::Delete => TerminalKey::Delete,
        KeyCode::Insert => TerminalKey::Insert,
        KeyCode::Esc => TerminalKey::Esc,
        KeyCode::F(index) => TerminalKey::Function(index),
        KeyCode::Char(c) => {
            if looks_like_altgr_printable_char(c, key.modifiers) {
                normalized_modifiers.remove(KeyModifiers::CONTROL);
                normalized_modifiers.remove(KeyModifiers::ALT);
            }
            TerminalKey::Char(c)
        }
        _ => return None,
    };

    Some(TerminalInputEvent::KeyChord(TerminalKeyChord {
        key: mapped,
        modifiers: map_crossterm_modifiers(normalized_modifiers),
        repeat: false,
    }))
}

fn map_crossterm_modifiers(modifiers: KeyModifiers) -> TerminalModifiers {
    TerminalModifiers {
        shift: modifiers.contains(KeyModifiers::SHIFT),
        alt: modifiers.contains(KeyModifiers::ALT),
        ctrl: modifiers.contains(KeyModifiers::CONTROL),
        command: false,
    }
}

fn looks_like_altgr_printable_char(value: char, modifiers: KeyModifiers) -> bool {
    if !(modifiers.contains(KeyModifiers::CONTROL) && modifiers.contains(KeyModifiers::ALT)) {
        return false;
    }
    if !value.is_ascii() {
        return true;
    }
    matches!(
        value,
        '@' | '#' | '$' | '%' | '^' | '&' | '*' | '|' | '~' | '{' | '}' | '[' | ']' | '\\'
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{TerminalInputEvent, TerminalKey, map_crossterm_key_event};

    #[test]
    fn map_crossterm_ctrl_c_to_terminal_key() {
        let cross =
            map_crossterm_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

        let cross_key = match cross {
            Some(TerminalInputEvent::KeyChord(chord)) => chord.key,
            _ => panic!("expected crossterm chord"),
        };

        assert_eq!(cross_key, TerminalKey::Char('c'));
    }

    #[test]
    fn normalize_altgr_char_from_crossterm_to_text_lane() {
        let mapped = map_crossterm_key_event(KeyEvent::new(
            KeyCode::Char('@'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ));

        let chord = match mapped {
            Some(TerminalInputEvent::KeyChord(chord)) => chord,
            _ => panic!("expected key chord"),
        };
        assert_eq!(chord.key, TerminalKey::Char('@'));
        assert!(!chord.modifiers.ctrl);
        assert!(!chord.modifiers.alt);
    }
}
