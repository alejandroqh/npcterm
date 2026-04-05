use serde::{Deserialize, Serialize};

/// Mouse action that can be performed by the AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum MouseAction {
    LeftClick { col: u16, row: u16 },
    RightClick { col: u16, row: u16 },
    DoubleClick { col: u16, row: u16 },
    MoveTo { col: u16, row: u16 },
    GetPosition,
    SetPosition { col: u16, row: u16 },
}

/// Current mouse state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MouseState {
    pub col: u16,
    pub row: u16,
}

impl Default for MouseState {
    fn default() -> Self {
        Self { col: 0, row: 0 }
    }
}

impl MouseState {
    pub fn set(&mut self, col: u16, row: u16) {
        self.col = col;
        self.row = row;
    }
}

/// Generate SGR mouse escape sequences for terminals that have requested mouse tracking
pub fn sgr_mouse_press(button: u8, col: u16, row: u16) -> Vec<u8> {
    // SGR format: CSI < button ; col ; row M (press) or m (release)
    // button: 0 = left, 1 = middle, 2 = right
    // col/row are 1-based
    format!("\x1b[<{};{};{}M", button, col + 1, row + 1).into_bytes()
}

pub fn sgr_mouse_release(button: u8, col: u16, row: u16) -> Vec<u8> {
    format!("\x1b[<{};{};{}m", button, col + 1, row + 1).into_bytes()
}

pub fn sgr_mouse_move(col: u16, row: u16) -> Vec<u8> {
    // Button 35 = motion with no buttons
    format!("\x1b[<35;{};{}M", col + 1, row + 1).into_bytes()
}

/// Result of a mouse action
#[derive(Debug, Clone, Serialize)]
pub struct MouseResult {
    pub mouse_col: u16,
    pub mouse_row: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
}
