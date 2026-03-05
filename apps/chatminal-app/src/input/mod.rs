mod ime_commit_deduper;
mod ime_composition_state;
mod pty_key_translator;
mod terminal_input_encoder;
mod terminal_input_event;
mod terminal_input_shortcut_filter;

pub use ime_commit_deduper::{ImeCommitDeduper, ImeDeduperKind};
pub use ime_composition_state::ImeCompositionState;
pub use terminal_input_encoder::encode_key_chord_to_pty_input;
pub use terminal_input_event::{TerminalInputEvent, TerminalInputSource, map_egui_key_event};
pub use pty_key_translator::map_key_event_to_pty_input;
pub use pty_key_translator::map_key_event_to_pty_input_legacy;
pub use terminal_input_shortcut_filter::{is_attach_exit_key, should_forward_egui_key_event};
