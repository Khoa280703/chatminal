use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use chatminal_protocol::{CreateSessionResponse, Request, Response, SessionStatus};
use eframe::egui::{self, Event as EguiEvent};
use egui::output::IMEOutput;

use crate::input::{ImeDeduperKind, TerminalInputEvent, TerminalInputSource};
use crate::terminal_pane_adapter::TerminalPaneAdapter;
use crate::terminal_session_commands::activate_session_with_snapshot;

use super::input_mapper::{
    map_egui_key_event_to_pty_input, map_egui_key_event_to_pty_input_legacy,
    map_egui_printable_key_event_to_text_input,
};
use super::reducer::compute_terminal_grid;
use super::{CHAR_HEIGHT_PX, CHAR_WIDTH_PX, ChatminalWindowApp};

const RESIZE_DEBOUNCE_MS: u64 = 120;
const RESIZE_TIMEOUT_MS: u64 = 150;
const CREATE_SESSION_TIMEOUT_SECS: u64 = 20;

impl ChatminalWindowApp {
    pub(super) fn activate_session(&mut self, session_id: &str) {
        match activate_session_with_snapshot(
            &self.client,
            &mut self.pane_registry,
            &mut self.state.adapter,
            session_id,
            self.pane_cols,
            self.pane_rows,
            self.preview_lines,
        ) {
            Ok(_) => {
                self.selected_session_id = Some(session_id.to_string());
                self.state.workspace.active_session_id = Some(session_id.to_string());
                for session in &mut self.state.workspace.sessions {
                    if session.session_id == session_id {
                        session.status = SessionStatus::Running;
                    }
                }
                self.render_dirty = true;
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(format!("activate session failed: {err}"));
            }
        }
    }

    pub(super) fn create_session(&mut self) {
        if self.pending_session_create.is_some() {
            self.last_error = Some("create session đang chạy".to_string());
            return;
        }
        let name = self.new_session_name.trim().to_string();
        if name.is_empty() {
            self.last_error = Some("session name is required".to_string());
            return;
        }

        let endpoint = self.endpoint.clone();
        let cols = self.pane_cols;
        let rows = self.pane_rows;
        let request_name = name.clone();
        let (tx, rx) = mpsc::sync_channel::<Result<CreateSessionResponse, String>>(1);
        self.pending_session_create = Some(super::PendingSessionCreate {
            started_at: Instant::now(),
            rx,
        });
        self.last_error = None;

        thread::spawn(move || {
            let response = request_session_create(&endpoint, request_name, cols, rows);
            let _ = tx.send(response);
        });
    }

    pub(super) fn poll_create_session_result(&mut self) -> bool {
        let Some(pending) = self.pending_session_create.take() else {
            return false;
        };
        match pending.rx.try_recv() {
            Ok(Ok(value)) => {
                self.new_session_name.clear();
                self.last_error = None;
                self.reload_workspace();
                if error_requires_main_client_reconnect(self.last_error.as_deref()) {
                    let _ = self.reconnect_main_client();
                    self.reload_workspace();
                }
                self.activate_session(&value.session_id);
                if error_requires_main_client_reconnect(self.last_error.as_deref()) {
                    let _ = self.reconnect_main_client();
                    self.activate_session(&value.session_id);
                }
                true
            }
            Ok(Err(err)) => {
                self.last_error = Some(if is_create_request_timeout_error(&err) {
                    format!(
                        "create session timeout sau {}s (daemon đang bận); kiểm tra sidebar và bấm Reload trước khi tạo lại",
                        CREATE_SESSION_TIMEOUT_SECS
                    )
                } else {
                    format!("create session failed: {err}")
                });
                true
            }
            Err(TryRecvError::Empty) => {
                self.pending_session_create = Some(pending);
                false
            }
            Err(TryRecvError::Disconnected) => {
                self.last_error = Some("create session worker disconnected".to_string());
                true
            }
        }
    }

    pub(super) fn handle_terminal_input_events(&mut self, ctx: &egui::Context) -> bool {
        if self.input_pipeline_mode == crate::config::InputPipelineMode::Legacy {
            return self.handle_terminal_input_events_legacy(ctx);
        }

        let events = ctx.input(|input| input.events.clone());
        self.ime_commit_deduper.start_frame();
        let has_ime_commit_event = events.iter().any(|event| {
            matches!(
                event,
                EguiEvent::Ime(egui::ImeEvent::Commit(text)) if !text.is_empty()
            )
        });
        let has_ime_activity = events
            .iter()
            .any(|event| matches!(event, EguiEvent::Ime(_)));
        let has_text_event = events.iter().any(|event| match event {
            EguiEvent::Text(text) | EguiEvent::Paste(text) => !text.is_empty(),
            EguiEvent::Ime(egui::ImeEvent::Commit(text)) => !text.is_empty(),
            _ => false,
        });
        let allow_macos_printable_key_fallback =
            cfg!(target_os = "macos") && !has_text_event && !has_ime_activity;
        let widget_has_focus = ctx.memory(|memory| memory.focused().is_some());
        let allow_blur_ime_commit = !self.terminal_has_focus
            && self.ime_blur_flush_armed
            && has_ime_commit_event
            && !widget_has_focus;
        if !self.terminal_has_focus && !allow_blur_ime_commit {
            self.ime_composition_state.on_focus_lost();
            if widget_has_focus {
                self.ime_blur_flush_armed = false;
            }
            return false;
        }

        let mut changed = false;
        let mut buffered_payload = String::new();
        let mut deduper_marks: Vec<(ImeDeduperKind, String)> = Vec::new();
        for event in events {
            match event {
                EguiEvent::Text(text) => {
                    if !self.terminal_has_focus {
                        continue;
                    }
                    let event = TerminalInputEvent::TextCommit {
                        text,
                        source: TerminalInputSource::Egui,
                    };
                    if let Some(payload) = event_text_payload(event) {
                        if self
                            .ime_commit_deduper
                            .should_skip(ImeDeduperKind::TextCommit, &payload)
                        {
                            continue;
                        }
                        buffered_payload.push_str(&payload);
                        deduper_marks.push((ImeDeduperKind::TextCommit, payload));
                    }
                }
                EguiEvent::Paste(text) => {
                    if !self.terminal_has_focus {
                        continue;
                    }
                    let event = TerminalInputEvent::Paste {
                        text,
                        source: TerminalInputSource::Egui,
                    };
                    if let Some(payload) = event_text_payload(event) {
                        buffered_payload.push_str(&payload);
                    }
                }
                EguiEvent::Copy => {
                    if !self.terminal_has_focus || cfg!(target_os = "macos") {
                        continue;
                    }
                    buffered_payload.push('\u{3}');
                }
                EguiEvent::Cut => {
                    if !self.terminal_has_focus || cfg!(target_os = "macos") {
                        continue;
                    }
                    buffered_payload.push('\u{18}');
                }
                EguiEvent::Ime(egui::ImeEvent::Commit(text)) => {
                    if !self.terminal_has_focus && !allow_blur_ime_commit {
                        continue;
                    }
                    self.ime_composition_state.mark_commit(&text);
                    let event = TerminalInputEvent::ImeCommit {
                        text,
                        source: TerminalInputSource::Egui,
                    };
                    if let Some(payload) = event_text_payload(event)
                        && !self
                            .ime_commit_deduper
                            .should_skip(ImeDeduperKind::ImeCommit, &payload)
                    {
                        buffered_payload.push_str(&payload);
                        deduper_marks.push((ImeDeduperKind::ImeCommit, payload));
                    }
                }
                EguiEvent::Ime(_) => {
                    self.ime_composition_state.mark_composing(None);
                }
                EguiEvent::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } => {
                    if !self.terminal_has_focus {
                        continue;
                    }
                    if !pressed {
                        continue;
                    }
                    if let Some(data) = map_egui_key_event_to_pty_input(key, modifiers, repeat) {
                        buffered_payload.push_str(&data);
                    } else if allow_macos_printable_key_fallback
                        && let Some(data) =
                            map_egui_printable_key_event_to_text_input(key, modifiers, repeat)
                    {
                        buffered_payload.push_str(&data);
                    }
                }
                _ => {}
            }
        }
        if !buffered_payload.is_empty() {
            let payload = std::mem::take(&mut buffered_payload);
            if self.send_terminal_payload(&payload) {
                for (kind, text) in deduper_marks.drain(..) {
                    self.ime_commit_deduper.mark_sent(kind, &text);
                }
                changed = true;
            } else {
                deduper_marks.clear();
            }
        }
        if allow_blur_ime_commit {
            self.ime_blur_flush_armed = false;
        }
        changed
    }

    fn handle_terminal_input_events_legacy(&mut self, ctx: &egui::Context) -> bool {
        if !self.terminal_has_focus {
            return false;
        }

        let events = ctx.input(|input| input.events.clone());
        let mut changed = false;
        let mut saw_text_like_event = false;
        let mut buffered_payload = String::new();
        for event in events {
            match event {
                EguiEvent::Text(text) | EguiEvent::Paste(text) => {
                    if text.is_empty() {
                        continue;
                    }
                    buffered_payload.push_str(&text);
                    saw_text_like_event = true;
                }
                EguiEvent::Copy => {
                    if cfg!(target_os = "macos") {
                        continue;
                    }
                    buffered_payload.push('\u{3}');
                    saw_text_like_event = true;
                }
                EguiEvent::Cut => {
                    if cfg!(target_os = "macos") {
                        continue;
                    }
                    buffered_payload.push('\u{18}');
                    saw_text_like_event = true;
                }
                EguiEvent::Ime(egui::ImeEvent::Commit(text)) => {
                    if text.is_empty() {
                        continue;
                    }
                    buffered_payload.push_str(&text);
                    saw_text_like_event = true;
                }
                EguiEvent::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } => {
                    if !pressed {
                        continue;
                    }
                    if let Some(data) =
                        map_egui_key_event_to_pty_input_legacy(key, modifiers, repeat)
                        && !(saw_text_like_event
                            && is_legacy_plain_text_key_payload(modifiers, &data))
                    {
                        buffered_payload.push_str(&data);
                    }
                }
                _ => {}
            }
        }
        if !buffered_payload.is_empty() && self.send_terminal_payload(&buffered_payload) {
            changed = true;
        }
        changed
    }

    pub(super) fn update_terminal_focus(
        &mut self,
        ctx: &egui::Context,
        terminal_rect: Option<egui::Rect>,
    ) {
        let pressed = ctx.input(|input| input.pointer.any_pressed());
        if !pressed {
            return;
        }
        let pointer_pos = ctx.input(|input| input.pointer.interact_pos());
        if let (Some(rect), Some(pos)) = (terminal_rect, pointer_pos) {
            let had_focus = self.terminal_has_focus;
            self.terminal_has_focus = rect.contains(pos);
            if had_focus && !self.terminal_has_focus {
                self.ime_composition_state.on_focus_lost();
                self.ime_blur_flush_armed = true;
            } else if self.terminal_has_focus {
                self.ime_blur_flush_armed = false;
                self.ime_commit_deduper.clear();
            }
        }
    }

    pub(super) fn sync_terminal_ime(&self, ctx: &egui::Context, terminal_rect: Option<egui::Rect>) {
        // On macOS, exposing the terminal as an IME target causes subsequent ASCII keystrokes
        // to stay in preedit instead of reaching the PTY as direct text commits.
        if cfg!(target_os = "macos") {
            return;
        }
        if !self.terminal_has_focus {
            return;
        }
        let _ime_snapshot = self.ime_composition_state.snapshot();
        let Some(rect) = terminal_rect else {
            return;
        };
        let cursor_rect = egui::Rect::from_min_size(
            rect.left_bottom() - egui::vec2(0.0, CHAR_HEIGHT_PX),
            egui::vec2(1.0, CHAR_HEIGHT_PX),
        );
        ctx.output_mut(|output| {
            output.ime = Some(IMEOutput { rect, cursor_rect });
        });
    }

    pub(super) fn sync_terminal_size(&mut self, available: egui::Vec2) {
        let (cols, rows) = compute_terminal_grid(
            available.x,
            available.y,
            CHAR_WIDTH_PX,
            CHAR_HEIGHT_PX,
            20,
            400,
            5,
            200,
        );
        if cols == self.pane_cols && rows == self.pane_rows {
            return;
        }
        self.pane_cols = cols;
        self.pane_rows = rows;
        self.pending_resize = Some((cols, rows));
    }

    pub(super) fn flush_pending_resize(&mut self) {
        let Some((cols, rows)) = self.pending_resize else {
            return;
        };
        if self.last_resize_request_at.elapsed() < Duration::from_millis(RESIZE_DEBOUNCE_MS) {
            return;
        }
        let Some(session_id) = self.selected_session_id.clone() else {
            self.pending_resize = None;
            return;
        };

        self.last_resize_request_at = std::time::Instant::now();
        let response = self.client.request(
            Request::SessionResize {
                session_id: session_id.clone(),
                cols,
                rows,
            },
            Duration::from_millis(RESIZE_TIMEOUT_MS),
        );
        match response {
            Ok(Response::Empty) => {
                let pane_id = self.pane_registry.ensure_pane_for_session(&session_id);
                self.state
                    .adapter
                    .on_session_resize(&session_id, &pane_id, cols, rows);
                self.pending_resize = None;
                self.render_dirty = true;
                self.last_error = None;
            }
            Ok(other) => self.last_error = Some(format!("unexpected resize response: {:?}", other)),
            Err(err) => self.last_error = Some(format!("resize session failed: {err}")),
        }
    }

    pub(super) fn poll_input_worker_results(&mut self) -> bool {
        let mut changed = false;
        while let Some(result) = self.input_worker.try_recv() {
            if let Some(err) = result.error {
                self.last_error = Some(format!("send input failed: {err}"));
                continue;
            }

            let pane_id = self
                .pane_registry
                .ensure_pane_for_session(&result.session_id);
            self.state
                .adapter
                .on_session_input(&result.session_id, &pane_id, result.bytes);
            self.render_dirty = true;
            self.last_error = None;
            changed = true;
        }
        changed
    }

    fn send_terminal_payload(&mut self, data: &str) -> bool {
        let Some(session_id) = self
            .selected_session_id
            .clone()
            .or_else(|| self.state.workspace.active_session_id.clone())
        else {
            self.last_error = Some("no session selected".to_string());
            return false;
        };

        let disconnected = self
            .state
            .workspace
            .sessions
            .iter()
            .find(|value| value.session_id == session_id)
            .is_some_and(|value| value.status == SessionStatus::Disconnected);
        if disconnected {
            self.activate_session(&session_id);
            let still_disconnected = self
                .state
                .workspace
                .sessions
                .iter()
                .find(|value| value.session_id == session_id)
                .is_some_and(|value| value.status == SessionStatus::Disconnected);
            if still_disconnected {
                return false;
            }
        }

        match self
            .input_worker
            .enqueue(session_id.clone(), data.to_string())
        {
            Ok(()) => {
                self.last_error = None;
                self.render_dirty = true;
                true
            }
            Err(err) => {
                self.last_error = Some(format!("queue input failed: {err}"));
                false
            }
        }
    }

    fn reconnect_main_client(&mut self) -> Result<(), String> {
        let client = crate::ipc::ChatminalClient::connect(&self.endpoint)?;
        self.client = client;
        Ok(())
    }
}

fn event_text_payload(event: TerminalInputEvent) -> Option<String> {
    match event {
        TerminalInputEvent::TextCommit { text, .. }
        | TerminalInputEvent::Paste { text, .. }
        | TerminalInputEvent::ImeCommit { text, .. } => {
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        TerminalInputEvent::KeyChord(_) => None,
    }
}

fn request_session_create(
    endpoint: &str,
    name: String,
    cols: usize,
    rows: usize,
) -> Result<CreateSessionResponse, String> {
    let client = crate::ipc::ChatminalClient::connect(endpoint)?;
    let response = client.request(
        Request::SessionCreate {
            name: Some(name),
            cols,
            rows,
            cwd: None,
            persist_history: Some(false),
        },
        Duration::from_secs(CREATE_SESSION_TIMEOUT_SECS),
    );
    match response {
        Ok(Response::SessionCreate(value)) => Ok(value),
        Ok(other) => Err(format!("unexpected create response: {:?}", other)),
        Err(err) => Err(err),
    }
}

fn is_create_request_timeout_error(err: &str) -> bool {
    err.starts_with("request timeout for id '")
        || err.starts_with("request timeout while waiting writer lock for id '")
        || err.starts_with("request timeout while writing id '")
        || err.starts_with("request timeout while flushing id '")
}

fn error_requires_main_client_reconnect(error: Option<&str>) -> bool {
    let Some(error) = error else {
        return false;
    };
    error.contains("reconnect is required")
        || error.contains("daemon stream disconnected")
        || error.contains("writer lock poisoned")
}

fn is_legacy_plain_text_key_payload(modifiers: egui::Modifiers, data: &str) -> bool {
    if modifiers.ctrl || modifiers.alt || modifiers.command || modifiers.mac_cmd {
        return false;
    }

    let mut chars = data.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    if chars.next().is_some() {
        return false;
    }
    ch >= ' ' && ch != '\u{7f}'
}

pub(super) trait SessionStatusLabel {
    fn as_ref(&self) -> &'static str;
}

impl SessionStatusLabel for SessionStatus {
    fn as_ref(&self) -> &'static str {
        match self {
            SessionStatus::Running => "running",
            SessionStatus::Disconnected => "disconnected",
        }
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::Modifiers;

    use super::is_legacy_plain_text_key_payload;

    #[test]
    fn legacy_plain_text_payload_detects_ascii_char() {
        assert!(is_legacy_plain_text_key_payload(Modifiers::NONE, "a"));
        assert!(is_legacy_plain_text_key_payload(Modifiers::NONE, " "));
    }

    #[test]
    fn legacy_plain_text_payload_keeps_control_and_modified_keys() {
        assert!(!is_legacy_plain_text_key_payload(Modifiers::NONE, "\u{3}"));
        assert!(!is_legacy_plain_text_key_payload(
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            "a"
        ));
        assert!(!is_legacy_plain_text_key_payload(
            Modifiers::NONE,
            "\u{1b}[D"
        ));
    }
}
