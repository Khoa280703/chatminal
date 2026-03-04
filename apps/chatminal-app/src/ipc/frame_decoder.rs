use chatminal_protocol::ServerFrame;

pub const MAX_FRAME_BUFFER_BYTES: usize = 256 * 1024;

pub fn decode_pending_frames(
    pending: &mut Vec<u8>,
    chunk: &[u8],
) -> (Vec<ServerFrame>, Option<String>) {
    pending.extend_from_slice(chunk);
    if pending.len() > MAX_FRAME_BUFFER_BYTES {
        pending.clear();
        return (
            Vec::new(),
            Some(format!(
                "daemon frame exceeds client buffer limit (>{} bytes)",
                MAX_FRAME_BUFFER_BYTES
            )),
        );
    }

    let mut output = Vec::new();
    while let Some(line_end) = pending.iter().position(|value| *value == b'\n') {
        let mut line = pending.drain(..=line_end).collect::<Vec<u8>>();
        if line.ends_with(b"\n") {
            line.pop();
        }
        if line.ends_with(b"\r") {
            line.pop();
        }
        if line.is_empty() {
            continue;
        }

        let raw = String::from_utf8_lossy(&line).trim().to_string();
        if raw.is_empty() {
            continue;
        }
        let frame = match serde_json::from_str::<ServerFrame>(&raw) {
            Ok(value) => value,
            Err(_) => {
                eprintln!("chatminal-app: ignored malformed daemon frame line");
                continue;
            }
        };
        output.push(frame);
    }
    (output, None)
}
