use serde::{Deserialize, Serialize};

use crate::manager::registry::TerminalInfo;
use crate::status::events::TerminalEvent;
use crate::status::query::TerminalState;
use crate::terminal::cell::{CellAttributes, Color, Cursor, TerminalCell};

use super::interaction::InteractionEntry;

/// A span of consecutive characters with identical attributes (RLE compression)
#[derive(Debug, Clone, Serialize)]
pub struct WsSpan {
    pub text: String,
    #[serde(skip_serializing_if = "is_default_color")]
    pub fg: Color,
    #[serde(skip_serializing_if = "is_default_color")]
    pub bg: Color,
    #[serde(skip_serializing_if = "is_default_attrs")]
    pub attrs: CellAttributes,
}

/// A single row of spans
#[derive(Debug, Clone, Serialize)]
pub struct WsRow {
    pub row: usize,
    pub spans: Vec<WsSpan>,
}

/// Server -> Client WebSocket messages
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    ScreenSnapshot {
        terminal_id: String,
        cols: usize,
        rows: usize,
        screen_rows: Vec<WsRow>,
        cursor: Cursor,
        state: TerminalState,
    },
    ScreenUpdate {
        terminal_id: String,
        changed_rows: Vec<WsRow>,
        cursor: Cursor,
        state: TerminalState,
    },
    #[allow(dead_code)]
    TerminalEvent {
        terminal_id: String,
        event: TerminalEvent,
    },
    Interaction {
        entry: InteractionEntry,
    },
    TerminalList {
        terminals: Vec<TerminalInfo>,
    },
}

/// Client -> Server WebSocket messages
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMessage {
    /// Subscribe to a specific terminal's updates
    Subscribe { terminal_id: String },
    /// Request the current terminal list
    ListTerminals,
}

fn is_default_color(c: &Color) -> bool {
    matches!(c, Color::Default)
}

fn is_default_attrs(a: &CellAttributes) -> bool {
    *a == CellAttributes::default()
}

/// Compress a row of TerminalCells into spans (RLE by attributes)
pub fn cells_to_spans(cells: &[TerminalCell]) -> Vec<WsSpan> {
    if cells.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::with_capacity(cells.len() / 4 + 1);
    let mut current_text = String::with_capacity(cells.len());
    let mut current_fg = cells[0].fg;
    let mut current_bg = cells[0].bg;
    let mut current_attrs = cells[0].attrs;

    for cell in cells {
        if cell.fg == current_fg && cell.bg == current_bg && cell.attrs == current_attrs {
            current_text.push(cell.c);
        } else {
            if !current_text.is_empty() {
                spans.push(WsSpan {
                    text: current_text,
                    fg: current_fg,
                    bg: current_bg,
                    attrs: current_attrs,
                });
            }
            current_text = String::new();
            current_text.push(cell.c);
            current_fg = cell.fg;
            current_bg = cell.bg;
            current_attrs = cell.attrs;
        }
    }

    if !current_text.is_empty() {
        spans.push(WsSpan {
            text: current_text,
            fg: current_fg,
            bg: current_bg,
            attrs: current_attrs,
        });
    }

    spans
}

/// Build WsRow from a grid row at a given index
pub fn row_to_ws_row(row_idx: usize, cells: &[TerminalCell]) -> WsRow {
    WsRow {
        row: row_idx,
        spans: cells_to_spans(cells),
    }
}
