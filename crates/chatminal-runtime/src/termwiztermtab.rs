//! a tab hosting a termwiz terminal applet
//! The idea is to use these when Chatminal needs to request
//! input from the user as part of setup flows.

use crate::SessionTerminalHandle;
use crate::pane::{
    CachePolicy, CloseReason, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, WithPaneLines,
    alloc_pane_id,
};
use crate::renderable::*;
use crossbeam::channel::{Receiver, Sender, unbounded as channel};
use engine_term::color::ColorPalette;
use engine_term::{
    KeyCode, KeyModifiers, MouseEvent, StableRowIndex, TerminalConfiguration, TerminalSize,
};
use filedescriptor::{FileDescriptor, Pipe};
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rangeset::RangeSet;
use std::io::{BufWriter, Write};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;
use termwiz::Context;
use termwiz::input::{InputEvent, KeyEvent, Modifiers, MouseEvent as TermWizMouseEvent};
use termwiz::render::terminfo::TerminfoRenderer;
use termwiz::surface::{Change, Line, SequenceNo};
use termwiz::terminal::{ScreenSize, TerminalWaker};
use url::Url;

pub struct TermWizTerminalPane {
    pane_id: PaneId,
    terminal: Mutex<engine_term::Terminal>,
    input_tx: Sender<InputEvent>,
    dead: Mutex<bool>,
    writer: Mutex<Vec<u8>>,
    render_rx: FileDescriptor,
}

impl TermWizTerminalPane {
    fn new(
        size: TerminalSize,
        input_tx: Sender<InputEvent>,
        render_rx: FileDescriptor,
        term_config: Option<Arc<dyn TerminalConfiguration + Send + Sync>>,
    ) -> Self {
        let pane_id = alloc_pane_id();

        let terminal = Mutex::new(engine_term::Terminal::new(
            size,
            term_config.unwrap_or_else(|| {
                Arc::new(config::TermConfig::with_config(
                    config::current_config_handle(),
                ))
            }),
            "Chatminal",
            config::engine_version(),
            Box::new(Vec::new()), // FIXME: connect to something?
        ));

        Self {
            pane_id,
            terminal,
            writer: Mutex::new(Vec::new()),
            render_rx,
            input_tx,
            dead: Mutex::new(false),
        }
    }
}

impl Pane for TermWizTerminalPane {
    fn terminal_handle(&self) -> SessionTerminalHandle {
        SessionTerminalHandle::new(self.pane_id as u64)
    }

    fn get_cursor_position(&self) -> StableCursorPosition {
        terminal_get_cursor_position(&mut self.terminal.lock())
    }

    fn get_current_seqno(&self) -> SequenceNo {
        self.terminal.lock().current_seqno()
    }

    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        terminal_get_dirty_lines(&mut self.terminal.lock(), lines, seqno)
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
        );
    }

    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        crate::pane::impl_get_logical_lines_via_get_lines(self, lines)
    }

    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        terminal_with_lines_mut(&mut self.terminal.lock(), lines, with_lines)
    }

    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        terminal_get_lines(&mut self.terminal.lock(), lines)
    }

    fn get_dimensions(&self) -> RenderableDimensions {
        terminal_get_dimensions(&mut self.terminal.lock())
    }

    fn get_title(&self) -> String {
        self.terminal.lock().get_title().to_string()
    }

    fn can_close_without_prompting(&self, _reason: CloseReason) -> bool {
        true
    }

    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        let paste = InputEvent::Paste(text.to_string());
        self.input_tx.send(paste)?;
        Ok(())
    }

    fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
        Ok(Some(Box::new(self.render_rx.try_clone()?)))
    }

    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        MutexGuard::map(self.writer.lock(), |writer| {
            let w: &mut dyn std::io::Write = writer;
            w
        })
    }

    fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
        self.input_tx.send(InputEvent::Resized {
            rows: size.rows as usize,
            cols: size.cols as usize,
        })?;

        self.terminal.lock().resize(size);

        Ok(())
    }

    fn key_down(&self, key: KeyCode, modifiers: KeyModifiers) -> anyhow::Result<()> {
        let event = InputEvent::Key(KeyEvent {
            key,
            modifiers: modifiers.remove_positional_mods(),
        });
        if let Err(e) = self.input_tx.send(event) {
            *self.dead.lock() = true;
            return Err(e.into());
        }
        Ok(())
    }

    fn key_up(&self, _key: KeyCode, _modifiers: KeyModifiers) -> anyhow::Result<()> {
        Ok(())
    }

    fn mouse_event(&self, event: MouseEvent) -> anyhow::Result<()> {
        use engine_term::input::MouseButton;
        use termwiz::input::MouseButtons as Buttons;

        let mouse_buttons = match event.button {
            MouseButton::Left => Buttons::LEFT,
            MouseButton::Middle => Buttons::MIDDLE,
            MouseButton::Right => Buttons::RIGHT,
            MouseButton::WheelUp(_) => Buttons::VERT_WHEEL | Buttons::WHEEL_POSITIVE,
            MouseButton::WheelDown(_) => Buttons::VERT_WHEEL,
            MouseButton::WheelLeft(_) => Buttons::HORZ_WHEEL | Buttons::WHEEL_POSITIVE,
            MouseButton::WheelRight(_) => Buttons::HORZ_WHEEL,
            MouseButton::None => Buttons::NONE,
        };

        let event = InputEvent::Mouse(TermWizMouseEvent {
            x: event.x as u16,
            y: event.y as u16,
            mouse_buttons,
            modifiers: event.modifiers,
        });
        if let Err(e) = self.input_tx.send(event) {
            *self.dead.lock() = true;
            return Err(e.into());
        }
        Ok(())
    }

    fn set_config(&self, config: Arc<dyn TerminalConfiguration>) {
        self.terminal.lock().set_config(config);
    }

    fn get_config(&self) -> Option<Arc<dyn TerminalConfiguration>> {
        Some(self.terminal.lock().get_config())
    }

    fn perform_actions(&self, actions: Vec<termwiz::escape::Action>) {
        self.terminal.lock().perform_actions(actions)
    }

    fn kill(&self) {
        *self.dead.lock() = true;
    }

    fn is_dead(&self) -> bool {
        *self.dead.lock()
    }

    fn palette(&self) -> ColorPalette {
        self.terminal.lock().palette()
    }

    fn is_mouse_grabbed(&self) -> bool {
        self.terminal.lock().is_mouse_grabbed()
    }

    fn is_alt_screen_active(&self) -> bool {
        self.terminal.lock().is_alt_screen_active()
    }

    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
        self.terminal.lock().get_current_dir().cloned()
    }
}

pub struct TermWizTerminal {
    render_tx: TermWizTerminalRenderTty,
    input_rx: Receiver<InputEvent>,
    renderer: TerminfoRenderer,
    grab_mouse: bool,
}

impl TermWizTerminal {
    pub fn no_grab_mouse_in_raw_mode(&mut self) {
        self.grab_mouse = false;
    }
}

struct TermWizTerminalRenderTty {
    render_tx: BufWriter<FileDescriptor>,
    screen_size: ScreenSize,
}

impl std::io::Write for TermWizTerminalRenderTty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.render_tx.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.render_tx.flush()
    }
}

impl termwiz::render::RenderTty for TermWizTerminalRenderTty {
    fn get_size_in_cells(&mut self) -> termwiz::Result<(usize, usize)> {
        Ok((self.screen_size.cols, self.screen_size.rows))
    }
}

impl TermWizTerminal {
    fn do_input_poll(&mut self, wait: Option<Duration>) -> termwiz::Result<Option<InputEvent>> {
        if let Some(timeout) = wait {
            match self.input_rx.recv_timeout(timeout) {
                Ok(input) => Ok(Some(input)),
                Err(err) => {
                    if err.is_timeout() {
                        Ok(None)
                    } else {
                        Err(err).context("receive from channel")
                    }
                }
            }
        } else {
            let input = self.input_rx.recv().context("receive from channel")?;
            Ok(Some(input))
        }
    }
}

impl termwiz::terminal::Terminal for TermWizTerminal {
    fn set_raw_mode(&mut self) -> termwiz::Result<()> {
        use termwiz::escape::csi::{CSI, DecPrivateMode, DecPrivateModeCode, Mode};

        macro_rules! decset {
            ($variant:ident) => {
                write!(
                    self.render_tx,
                    "{}",
                    CSI::Mode(Mode::SetDecPrivateMode(DecPrivateMode::Code(
                        DecPrivateModeCode::$variant
                    )))
                )?;
            };
        }

        decset!(BracketedPaste);
        if self.grab_mouse {
            decset!(AnyEventMouse);
            decset!(SGRMouse);
        }
        self.flush()?;

        Ok(())
    }

    fn set_cooked_mode(&mut self) -> termwiz::Result<()> {
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> termwiz::Result<()> {
        termwiz::bail!("TermWizTerminalPane has no alt screen");
    }

    fn exit_alternate_screen(&mut self) -> termwiz::Result<()> {
        termwiz::bail!("TermWizTerminalPane has no alt screen");
    }

    fn get_screen_size(&mut self) -> termwiz::Result<ScreenSize> {
        Ok(self.render_tx.screen_size)
    }

    fn set_screen_size(&mut self, _size: ScreenSize) -> termwiz::Result<()> {
        termwiz::bail!("TermWizTerminalPane cannot set screen size");
    }

    fn render(&mut self, changes: &[Change]) -> termwiz::Result<()> {
        self.renderer.render_to(changes, &mut self.render_tx)?;
        Ok(())
    }

    fn flush(&mut self) -> termwiz::Result<()> {
        self.render_tx.render_tx.flush()?;
        Ok(())
    }

    fn poll_input(&mut self, wait: Option<Duration>) -> termwiz::Result<Option<InputEvent>> {
        self.do_input_poll(wait).map(|i| {
            if let Some(InputEvent::Resized { cols, rows }) = i.as_ref() {
                self.render_tx.screen_size.cols = *cols;
                self.render_tx.screen_size.rows = *rows;
            }
            match i {
                // Urgh, we get normalized-to-lowercase CTRL-c,
                // but eg: termwiz and other terminal input expect
                // to get CTRL-C instead.  Adjust for that here.
                Some(InputEvent::Key(KeyEvent {
                    key: KeyCode::Char(c),
                    modifiers: Modifiers::CTRL,
                })) if c.is_ascii_lowercase() => Some(InputEvent::Key(KeyEvent {
                    key: KeyCode::Char(c.to_ascii_uppercase()),
                    modifiers: Modifiers::CTRL,
                })),
                i @ _ => i,
            }
        })
    }

    fn waker(&self) -> TerminalWaker {
        // TODO: TerminalWaker assumes that we're a SystemTerminal but that
        // isn't the case here.
        panic!("TermWizTerminal::waker called!?");
    }
}

pub fn allocate(
    size: TerminalSize,
    config: Arc<dyn TerminalConfiguration + Send + Sync>,
) -> (TermWizTerminal, Arc<dyn Pane>) {
    let render_pipe = Pipe::new().expect("Pipe creation not to fail");

    let (input_tx, input_rx) = channel();

    let renderer = termwiz_funcs::new_chatminal_terminfo_renderer();

    let tw_term = TermWizTerminal {
        render_tx: TermWizTerminalRenderTty {
            render_tx: BufWriter::new(render_pipe.write),
            screen_size: ScreenSize {
                cols: size.cols as usize,
                rows: size.rows as usize,
                xpixel: (size.pixel_width / size.cols) as usize,
                ypixel: (size.pixel_height / size.rows) as usize,
            },
        },
        input_rx,
        renderer,
        grab_mouse: true,
    };

    let pane = TermWizTerminalPane::new(size, input_tx, render_pipe.read, Some(config));

    let pane: Arc<dyn Pane> = Arc::new(pane);

    (tw_term, pane)
}
