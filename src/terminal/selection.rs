use serde::{Deserialize, Serialize};

/// Selection type determines how text is selected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionType {
    Character,
    Word,
    Line,
    Block,
}

/// Selection state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    Active,
    Complete,
}

/// Represents a position in the terminal grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub col: u16,
    pub row: u16,
}

impl Position {
    pub fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }
}

/// Represents a text selection in the terminal
#[derive(Debug, Clone)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
    pub selection_type: SelectionType,
    pub state: SelectionState,
}

impl Selection {
    pub fn new(start: Position, selection_type: SelectionType) -> Self {
        Self {
            start,
            end: start,
            selection_type,
            state: SelectionState::Active,
        }
    }

    /// Create a range selection from start to end
    pub fn from_range(start: Position, end: Position) -> Self {
        Self {
            start,
            end,
            selection_type: SelectionType::Character,
            state: SelectionState::Complete,
        }
    }

    #[allow(dead_code)]
    pub fn update_end(&mut self, end: Position) {
        self.end = end;
    }

    pub fn complete(&mut self) {
        self.state = SelectionState::Complete;
    }

    /// Check if a position is within the selection
    #[allow(dead_code)]
    pub fn contains(&self, pos: Position) -> bool {
        let (start, end) = self.normalized_bounds();

        match self.selection_type {
            SelectionType::Block => {
                let min_col = start.col.min(end.col);
                let max_col = start.col.max(end.col);
                let min_row = start.row.min(end.row);
                let max_row = start.row.max(end.row);

                pos.col >= min_col && pos.col <= max_col && pos.row >= min_row && pos.row <= max_row
            }
            _ => {
                let start_idx = (start.row as usize * 1000) + start.col as usize;
                let end_idx = (end.row as usize * 1000) + end.col as usize;
                let pos_idx = (pos.row as usize * 1000) + pos.col as usize;

                pos_idx >= start_idx && pos_idx <= end_idx
            }
        }
    }

    /// Get normalized bounds (ensures start <= end)
    pub fn normalized_bounds(&self) -> (Position, Position) {
        match self.selection_type {
            SelectionType::Block => (self.start, self.end),
            _ => {
                let start_idx = (self.start.row as usize * 1000) + self.start.col as usize;
                let end_idx = (self.end.row as usize * 1000) + self.end.col as usize;

                if start_idx <= end_idx {
                    (self.start, self.end)
                } else {
                    (self.end, self.start)
                }
            }
        }
    }

    /// Expand selection to word boundaries
    pub fn expand_to_word(&mut self, get_char: impl Fn(Position) -> Option<char>) {
        self.selection_type = SelectionType::Word;

        let mut start = self.start;
        while start.col > 0 {
            let prev_pos = Position::new(start.col - 1, start.row);
            if let Some(ch) = get_char(prev_pos) {
                if is_word_char(ch) {
                    start = prev_pos;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let mut end = self.end;
        while let Some(ch) = get_char(end) {
            if is_word_char(ch) {
                end.col += 1;
            } else {
                break;
            }
        }

        self.start = start;
        self.end = end;
    }

    /// Expand selection to line boundaries
    #[allow(dead_code)]
    pub fn expand_to_line(&mut self, width: u16) {
        self.selection_type = SelectionType::Line;
        let (start, end) = self.normalized_bounds();
        self.start = Position::new(0, start.row);
        self.end = Position::new(width.saturating_sub(1), end.row);
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_contains() {
        let mut sel = Selection::new(Position::new(5, 0), SelectionType::Character);
        sel.update_end(Position::new(10, 0));

        assert!(sel.contains(Position::new(5, 0)));
        assert!(sel.contains(Position::new(7, 0)));
        assert!(sel.contains(Position::new(10, 0)));
        assert!(!sel.contains(Position::new(4, 0)));
        assert!(!sel.contains(Position::new(11, 0)));
    }

    #[test]
    fn test_from_range() {
        let sel = Selection::from_range(Position::new(2, 0), Position::new(10, 0));
        assert_eq!(sel.state, SelectionState::Complete);
        assert!(sel.contains(Position::new(5, 0)));
        assert!(!sel.contains(Position::new(1, 0)));
    }

    #[test]
    fn test_normalized_bounds() {
        let mut sel = Selection::new(Position::new(10, 0), SelectionType::Character);
        sel.update_end(Position::new(5, 0));

        let (start, end) = sel.normalized_bounds();
        assert_eq!(start.col, 5);
        assert_eq!(end.col, 10);
    }
}
