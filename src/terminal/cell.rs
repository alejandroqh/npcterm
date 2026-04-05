use serde::{Deserialize, Serialize};
use std::fmt;

/// Terminal color representation supporting 256-color palette and 24-bit truecolor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    #[default]
    Default,
    Named(NamedColor),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

/// Character cell attributes (bold, italic, underline, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellAttributes {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
}

/// A single terminal cell containing a character and its display attributes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCell {
    pub c: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttributes,
    /// Whether this cell is part of a wide character (emoji, CJK, etc.)
    pub wide: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttributes::default(),
            wide: false,
        }
    }
}

impl TerminalCell {
    pub fn reset(&mut self) {
        self.c = ' ';
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.attrs = CellAttributes::default();
        self.wide = false;
    }
}

/// Cursor position and visibility state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub shape: CursorShape,
}

/// Saved cursor state (for DECSC/DECRC)
#[derive(Debug, Clone, Copy)]
pub struct SavedCursorState {
    pub cursor: Cursor,
    pub attrs: CellAttributes,
    pub fg: Color,
    pub bg: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
}

/// VT100 Character Set designation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterSet {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            visible: true,
            shape: CursorShape::Block,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Default => write!(f, "default"),
            Color::Named(n) => write!(f, "{:?}", n),
            Color::Indexed(i) => write!(f, "idx:{}", i),
            Color::Rgb(r, g, b) => write!(f, "#{:02x}{:02x}{:02x}", r, g, b),
        }
    }
}
