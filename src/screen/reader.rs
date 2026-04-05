use std::fmt::Write;

use crate::terminal::cell::TerminalCell;
use crate::terminal::grid::TerminalGrid;

/// Write the column-number header (hundreds/tens/units) into the output string
pub fn write_column_header(output: &mut String, cols: usize) {
    output.push_str("   ");
    for c in 0..cols {
        output.push(char::from(b'0' + (c / 100 % 10) as u8));
    }
    output.push('\n');

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

/// Read the full screen as text with coordinate overlay
pub fn read_screen_text(grid: &TerminalGrid) -> String {
    let cols = grid.cols();
    let rows = grid.rows();
    let mut output = String::with_capacity((cols + 4) * (rows + 3));

    write_column_header(&mut output, cols);

    let screen_rows = grid.get_rows();
    for (y, row) in screen_rows.iter().enumerate().take(rows) {
        let _ = write!(output, "{:02} ", y);
        for cell in row.iter().take(cols) {
            output.push(cell.c);
        }
        output.push('\n');
    }

    output
}

/// Read specific rows as text with coordinate overlay
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
            let _ = write!(output, "{:02} ", y);
            for cell in row.iter().take(cols) {
                output.push(cell.c);
            }
            output.push('\n');
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
    let mut output = String::new();

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

fn line_to_string(cells: &[TerminalCell]) -> String {
    let mut s: String = cells.iter().map(|c| c.c).collect();
    let trimmed = s.trim_end().len();
    s.truncate(trimmed);
    s
}
