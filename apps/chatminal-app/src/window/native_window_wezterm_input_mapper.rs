use eframe::egui::{Key, Modifiers};

use crate::input::{
    TerminalInputEvent, TerminalKey, encode_key_chord_to_pty_input, map_egui_key_event,
    should_forward_egui_key_event,
};

pub(super) fn map_egui_key_event_to_pty_input(
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Option<String> {
    if !should_forward_egui_key_event(key, modifiers) {
        return None;
    }

    match map_egui_key_event(key, modifiers, repeat) {
        Some(TerminalInputEvent::KeyChord(chord)) => encode_key_chord_to_pty_input(chord),
        _ => None,
    }
}

pub(super) fn map_egui_key_event_to_pty_input_legacy(
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Option<String> {
    match map_egui_key_event(key, modifiers, repeat) {
        Some(TerminalInputEvent::KeyChord(chord)) => encode_key_chord_to_pty_input(chord),
        _ => None,
    }
}

pub(super) fn map_egui_printable_key_event_to_text_input(
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Option<String> {
    if modifiers.ctrl || modifiers.alt || modifiers.command || modifiers.mac_cmd {
        return None;
    }

    match map_egui_key_event(key, modifiers, repeat) {
        Some(TerminalInputEvent::KeyChord(chord)) => match chord.key {
            TerminalKey::Char(_) => encode_key_chord_to_pty_input(chord),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Key, Modifiers};

    use super::{
        map_egui_key_event_to_pty_input, map_egui_key_event_to_pty_input_legacy,
        map_egui_printable_key_event_to_text_input,
    };

    #[test]
    fn map_egui_key_event_to_pty_input_supports_common_keys() {
        let enter = map_egui_key_event_to_pty_input(Key::Enter, Modifiers::NONE, false);
        assert_eq!(enter.as_deref(), Some("\r"));

        let ctrl_c = map_egui_key_event_to_pty_input(Key::C, Modifiers::CTRL, false);
        assert_eq!(ctrl_c.as_deref(), Some("\u{3}"));

        let backtab = map_egui_key_event_to_pty_input(
            Key::Tab,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            false,
        );
        assert_eq!(backtab.as_deref(), Some("\u{1b}[Z"));
    }

    #[test]
    fn legacy_mapper_bypasses_shortcut_filter() {
        let command_c = map_egui_key_event_to_pty_input_legacy(
            Key::C,
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
            false,
        );
        if cfg!(target_os = "macos") {
            assert_eq!(command_c.as_deref(), Some("c"));
        } else {
            assert_eq!(command_c.as_deref(), Some("\u{3}"));
        }
    }

    #[test]
    fn printable_key_mapper_returns_plain_text_only() {
        let lower = map_egui_printable_key_event_to_text_input(Key::A, Modifiers::NONE, false);
        assert_eq!(lower.as_deref(), Some("a"));

        let upper = map_egui_printable_key_event_to_text_input(
            Key::A,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            false,
        );
        assert_eq!(upper.as_deref(), Some("A"));

        let ctrl = map_egui_printable_key_event_to_text_input(Key::A, Modifiers::CTRL, false);
        assert_eq!(ctrl, None);

        let enter = map_egui_printable_key_event_to_text_input(Key::Enter, Modifiers::NONE, false);
        assert_eq!(enter, None);
    }
}
