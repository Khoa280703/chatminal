use crossterm::event::KeyEvent;

use super::terminal_input_encoder::encode_key_chord_to_pty_input;
use super::terminal_input_event::{TerminalInputEvent, map_crossterm_key_event};

pub fn map_key_event_to_pty_input(key: KeyEvent) -> Option<String> {
    match map_crossterm_key_event(key) {
        Some(TerminalInputEvent::KeyChord(chord)) => encode_key_chord_to_pty_input(chord),
        _ => None,
    }
}

pub fn map_key_event_to_pty_input_legacy(key: KeyEvent) -> Option<String> {
    use crossterm::event::KeyCode;

    let mut value = match key.code {
        KeyCode::Backspace => "\x7f".to_string(),
        KeyCode::Enter => "\r".to_string(),
        KeyCode::Left => "\x1b[D".to_string(),
        KeyCode::Right => "\x1b[C".to_string(),
        KeyCode::Up => "\x1b[A".to_string(),
        KeyCode::Down => "\x1b[B".to_string(),
        KeyCode::Home => "\x1b[H".to_string(),
        KeyCode::End => "\x1b[F".to_string(),
        KeyCode::PageUp => "\x1b[5~".to_string(),
        KeyCode::PageDown => "\x1b[6~".to_string(),
        KeyCode::Tab => "\t".to_string(),
        KeyCode::BackTab => "\x1b[Z".to_string(),
        KeyCode::Delete => "\x1b[3~".to_string(),
        KeyCode::Insert => "\x1b[2~".to_string(),
        KeyCode::Esc => "\x1b".to_string(),
        KeyCode::F(index) => map_function_key_legacy(index)?,
        KeyCode::Char(value) => map_char_legacy(value, key.modifiers)?,
        _ => return None,
    };

    if key.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        value.insert(0, '\x1b');
    }
    Some(value)
}

fn map_char_legacy(value: char, modifiers: crossterm::event::KeyModifiers) -> Option<String> {
    if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        let lower = value.to_ascii_lowercase();
        let encoded = match lower {
            'a'..='z' => (lower as u8) - b'a' + 1,
            ' ' | '@' | '`' | '2' => 0,
            '[' | '3' => 27,
            '\\' | '4' => 28,
            ']' | '5' => 29,
            '^' | '6' => 30,
            '_' | '7' | '/' => 31,
            '?' | '8' => 127,
            _ => return Some(value.to_string()),
        };
        if encoded == 127 {
            return Some("\x7f".to_string());
        }
        return Some((encoded as char).to_string());
    }

    Some(value.to_string())
}

fn map_function_key_legacy(index: u8) -> Option<String> {
    let sequence = match index {
        1 => "\x1bOP",
        2 => "\x1bOQ",
        3 => "\x1bOR",
        4 => "\x1bOS",
        5 => "\x1b[15~",
        6 => "\x1b[17~",
        7 => "\x1b[18~",
        8 => "\x1b[19~",
        9 => "\x1b[20~",
        10 => "\x1b[21~",
        11 => "\x1b[23~",
        12 => "\x1b[24~",
        _ => return None,
    };
    Some(sequence.to_string())
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{map_key_event_to_pty_input, map_key_event_to_pty_input_legacy};

    struct KeyMapCase {
        name: &'static str,
        key: KeyEvent,
        expected: Option<&'static str>,
    }

    #[test]
    fn key_mapping_table_driven_cases() {
        let cases = [
            KeyMapCase {
                name: "ctrl-c",
                key: KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                expected: Some("\u{3}"),
            },
            KeyMapCase {
                name: "ctrl-space",
                key: KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL),
                expected: Some("\0"),
            },
            KeyMapCase {
                name: "ctrl-bracket",
                key: KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL),
                expected: Some("\u{1b}"),
            },
            KeyMapCase {
                name: "alt-a",
                key: KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
                expected: Some("\u{1b}a"),
            },
            KeyMapCase {
                name: "backtab",
                key: KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
                expected: Some("\u{1b}[Z"),
            },
            KeyMapCase {
                name: "f5",
                key: KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE),
                expected: Some("\u{1b}[15~"),
            },
            KeyMapCase {
                name: "f12",
                key: KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE),
                expected: Some("\u{1b}[24~"),
            },
            KeyMapCase {
                name: "unsupported-null",
                key: KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
                expected: None,
            },
        ];

        for case in cases {
            let mapped = map_key_event_to_pty_input(case.key);
            assert_eq!(mapped.as_deref(), case.expected, "case={}", case.name);
        }
    }

    #[test]
    fn legacy_key_mapping_preserves_basic_ctrl_and_navigation() {
        let ctrl_c = map_key_event_to_pty_input_legacy(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(ctrl_c.as_deref(), Some("\u{3}"));

        let alt_left =
            map_key_event_to_pty_input_legacy(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
        assert_eq!(alt_left.as_deref(), Some("\u{1b}\u{1b}[D"));
    }
}
