use crossterm::event::{KeyCode, KeyEvent};
use eframe::egui::{Key, Modifiers};

pub fn should_forward_egui_key_event(key: Key, modifiers: Modifiers) -> bool {
    // Giữ lại shortcut hệ điều hành cho macOS (Cmd+C/V/X/A...), không đẩy vào PTY.
    if cfg!(target_os = "macos") && modifiers.command && !modifiers.ctrl {
        return false;
    }

    if is_altgr_printable_combo(key, modifiers) {
        return false;
    }

    if has_control_modifier(modifiers) || modifiers.alt {
        return true;
    }

    matches!(
        key,
        Key::Enter
            | Key::Backspace
            | Key::Tab
            | Key::Escape
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Delete
            | Key::Insert
            | Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F6
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F10
            | Key::F11
            | Key::F12
    )
}

pub fn is_attach_exit_key(key: KeyEvent) -> bool {
    key.code == KeyCode::F(10) && key.modifiers.is_empty()
}

fn has_control_modifier(modifiers: Modifiers) -> bool {
    modifiers.ctrl || (!cfg!(target_os = "macos") && modifiers.command)
}

fn is_altgr_printable_combo(key: Key, modifiers: Modifiers) -> bool {
    if !(modifiers.alt && has_control_modifier(modifiers)) {
        return false;
    }
    matches!(
        key,
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
            | Key::Z
            | Key::Num0
            | Key::Num1
            | Key::Num2
            | Key::Num3
            | Key::Num4
            | Key::Num5
            | Key::Num6
            | Key::Num7
            | Key::Num8
            | Key::Num9
            | Key::Space
            | Key::Colon
            | Key::Comma
            | Key::Backslash
            | Key::Slash
            | Key::Pipe
            | Key::Questionmark
            | Key::Exclamationmark
            | Key::OpenBracket
            | Key::CloseBracket
            | Key::OpenCurlyBracket
            | Key::CloseCurlyBracket
            | Key::Backtick
            | Key::Minus
            | Key::Period
            | Key::Plus
            | Key::Equals
            | Key::Semicolon
            | Key::Quote
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use eframe::egui::{Key, Modifiers};

    use super::{is_attach_exit_key, should_forward_egui_key_event};

    #[test]
    fn should_forward_control_and_alt_keys() {
        assert!(should_forward_egui_key_event(Key::C, Modifiers::CTRL));
        assert!(should_forward_egui_key_event(
            Key::A,
            Modifiers {
                alt: true,
                ..Modifiers::NONE
            }
        ));
        assert!(!should_forward_egui_key_event(
            Key::Q,
            Modifiers {
                ctrl: true,
                alt: true,
                ..Modifiers::NONE
            }
        ));
        assert!(should_forward_egui_key_event(
            Key::ArrowUp,
            Modifiers {
                ctrl: true,
                alt: true,
                ..Modifiers::NONE
            }
        ));
    }

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
