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

    match key.as_ref() {
        Key::Named(keyboard::key::Named::Enter) => vec![b'\r'],
        Key::Named(keyboard::key::Named::Backspace) => vec![0x7f],
        Key::Named(keyboard::key::Named::Tab) => vec![b'\t'],
        Key::Named(keyboard::key::Named::Escape) => vec![0x1b],
        Key::Named(keyboard::key::Named::ArrowUp) => b"\x1b[A".to_vec(),
        Key::Named(keyboard::key::Named::ArrowDown) => b"\x1b[B".to_vec(),
        Key::Named(keyboard::key::Named::ArrowRight) => b"\x1b[C".to_vec(),
        Key::Named(keyboard::key::Named::ArrowLeft) => b"\x1b[D".to_vec(),
        Key::Named(keyboard::key::Named::Home) => b"\x1b[H".to_vec(),
        Key::Named(keyboard::key::Named::End) => b"\x1b[F".to_vec(),
        Key::Named(keyboard::key::Named::Delete) => b"\x1b[3~".to_vec(),
        Key::Named(keyboard::key::Named::PageUp) => b"\x1b[5~".to_vec(),
        Key::Named(keyboard::key::Named::PageDown) => b"\x1b[6~".to_vec(),
        Key::Character(text) => {
            let mut bytes = text.as_bytes().to_vec();
            if modifiers.alt() {
                bytes.insert(0, 0x1b);
            }
            bytes
        }
        _ => Vec::new(),
    }
}

fn ctrl_byte(key: &Key, physical_key: keyboard::key::Physical) -> Option<u8> {
    let latin = key.to_latin(physical_key)?.to_ascii_lowercase();

    if latin.is_ascii_lowercase() {
        return Some((latin as u8) & 0x1f);
    }

    None
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
}
