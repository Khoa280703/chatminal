use super::{Cell, TerminalGrid};

#[test]
fn set_cell_writes_character() {
    let mut grid = TerminalGrid::new(4, 2, 10);
    grid.set_cell(
        0,
        0,
        Cell {
            c: 'A',
            ..Default::default()
        },
    );

    assert_eq!(grid.active_cells()[0][0].c, 'A');
}

#[test]
fn primary_scroll_pushes_to_scrollback() {
    let mut grid = TerminalGrid::new(4, 2, 10);
    grid.set_cell(
        0,
        0,
        Cell {
            c: 'X',
            ..Default::default()
        },
    );

    let added = grid.scroll_up(1);

    assert_eq!(added, 1);
    assert_eq!(grid.scrollback.len(), 1);
    assert_eq!(grid.scrollback[0][0].c, 'X');
}

#[test]
fn resize_clamps_cursor() {
    let mut grid = TerminalGrid::new(80, 24, 10);
    grid.cursor_col = 79;
    grid.cursor_row = 23;

    grid.resize(40, 10);

    assert_eq!(grid.cols, 40);
    assert_eq!(grid.rows, 10);
    assert_eq!(grid.cursor_col, 39);
    assert_eq!(grid.cursor_row, 9);
}

#[test]
fn scrollback_capacity_is_enforced() {
    let mut grid = TerminalGrid::new(4, 2, 3);
    let limit = grid.scrollback_max_lines;

    for i in 0..(limit + 50) {
        grid.set_cell(
            0,
            0,
            Cell {
                c: char::from(b'0' + (i % 10) as u8),
                ..Default::default()
            },
        );
        let _ = grid.scroll_up(1);
    }

    assert_eq!(limit, 100);
    assert_eq!(grid.scrollback.len(), limit);
}

#[test]
fn scroll_down_inserts_blank_line_at_top() {
    let mut grid = TerminalGrid::new(4, 2, 10);
    grid.set_cell(
        0,
        0,
        Cell {
            c: 'A',
            ..Default::default()
        },
    );
    grid.set_cell(
        1,
        0,
        Cell {
            c: 'B',
            ..Default::default()
        },
    );

    grid.scroll_down(1);

    assert_eq!(grid.active_cells()[0][0].c, ' ');
    assert_eq!(grid.active_cells()[1][0].c, 'A');
    assert!(grid.scrollback.is_empty());
}

#[test]
fn scrollback_limit_is_clamped_on_new_grid() {
    let grid = TerminalGrid::new(4, 2, usize::MAX);
    assert_eq!(grid.scrollback_max_lines, 200_000);
}
