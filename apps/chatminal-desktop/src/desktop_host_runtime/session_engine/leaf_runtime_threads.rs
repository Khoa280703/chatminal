use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

use chatminal_terminal_core::TerminalSize;
use engine_term::Terminal as IoTerminal;
use portable_pty::{Child, CommandBuilder, PtySize};

use super::leaf_runtime::{
    TerminalInstanceRuntimeEvent, TerminalInstanceRuntimeHooks, TerminalInstanceRuntimeSpawn,
};
use super::output_history::OutputHistory;

pub(crate) fn spawn_reader_waiter_loop(
    io_terminal: Arc<Mutex<IoTerminal>>,
    output_history: Arc<Mutex<OutputHistory>>,
    spawn: TerminalInstanceRuntimeSpawn,
    hooks: TerminalInstanceRuntimeHooks,
    mut reader: Box<dyn Read + Send>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
) {
    thread::spawn(move || {
        let session_id: Arc<str> = Arc::from(spawn.session_id.as_str());
        let mut buffer = vec![0u8; 64 * 1024];
        let mut restored_prompt_fragment = spawn
            .initial_scrollback
            .as_deref()
            .and_then(snapshot_trailing_fragment)
            .filter(|fragment| looks_like_shell_prompt_fragment(fragment));
        let mut last_visible_prompt = restored_prompt_fragment
            .as_deref()
            .map(visible_terminal_fragment)
            .filter(|visible| !visible.is_empty());

        // Reader loop — blocking read on PTY fd.
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let raw_chunk = String::from_utf8_lossy(&buffer[..read]).to_string();
                    io_terminal
                        .lock()
                        .unwrap()
                        .advance_bytes(raw_chunk.as_bytes());
                    let mut chunk = raw_chunk;
                    if let Some(restored) = restored_prompt_fragment.as_deref() {
                        if visible_terminal_fragment(&chunk).is_empty() {
                            // Keep prompt-dedupe armed across pure control-sequence chunks.
                        } else if let Some(stripped) =
                            strip_duplicate_restored_prompt_redraws(restored, &chunk)
                        {
                            if stripped.is_empty() {
                                continue;
                            }
                            chunk = stripped;
                            restored_prompt_fragment = None;
                        } else {
                            restored_prompt_fragment = None;
                        }
                    }

                    let visible = visible_terminal_fragment(&chunk);
                    if visible.is_empty() {
                        if chunk.contains('\r') || chunk.contains('\n') {
                            last_visible_prompt = None;
                        }
                    } else if looks_like_shell_prompt_fragment(&chunk) {
                        if last_visible_prompt.as_deref() == Some(visible.as_str()) {
                            continue;
                        }
                        last_visible_prompt = Some(visible);
                    } else {
                        last_visible_prompt = None;
                    }

                    output_history.lock().unwrap().push(chunk.clone());
                    (hooks.on_output)(TerminalInstanceRuntimeEvent::Output {
                        session_id: Arc::clone(&session_id),
                        generation: spawn.generation,
                        runtime_id: spawn.runtime_id,
                        terminal_instance_id: spawn.terminal_instance_id,
                        chunk,
                    });
                }
                Err(err) => {
                    (hooks.on_error)(TerminalInstanceRuntimeEvent::Error {
                        session_id: Arc::clone(&session_id),
                        generation: spawn.generation,
                        runtime_id: spawn.runtime_id,
                        terminal_instance_id: spawn.terminal_instance_id,
                        message: format!("pty read failed: {err}"),
                    });
                    break;
                }
            }
        }

        // Waiter phase — reader EOF/error means child is exiting or exited.
        // Poll try_wait until the child process fully exits.
        loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut guard| guard.try_wait().ok())
                .flatten();
            if let Some(status) = status {
                (hooks.on_exit)(TerminalInstanceRuntimeEvent::Exited {
                    session_id,
                    generation: spawn.generation,
                    runtime_id: spawn.runtime_id,
                    terminal_instance_id: spawn.terminal_instance_id,
                    exit_code: Some(status.exit_code() as i32),
                });
                break;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

pub(crate) fn to_pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows.clamp(2, u16::MAX as usize) as u16,
        cols: size.cols.clamp(2, u16::MAX as usize) as u16,
        pixel_width: size.pixel_width as u16,
        pixel_height: size.pixel_height as u16,
    }
}

pub(crate) fn command_label(command: &CommandBuilder) -> Option<String> {
    command
        .get_argv()
        .first()
        .map(|value| value.to_string_lossy().to_string())
}

pub(crate) fn sanitize_zsh_prompt_spacer(bytes: &[u8]) -> Vec<u8> {
    const PREFIX: &[u8] = b"\x1b[1m\x1b[7m%\x1b[27m\x1b[1m\x1b[0m";
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(PREFIX) {
            let mut j = i + PREFIX.len();
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            if bytes.get(j) == Some(&b'\r') {
                j += 1;
                if bytes.get(j) == Some(&b' ') {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'\r') {
                    i = j + 1;
                    continue;
                }
            }
        }

        out.push(bytes[i]);
        i += 1;
    }

    out
}

fn snapshot_trailing_fragment(content: &str) -> Option<String> {
    chatminal_runtime::terminal_text_utils::snapshot_trailing_fragment(content)
}

fn visible_terminal_fragment(value: &str) -> String {
    chatminal_runtime::terminal_text_utils::visible_terminal_fragment(value)
}

fn looks_like_shell_prompt_fragment(value: &str) -> bool {
    let visible = visible_terminal_fragment(value);
    if !visible.ends_with(' ') {
        return false;
    }
    let trimmed = visible.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    match trimmed.chars().last() {
        Some('%') => trimmed.contains('@') || trimmed.contains('~'),
        Some('$') | Some('#') => {
            trimmed.contains('@')
                || trimmed.starts_with("PS ")
                || trimmed.contains(":~")
                || trimmed.contains(":/")
                || trimmed.contains(":\\")
        }
        Some('>') => trimmed.starts_with("PS ") || trimmed.contains('@'),
        _ => false,
    }
}

fn visible_prefix_end_for_fragment(chunk: &str, expected_visible: &str) -> Option<usize> {
    let expected_chars: Vec<char> = expected_visible.chars().collect();
    if expected_chars.is_empty() {
        return None;
    }

    let chars: Vec<(usize, char)> = chunk.char_indices().collect();
    let mut idx = 0usize;
    let mut matched = 0usize;
    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        if ch == '\u{1b}' {
            idx += 1;
            if idx >= chars.len() {
                break;
            }
            match chars[idx].1 {
                '[' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                ']' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                'P' | '^' | '_' | 'X' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if c == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                _ => {
                    idx += 1;
                }
            }
            continue;
        }

        idx += 1;
        if ch == '\r' || ch == '\n' || ch.is_control() {
            continue;
        }
        if expected_chars.get(matched) != Some(&ch) {
            return None;
        }
        matched += 1;
        if matched == expected_chars.len() {
            return Some(byte_idx + ch.len_utf8());
        }
    }

    None
}

fn strip_duplicate_restored_prompt_redraws(restored: &str, chunk: &str) -> Option<String> {
    let expected_visible = visible_terminal_fragment(restored);
    if !looks_like_shell_prompt_fragment(restored) {
        return None;
    }
    let mut remainder = chunk.to_string();
    let mut stripped_any = false;

    while let Some(prefix_end) = visible_prefix_end_for_fragment(&remainder, &expected_visible) {
        stripped_any = true;
        remainder = remainder[prefix_end..].to_string();
        let remaining_visible = visible_terminal_fragment(&remainder);
        if remaining_visible.is_empty() || !remaining_visible.starts_with(&expected_visible) {
            break;
        }
    }

    stripped_any.then_some(remainder)
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_shell_prompt_fragment, sanitize_zsh_prompt_spacer, snapshot_trailing_fragment,
        strip_duplicate_restored_prompt_redraws, visible_terminal_fragment,
    };

    #[test]
    fn strips_zsh_prompt_spacer_artifact() {
        let raw = b"\x1b[1m\x1b[7m%\x1b[27m\x1b[1m\x1b[0m     \r \r\x1b[0muser@host % ";
        let sanitized = sanitize_zsh_prompt_spacer(raw);
        assert_eq!(sanitized, b"\x1b[0muser@host % ");
    }

    #[test]
    fn keeps_normal_prompt_output_untouched() {
        let raw = b"\r\r\x1b[0m\x1b[27m\x1b[24m\x1b[Juser@host % ";
        let sanitized = sanitize_zsh_prompt_spacer(raw);
        assert_eq!(sanitized, raw);
    }

    #[test]
    fn extracts_snapshot_trailing_prompt_fragment() {
        assert_eq!(
            snapshot_trailing_fragment("echo hi\nuser@host ~ % "),
            Some("user@host ~ % ".to_string())
        );
        assert_eq!(snapshot_trailing_fragment("echo hi\n"), None);
    }

    #[test]
    fn strips_duplicate_prompt_redraw_prefix_from_runtime_chunk() {
        let stripped = strip_duplicate_restored_prompt_redraws(
            "user@host ~ % ",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Juser@host ~ % ",
        )
        .expect("prompt prefix stripped");
        assert_eq!(stripped, "");
    }

    #[test]
    fn strips_repeated_duplicate_prompt_redraws_from_runtime_chunk() {
        let stripped = strip_duplicate_restored_prompt_redraws(
            "user@host ~ % ",
            "\r\x1b[0muser@host ~ % \r\n\x1b[0muser@host ~ % ",
        )
        .expect("duplicate prompt redraws stripped");
        assert_eq!(stripped, "");
    }

    #[test]
    fn preserves_real_output_after_duplicate_prompt_redraw() {
        let stripped = strip_duplicate_restored_prompt_redraws(
            "user@host ~ % ",
            "\r\x1b[0muser@host ~ % \r\nls\r\nfile.txt\r\n",
        )
        .expect("leading prompt redraw stripped");
        assert_eq!(stripped, "\r\nls\r\nfile.txt\r\n");
    }

    #[test]
    fn ignores_dcs_xtversion_when_computing_visible_fragment() {
        let value = concat!(
            "\x1bP>|Chatminal 20260327-085528-1b1caf5365\x1b\\",
            "\x1b[?65;4;6;18;22;52c",
            "\r\x1b[0muser@host ~ % "
        );
        assert_eq!(visible_terminal_fragment(value), "user@host ~ % ");
    }

    #[test]
    fn prompt_like_fragment_detection_matches_real_shell_prompt() {
        assert!(looks_like_shell_prompt_fragment(
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Juser@host ~ % \x1b[K"
        ));
        assert!(!looks_like_shell_prompt_fragment("command output\n"));
    }
}
