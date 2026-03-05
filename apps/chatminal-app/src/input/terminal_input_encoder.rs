use super::terminal_input_event::{TerminalKey, TerminalKeyChord, TerminalModifiers};

pub fn encode_key_chord_to_pty_input(chord: TerminalKeyChord) -> Option<String> {
    let mut value = match chord.key {
        TerminalKey::Backspace => "\x7f".to_string(),
        TerminalKey::Enter => "\r".to_string(),
        TerminalKey::Left => "\x1b[D".to_string(),
        TerminalKey::Right => "\x1b[C".to_string(),
        TerminalKey::Up => "\x1b[A".to_string(),
        TerminalKey::Down => "\x1b[B".to_string(),
        TerminalKey::Home => "\x1b[H".to_string(),
        TerminalKey::End => "\x1b[F".to_string(),
        TerminalKey::PageUp => "\x1b[5~".to_string(),
        TerminalKey::PageDown => "\x1b[6~".to_string(),
        TerminalKey::Tab => "\t".to_string(),
        TerminalKey::BackTab => "\x1b[Z".to_string(),
        TerminalKey::Delete => "\x1b[3~".to_string(),
        TerminalKey::Insert => "\x1b[2~".to_string(),
        TerminalKey::Esc => "\x1b".to_string(),
        TerminalKey::Function(index) => map_function_key(index)?,
        TerminalKey::Char(c) => map_char_with_modifiers(c, chord.modifiers)?,
    };

    if chord.modifiers.alt {
        value.insert(0, '\x1b');
    }
    Some(value)
}

fn map_char_with_modifiers(value: char, modifiers: TerminalModifiers) -> Option<String> {
    if has_control_modifier(modifiers) {
        if let Some(encoded) = map_ascii_control(value) {
            if encoded == 127 {
                return Some("\x7f".to_string());
            }
            return Some((encoded as char).to_string());
        }
    }
    Some(value.to_string())
}

fn has_control_modifier(modifiers: TerminalModifiers) -> bool {
    modifiers.ctrl || (!cfg!(target_os = "macos") && modifiers.command)
}

fn map_ascii_control(value: char) -> Option<u8> {
    let lower = value.to_ascii_lowercase();
    match lower {
        'a'..='z' => Some((lower as u8) - b'a' + 1),
        ' ' | '@' | '`' | '2' => Some(0),
        '[' | '3' => Some(27),
        '\\' | '4' => Some(28),
        ']' | '5' => Some(29),
        '^' | '6' => Some(30),
        '_' | '7' | '/' => Some(31),
        '?' | '8' => Some(127),
        _ => None,
    }
}

fn map_function_key(index: u8) -> Option<String> {
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
    use super::super::terminal_input_event::{
        TerminalInputSource, TerminalKey, TerminalKeyChord, TerminalModifiers,
    };
    use super::encode_key_chord_to_pty_input;

    fn chord(key: TerminalKey, modifiers: TerminalModifiers) -> TerminalKeyChord {
        TerminalKeyChord {
            key,
            modifiers,
            repeat: false,
            source: TerminalInputSource::Crossterm,
        }
    }

    #[test]
    fn encode_common_control_and_alt_sequences() {
        let ctrl_c = encode_key_chord_to_pty_input(chord(
            TerminalKey::Char('c'),
            TerminalModifiers {
                ctrl: true,
                ..TerminalModifiers::default()
            },
        ));
        assert_eq!(ctrl_c.as_deref(), Some("\u{3}"));

        let ctrl_space = encode_key_chord_to_pty_input(chord(
            TerminalKey::Char(' '),
            TerminalModifiers {
                ctrl: true,
                ..TerminalModifiers::default()
            },
        ));
        assert_eq!(ctrl_space.as_deref(), Some("\0"));

        let ctrl_bracket = encode_key_chord_to_pty_input(chord(
            TerminalKey::Char('['),
            TerminalModifiers {
                ctrl: true,
                ..TerminalModifiers::default()
            },
        ));
        assert_eq!(ctrl_bracket.as_deref(), Some("\u{1b}"));

        let alt_a = encode_key_chord_to_pty_input(chord(
            TerminalKey::Char('a'),
            TerminalModifiers {
                alt: true,
                ..TerminalModifiers::default()
            },
        ));
        assert_eq!(alt_a.as_deref(), Some("\u{1b}a"));
    }

    #[test]
    fn encode_backtab_and_function() {
        let backtab = encode_key_chord_to_pty_input(chord(
            TerminalKey::BackTab,
            TerminalModifiers::default(),
        ));
        assert_eq!(backtab.as_deref(), Some("\u{1b}[Z"));

        let f12 = encode_key_chord_to_pty_input(chord(
            TerminalKey::Function(12),
            TerminalModifiers::default(),
        ));
        assert_eq!(f12.as_deref(), Some("\u{1b}[24~"));

        let alt_up = encode_key_chord_to_pty_input(chord(
            TerminalKey::Up,
            TerminalModifiers {
                alt: true,
                ..TerminalModifiers::default()
            },
        ));
        assert_eq!(alt_up.as_deref(), Some("\u{1b}\u{1b}[A"));
    }
}
