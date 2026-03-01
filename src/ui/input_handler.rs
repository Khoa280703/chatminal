use iced::keyboard::{self, Key, Modifiers};

pub fn key_to_bytes(
    key: &Key,
    physical_key: keyboard::key::Physical,
    modifiers: Modifiers,
) -> Vec<u8> {
    if modifiers.control()
        && let Some(byte) = ctrl_byte(key, physical_key)
    {
        return vec![byte];
    }

    let bytes = match key.as_ref() {
        Key::Named(keyboard::key::Named::Enter) => vec![b'\r'],
        Key::Named(keyboard::key::Named::Backspace) => vec![0x7f],
        Key::Named(keyboard::key::Named::Tab) if modifiers.shift() => b"\x1b[Z".to_vec(),
        Key::Named(keyboard::key::Named::Tab) => vec![b'\t'],
        Key::Named(keyboard::key::Named::Escape) => vec![0x1b],
        Key::Named(keyboard::key::Named::ArrowUp) => b"\x1b[A".to_vec(),
        Key::Named(keyboard::key::Named::ArrowDown) => b"\x1b[B".to_vec(),
        Key::Named(keyboard::key::Named::ArrowRight) => b"\x1b[C".to_vec(),
        Key::Named(keyboard::key::Named::ArrowLeft) => b"\x1b[D".to_vec(),
        Key::Named(keyboard::key::Named::Home) => b"\x1b[H".to_vec(),
        Key::Named(keyboard::key::Named::End) => b"\x1b[F".to_vec(),
        Key::Named(keyboard::key::Named::Insert) => b"\x1b[2~".to_vec(),
        Key::Named(keyboard::key::Named::Delete) => b"\x1b[3~".to_vec(),
        Key::Named(keyboard::key::Named::PageUp) => b"\x1b[5~".to_vec(),
        Key::Named(keyboard::key::Named::PageDown) => b"\x1b[6~".to_vec(),
        Key::Named(keyboard::key::Named::F1) => b"\x1bOP".to_vec(),
        Key::Named(keyboard::key::Named::F2) => b"\x1bOQ".to_vec(),
        Key::Named(keyboard::key::Named::F3) => b"\x1bOR".to_vec(),
        Key::Named(keyboard::key::Named::F4) => b"\x1bOS".to_vec(),
        Key::Named(keyboard::key::Named::F5) => b"\x1b[15~".to_vec(),
        Key::Named(keyboard::key::Named::F6) => b"\x1b[17~".to_vec(),
        Key::Named(keyboard::key::Named::F7) => b"\x1b[18~".to_vec(),
        Key::Named(keyboard::key::Named::F8) => b"\x1b[19~".to_vec(),
        Key::Named(keyboard::key::Named::F9) => b"\x1b[20~".to_vec(),
        Key::Named(keyboard::key::Named::F10) => b"\x1b[21~".to_vec(),
        Key::Named(keyboard::key::Named::F11) => b"\x1b[23~".to_vec(),
        Key::Named(keyboard::key::Named::F12) => b"\x1b[24~".to_vec(),
        Key::Named(keyboard::key::Named::Space) => vec![b' '],
        Key::Character(text) => text.as_bytes().to_vec(),
        _ => Vec::new(),
    };

    apply_alt_prefix(bytes, modifiers)
}

fn apply_alt_prefix(mut bytes: Vec<u8>, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.alt() && !bytes.is_empty() {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn ctrl_byte(key: &Key, physical_key: keyboard::key::Physical) -> Option<u8> {
    let latin = key.to_latin(physical_key)?.to_ascii_lowercase();

    if latin.is_ascii_lowercase() {
        return Some((latin as u8) & 0x1f);
    }

    match latin {
        '@' | ' ' => Some(0),
        '[' => Some(27),
        '\\' => Some(28),
        ']' => Some(29),
        '^' => Some(30),
        '_' => Some(31),
        '?' => Some(127),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{self, Key, Modifiers};

    use super::key_to_bytes;

    #[test]
    fn maps_arrow_up() {
        let bytes = key_to_bytes(
            &Key::Named(keyboard::key::Named::ArrowUp),
            keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
            Modifiers::empty(),
        );
        assert_eq!(bytes, b"\x1b[A");
    }

    #[test]
    fn maps_ctrl_a() {
        let bytes = key_to_bytes(
            &Key::Character("a".into()),
            keyboard::key::Physical::Code(keyboard::key::Code::KeyA),
            Modifiers::CTRL,
        );
        assert_eq!(bytes, [1]);
    }

    #[test]
    fn maps_named_space() {
        let bytes = key_to_bytes(
            &Key::Named(keyboard::key::Named::Space),
            keyboard::key::Physical::Code(keyboard::key::Code::Space),
            Modifiers::empty(),
        );
        assert_eq!(bytes, [b' ']);
    }

    #[test]
    fn maps_shift_tab() {
        let bytes = key_to_bytes(
            &Key::Named(keyboard::key::Named::Tab),
            keyboard::key::Physical::Code(keyboard::key::Code::Tab),
            Modifiers::SHIFT,
        );
        assert_eq!(bytes, b"\x1b[Z");
    }

    #[test]
    fn maps_alt_named_arrow() {
        let bytes = key_to_bytes(
            &Key::Named(keyboard::key::Named::ArrowLeft),
            keyboard::key::Physical::Code(keyboard::key::Code::ArrowLeft),
            Modifiers::ALT,
        );
        assert_eq!(bytes, b"\x1b\x1b[D");
    }

    #[test]
    fn maps_f_keys() {
        let f5 = key_to_bytes(
            &Key::Named(keyboard::key::Named::F5),
            keyboard::key::Physical::Code(keyboard::key::Code::F5),
            Modifiers::empty(),
        );
        assert_eq!(f5, b"\x1b[15~");
    }
}
