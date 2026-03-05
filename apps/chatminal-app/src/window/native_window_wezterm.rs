use std::time::{Duration, Instant};

use eframe::{NativeOptions, egui};
use egui::{RichText, TextStyle, ViewportBuilder};

use crate::config::{InputPipelineMode, parse_usize};
use crate::input::{ImeCommitDeduper, ImeCompositionState};
use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::SessionPaneRegistry;
use crate::terminal_workspace_binding_runtime::WorkspaceBindingState;

#[path = "native_window_wezterm_actions.rs"]
mod actions;
#[path = "native_window_wezterm_controller.rs"]
mod controller;
#[path = "native_window_wezterm_input_mapper.rs"]
mod input_mapper;
#[path = "native_window_wezterm_input_worker.rs"]
mod input_worker;
#[path = "native_window_wezterm_reducer.rs"]
mod reducer;

use actions::SessionStatusLabel;
use input_worker::TerminalInputWorker;

pub(super) const CHAR_WIDTH_PX: f32 = 8.0;
pub(super) const CHAR_HEIGHT_PX: f32 = 18.0;
pub(super) const UI_ACTIVE_REPAINT_MS: u64 = 16;
pub(super) const UI_IDLE_REPAINT_MS: u64 = 33;

pub fn run_window_wezterm(
    endpoint: &str,
    args: &[String],
    input_pipeline_mode: InputPipelineMode,
) -> Result<(), String> {
    let (initial_session_id, arg_offset) = match args.get(2) {
        Some(value) if value.parse::<usize>().is_err() => (Some(value.clone()), 3usize),
        _ => (None, 2usize),
    };
    let preview_lines = parse_usize(args.get(arg_offset), 500).clamp(50, 10_000);
    let cols = parse_usize(args.get(arg_offset + 1), 120).clamp(20, 400);
    let rows = parse_usize(args.get(arg_offset + 2), 32).clamp(5, 200);

    let mut app =
        ChatminalWindowApp::new(endpoint, preview_lines, cols, rows, input_pipeline_mode)?;
    if let Some(session_id) = initial_session_id {
        app.activate_session(&session_id);
    }
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Chatminal")
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([960.0, 620.0]),
        ..NativeOptions::default()
    };

    eframe::run_native("Chatminal", options, Box::new(move |_cc| Ok(Box::new(app))))
        .map_err(|err| format!("run native window failed: {err}"))?;
    Ok(())
}

pub(super) struct ChatminalWindowApp {
    pub(super) client: ChatminalClient,
    pub(super) pane_registry: SessionPaneRegistry,
    pub(super) state: WorkspaceBindingState,
    pub(super) selected_session_id: Option<String>,
    pub(super) preview_lines: usize,
    pub(super) pane_cols: usize,
    pub(super) pane_rows: usize,
    pub(super) new_session_name: String,
    pub(super) last_error: Option<String>,
    pub(super) input_worker: TerminalInputWorker,
    pub(super) cached_terminal_text: String,
    pub(super) render_dirty: bool,
    pub(super) pending_resize: Option<(usize, usize)>,
    pub(super) last_resize_request_at: Instant,
    pub(super) terminal_has_focus: bool,
    pub(super) ime_blur_flush_armed: bool,
    pub(super) ime_composition_state: ImeCompositionState,
    pub(super) ime_commit_deduper: ImeCommitDeduper,
    pub(super) input_pipeline_mode: InputPipelineMode,
    pub(super) legacy_ready_marker_written: bool,
}

impl eframe::App for ChatminalWindowApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.write_legacy_ready_marker_once();
        let mut should_repaint = self.poll_daemon_events();
        if should_repaint {
            self.render_dirty = true;
        }
        if self.poll_input_worker_results() {
            self.render_dirty = true;
            should_repaint = true;
        }
        if self.state.is_stale() {
            self.reload_workspace();
            should_repaint = true;
        }

        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Chatminal Native (WezTerm Core)")
                        .text_style(TextStyle::Monospace)
                        .strong(),
                );
                if ui.button("Reload").clicked() {
                    self.reload_workspace();
                    should_repaint = true;
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            if let Some(error) = &self.last_error {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 90), error);
            }
        });

        let mut next_selected = None::<String>;
        egui::SidePanel::left("session_sidebar")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Sessions");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_session_name)
                            .hint_text("New session name"),
                    );
                    if ui.button("+").clicked() {
                        self.create_session();
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for session in &self.state.workspace.sessions {
                        let selected = self.selected_session_id.as_deref()
                            == Some(session.session_id.as_str());
                        let label = format!("{} [{}]", session.name, session.status.as_ref());
                        if ui.selectable_label(selected, label).clicked() {
                            next_selected = Some(session.session_id.clone());
                        }
                    }
                });
            });
        if let Some(session_id) = next_selected {
            self.activate_session(&session_id);
            should_repaint = true;
        }

        if self.handle_terminal_input_events(ctx) {
            should_repaint = true;
        }
        self.flush_pending_resize();

        if self.poll_input_worker_results() {
            self.render_dirty = true;
            should_repaint = true;
        }
        if self.poll_daemon_events() {
            self.render_dirty = true;
            should_repaint = true;
        }
        self.refresh_cached_terminal_text();

        let mut terminal_rect = None::<egui::Rect>;
        egui::CentralPanel::default().show(ctx, |ui| {
            terminal_rect = Some(ui.max_rect());
            let output_height = ui.available_height().max(180.0);
            self.sync_terminal_size(egui::Vec2::new(ui.available_width(), output_height));

            egui::ScrollArea::both()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .max_height(output_height)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.cached_terminal_text)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(self.pane_rows.max(5))
                            .code_editor()
                            .interactive(false),
                    );
                });
        });
        self.update_terminal_focus(ctx, terminal_rect);
        self.sync_terminal_ime(ctx, terminal_rect);

        if self.render_dirty || self.pending_resize.is_some() || should_repaint {
            ctx.request_repaint_after(Duration::from_millis(UI_ACTIVE_REPAINT_MS));
        } else {
            ctx.request_repaint_after(Duration::from_millis(UI_IDLE_REPAINT_MS));
        }
    }
}

impl ChatminalWindowApp {
    fn write_legacy_ready_marker_once(&mut self) {
        if self.legacy_ready_marker_written {
            return;
        }
        let path = std::env::var("CHATMINAL_LEGACY_WINDOW_READY_FILE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(path) = path else {
            self.legacy_ready_marker_written = true;
            return;
        };
        if std::fs::write(path, "ready\n").is_ok() {
            self.legacy_ready_marker_written = true;
        }
    }
}
