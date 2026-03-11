#![allow(dead_code)]

use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::Write;
use std::ops::Range;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chatminal_session_runtime::{LeafId, SessionEngineShared, SessionRuntimeEvent, SurfaceId};
use chatminal_terminal_core::TerminalSize as CoreTerminalSize;
use config::TermConfig;
use config::keyassignment::ScrollbackEraseMode;
use engine_dynamic::Value;
use engine_term::color::ColorPalette;
use engine_term::{
    Clipboard, DownloadHandler, KeyCode, KeyModifiers, MouseEvent, Progress, SemanticZone,
    StableRowIndex, Terminal, TerminalConfiguration, TerminalSize,
};
use mux::domain::DomainId;
use mux::pane::{
    CachePolicy, CloseReason, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, Pattern,
    SearchResult, WithPaneLines, alloc_pane_id,
};
use mux::renderable::{
    RenderableDimensions, StableCursorPosition, terminal_for_each_logical_line_in_stable_range_mut,
    terminal_get_cursor_position, terminal_get_dimensions, terminal_get_dirty_lines,
    terminal_get_lines, terminal_with_lines_mut,
};
use mux::{Mux, MuxNotification};
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rangeset::RangeSet;
use termwiz::escape::Action;
use termwiz::input::KeyboardEncoding;
use termwiz::surface::{Line, SequenceNo};
use url::Url;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Clone)]
struct SessionPaneWriter {
    inner: Arc<Mutex<SessionPaneWriterState>>,
}

struct SessionPaneWriterState {
    shared: Arc<SessionEngineShared>,
    leaf_id: LeafId,
    pending_input_bytes: Vec<u8>,
}

impl SessionPaneWriter {
    fn new(shared: Arc<SessionEngineShared>, leaf_id: LeafId) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionPaneWriterState {
                shared,
                leaf_id,
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
                .leaf_runtimes()
                .runtime(state.leaf_id)
                .ok_or_else(|| std::io::Error::other("session runtime missing"))?
                .write_input(&chunk)
                .map_err(std::io::Error::other)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct ChatminalSessionPane {
    pane_id: PaneId,
    domain_id: DomainId,
    session_id: String,
    surface_id: SurfaceId,
    leaf_id: LeafId,
    shared: Arc<SessionEngineShared>,
    terminal: Mutex<Terminal>,
    writer: Mutex<SessionPaneWriter>,
    dead: Mutex<bool>,
    config: Mutex<Option<Arc<dyn TerminalConfiguration>>>,
}

impl ChatminalSessionPane {
    pub fn new(
        shared: Arc<SessionEngineShared>,
        domain_id: DomainId,
        session_id: String,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        size: TerminalSize,
    ) -> anyhow::Result<Arc<Self>> {
        let writer = SessionPaneWriter::new(Arc::clone(&shared), leaf_id);
        let pane = Arc::new(Self {
            pane_id: pane_id_for_leaf(leaf_id),
            domain_id,
            session_id,
            surface_id,
            leaf_id,
            shared,
            terminal: Mutex::new(Terminal::new(
                size,
                Arc::new(TermConfig::new()),
                "Chatminal",
                config::engine_version(),
                Box::new(writer.clone()),
            )),
            writer: Mutex::new(writer),
            dead: Mutex::new(false),
            config: Mutex::new(None),
        });

        pane.seed_replay_output();
        pane.spawn_event_loop()?;
        Ok(pane)
    }

    fn seed_replay_output(&self) {
        if let Some(replay) = self.shared.leaf_runtimes().replay_output(self.leaf_id) {
            self.apply_output(&replay);
        }
    }

    fn spawn_event_loop(self: &Arc<Self>) -> anyhow::Result<()> {
        let subscription = self.shared.event_hub().subscribe();
        let pane = Arc::downgrade(self);
        thread::spawn(move || {
            loop {
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
            }
        });
        Ok(())
    }

    fn handle_event(&self, event: SessionRuntimeEvent) {
        match event {
            SessionRuntimeEvent::LeafOutput {
                surface_id,
                leaf_id,
                chunk,
                ..
            } if surface_id == self.surface_id && leaf_id == self.leaf_id => {
                self.apply_output(&chunk);
                Mux::get().notify(MuxNotification::PaneOutput(self.pane_id));
            }
            SessionRuntimeEvent::LeafExited {
                surface_id,
                leaf_id,
                ..
            } if surface_id == self.surface_id && leaf_id == self.leaf_id => {
                *self.dead.lock() = true;
                Mux::get().notify(MuxNotification::PaneOutput(self.pane_id));
            }
            SessionRuntimeEvent::LeafError {
                surface_id,
                leaf_id,
                message,
                ..
            } if surface_id == self.surface_id && leaf_id == self.leaf_id => {
                self.apply_output(&format!("\r\n[chatminal session error] {}\r\n", message));
                Mux::get().notify(MuxNotification::PaneOutput(self.pane_id));
            }
            SessionRuntimeEvent::SurfaceClosed { surface_id, .. }
                if surface_id == self.surface_id =>
            {
                *self.dead.lock() = true;
                Mux::get().notify(MuxNotification::PaneOutput(self.pane_id));
            }
            _ => {}
        }
    }

    fn apply_output(&self, message: &str) {
        let mut parser = termwiz::escape::parser::Parser::new();
        let mut actions = vec![Action::CSI(termwiz::escape::csi::CSI::Sgr(
            termwiz::escape::csi::Sgr::Reset,
        ))];
        parser.parse(message.as_bytes(), |action| actions.push(action));
        self.terminal.lock().perform_actions(actions);
    }
}

fn pane_id_for_leaf(leaf_id: LeafId) -> PaneId {
    usize::try_from(leaf_id.as_u64()).unwrap_or_else(|_| alloc_pane_id())
}

#[async_trait::async_trait(?Send)]
impl Pane for ChatminalSessionPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }
    fn get_cursor_position(&self) -> StableCursorPosition {
        terminal_get_cursor_position(&mut self.terminal.lock())
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
            Value::String("chatminal_surface_id".into()),
            Value::U64(self.surface_id.as_u64()),
        );
        map.insert(
            Value::String("chatminal_leaf_id".into()),
            Value::U64(self.leaf_id.as_u64()),
        );
        Value::Object(map.into())
    }
    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        terminal_get_dirty_lines(&mut self.terminal.lock(), lines, seqno)
    }
    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        terminal_get_lines(&mut self.terminal.lock(), lines)
    }
    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        terminal_with_lines_mut(&mut self.terminal.lock(), lines, with_lines)
    }
    fn for_each_logical_line_in_stable_range_mut(
        &self,
        lines: Range<StableRowIndex>,
        for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        terminal_for_each_logical_line_in_stable_range_mut(
            &mut self.terminal.lock(),
            lines,
            for_line,
        )
    }
    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        mux::pane::impl_get_logical_lines_via_get_lines(self, lines)
    }
    fn get_dimensions(&self) -> RenderableDimensions {
        terminal_get_dimensions(&mut self.terminal.lock())
    }
    fn get_title(&self) -> String {
        let title = self.terminal.lock().get_title().to_string();
        if title.is_empty() || title == "Chatminal" {
            self.session_id.clone()
        } else {
            title
        }
    }
    fn get_progress(&self) -> Progress {
        self.terminal.lock().get_progress()
    }
    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        self.terminal.lock().send_paste(text)
    }
    fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
        Ok(None)
    }
    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        MutexGuard::map(self.writer.lock(), |writer| {
            let w: &mut dyn std::io::Write = writer;
            w
        })
    }
    fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
        self.shared
            .leaf_runtimes()
            .runtime(self.leaf_id)
            .ok_or_else(|| anyhow::anyhow!("session runtime missing"))?
            .resize(CoreTerminalSize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
                dpi: size.dpi,
            })
            .map_err(anyhow::Error::msg)?;
        self.terminal.lock().resize(size);
        Ok(())
    }
    fn key_down(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        Mux::get().record_input_for_current_identity();
        self.terminal.lock().key_down(key, mods)
    }
    fn key_up(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        Mux::get().record_input_for_current_identity();
        self.terminal.lock().key_up(key, mods)
    }
    fn mouse_event(&self, event: MouseEvent) -> anyhow::Result<()> {
        Mux::get().record_input_for_current_identity();
        self.terminal.lock().mouse_event(event)
    }
    fn perform_actions(&self, actions: Vec<Action>) {
        self.terminal.lock().perform_actions(actions)
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
    fn domain_id(&self) -> DomainId {
        self.domain_id
    }
    fn get_keyboard_encoding(&self) -> KeyboardEncoding {
        KeyboardEncoding::Xterm
    }
    fn copy_user_vars(&self) -> HashMap<String, String> {
        self.terminal.lock().user_vars().clone()
    }
    fn erase_scrollback(&self, erase_mode: ScrollbackEraseMode) {
        match erase_mode {
            ScrollbackEraseMode::ScrollbackOnly => self.terminal.lock().erase_scrollback(),
            ScrollbackEraseMode::ScrollbackAndViewport => {
                self.terminal.lock().erase_scrollback_and_viewport()
            }
        }
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
        _pattern: Pattern,
        _range: Range<StableRowIndex>,
        _limit: Option<u32>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![])
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
    use super::decode_input_payload_chunks;

    #[test]
    fn decode_input_payload_chunks_keeps_partial_utf8_until_complete() {
        let mut pending = Vec::new();
        assert!(decode_input_payload_chunks(&mut pending, &[0xE1, 0xBB]).is_empty());
        let chunks = decode_input_payload_chunks(&mut pending, &[0x8B, b'a']);
        assert_eq!(chunks, vec!["ịa".to_string()]);
    }
}
