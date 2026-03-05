pub fn choose_selected_session_id(
    session_ids: &[String],
    current_selected: Option<&str>,
    active_session: Option<&str>,
) -> Option<String> {
    if let Some(selected) = current_selected
        && session_ids.iter().any(|value| value == selected)
    {
        return Some(selected.to_string());
    }

    if let Some(active) = active_session
        && session_ids.iter().any(|value| value == active)
    {
        return Some(active.to_string());
    }

    session_ids.first().cloned()
}

pub fn compute_terminal_grid(
    width_px: f32,
    height_px: f32,
    char_width_px: f32,
    char_height_px: f32,
    min_cols: usize,
    max_cols: usize,
    min_rows: usize,
    max_rows: usize,
) -> (usize, usize) {
    let safe_char_width = if char_width_px <= 0.0 {
        1.0
    } else {
        char_width_px
    };
    let safe_char_height = if char_height_px <= 0.0 {
        1.0
    } else {
        char_height_px
    };

    let cols = ((width_px / safe_char_width).floor() as usize).clamp(min_cols, max_cols);
    let rows = ((height_px / safe_char_height).floor() as usize).clamp(min_rows, max_rows);
    (cols, rows)
}

#[cfg(test)]
mod tests {
    use super::{choose_selected_session_id, compute_terminal_grid};

    #[test]
    fn choose_selected_keeps_existing_valid_selection() {
        let sessions = vec!["s-1".to_string(), "s-2".to_string()];
        let selected = choose_selected_session_id(&sessions, Some("s-2"), Some("s-1"));
        assert_eq!(selected.as_deref(), Some("s-2"));
    }

    #[test]
    fn choose_selected_falls_back_to_active_then_first() {
        let sessions = vec!["s-1".to_string(), "s-2".to_string()];
        let selected = choose_selected_session_id(&sessions, Some("missing"), Some("s-1"));
        assert_eq!(selected.as_deref(), Some("s-1"));

        let selected = choose_selected_session_id(&sessions, Some("missing"), Some("none"));
        assert_eq!(selected.as_deref(), Some("s-1"));
    }

    #[test]
    fn compute_terminal_grid_clamps_limits() {
        let (cols, rows) = compute_terminal_grid(2000.0, 1200.0, 8.0, 18.0, 20, 400, 5, 200);
        assert_eq!(cols, 250);
        assert_eq!(rows, 66);

        let (cols, rows) = compute_terminal_grid(1.0, 1.0, 8.0, 18.0, 20, 400, 5, 200);
        assert_eq!(cols, 20);
        assert_eq!(rows, 5);
    }
}
