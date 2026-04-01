/// Shared terminal text utilities used by both `chatminal-runtime` and
/// `desktop_host_runtime::session_engine` to avoid duplicate implementations.

/// Strips terminal escape sequences and control characters from `value`,
/// returning only the human-visible text content.
pub fn visible_terminal_fragment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\x1b' => {
                i += 1;
                match bytes.get(i).copied() {
                    Some(b'[') => {
                        i += 1;
                        while i < bytes.len() {
                            let byte = bytes[i];
                            i += 1;
                            if (0x40..=0x7e).contains(&byte) {
                                break;
                            }
                        }
                    }
                    Some(b']') => {
                        i += 1;
                        while i < bytes.len() {
                            match bytes[i] {
                                0x07 => {
                                    i += 1;
                                    break;
                                }
                                0x1b if bytes.get(i + 1) == Some(&b'\\') => {
                                    i += 2;
                                    break;
                                }
                                _ => i += 1,
                            }
                        }
                    }
                    Some(b'P') | Some(b'^') | Some(b'_') | Some(b'X') => {
                        i += 1;
                        while i < bytes.len() {
                            match bytes[i] {
                                0x1b if bytes.get(i + 1) == Some(&b'\\') => {
                                    i += 2;
                                    break;
                                }
                                _ => i += 1,
                            }
                        }
                    }
                    Some(_) => {
                        i += 1;
                    }
                    None => break,
                }
            }
            b'\r' | b'\n' => {
                i += 1;
            }
            byte if byte.is_ascii_control() => {
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}

/// Extracts the trailing fragment (text after the last newline) from terminal
/// content. Returns `None` if the content is empty or ends with a newline.
pub fn snapshot_trailing_fragment(content: &str) -> Option<String> {
    if content.is_empty() || content.ends_with('\n') || content.ends_with('\r') {
        return None;
    }

    let cut = content
        .rfind(['\n', '\r'])
        .map(|index| index.saturating_add(1))
        .unwrap_or(0);
    let fragment = content[cut..].to_string();
    (!fragment.is_empty()).then_some(fragment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_csi_and_osc_escapes() {
        let value = "\x1b[31mhello\x1b[0m";
        assert_eq!(visible_terminal_fragment(value), "hello");
    }

    #[test]
    fn strips_dcs_sequences() {
        let value = "\x1bP>|Chatminal\x1b\\world";
        assert_eq!(visible_terminal_fragment(value), "world");
    }

    #[test]
    fn returns_none_for_newline_terminated_content() {
        assert_eq!(snapshot_trailing_fragment("hello\n"), None);
        assert_eq!(snapshot_trailing_fragment("hello\r"), None);
    }

    #[test]
    fn extracts_trailing_fragment() {
        assert_eq!(
            snapshot_trailing_fragment("line1\nuser@host ~ % "),
            Some("user@host ~ % ".to_string())
        );
    }
}
