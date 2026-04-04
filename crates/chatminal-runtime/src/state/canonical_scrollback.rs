use std::collections::{BTreeMap, BTreeSet};

use chatminal_store::{
    Store, StoredLegacyScrollbackChunk, StoredScrollbackRecord, StoredScrollbackRecordInput,
    StoredScrollbackRecordKind, StoredSessionSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogicalSnapshot {
    pub(super) lines: Vec<String>,
    pub(super) open_fragment: String,
    pub(super) seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MaterializedChunk {
    pub(super) records: Vec<StoredScrollbackRecordInput>,
    pub(super) open_fragment: String,
    pub(super) cursor_col: usize,
    pub(super) pending_carriage_return: bool,
}

pub(super) fn materialize_output_chunk(
    current_fragment: &str,
    cursor_col: usize,
    pending_carriage_return: bool,
    chunk: &str,
) -> MaterializedChunk {
    let mut reducer = LogicalReducer::new(current_fragment, cursor_col, pending_carriage_return);
    reducer.apply_chunk(chunk);
    let open_fragment = reducer.render_current_fragment();

    let mut records = Vec::with_capacity(reducer.lines.len().saturating_add(1));
    for (ord, line) in reducer.lines.into_iter().enumerate() {
        records.push(StoredScrollbackRecordInput {
            ord: ord as u64,
            kind: StoredScrollbackRecordKind::Line,
            text: line,
        });
    }
    records.push(StoredScrollbackRecordInput {
        ord: records.len() as u64,
        kind: StoredScrollbackRecordKind::Fragment,
        text: open_fragment.clone(),
    });

    MaterializedChunk {
        records,
        open_fragment,
        cursor_col: reducer.cursor_col,
        pending_carriage_return: reducer.pending_carriage_return,
    }
}

pub(super) fn build_logical_snapshot(
    store: &Store,
    session_id: &str,
) -> Result<LogicalSnapshot, String> {
    migrate_legacy_scrollback_if_needed(store, session_id)?;
    let canonical = store.list_scrollback_records(session_id)?;

    let mut canonical_by_seq = BTreeMap::<u64, Vec<StoredScrollbackRecord>>::new();
    for record in canonical {
        canonical_by_seq.entry(record.seq).or_default().push(record);
    }
    for records in canonical_by_seq.values_mut() {
        records.sort_by_key(|record| (record.seq, record.ord));
    }

    let mut lines = Vec::new();
    let mut open_fragment = String::new();
    let mut pending_prompt_line: Option<String> = None;
    let mut prompt_prefix_override: Option<String> = None;
    let mut max_seq = 0u64;

    for records in canonical_by_seq.into_values() {
        for record in records {
            max_seq = max_seq.max(record.seq);
            match record.kind {
                StoredScrollbackRecordKind::Line => {
                    apply_canonical_line_record(
                        &mut lines,
                        &mut open_fragment,
                        &mut pending_prompt_line,
                        &mut prompt_prefix_override,
                        &record.text,
                    );
                }
                StoredScrollbackRecordKind::Fragment => {
                    apply_canonical_fragment_record(
                        &mut lines,
                        &mut open_fragment,
                        &mut pending_prompt_line,
                        &mut prompt_prefix_override,
                        &record.text,
                    );
                }
            }
        }
    }

    if let Some(prompt) = pending_prompt_line {
        if open_fragment.is_empty() {
            open_fragment = prompt;
        } else {
            lines.push(prompt);
        }
    } else if let Some(prompt) = prompt_prefix_override {
        if open_fragment.is_empty() {
            open_fragment = prompt;
        }
    }

    Ok(LogicalSnapshot {
        lines,
        open_fragment,
        seq: max_seq,
    })
}

fn migrate_legacy_scrollback_if_needed(store: &Store, session_id: &str) -> Result<(), String> {
    let legacy = store.list_legacy_scrollback_chunks(session_id)?;
    if legacy.is_empty() {
        return Ok(());
    }

    let canonical = store.list_scrollback_records(session_id)?;
    let mut legacy_by_seq = BTreeMap::<u64, StoredLegacyScrollbackChunk>::new();
    for chunk in legacy {
        legacy_by_seq.insert(chunk.seq, chunk);
    }

    let mut canonical_by_seq = BTreeMap::<u64, Vec<StoredScrollbackRecord>>::new();
    for record in canonical {
        canonical_by_seq.entry(record.seq).or_default().push(record);
    }
    for records in canonical_by_seq.values_mut() {
        records.sort_by_key(|record| (record.seq, record.ord));
    }

    let mut all_seqs = BTreeSet::new();
    all_seqs.extend(legacy_by_seq.keys().copied());
    all_seqs.extend(canonical_by_seq.keys().copied());

    let mut lines = Vec::new();
    let mut open_fragment = String::new();
    let mut pending_prompt_line: Option<String> = None;
    let mut prompt_prefix_override: Option<String> = None;
    let mut migrated_records = Vec::<(u64, Vec<StoredScrollbackRecordInput>, u64)>::new();

    for seq in all_seqs {
        if let Some(records) = canonical_by_seq.get(&seq) {
            apply_canonical_records(
                records.iter().cloned(),
                &mut lines,
                &mut open_fragment,
                &mut pending_prompt_line,
                &mut prompt_prefix_override,
            );
            continue;
        }

        let Some(chunk) = legacy_by_seq.get(&seq) else {
            continue;
        };
        let materialized = materialize_output_chunk(
            &open_fragment,
            open_fragment.chars().count(),
            false,
            &chunk.chunk_text,
        );
        apply_canonical_records(
            materialized
                .records
                .iter()
                .cloned()
                .map(|record| StoredScrollbackRecord {
                    session_id: session_id.to_string(),
                    seq,
                    ord: record.ord,
                    kind: record.kind,
                    text: record.text,
                    ts: chunk.ts,
                }),
            &mut lines,
            &mut open_fragment,
            &mut pending_prompt_line,
            &mut prompt_prefix_override,
        );
        migrated_records.push((seq, materialized.records, chunk.ts));
    }

    for (seq, records, ts) in migrated_records {
        store.append_scrollback_records(session_id, seq, &records, ts)?;
    }
    store.clear_legacy_scrollback_chunks(session_id)?;
    Ok(())
}

fn apply_canonical_records<I>(
    records: I,
    lines: &mut Vec<String>,
    open_fragment: &mut String,
    pending_prompt_line: &mut Option<String>,
    prompt_prefix_override: &mut Option<String>,
) where
    I: IntoIterator<Item = StoredScrollbackRecord>,
{
    for record in records {
        match record.kind {
            StoredScrollbackRecordKind::Line => {
                apply_canonical_line_record(
                    lines,
                    open_fragment,
                    pending_prompt_line,
                    prompt_prefix_override,
                    &record.text,
                );
            }
            StoredScrollbackRecordKind::Fragment => {
                apply_canonical_fragment_record(
                    lines,
                    open_fragment,
                    pending_prompt_line,
                    prompt_prefix_override,
                    &record.text,
                );
            }
        }
    }
}

fn apply_canonical_line_record(
    lines: &mut Vec<String>,
    open_fragment: &mut String,
    pending_prompt_line: &mut Option<String>,
    prompt_prefix_override: &mut Option<String>,
    line: &str,
) {
    if let Some(prompt) = pending_prompt_line.take() {
        lines.push(prompt);
    }

    if let Some(prompt) = prompt_prefix_override.clone() {
        if looks_like_shell_prompt_fragment(line) {
            lines.push(prompt);
            *pending_prompt_line = Some(line.to_string());
            *prompt_prefix_override = None;
            open_fragment.clear();
            return;
        }

        lines.push(format!("{prompt}{line}"));
        *prompt_prefix_override = None;
        open_fragment.clear();
        return;
    }

    if looks_like_shell_prompt_fragment(line) {
        *pending_prompt_line = Some(line.to_string());
        open_fragment.clear();
    } else {
        lines.push(line.to_string());
        open_fragment.clear();
    }
}

fn apply_canonical_fragment_record(
    lines: &mut Vec<String>,
    open_fragment: &mut String,
    pending_prompt_line: &mut Option<String>,
    prompt_prefix_override: &mut Option<String>,
    fragment: &str,
) {
    if prompt_prefix_override.is_none()
        && let Some(prompt) = pending_prompt_line.clone()
    {
        if fragment.is_empty() {
            open_fragment.clear();
            return;
        }

        if looks_like_shell_prompt_fragment(fragment) {
            lines.push(prompt);
            *pending_prompt_line = None;
            *open_fragment = fragment.to_string();
            return;
        }

        *prompt_prefix_override = Some(prompt);
        *pending_prompt_line = None;
    }

    if let Some(prompt) = prompt_prefix_override.clone() {
        if fragment.is_empty() {
            open_fragment.clear();
            return;
        }

        if looks_like_shell_prompt_fragment(fragment) {
            lines.push(prompt);
            *prompt_prefix_override = None;
            *open_fragment = fragment.to_string();
            return;
        }

        *open_fragment = format!("{prompt}{fragment}");
        return;
    }

    *open_fragment = fragment.to_string();
}

pub(super) fn render_snapshot(
    snapshot: &LogicalSnapshot,
    preview_lines: Option<usize>,
) -> StoredSessionSnapshot {
    render_snapshot_with_line_ending(snapshot, preview_lines, "\n")
}

pub(super) fn render_snapshot_for_terminal(snapshot: &LogicalSnapshot) -> StoredSessionSnapshot {
    render_snapshot_with_line_ending(snapshot, None, "\r\n")
}

fn render_snapshot_with_line_ending(
    snapshot: &LogicalSnapshot,
    preview_lines: Option<usize>,
    line_ending: &str,
) -> StoredSessionSnapshot {
    let selected_lines = if let Some(limit) = preview_lines {
        if limit == usize::MAX {
            snapshot.lines.clone()
        } else {
            let start = snapshot.lines.len().saturating_sub(limit);
            snapshot.lines[start..].to_vec()
        }
    } else {
        snapshot.lines.clone()
    };

    let mut content = String::new();
    for line in &selected_lines {
        content.push_str(line);
        content.push_str(line_ending);
    }

    if !snapshot.open_fragment.is_empty() {
        content.push_str(&snapshot.open_fragment);
    }

    StoredSessionSnapshot {
        content,
        seq: snapshot.seq,
    }
}

struct LogicalReducer {
    current_line: Vec<char>,
    cursor_col: usize,
    lines: Vec<String>,
    pending_carriage_return: bool,
}

impl LogicalReducer {
    fn new(current_fragment: &str, cursor_col: usize, pending_carriage_return: bool) -> Self {
        let current_line = current_fragment.chars().collect::<Vec<_>>();
        let line_len = current_line.len();
        Self {
            current_line,
            cursor_col: cursor_col.min(line_len),
            lines: Vec::new(),
            pending_carriage_return,
        }
    }

    fn apply_chunk(&mut self, chunk: &str) {
        let bytes = chunk.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'\x1b' => {
                    i = skip_escape_sequence(bytes, i, self);
                }
                b'\r' => {
                    self.pending_carriage_return = true;
                    i += 1;
                }
                b'\n' => {
                    self.pending_carriage_return = false;
                    self.lines.push(self.render_current_fragment());
                    self.current_line.clear();
                    self.cursor_col = 0;
                    i += 1;
                }
                0x08 => {
                    self.resolve_pending_carriage_return();
                    self.delete_before_cursor();
                    i += 1;
                }
                b'\t' => {
                    self.resolve_pending_carriage_return();
                    self.write_char('\t');
                    i += 1;
                }
                byte if byte.is_ascii_control() => {
                    i += 1;
                }
                _ => {
                    self.resolve_pending_carriage_return();
                    let Some(ch) = chunk[i..].chars().next() else {
                        break;
                    };
                    self.write_char(ch);
                    i += ch.len_utf8();
                }
            }
        }
    }

    fn resolve_pending_carriage_return(&mut self) {
        if self.pending_carriage_return {
            self.cursor_col = 0;
            self.pending_carriage_return = false;
        }
    }

    fn render_current_fragment(&self) -> String {
        self.current_line.iter().collect()
    }

    fn write_char(&mut self, ch: char) {
        if self.cursor_col > self.current_line.len() {
            self.current_line.resize(self.cursor_col, ' ');
        }
        if self.cursor_col == self.current_line.len() {
            self.current_line.push(ch);
        } else {
            self.current_line[self.cursor_col] = ch;
        }
        self.cursor_col += 1;
    }

    fn delete_before_cursor(&mut self) {
        if self.cursor_col == 0 {
            return;
        }
        self.cursor_col -= 1;
        if self.cursor_col < self.current_line.len() {
            self.current_line.remove(self.cursor_col);
        }
    }

    fn erase_in_line(&mut self, mode: usize) {
        self.resolve_pending_carriage_return();
        match mode {
            1 => {
                let end = self.cursor_col.min(self.current_line.len());
                self.current_line.drain(..end);
                self.cursor_col = 0;
            }
            2 => {
                self.current_line.clear();
                self.cursor_col = 0;
            }
            _ => {
                if self.cursor_col < self.current_line.len() {
                    self.current_line.truncate(self.cursor_col);
                }
            }
        }
    }

    fn move_cursor_forward(&mut self, count: usize) {
        self.resolve_pending_carriage_return();
        self.cursor_col = self.cursor_col.saturating_add(count);
    }

    fn move_cursor_backward(&mut self, count: usize) {
        self.resolve_pending_carriage_return();
        self.cursor_col = self.cursor_col.saturating_sub(count);
    }

    fn move_cursor_absolute(&mut self, column: usize) {
        self.resolve_pending_carriage_return();
        self.cursor_col = column;
    }
}

fn skip_escape_sequence(bytes: &[u8], start: usize, reducer: &mut LogicalReducer) -> usize {
    let Some(next) = bytes.get(start + 1).copied() else {
        return bytes.len();
    };

    match next {
        b'[' => skip_csi_sequence(bytes, start, reducer),
        b']' => skip_osc_sequence(bytes, start + 2),
        b'P' | b'^' | b'_' | b'X' => skip_st_terminated_sequence(bytes, start + 2),
        _ => start.saturating_add(2).min(bytes.len()),
    }
}

fn skip_csi_sequence(bytes: &[u8], start: usize, reducer: &mut LogicalReducer) -> usize {
    let mut i = start + 2;
    let params_start = i;
    while i < bytes.len() {
        let byte = bytes[i];
        i += 1;
        if !(0x40..=0x7e).contains(&byte) {
            continue;
        }

        let params = std::str::from_utf8(&bytes[params_start..i - 1]).unwrap_or_default();
        let first_param = params
            .split(';')
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        match byte {
            b'K' => reducer.erase_in_line(first_param),
            b'C' => reducer.move_cursor_forward(first_param),
            b'D' => reducer.move_cursor_backward(first_param),
            b'G' => reducer.move_cursor_absolute(first_param.saturating_sub(1)),
            _ => {}
        }
        break;
    }
    i
}

fn skip_osc_sequence(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            0x07 => return i + 1,
            0x1b if bytes.get(i + 1) == Some(&b'\\') => return i + 2,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn skip_st_terminated_sequence(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

fn looks_like_shell_prompt_fragment(value: &str) -> bool {
    let trimmed = value.trim_end();
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
