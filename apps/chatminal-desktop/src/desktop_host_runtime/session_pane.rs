use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::Write;
use std::ops::Range;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::session_engine::{
    RuntimeId, SessionEngineShared, SessionRuntimeEvent, TerminalInstanceId,
};
use chatminal_terminal_core::TerminalSize as CoreTerminalSize;
use config::{ConfigHandle, TermConfig};
use engine_dynamic::Value;
use engine_term::color::ColorPalette;
use engine_term::{
    Clipboard, DownloadHandler, KeyCode, KeyModifiers, MouseEvent, Progress, SemanticZone,
    StableRowIndex, Terminal, TerminalConfiguration, TerminalSize,
};
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rangeset::RangeSet;
use regex::Regex;
use termwiz::escape::parser::Parser as EscapeParser;
use termwiz::escape::Action;
use termwiz::input::KeyboardEncoding;
use termwiz::surface::{Line, SequenceNo};
use url::Url;

use super::{
    alloc_host_terminal_handle, host_impl_get_logical_lines_via_get_lines,
    host_terminal_for_each_logical_line_in_stable_range_mut, host_terminal_get_cursor_position,
    host_terminal_get_dimensions, host_terminal_get_dirty_lines, host_terminal_get_lines,
    host_terminal_with_lines_mut, publish_runtime_notification_from_any_thread,
    record_host_input_for_current_identity, HostCachePolicy as CachePolicy,
    HostCloseReason as CloseReason, HostLogicalLine as LogicalLine, HostPattern as Pattern,
    HostRenderableDimensions as RenderableDimensions, HostSearchResult as SearchResult,
    HostStableCursorPosition as StableCursorPosition, HostTerminal, HostTerminalHandle,
    RuntimeNotification,
};
use crate::chatminal_runtime::overlay_compat::{
    OverlayForEachLogicalLine as ForEachPaneLogicalLine, OverlayWithPaneLines as WithPaneLines,
};
use crate::chatminal_runtime::SessionTerminalHandle;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);
const DISPLAY_RESET_FLASH_DURATION: Duration = Duration::from_millis(75);

fn notify_pane_output(pane_id: HostTerminalHandle) {
    publish_runtime_notification_from_any_thread(RuntimeNotification::PaneOutput(pane_id));
}

fn record_input_for_current_identity() {
    record_host_input_for_current_identity();
}

fn normalize_direct_terminal_key(key: KeyCode) -> KeyCode {
    key
}

#[derive(Clone)]
struct SessionPaneWriter {
    inner: Arc<Mutex<SessionPaneWriterState>>,
}

struct SessionPaneWriterState {
    shared: Arc<SessionEngineShared>,
    terminal_instance_id: TerminalInstanceId,
    pending_input_bytes: Vec<u8>,
}

impl SessionPaneWriter {
    fn new(shared: Arc<SessionEngineShared>, terminal_instance_id: TerminalInstanceId) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionPaneWriterState {
                shared,
                terminal_instance_id,
                pending_input_bytes: Vec::new(),
            })),
        }
    }
}

impl Write for SessionPaneWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut state = self.inner.lock();
        let chunks = decode_input_payload_chunks(&mut state.pending_input_bytes, buf);
        for chunk in chunks {
            state
                .shared
                .write_terminal_input(state.terminal_instance_id, &chunk)
                .map_err(std::io::Error::other)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) struct ChatminalSessionPane {
    pane_id: HostTerminalHandle,
    session_id: String,
    runtime_id: RuntimeId,
    terminal_instance_id: TerminalInstanceId,
    shared: Arc<SessionEngineShared>,
    display_state: Mutex<()>,
    parser: Mutex<EscapeParser>,
    terminal: Mutex<Terminal>,
    input_writer: Mutex<SessionPaneWriter>,
    dead: Mutex<bool>,
    config: Mutex<Option<Arc<dyn TerminalConfiguration>>>,
}

impl ChatminalSessionPane {
    pub(crate) fn new(
        shared: Arc<SessionEngineShared>,
        session_id: String,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        size: TerminalSize,
        config: ConfigHandle,
    ) -> anyhow::Result<Arc<Self>> {
        let input_writer = SessionPaneWriter::new(Arc::clone(&shared), terminal_instance_id);
        let pane = Arc::new(Self {
            pane_id: pane_id_for_terminal_instance(terminal_instance_id),
            session_id,
            runtime_id,
            terminal_instance_id,
            shared,
            display_state: Mutex::new(()),
            parser: Mutex::new(EscapeParser::new()),
            terminal: Mutex::new(Terminal::new(
                size,
                Arc::new(TermConfig::with_config(config.clone())),
                "Chatminal",
                config::engine_version(),
                Box::new(std::io::sink()),
            )),
            input_writer: Mutex::new(input_writer),
            dead: Mutex::new(false),
            config: Mutex::new(None),
        });

        pane.seed_replay_output();
        pane.spawn_event_loop()?;
        Ok(pane)
    }

    fn seed_replay_output(&self) {
        if let Some(replay) = self.shared.replay_output(self.terminal_instance_id) {
            self.apply_output(&replay);
        }
    }

    fn reset_display_state_and_replay(&self) {
        let _display_state = self.display_state.lock();
        let replay = self.shared.replay_output(self.terminal_instance_id);
        let mut parser = self.parser.lock();
        let mut terminal = self.terminal.lock();

        *parser = EscapeParser::new();
        terminal.perform_actions(vec![Action::Esc(termwiz::escape::Esc::Code(
            termwiz::escape::EscCode::FullReset,
        ))]);

        if let Some(replay) = replay.as_deref() {
            let replay_actions = parse_output_actions(&mut parser, replay.as_bytes());
            if !replay_actions.is_empty() {
                terminal.perform_actions(replay_actions);
            }
        }

        notify_pane_output(self.pane_id);
    }

    pub(crate) fn reset_display_state_with_flash(self: &Arc<Self>) {
        let pane = Arc::clone(self);
        thread::spawn(move || {
            let replay = pane.shared.replay_output(pane.terminal_instance_id);
            let _display_state = pane.display_state.lock();

            {
                let mut parser = pane.parser.lock();
                let mut terminal = pane.terminal.lock();
                *parser = EscapeParser::new();
                terminal.perform_actions(vec![Action::Esc(termwiz::escape::Esc::Code(
                    termwiz::escape::EscCode::FullReset,
                ))]);
            }

            notify_pane_output(pane.pane_id);

            thread::sleep(DISPLAY_RESET_FLASH_DURATION);

            let replay_actions = {
                let mut parser = pane.parser.lock();
                replay
                    .as_deref()
                    .map(|replay| parse_output_actions(&mut parser, replay.as_bytes()))
                    .unwrap_or_default()
            };

            if !replay_actions.is_empty() {
                pane.terminal.lock().perform_actions(replay_actions);
            }

            notify_pane_output(pane.pane_id);
        });
    }

    fn spawn_event_loop(self: &Arc<Self>) -> anyhow::Result<()> {
        let subscription = self.shared.subscribe();
        let pane = Arc::downgrade(self);
        thread::spawn(move || loop {
            let Some(pane) = pane.upgrade() else {
                break;
            };
            match subscription.recv_timeout(EVENT_POLL_TIMEOUT) {
                Ok(Some(event)) => pane.handle_event(event),
                Ok(None) => {}
                Err(err) => {
                    log::error!("chatminal session pane event loop failed: {err}");
                    *pane.dead.lock() = true;
                    break;
                }
            }
            if *pane.dead.lock() {
                break;
            }
        });
        Ok(())
    }

    fn handle_event(&self, event: SessionRuntimeEvent) {
        match event {
            SessionRuntimeEvent::TerminalInstanceOutput {
                runtime_id,
                terminal_instance_id,
                chunk,
                ..
            } if runtime_id == self.runtime_id
                && terminal_instance_id == self.terminal_instance_id =>
            {
                self.apply_output(&chunk);
                notify_pane_output(self.pane_id);
            }
            SessionRuntimeEvent::TerminalInstanceExited {
                runtime_id,
                terminal_instance_id,
                ..
            } if runtime_id == self.runtime_id
                && terminal_instance_id == self.terminal_instance_id =>
            {
                *self.dead.lock() = true;
                notify_pane_output(self.pane_id);
            }
            SessionRuntimeEvent::TerminalInstanceError {
                runtime_id,
                terminal_instance_id,
                message,
                ..
            } if runtime_id == self.runtime_id
                && terminal_instance_id == self.terminal_instance_id =>
            {
                self.apply_output(&format!("\r\n[chatminal session error] {}\r\n", message));
                notify_pane_output(self.pane_id);
            }
            SessionRuntimeEvent::RuntimeClosed { runtime_id, .. }
                if runtime_id == self.runtime_id =>
            {
                *self.dead.lock() = true;
                notify_pane_output(self.pane_id);
            }
            _ => {}
        }
    }

    fn apply_output(&self, message: &str) {
        let _display_state = self.display_state.lock();
        let actions = {
            let mut parser = self.parser.lock();
            parse_output_actions(&mut parser, message.as_bytes())
        };
        if !actions.is_empty() {
            self.terminal.lock().perform_actions(actions);
        }
    }

    pub(crate) fn pane_id_value(&self) -> HostTerminalHandle {
        self.pane_id
    }

    pub(crate) fn runtime_id_value(&self) -> RuntimeId {
        self.runtime_id
    }

    pub(crate) fn terminal_instance_id_value(&self) -> TerminalInstanceId {
        self.terminal_instance_id
    }

    pub(crate) fn session_id_value(&self) -> &str {
        &self.session_id
    }
}

fn parse_output_actions(parser: &mut EscapeParser, bytes: &[u8]) -> Vec<Action> {
    let mut actions = Vec::new();
    parser.parse(bytes, |action| actions.push(action));
    actions
}

#[cfg(test)]
mod parser_tests {
    use super::parse_output_actions;
    use termwiz::escape::csi::{Sgr, CSI};
    use termwiz::escape::parser::Parser as EscapeParser;
    use termwiz::escape::Action;

    #[test]
    fn parser_state_survives_across_split_escape_chunks() {
        let mut parser = EscapeParser::new();

        let first = parse_output_actions(&mut parser, b"\x1b[3");
        let second = parse_output_actions(&mut parser, b"1mhi");

        assert!(first.is_empty());
        assert!(matches!(
            second.first(),
            Some(Action::CSI(CSI::Sgr(Sgr::Foreground(_))))
        ));
        assert!(
            second
                .iter()
                .any(|action| matches!(action, Action::Print('h')))
                || second
                    .iter()
                    .any(|action| matches!(action, Action::PrintString(s) if s == "hi"))
        );
    }
}

pub(crate) fn pane_id_for_terminal_instance(
    terminal_instance_id: TerminalInstanceId,
) -> HostTerminalHandle {
    SessionTerminalHandle::new(
        u64::try_from(alloc_host_terminal_handle())
            .ok()
            .filter(|_| terminal_instance_id.as_u64() > usize::MAX as u64)
            .unwrap_or_else(|| terminal_instance_id.as_u64()),
    )
}

fn looks_like_chatminal_internal_title(title: &str) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() || trimmed == "Chatminal" {
        return true;
    }

    let marker = trimmed
        .strip_prefix(">|Chatminal ")
        .or_else(|| trimmed.strip_prefix("Chatminal "))
        .unwrap_or(trimmed);

    let mut parts = marker.split('-');
    let Some(date) = parts.next() else {
        return false;
    };
    let Some(time) = parts.next() else {
        return false;
    };
    let Some(suffix) = parts.next() else {
        return false;
    };

    if parts.next().is_some() {
        return false;
    }

    date.len() == 8
        && date.bytes().all(|b| b.is_ascii_digit())
        && time.len() == 6
        && time.bytes().all(|b| b.is_ascii_digit())
        && suffix.len() >= 8
        && suffix.bytes().all(|b| b.is_ascii_hexdigit())
}

#[async_trait::async_trait(?Send)]
impl HostTerminal for ChatminalSessionPane {
    fn terminal_handle(&self) -> SessionTerminalHandle {
        self.pane_id
    }
    fn get_cursor_position(&self) -> StableCursorPosition {
        host_terminal_get_cursor_position(&mut self.terminal.lock())
    }
    fn get_current_seqno(&self) -> SequenceNo {
        self.terminal.lock().current_seqno()
    }
    fn get_metadata(&self) -> Value {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            Value::String("chatminal_session_id".into()),
            Value::String(self.session_id.clone()),
        );
        map.insert(
            Value::String("chatminal_runtime_id".into()),
            Value::U64(self.runtime_id.as_u64()),
        );
        map.insert(
            Value::String("chatminal_terminal_instance_id".into()),
            Value::U64(self.terminal_instance_id.as_u64()),
        );
        Value::Object(map.into())
    }
    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        host_terminal_get_dirty_lines(&mut self.terminal.lock(), lines, seqno)
    }
    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        host_terminal_get_lines(&mut self.terminal.lock(), lines)
    }
    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        host_terminal_with_lines_mut(&mut self.terminal.lock(), lines, with_lines)
    }
    fn for_each_logical_line_in_stable_range_mut(
        &self,
        lines: Range<StableRowIndex>,
        for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        host_terminal_for_each_logical_line_in_stable_range_mut(
            &mut self.terminal.lock(),
            lines,
            for_line,
        )
    }
    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        host_impl_get_logical_lines_via_get_lines(self, lines)
    }
    fn get_dimensions(&self) -> RenderableDimensions {
        host_terminal_get_dimensions(&mut self.terminal.lock())
    }
    fn get_title(&self) -> String {
        let title = self.terminal.lock().get_title().to_string();
        if looks_like_chatminal_internal_title(&title) {
            "Chatminal".to_string()
        } else {
            title
        }
    }
    fn get_progress(&self) -> Progress {
        self.terminal.lock().get_progress()
    }
    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        self.shared
            .paste_terminal_input(self.terminal_instance_id, text)
            .map_err(anyhow::Error::msg)
    }
    fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
        Ok(None)
    }
    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        MutexGuard::map(self.input_writer.lock(), |writer| {
            let w: &mut dyn std::io::Write = writer;
            w
        })
    }
    fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
        self.shared
            .resize_terminal_instance(
                self.terminal_instance_id,
                CoreTerminalSize {
                    rows: size.rows,
                    cols: size.cols,
                    pixel_width: size.pixel_width,
                    pixel_height: size.pixel_height,
                    dpi: size.dpi,
                },
            )
            .map_err(anyhow::Error::msg)?;
        self.terminal.lock().resize(size);
        Ok(())
    }
    fn key_down(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        record_input_for_current_identity();
        if matches!(key, KeyCode::Backspace) && mods == KeyModifiers::default() {
            return self
                .shared
                .write_terminal_input(self.terminal_instance_id, [0x7f])
                .map_err(anyhow::Error::msg);
        }
        self.shared
            .key_down_terminal_input(
                self.terminal_instance_id,
                normalize_direct_terminal_key(key),
                mods,
            )
            .map_err(anyhow::Error::msg)
    }
    fn key_up(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        record_input_for_current_identity();
        self.shared
            .key_up_terminal_input(
                self.terminal_instance_id,
                normalize_direct_terminal_key(key),
                mods,
            )
            .map_err(anyhow::Error::msg)
    }
    fn mouse_event(&self, event: MouseEvent) -> anyhow::Result<()> {
        record_input_for_current_identity();
        self.shared
            .mouse_terminal_input(self.terminal_instance_id, event)
            .map_err(anyhow::Error::msg)
    }
    fn perform_actions(&self, actions: Vec<Action>) {
        self.terminal.lock().perform_actions(actions)
    }
    fn reset_display_state(&self) {
        self.reset_display_state_and_replay();
    }
    fn is_dead(&self) -> bool {
        *self.dead.lock()
    }
    fn kill(&self) {
        *self.dead.lock() = true;
    }
    fn palette(&self) -> ColorPalette {
        self.terminal.lock().palette()
    }
    fn get_keyboard_encoding(&self) -> KeyboardEncoding {
        KeyboardEncoding::Xterm
    }
    fn copy_user_vars(&self) -> HashMap<String, String> {
        self.terminal.lock().user_vars().clone()
    }
    fn focus_changed(&self, focused: bool) {
        self.terminal.lock().focus_changed(focused)
    }
    fn has_unseen_output(&self) -> bool {
        self.terminal.lock().has_unseen_output()
    }
    fn can_close_without_prompting(&self, _reason: CloseReason) -> bool {
        true
    }
    async fn search(
        &self,
        pattern: Pattern,
        range: Range<StableRowIndex>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let term = self.terminal.lock();
        let screen = term.screen();

        enum CompiledPattern {
            CaseSensitiveString(String),
            CaseInSensitiveString(String),
            Regex(Regex),
        }

        let pattern = match pattern {
            Pattern::CaseSensitiveString(s) => CompiledPattern::CaseSensitiveString(s),
            Pattern::CaseInSensitiveString(s) => {
                CompiledPattern::CaseInSensitiveString(s.to_lowercase())
            }
            Pattern::Regex(r) => CompiledPattern::Regex(Regex::new(&r)?),
        };

        let mut results = vec![];
        let mut uniq_matches: HashMap<String, usize> = HashMap::new();

        screen.for_each_logical_line_in_stable_range(range, |sr, lines| {
            if let Some(limit) = limit {
                if results.len() == limit as usize {
                    return false;
                }
            }

            if lines.is_empty() {
                return true;
            }

            let haystack = if lines.len() == 1 {
                lines[0].as_str()
            } else {
                let mut s = String::new();
                for line in lines {
                    s.push_str(&line.as_str());
                }
                Cow::Owned(s)
            };
            let stable_idx = sr.start;

            if haystack.is_empty() {
                return true;
            }

            let haystack = match &pattern {
                CompiledPattern::CaseInSensitiveString(_) => Cow::Owned(haystack.to_lowercase()),
                _ => haystack,
            };
            let mut coords = None;

            match &pattern {
                CompiledPattern::CaseInSensitiveString(s)
                | CompiledPattern::CaseSensitiveString(s) => {
                    for (idx, s) in haystack.match_indices(s) {
                        found_match(
                            s,
                            idx,
                            lines,
                            stable_idx,
                            &mut uniq_matches,
                            &mut coords,
                            &mut results,
                        );
                    }
                }
                CompiledPattern::Regex(re) => {
                    for c in re.captures_iter(&haystack) {
                        for idx in (0..c.len()).rev() {
                            if let Some(m) = c.get(idx) {
                                found_match(
                                    m.as_str(),
                                    m.start(),
                                    lines,
                                    stable_idx,
                                    &mut uniq_matches,
                                    &mut coords,
                                    &mut results,
                                );
                                break;
                            }
                        }
                    }
                }
            }

            true
        });

        #[derive(Copy, Clone, Debug)]
        struct Coord {
            byte_idx: usize,
            grapheme_idx: usize,
            stable_row: StableRowIndex,
        }

        fn found_match(
            text: &str,
            byte_idx: usize,
            lines: &[&Line],
            stable_idx: StableRowIndex,
            uniq_matches: &mut HashMap<String, usize>,
            coords: &mut Option<Vec<Coord>>,
            results: &mut Vec<SearchResult>,
        ) {
            if coords.is_none() {
                coords.replace(make_coords(lines, stable_idx));
            }
            let coords = coords.as_ref().unwrap();

            let match_id = match uniq_matches.get(text).copied() {
                Some(id) => id,
                None => {
                    let id = uniq_matches.len();
                    uniq_matches.insert(text.to_owned(), id);
                    id
                }
            };
            let (start_x, start_y) = haystack_idx_to_coord(byte_idx, coords);
            let (end_x, end_y) = haystack_idx_to_coord(byte_idx + text.len(), coords);
            results.push(SearchResult {
                start_x,
                start_y,
                end_x,
                end_y,
                match_id,
            });
        }

        fn make_coords(lines: &[&Line], stable_row: StableRowIndex) -> Vec<Coord> {
            let mut byte_idx = 0;
            let mut coords = vec![];

            for (row_idx, line) in lines.iter().enumerate() {
                for cell in line.visible_cells() {
                    coords.push(Coord {
                        byte_idx,
                        grapheme_idx: cell.cell_index(),
                        stable_row: stable_row + row_idx as StableRowIndex,
                    });
                    byte_idx += cell.str().len();
                }
            }

            coords
        }

        fn haystack_idx_to_coord(idx: usize, coords: &[Coord]) -> (usize, StableRowIndex) {
            let c = coords
                .binary_search_by(|ele| ele.byte_idx.cmp(&idx))
                .or_else(|i| -> Result<usize, usize> { Ok(i) })
                .unwrap();
            let coord = coords.get(c).copied().unwrap_or_else(|| {
                let last = coords.last().unwrap();
                Coord {
                    byte_idx: idx,
                    grapheme_idx: last.grapheme_idx + 1,
                    stable_row: last.stable_row,
                }
            });
            (coord.grapheme_idx, coord.stable_row)
        }

        Ok(results)
    }
    fn get_semantic_zones(&self) -> anyhow::Result<Vec<SemanticZone>> {
        Ok(vec![])
    }
    fn is_mouse_grabbed(&self) -> bool {
        self.terminal.lock().is_mouse_grabbed()
    }
    fn is_alt_screen_active(&self) -> bool {
        self.terminal.lock().is_alt_screen_active()
    }
    fn set_clipboard(&self, clipboard: &Arc<dyn Clipboard>) {
        self.terminal.lock().set_clipboard(clipboard)
    }
    fn set_download_handler(&self, handler: &Arc<dyn DownloadHandler>) {
        self.terminal.lock().set_download_handler(handler)
    }
    fn set_config(&self, config: Arc<dyn TerminalConfiguration>) {
        self.terminal.lock().set_config(Arc::clone(&config));
        *self.config.lock() = Some(config);
    }
    fn get_config(&self) -> Option<Arc<dyn TerminalConfiguration>> {
        self.config.lock().clone()
    }
    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
        self.terminal.lock().get_current_dir().cloned()
    }
    fn get_foreground_process_name(&self, _policy: CachePolicy) -> Option<String> {
        Some(self.session_id.clone())
    }
}

fn decode_input_payload_chunks(pending: &mut Vec<u8>, payload: &[u8]) -> Vec<String> {
    if !payload.is_empty() {
        pending.extend_from_slice(payload);
    }
    let mut chunks = Vec::<String>::new();
    loop {
        if pending.is_empty() {
            break;
        }
        match std::str::from_utf8(pending) {
            Ok(text) => {
                if !text.is_empty() {
                    chunks.push(text.to_string());
                }
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to > 0 {
                    let valid = String::from_utf8_lossy(&pending[..valid_up_to]).to_string();
                    if !valid.is_empty() {
                        chunks.push(valid);
                    }
                    pending.drain(..valid_up_to);
                    continue;
                }
                match err.error_len() {
                    None => break,
                    Some(invalid_len) => {
                        let lossy = String::from_utf8_lossy(&pending[..invalid_len]).to_string();
                        if !lossy.is_empty() {
                            chunks.push(lossy);
                        }
                        pending.drain(..invalid_len);
                    }
                }
            }
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::{Duration, Instant};

    use engine_term::{KeyCode, KeyModifiers, StableRowIndex};
    use host_runtime::pane::Pattern;
    use portable_pty::CommandBuilder;

    use super::super::session_engine::{
        LayoutNodeId, RuntimeId, SessionCoreState, SessionEngineShared, SessionLayoutSnapshot,
        TerminalInstanceId,
    };
    use super::super::{
        acquire_legacy_host_mux_test_lock, build_initial_host_mux, shutdown_host_mux,
    };
    use super::{
        decode_input_payload_chunks, looks_like_chatminal_internal_title, Action,
        ChatminalSessionPane, HostTerminal, TerminalSize,
    };

    struct HostMuxTestGuard {
        _guard: MutexGuard<'static, ()>,
    }

    impl Drop for HostMuxTestGuard {
        fn drop(&mut self) {
            shutdown_host_mux();
        }
    }

    fn init_host_mux_test() -> HostMuxTestGuard {
        let guard = acquire_legacy_host_mux_test_lock();
        shutdown_host_mux();
        let config = config::current_config_handle();
        build_initial_host_mux(&config, None).expect("init host mux");
        HostMuxTestGuard { _guard: guard }
    }

    fn shell_command(script: &str) -> CommandBuilder {
        let mut command = CommandBuilder::new("/bin/sh");
        command.arg("-lc");
        command.arg(script);
        command
    }

    fn rendered_text(pane: &ChatminalSessionPane) -> String {
        let terminal = pane.terminal.lock();
        terminal
            .screen()
            .lines_in_phys_range(0..terminal.screen().scrollback_rows())
            .iter()
            .map(|line| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn rendered_lines(pane: &ChatminalSessionPane) -> Vec<String> {
        let terminal = pane.terminal.lock();
        terminal
            .screen()
            .lines_in_phys_range(0..terminal.screen().scrollback_rows())
            .iter()
            .map(|line| line.as_str().to_string())
            .collect()
    }

    #[test]
    fn decode_input_payload_chunks_keeps_partial_utf8_until_complete() {
        let mut pending = Vec::new();
        assert!(decode_input_payload_chunks(&mut pending, &[0xE1, 0xBB]).is_empty());
        let chunks = decode_input_payload_chunks(&mut pending, &[0x8B, b'a']);
        assert_eq!(chunks, vec!["ịa".to_string()]);
    }

    #[test]
    fn internal_title_filter_detects_chatminal_marker_titles() {
        assert!(looks_like_chatminal_internal_title(
            "Chatminal 20260327-085528-1b1caf5365"
        ));
        assert!(looks_like_chatminal_internal_title(
            ">|Chatminal 20260327-085528-1b1caf5365"
        ));
        assert!(looks_like_chatminal_internal_title(
            "20260327-085528-1b1caf5365"
        ));
        assert!(!looks_like_chatminal_internal_title("claude"));
        assert!(!looks_like_chatminal_internal_title("npm run dev"));
    }

    #[test]
    fn marker_title_falls_back_to_chatminal_title() {
        let title = "Chatminal 20260327-085528-1b1caf5365";
        let user_facing = if looks_like_chatminal_internal_title(title) {
            "Chatminal".to_string()
        } else {
            title.to_string()
        };

        assert_eq!(user_facing, "Chatminal");
    }

    #[test]
    fn reset_display_state_replays_existing_output_immediately() {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(7);
        let terminal_instance_id = TerminalInstanceId::new(11);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command("sleep 1"),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                Some("restored-line-1\nrestored-line-2\n".to_string()),
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        let initial = rendered_text(&pane);
        assert!(initial.contains("restored-line-1"));
        assert!(initial.contains("restored-line-2"));

        pane.perform_actions(vec![Action::Esc(termwiz::escape::Esc::Code(
            termwiz::escape::EscCode::FullReset,
        ))]);
        let cleared = rendered_text(&pane);
        assert!(!cleared.contains("restored-line-1"));

        pane.reset_display_state();
        let restored = rendered_text(&pane);
        assert!(restored.contains("restored-line-1"));
        assert!(restored.contains("restored-line-2"));
    }

    #[test]
    fn replay_seed_stays_at_top_of_viewport_for_short_history() {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(17);
        let terminal_instance_id = TerminalInstanceId::new(18);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command("sleep 1"),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                Some("restored-line-1\nrestored-line-2\nprompt % ".to_string()),
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        let lines = rendered_lines(&pane);
        let first_non_empty = lines.iter().position(|line| !line.is_empty());
        assert_eq!(
            first_non_empty,
            Some(0),
            "replay seed unexpectedly sank in viewport: {:?}",
            lines
        );
    }

    #[test]
    fn search_finds_matches_in_replayed_runtime_output() {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(1);
        let terminal_instance_id = TerminalInstanceId::new(2);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command("sleep 1"),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                Some("https://example.com\n/Users/khoa2807/test.txt\n12345678\n".to_string()),
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        let dims = pane.get_dimensions();
        let results = smol::block_on(pane.search(
            Pattern::Regex(
                "(?m)(https?://\\S+|(?:[.\\w\\-@~]+)?(?:/+[.\\w\\-@]+)+|[0-9]{4,})".into(),
            ),
            dims.scrollback_top..dims.physical_top + dims.viewport_rows as StableRowIndex,
            None,
        ))
        .expect("search quick select patterns");

        assert!(
            !results.is_empty(),
            "expected search matches in replayed output"
        );
    }

    #[test]
    fn pane_key_input_is_forwarded_to_runtime() {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(21);
        let terminal_instance_id = TerminalInstanceId::new(22);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command("stty raw -echo; dd bs=1 count=1 2>/dev/null"),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                None,
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        pane.key_down(KeyCode::Char('Z'), KeyModifiers::default())
            .expect("forward key input");

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            if shared
                .replay_output(terminal_instance_id)
                .as_deref()
                .is_some_and(|output| output.contains('Z'))
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        panic!("expected pane key input to reach runtime output");
    }

    #[test]
    fn pane_direct_writer_input_is_forwarded_to_runtime() {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(31);
        let terminal_instance_id = TerminalInstanceId::new(32);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command("stty raw -echo; dd bs=1 count=1 2>/dev/null"),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                None,
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        pane.writer()
            .write_all(b"Z")
            .expect("forward direct writer input");

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            if shared
                .replay_output(terminal_instance_id)
                .as_deref()
                .is_some_and(|output| output.contains('Z'))
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        panic!("expected pane direct writer input to reach runtime output");
    }

    fn assert_key_input_hex(key: KeyCode, expected_hex: &str) {
        let _guard = init_host_mux_test();
        let runtime_id = RuntimeId::new(41);
        let terminal_instance_id = TerminalInstanceId::new(42);
        let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
        core_state.lock().unwrap().sync_runtime_layout(
            "session-a",
            runtime_id,
            &SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                terminal_instance_id,
                None,
            ),
        );
        let shared = Arc::new(SessionEngineShared::new(Arc::clone(&core_state)));
        let (events_tx, _events_rx) = mpsc::sync_channel(32);

        shared
            .leaf_runtimes()
            .spawn_for_runtime(
                &core_state,
                "session-a",
                1,
                runtime_id,
                terminal_instance_id,
                shell_command(
                    "printf 'ready\\n'; stty raw -echo; printf 'input-ready\\n'; dd bs=1 count=1 2>/dev/null | od -An -t x1",
                ),
                chatminal_terminal_core::TerminalSize {
                    rows: 12,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                    dpi: 96,
                },
                None,
                events_tx,
            )
            .expect("spawn runtime");

        let pane = ChatminalSessionPane::new(
            Arc::clone(&shared),
            "session-a".to_string(),
            runtime_id,
            terminal_instance_id,
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config::current_config_handle(),
        )
        .expect("create pane");

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            if shared
                .replay_output(terminal_instance_id)
                .as_deref()
                .is_some_and(|output| output.contains("input-ready"))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        pane.key_down(key, KeyModifiers::default())
            .expect("forward special key input");

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            if shared
                .replay_output(terminal_instance_id)
                .as_deref()
                .is_some_and(|output| output.contains(expected_hex))
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let observed = shared
            .replay_output(terminal_instance_id)
            .unwrap_or_default();
        panic!(
            "expected hex {} in runtime output, got {:?}",
            expected_hex, observed
        );
    }

    #[test]
    fn pane_space_key_is_forwarded_to_runtime() {
        assert_key_input_hex(KeyCode::Char(' '), "20");
    }

    #[test]
    fn pane_enter_key_is_forwarded_to_runtime() {
        assert_key_input_hex(KeyCode::Enter, "0d");
    }

    #[test]
    fn pane_backspace_key_is_forwarded_to_runtime() {
        assert_key_input_hex(KeyCode::Backspace, "7f");
    }
}
