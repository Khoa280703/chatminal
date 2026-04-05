use crate::desktop_session_host::overlay_shell::OverlayTerminal;
use crate::scripting::guiwin::GuiWin;
use chrono::prelude::*;
use futures::FutureExt;
use log::Level;
use luahelper::ValuePrinter;
use mlua::Value;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use termwiz::cell::{AttributeChange, CellAttributes, Intensity};
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::lineedit::*;
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

lazy_static::lazy_static! {
    static ref LATEST_LOG_ENTRY: Mutex<Option<DateTime<Local>>> = Mutex::new(None);
}

struct LuaReplHost {
    history: BasicHistory,
    lua: Option<mlua::Lua>,
    disabled_reason: Option<String>,
}

fn history_file_name() -> PathBuf {
    config::DATA_DIR.join("repl-history")
}

impl LuaReplHost {
    fn new(lua: Option<mlua::Lua>, disabled_reason: Option<String>) -> Self {
        let mut history = BasicHistory::default();
        if let Ok(data) = std::fs::read_to_string(history_file_name()) {
            for line in data.lines() {
                history.add(line);
            }
        }
        Self {
            history,
            lua,
            disabled_reason,
        }
    }

    fn is_enabled(&self) -> bool {
        self.lua.is_some()
    }

    fn disabled_reason(&self) -> &str {
        self.disabled_reason
            .as_deref()
            .unwrap_or("Lua REPL unavailable")
    }

    fn add_history(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }

        if let Some(last) = self.history.last() {
            if self.history.get(last).as_deref() == Some(line) {
                // Don't add duplicate lines
                return;
            }
        }
        self.history.add(line);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(history_file_name())
        {
            writeln!(file, "{}", line).ok();
        }
    }
}

fn format_lua_err(err: mlua::Error) -> String {
    match err {
        mlua::Error::SyntaxError {
            incomplete_input: true,
            ..
        } => "...".to_string(),
        _ => format!("{:#}", err),
    }
}

fn fragment_to_expr_or_statement(lua: &mlua::Lua, text: &str) -> Result<String, String> {
    let expr = format!("return {};", text);

    let chunk = lua.load(&expr).set_name("=repl");
    match chunk.into_function() {
        Ok(_) => {
            // It's an expression
            Ok(text.to_string())
        }
        Err(_) => {
            // Try instead as a statement
            let chunk = lua.load(text).set_name("=repl");
            match chunk.into_function() {
                Ok(_) => Ok(text.to_string()),
                Err(err) => Err(format_lua_err(err)),
            }
        }
    }
}

impl LineEditorHost for LuaReplHost {
    fn history(&mut self) -> &mut dyn History {
        &mut self.history
    }

    fn resolve_action(
        &mut self,
        event: &InputEvent,
        editor: &mut LineEditor<'_>,
    ) -> Option<Action> {
        let (line, _cursor) = editor.get_line_and_cursor();
        if line.is_empty()
            && matches!(
                event,
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                })
            )
        {
            Some(Action::Cancel)
        } else {
            None
        }
    }

    fn render_preview(&self, line: &str) -> Vec<OutputElement> {
        let mut preview = vec![];

        let Some(lua) = self.lua.as_ref() else {
            if !line.is_empty() {
                preview.push(OutputElement::Text(self.disabled_reason().to_string()));
            }
            return preview;
        };

        if let Err(err) = fragment_to_expr_or_statement(lua, line) {
            preview.push(OutputElement::Text(err))
        }

        preview
    }
}

pub fn show_debug_overlay(
    mut term: OverlayTerminal,
    gui_win: GuiWin,
    opengl_info: String,
    connection_info: String,
) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();

    let (lua, lua_version, disabled_reason) = match initialize_debug_overlay_lua(&gui_win) {
        Ok((lua, version)) => (Some(lua), version, None),
        Err(err) => {
            log::error!("debug overlay: lua repl unavailable: {err:#}");
            (
                None,
                "Unavailable".to_string(),
                Some(format!("Lua REPL unavailable: {err:#}")),
            )
        }
    };

    let mut host = Some(LuaReplHost::new(lua, disabled_reason.clone()));

    term.render(&[Change::Title("Debug".to_string())])?;

    fn print_new_log_entries(term: &mut OverlayTerminal) -> termwiz::Result<()> {
        let entries = env_bootstrap::ringlog::get_entries();
        let mut changes = vec![];
        for entry in entries {
            if let Some(latest) = LATEST_LOG_ENTRY.lock().unwrap().as_ref() {
                if entry.then <= *latest {
                    // already seen this one
                    continue;
                }
            }
            LATEST_LOG_ENTRY.lock().unwrap().replace(entry.then);

            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(Change::Text(entry.then.format("%H:%M:%S%.3f ").to_string()));

            changes.push(
                AttributeChange::Foreground(match entry.level {
                    Level::Error => AnsiColor::Maroon.into(),
                    Level::Warn => AnsiColor::Red.into(),
                    Level::Info => AnsiColor::Green.into(),
                    Level::Debug => AnsiColor::Blue.into(),
                    Level::Trace => AnsiColor::Fuchsia.into(),
                })
                .into(),
            );
            changes.push(Change::Text(entry.level.as_str().to_string()));
            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(AttributeChange::Intensity(Intensity::Bold).into());
            changes.push(Change::Text(format!(" {}", entry.target)));
            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(Change::Text(format!(
                " > {}\r\n",
                entry.msg.replace("\n", "\r\n")
            )));
        }
        term.render(&changes)
    }

    let version = config::engine_version();
    let triple = config::engine_target_triple();

    term.render(&[Change::Text(format!(
        "Debug Overlay\r\n\
         Chatminal Desktop version: {version} {triple}\r\n\
         Window Environment: {connection_info}\r\n\
         Lua Version: {lua_version}\r\n\
         {opengl_info}\r\n\
         {}\r\n\
         Press ESC or CTRL-D to exit\r\n",
        disabled_reason
            .as_deref()
            .unwrap_or("Enter lua statements or expressions and hit Enter."),
    ))])?;

    loop {
        print_new_log_entries(&mut term)?;
        let mut editor = LineEditor::new(&mut term);
        editor.set_prompt("> ");
        if let Some(line) = editor.read_line(host.as_mut().unwrap())? {
            if line.is_empty() {
                continue;
            }
            host.as_mut().unwrap().add_history(&line);
            if !host.as_ref().unwrap().is_enabled() {
                term.render(&[Change::Text(format!(
                    "{}\r\n",
                    host.as_ref()
                        .unwrap()
                        .disabled_reason()
                        .replace('\n', "\r\n")
                ))])?;
                continue;
            }

            let passed_host = host.take().unwrap();

            let (host_res, text) =
                smol::block_on(promise::spawn::spawn_into_main_thread(async move {
                    evaluate_trampoline(passed_host, line)
                        .recv()
                        .await
                        .map_err(|e| mlua::Error::external(format!("{:#}", e)))
                        .expect("returning result not to fail")
                }));

            host.replace(host_res);

            if text != "nil" {
                term.render(&[Change::Text(format!("{}\r\n", text.replace("\n", "\r\n")))])?;
            }
        } else {
            return Ok(());
        }
    }
}

// A bit of indirection because spawn_into_main_thread wants the
// overall future to be Send but mlua::Value, mlua::Chunk are not
// Send.  We need to split off the actual evaluation future to
// run separately, so we spawn it and use a channel to funnel
// the result back to the caller without blocking the gui thread.
fn evaluate_trampoline(
    host: LuaReplHost,
    expr: String,
) -> smol::channel::Receiver<(LuaReplHost, String)> {
    let (tx, rx) = smol::channel::bounded(1);
    promise::spawn::spawn(async move {
        let _ = tx.send(evaluate(host, expr).await).await;
    })
    .detach();
    rx
}

async fn evaluate(host: LuaReplHost, expr: String) -> (LuaReplHost, String) {
    async fn do_it(host: &LuaReplHost, expr: &str) -> String {
        let Some(lua) = host.lua.as_ref() else {
            return host.disabled_reason().to_string();
        };
        let code = match fragment_to_expr_or_statement(lua, expr) {
            Ok(code) => code,
            Err(err) => return err,
        };
        let chunk = lua.load(&code).set_name("repl");

        let result = chunk
            .eval_async::<Value>()
            .map(|result| match result {
                Ok(result) => {
                    let value = ValuePrinter(result);
                    format!("{:#?}", value)
                }
                Err(err) => format_lua_err(err),
            })
            .await;

        result
    }

    let result = do_it(&host, &expr).await;
    (host, result)
}

fn initialize_debug_overlay_lua(gui_win: &GuiWin) -> anyhow::Result<(mlua::Lua, String)> {
    let config::LoadedConfig { lua, .. } = config::Config::load();
    let lua = match lua {
        Some(lua) => lua,
        None => match config::Config::try_default() {
            Ok(config::LoadedConfig { lua: Some(lua), .. }) => lua,
            _ => config::lua::make_lua_context(std::path::Path::new(""))?,
        },
    };

    lua.load("chatminal = require 'chatminal'").exec()?;
    lua.globals().set("window", gui_win.clone())?;
    let lua_version: String = lua.globals().get("_VERSION")?;
    Ok((lua, lua_version))
}
