use std::io::{Stdout, Write};

use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};

use crate::terminal_wezterm_core::WeztermTerminalPaneAdapter;

pub fn render_attach_frame(
    out: &mut Stdout,
    adapter: &WeztermTerminalPaneAdapter,
    pane_id: &str,
    session_id: &str,
    cols: usize,
    rows: usize,
) -> Result<(), String> {
    let text = adapter
        .pane_snapshot(pane_id)
        .map(|value| value.visible_text)
        .unwrap_or_default();
    let body = fit_text_for_terminal(&text, cols, rows);
    let status = truncate_line(
        &format!(
            "Attached {} pane={} {}x{} | F10 quit",
            abbreviate_id(session_id),
            pane_id,
            cols,
            rows
        ),
        cols,
    );

    queue!(out, MoveTo(0, 0), Clear(ClearType::All))
        .map_err(|err| format!("prepare frame failed: {err}"))?;
    for (row, line) in body.iter().enumerate() {
        queue!(out, MoveTo(0, row as u16), Print(line))
            .map_err(|err| format!("render row failed: {err}"))?;
    }
    queue!(out, MoveTo(0, rows as u16), Print(status))
        .map_err(|err| format!("render status failed: {err}"))?;
    out.flush()
        .map_err(|err| format!("flush frame failed: {err}"))?;
    Ok(())
}

fn fit_text_for_terminal(input: &str, cols: usize, rows: usize) -> Vec<String> {
    let mut normalized = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch == '\r' {
            normalized.push('\n');
        } else if ch == '\n' || ch == '\t' || !ch.is_control() {
            normalized.push(ch);
        }
    }
    let lines = normalized
        .lines()
        .map(|line| truncate_line(line, cols))
        .collect::<Vec<_>>();
    let start = lines.len().saturating_sub(rows);
    lines[start..].to_vec()
}

fn truncate_line(line: &str, max_cols: usize) -> String {
    if line.chars().count() <= max_cols {
        return line.to_string();
    }
    let keep = max_cols.saturating_sub(1);
    let mut output = line.chars().take(keep).collect::<String>();
    output.push('…');
    output
}

fn abbreviate_id(value: &str) -> String {
    if value.len() <= 12 {
        return value.to_string();
    }
    format!("{}…{}", &value[..8], &value[value.len() - 3..])
}
