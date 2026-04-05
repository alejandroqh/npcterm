use crate::terminal::cell::{CellAttributes, Color, TerminalCell};
use serde::Serialize;

/// JSON representation of a cell for MCP responses
#[derive(Debug, Serialize)]
pub struct CellInfo {
    pub c: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<ColorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<ColorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<AttrsInfo>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub wide: bool,
}

/// JSON representation of a color
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorInfo {
    Default,
    Named(String),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// JSON representation of cell attributes (only non-default fields)
#[derive(Debug, Serialize)]
pub struct AttrsInfo {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dim: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub blink: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reverse: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub strikethrough: bool,
}

impl From<&Color> for ColorInfo {
    fn from(color: &Color) -> Self {
        match color {
            Color::Default => ColorInfo::Default,
            Color::Named(n) => ColorInfo::Named(format!("{:?}", n)),
            Color::Indexed(i) => ColorInfo::Indexed(*i),
            Color::Rgb(r, g, b) => ColorInfo::Rgb(*r, *g, *b),
        }
    }
}

impl From<&CellAttributes> for AttrsInfo {
    fn from(attrs: &CellAttributes) -> Self {
        AttrsInfo {
            bold: attrs.bold,
            dim: attrs.dim,
            italic: attrs.italic,
            underline: attrs.underline,
            blink: attrs.blink,
            reverse: attrs.reverse,
            hidden: attrs.hidden,
            strikethrough: attrs.strikethrough,
        }
    }
}

/// Convert a TerminalCell to a compact CellInfo (omits defaults)
pub fn cell_to_info(cell: &TerminalCell) -> CellInfo {
    let fg = if cell.fg != Color::Default {
        Some(ColorInfo::from(&cell.fg))
    } else {
        None
    };

    let bg = if cell.bg != Color::Default {
        Some(ColorInfo::from(&cell.bg))
    } else {
        None
    };

    let attrs = if cell.attrs != CellAttributes::default() {
        Some(AttrsInfo::from(&cell.attrs))
    } else {
        None
    };

    CellInfo {
        c: cell.c.to_string(),
        fg,
        bg,
        attrs,
        wide: cell.wide,
    }
}
