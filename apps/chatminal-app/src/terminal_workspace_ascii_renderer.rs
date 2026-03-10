use std::collections::HashMap;

use crate::terminal_pane_emulator::PaneSnapshotSummary;
use crate::terminal_workspace_view_model::TerminalWorkspaceViewModel;

pub fn render_terminal_workspace_ascii(
    view_model: &TerminalWorkspaceViewModel,
    pane_snapshots: &[PaneSnapshotSummary],
    max_pane_preview_lines: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push("=== Chatminal Native Workspace Preview ===".to_string());
    lines.push(format!("Status: {}", view_model.status_line));
    lines.push(String::new());

    lines.push("Profiles:".to_string());
    for profile in &view_model.profiles {
        let marker = if profile.is_active { "*" } else { " " };
        lines.push(format!(
            "[{marker}] {} ({})",
            profile.name,
            abbreviate_id(&profile.profile_id)
        ));
    }
    lines.push(String::new());

    lines.push("Sessions:".to_string());
    for session in &view_model.sessions {
        let active = if session.is_active { "*" } else { " " };
        let pane = session
            .pane_id
            .as_deref()
            .map_or_else(|| "none".to_string(), |pane_id| abbreviate_id(pane_id));
        lines.push(format!(
            "[{active}] {} [{}] pane={} session={}",
            session.name,
            session.status,
            pane,
            abbreviate_id(&session.session_id)
        ));
    }
    lines.push(String::new());

    let pane_by_id: HashMap<String, &PaneSnapshotSummary> = pane_snapshots
        .iter()
        .map(|pane| (pane.pane_id.clone(), pane))
        .collect();

    lines.push("Active Pane:".to_string());
    if let Some(active_pane_id) = view_model.active_pane_id.as_deref() {
        if let Some(active_pane) = pane_by_id.get(active_pane_id) {
            lines.push(format!(
                "pane={} session={} size={}x{}",
                active_pane.pane_id, active_pane.session_id, active_pane.cols, active_pane.rows
            ));
            lines.push("---".to_string());
            let preview = limit_trailing_lines(&active_pane.visible_text, max_pane_preview_lines);
            if preview.is_empty() {
                lines.push("(empty)".to_string());
            } else {
                lines.extend(preview.lines().map(|line| line.to_string()));
            }
        } else {
            lines.push(format!(
                "pane={} (snapshot not available yet)",
                active_pane_id
            ));
        }
    } else {
        lines.push("(none)".to_string());
    }

    lines.join("\n")
}

pub fn fit_dashboard_for_terminal(input: &str, cols: usize, rows: usize) -> String {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let max_visible_rows = rows.saturating_sub(1);
    let mut output = Vec::new();
    for line in sanitize_for_terminal(input).lines().take(max_visible_rows) {
        output.push(truncate_line(line, cols));
    }
    output.join("\n")
}

fn limit_trailing_lines(input: &str, max_lines: usize) -> String {
    let max_lines = max_lines.max(1);
    let normalized = sanitize_for_terminal(input);
    let all = normalized.lines().collect::<Vec<_>>();
    if all.len() <= max_lines {
        return all.join("\n");
    }
    all[all.len() - max_lines..].join("\n")
}

fn sanitize_for_terminal(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch == '\r' {
            output.push('\n');
            continue;
        }
        if ch == '\n' || ch == '\t' || !ch.is_control() {
            output.push(ch);
        }
    }
    output
}

fn truncate_line(line: &str, max_cols: usize) -> String {
    if line.chars().count() <= max_cols {
        return line.to_string();
    }
    let keep = max_cols.saturating_sub(1);
    let mut truncated = line.chars().take(keep).collect::<String>();
    truncated.push('…');
    truncated
}

fn abbreviate_id(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= 12 {
        return trimmed.to_string();
    }
    format!("{}…{}", &trimmed[..8], &trimmed[trimmed.len() - 3..])
}

#[cfg(test)]
#[path = "terminal_workspace_ascii_renderer_tests.rs"]
mod terminal_workspace_ascii_renderer_tests;
