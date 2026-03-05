use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eframe::egui::{Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalInputSource {
    Crossterm,
    Egui,
}

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
    pub source: TerminalInputSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TerminalInputEvent {
    KeyChord(TerminalKeyChord),
    TextCommit {
        text: String,
        source: TerminalInputSource,
    },
    Paste {
        text: String,
        source: TerminalInputSource,
    },
    ImeCommit {
        text: String,
        source: TerminalInputSource,
    },
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
        source: TerminalInputSource::Crossterm,
    }))
}

pub fn map_egui_key_event(
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Option<TerminalInputEvent> {
    let mapped = match key {
        Key::Backspace => TerminalKey::Backspace,
        Key::Enter => TerminalKey::Enter,
        Key::ArrowLeft => TerminalKey::Left,
        Key::ArrowRight => TerminalKey::Right,
        Key::ArrowUp => TerminalKey::Up,
        Key::ArrowDown => TerminalKey::Down,
        Key::Home => TerminalKey::Home,
        Key::End => TerminalKey::End,
        Key::PageUp => TerminalKey::PageUp,
        Key::PageDown => TerminalKey::PageDown,
        Key::Delete => TerminalKey::Delete,
        Key::Insert => TerminalKey::Insert,
        Key::Escape => TerminalKey::Esc,
        Key::Tab => {
            if modifiers.shift {
                TerminalKey::BackTab
            } else {
                TerminalKey::Tab
            }
        }
        Key::F1 => TerminalKey::Function(1),
        Key::F2 => TerminalKey::Function(2),
        Key::F3 => TerminalKey::Function(3),
        Key::F4 => TerminalKey::Function(4),
        Key::F5 => TerminalKey::Function(5),
        Key::F6 => TerminalKey::Function(6),
        Key::F7 => TerminalKey::Function(7),
        Key::F8 => TerminalKey::Function(8),
        Key::F9 => TerminalKey::Function(9),
        Key::F10 => TerminalKey::Function(10),
        Key::F11 => TerminalKey::Function(11),
        Key::F12 => TerminalKey::Function(12),
        _ => TerminalKey::Char(key_to_char(key, modifiers.shift)?),
    };

    Some(TerminalInputEvent::KeyChord(TerminalKeyChord {
        key: mapped,
        modifiers: map_egui_modifiers(modifiers),
        repeat,
        source: TerminalInputSource::Egui,
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

fn map_egui_modifiers(modifiers: Modifiers) -> TerminalModifiers {
    TerminalModifiers {
        shift: modifiers.shift,
        alt: modifiers.alt,
        ctrl: modifiers.ctrl,
        command: modifiers.command,
    }
}

fn key_to_char(key: Key, shift: bool) -> Option<char> {
    let ch = match key {
        Key::A
        | Key::B
        | Key::C
        | Key::D
        | Key::E
        | Key::F
        | Key::G
        | Key::H
        | Key::I
        | Key::J
        | Key::K
        | Key::L
        | Key::M
        | Key::N
        | Key::O
        | Key::P
        | Key::Q
        | Key::R
        | Key::S
        | Key::T
        | Key::U
        | Key::V
        | Key::W
        | Key::X
        | Key::Y
        | Key::Z => {
            let idx = key_to_alpha_index(key)?;
            let base = if shift { b'A' } else { b'a' };
            (base + idx) as char
        }
        Key::Num0 => '0',
        Key::Num1 => '1',
        Key::Num2 => '2',
        Key::Num3 => '3',
        Key::Num4 => '4',
        Key::Num5 => '5',
        Key::Num6 => '6',
        Key::Num7 => '7',
        Key::Num8 => '8',
        Key::Num9 => '9',
        Key::Space => ' ',
        Key::Colon => ':',
        Key::Comma => ',',
        Key::Backslash => '\\',
        Key::Slash => '/',
        Key::Pipe => '|',
        Key::Questionmark => '?',
        Key::Exclamationmark => '!',
        Key::OpenBracket => '[',
        Key::CloseBracket => ']',
        Key::OpenCurlyBracket => '{',
        Key::CloseCurlyBracket => '}',
        Key::Backtick => '`',
        Key::Minus => '-',
        Key::Period => '.',
        Key::Plus => '+',
        Key::Equals => '=',
        Key::Semicolon => ';',
        Key::Quote => '\'',
        _ => return None,
    };
    Some(ch)
}

fn key_to_alpha_index(key: Key) -> Option<u8> {
    match key {
        Key::A => Some(0),
        Key::B => Some(1),
        Key::C => Some(2),
        Key::D => Some(3),
        Key::E => Some(4),
        Key::F => Some(5),
        Key::G => Some(6),
        Key::H => Some(7),
        Key::I => Some(8),
        Key::J => Some(9),
        Key::K => Some(10),
        Key::L => Some(11),
        Key::M => Some(12),
        Key::N => Some(13),
        Key::O => Some(14),
        Key::P => Some(15),
        Key::Q => Some(16),
        Key::R => Some(17),
        Key::S => Some(18),
        Key::T => Some(19),
        Key::U => Some(20),
        Key::V => Some(21),
        Key::W => Some(22),
        Key::X => Some(23),
        Key::Y => Some(24),
        Key::Z => Some(25),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use eframe::egui::{Key, Modifiers};

    use super::{TerminalInputEvent, TerminalKey, map_crossterm_key_event, map_egui_key_event};

    #[test]
    fn map_crossterm_and_egui_ctrl_c_to_same_terminal_key() {
        let cross =
            map_crossterm_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        let egui = map_egui_key_event(Key::C, Modifiers::CTRL, false);

        let cross_key = match cross {
            Some(TerminalInputEvent::KeyChord(chord)) => chord.key,
            _ => panic!("expected crossterm chord"),
        };
        let egui_key = match egui {
            Some(TerminalInputEvent::KeyChord(chord)) => chord.key,
            _ => panic!("expected egui chord"),
        };

        assert_eq!(cross_key, TerminalKey::Char('c'));
        assert_eq!(egui_key, TerminalKey::Char('c'));
    }

    #[test]
    fn map_egui_shift_tab_to_backtab() {
        let mapped = map_egui_key_event(
            Key::Tab,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            false,
        );

        let key = match mapped {
            Some(TerminalInputEvent::KeyChord(chord)) => chord.key,
            _ => panic!("expected key chord"),
        };
        assert_eq!(key, TerminalKey::BackTab);
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
