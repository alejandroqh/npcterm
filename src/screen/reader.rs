use std::collections::VecDeque;
use std::fmt::Write;

use crate::terminal::cell::TerminalCell;
use crate::terminal::grid::TerminalGrid;

/// Find the index past the last non-whitespace cell (0 if row is all spaces)
fn trim_end_index(row: &[TerminalCell], cols: usize) -> usize {
    for i in (0..cols.min(row.len())).rev() {
        if row[i].c != ' ' {
            return i + 1;
        }
    }
    0
}

/// Write trimmed row cells directly into output (no intermediate String)
fn push_row_cells(output: &mut String, row: &[TerminalCell], end: usize) {
    for cell in row.iter().take(end) {
        output.push(cell.c);
    }
}

/// Emit collapsed empty-row marker
fn emit_empty_range(output: &mut String, start: usize, end: usize) {
    if start == end {
        let _ = write!(output, "{:02}\n", start);
    } else {
        let _ = write!(output, "··· (rows {:02}-{:02} empty)\n", start, end);
    }
}

/// Write the column-number header into the output string.
/// Skips the hundreds row when all columns < 100 (i.e., 80-col terminals).
pub fn write_column_header(output: &mut String, cols: usize) {
    if cols > 100 {
        output.push_str("   ");
        for c in 0..cols {
            output.push(char::from(b'0' + (c / 100 % 10) as u8));
        }
        output.push('\n');
    }

    output.push_str("   ");
    for c in 0..cols {
        output.push(char::from(b'0' + (c / 10 % 10) as u8));
    }
    output.push('\n');

    output.push_str("   ");
    for c in 0..cols {
        output.push(char::from(b'0' + (c % 10) as u8));
    }
    output.push('\n');
}

/// Read the full screen as text with coordinate overlay.
/// Trims trailing whitespace per row and collapses consecutive empty rows.
pub fn read_screen_text(grid: &TerminalGrid) -> String {
    let cols = grid.cols();
    let rows = grid.rows();
    let mut output = String::with_capacity((cols + 4) * (rows + 3));

    write_column_header(&mut output, cols);

    let screen_rows = grid.get_rows();
    let mut empty_start: Option<usize> = None;

    for (y, row) in screen_rows.iter().enumerate().take(rows) {
        let end = trim_end_index(row, cols);
        if end == 0 {
            if empty_start.is_none() {
                empty_start = Some(y);
            }
        } else {
            if let Some(start) = empty_start {
                emit_empty_range(&mut output, start, y - 1);
                empty_start = None;
            }
            let _ = write!(output, "{:02} ", y);
            push_row_cells(&mut output, row, end);
            output.push('\n');
        }
    }

    if let Some(start) = empty_start {
        emit_empty_range(&mut output, start, rows - 1);
    }

    output
}

/// Read specific rows as text with coordinate overlay (trimmed)
pub fn read_rows_text(grid: &TerminalGrid, start_row: usize, end_row: usize) -> String {
    let cols = grid.cols();
    let rows = grid.rows();
    let start = start_row.min(rows);
    let end = end_row.min(rows);
    let mut output = String::with_capacity((cols + 4) * (end - start + 3));

    write_column_header(&mut output, cols);

    let screen_rows = grid.get_rows();
    for y in start..end {
        if let Some(row) = screen_rows.get(y) {
            let trim = trim_end_index(row, cols);
            if trim == 0 {
                let _ = write!(output, "{:02}\n", y);
            } else {
                let _ = write!(output, "{:02} ", y);
                push_row_cells(&mut output, row, trim);
                output.push('\n');
            }
        }
    }

    output
}

/// Read a rectangular region
pub fn read_region_text(
    grid: &TerminalGrid,
    col1: usize,
    row1: usize,
    col2: usize,
    row2: usize,
) -> String {
    let cols = grid.cols();
    let rows = grid.rows();
    let c1 = col1.min(cols);
    let c2 = col2.min(cols);
    let r1 = row1.min(rows);
    let r2 = row2.min(rows);
    let mut output = String::with_capacity((c2 - c1 + 4) * (r2 - r1));

    let screen_rows = grid.get_rows();
    for y in r1..r2 {
        if let Some(row) = screen_rows.get(y) {
            let _ = write!(output, "{:02} ", y);
            for x in c1..c2 {
                if let Some(cell) = row.get(x) {
                    output.push(cell.c);
                }
            }
            output.push('\n');
        }
    }

    output
}

/// Read the full screen as clean text (no coordinate overlay, for human display)
pub fn show_screen_text(grid: &TerminalGrid) -> String {
    let cols = grid.cols();
    let rows = grid.rows();
    let mut output = String::with_capacity((cols + 1) * rows);

    let screen_rows = grid.get_rows();
    for row in screen_rows.iter().take(rows) {
        for cell in row.iter().take(cols) {
            output.push(cell.c);
        }
        output.push('\n');
    }

    output
}

/// Render scrollback content into output, optionally with coordinate overlay
pub fn render_scrollback(
    output: &mut String,
    scrollback: &VecDeque<Vec<TerminalCell>>,
    screen: &[Vec<TerminalCell>],
    scroll_offset: usize,
    cols: usize,
    rows: usize,
    with_coords: bool,
) {
    let scrollback_len = scrollback.len();
    let start_line = scrollback_len.saturating_sub(scroll_offset);

    for y in 0..rows {
        let line_idx = start_line + y;
        if with_coords {
            let _ = write!(output, "{:02} ", y);
        }
        if line_idx < scrollback_len {
            for cell in scrollback[line_idx].iter().take(cols) {
                output.push(cell.c);
            }
        } else {
            let screen_idx = line_idx - scrollback_len;
            if let Some(row) = screen.get(screen_idx) {
                for cell in row.iter().take(cols) {
                    output.push(cell.c);
                }
            }
        }
        output.push('\n');
    }
}

/// Get the last N non-empty lines from the visible screen
pub fn last_n_lines(grid: &TerminalGrid, n: usize) -> Vec<String> {
    let screen_rows = grid.get_rows();
    let mut lines: Vec<String> = Vec::with_capacity(n);

    for row in screen_rows.iter().rev() {
        let text = line_to_string(row);
        if !text.is_empty() || lines.len() < n {
            lines.push(text);
        }
        if lines.len() >= n {
            break;
        }
    }

    lines.reverse();
    lines
}

/// Read only the specified dirty rows as text with coordinate overlay
pub fn read_changed_rows_text(grid: &TerminalGrid, dirty_indices: &[usize]) -> String {
    let cols = grid.cols();
    let mut output = String::with_capacity((cols + 4) * (dirty_indices.len() + 3));

    write_column_header(&mut output, cols);

    let screen_rows = grid.get_rows();
    for &y in dirty_indices {
        if let Some(row) = screen_rows.get(y) {
            let trim = trim_end_index(row, cols);
            if trim == 0 {
                let _ = write!(output, "{:02}\n", y);
            } else {
                let _ = write!(output, "{:02} ", y);
                push_row_cells(&mut output, row, trim);
                output.push('\n');
            }
        }
    }

    output
}

/// Append scrollback lines directly to output as plain text (trimmed)
pub fn append_scrollback_lines(
    output: &mut String,
    scrollback: &VecDeque<Vec<TerminalCell>>,
    start: usize,
    end: usize,
) {
    for i in start..end.min(scrollback.len()) {
        output.push_str(&line_to_string(&scrollback[i]));
        output.push('\n');
    }
}

fn line_to_string(cells: &[TerminalCell]) -> String {
    let end = trim_end_index(cells, cells.len());
    cells[..end].iter().map(|c| c.c).collect()
}
