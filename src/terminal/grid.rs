use std::collections::VecDeque;
use std::fmt;
use unicode_width::UnicodeWidthChar;

use super::cell::{
    CellAttributes, CharacterSet, Color, Cursor, NamedColor, SavedCursorState,
    TerminalCell,
};

/// Terminal grid with scrollback buffer and dirty tracking
pub struct TerminalGrid {
    /// Screen rows (visible portion)
    rows: Vec<Vec<TerminalCell>>,
    /// Scrollback buffer (lines that have scrolled off the top)
    scrollback: VecDeque<Vec<TerminalCell>>,
    /// Maximum scrollback lines
    max_scrollback: usize,
    /// Terminal dimensions
    cols: usize,
    rows_count: usize,
    /// Cursor state
    pub cursor: Cursor,
    /// Current cell attributes (for new characters)
    pub current_attrs: CellAttributes,
    pub current_fg: Color,
    pub current_bg: Color,
    /// Scroll region (for CSI scrolling)
    scroll_region_top: usize,
    scroll_region_bottom: usize,
    /// Saved cursor state (for DECSC/DECRC)
    saved_cursor: Option<SavedCursorState>,
    /// Alternate screen buffer
    alt_screen: Option<Vec<Vec<TerminalCell>>>,
    /// Tab stops (every 8 columns by default)
    tab_stops: Vec<bool>,
    /// DEC Private Modes
    pub application_cursor_keys: bool,
    pub bracketed_paste_mode: bool,
    pub focus_event_mode: bool,
    pub synchronized_output: bool,
    sync_snapshot: Option<Vec<Vec<TerminalCell>>>,
    sync_cursor_snapshot: Option<Cursor>,
    /// Mouse tracking modes
    pub mouse_normal_tracking: bool,
    pub mouse_button_tracking: bool,
    pub mouse_any_event_tracking: bool,
    pub mouse_utf8_mode: bool,
    pub mouse_sgr_mode: bool,
    pub mouse_urxvt_mode: bool,
    /// Line Feed/New Line Mode (LNM)
    pub lnm_mode: bool,
    /// Auto-wrap mode (DECAWM)
    pub auto_wrap_mode: bool,
    /// Pending wrap flag
    wrap_pending: bool,
    /// Insert/Replace mode (IRM)
    pub insert_mode: bool,
    /// Origin mode (DECOM)
    pub origin_mode: bool,
    /// Response queue for DSR and other queries
    response_queue: Vec<String>,
    /// Character sets
    pub charset_g0: CharacterSet,
    pub charset_g1: CharacterSet,
    pub charset_use_g0: bool,
    /// Generation counter - incremented on content changes
    generation: u64,

    // === AiTerm39 additions ===
    /// Track which rows changed since last read
    dirty_rows: Vec<bool>,
    /// Global dirty flag
    dirty_flag: bool,
    /// Bell character received
    pub bell_pending: bool,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize, max_scrollback: usize) -> Self {
        let mut tab_stops = vec![false; cols];
        for i in (0..cols).step_by(8) {
            tab_stops[i] = true;
        }

        Self {
            rows: vec![vec![TerminalCell::default(); cols]; rows],
            scrollback: VecDeque::new(),
            max_scrollback,
            cols,
            rows_count: rows,
            cursor: Cursor::default(),
            current_attrs: CellAttributes::default(),
            current_fg: Color::Default,
            current_bg: Color::Default,
            scroll_region_top: 0,
            scroll_region_bottom: rows.saturating_sub(1),
            saved_cursor: None,
            alt_screen: None,
            tab_stops,
            application_cursor_keys: false,
            bracketed_paste_mode: false,
            focus_event_mode: false,
            synchronized_output: false,
            sync_snapshot: None,
            sync_cursor_snapshot: None,
            mouse_normal_tracking: false,
            mouse_button_tracking: false,
            mouse_any_event_tracking: false,
            mouse_utf8_mode: false,
            mouse_sgr_mode: false,
            mouse_urxvt_mode: false,
            lnm_mode: false,
            auto_wrap_mode: true,
            wrap_pending: false,
            insert_mode: false,
            origin_mode: false,
            response_queue: Vec::new(),
            charset_g0: CharacterSet::Ascii,
            charset_g1: CharacterSet::Ascii,
            charset_use_g0: true,
            generation: 0,
            dirty_rows: vec![false; rows],
            dirty_flag: false,
            bell_pending: false,
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows_count
    }

    pub fn scroll_region_top(&self) -> usize {
        self.scroll_region_top
    }

    pub fn scroll_region_bottom(&self) -> usize {
        self.scroll_region_bottom
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get all visible rows
    pub fn get_rows(&self) -> &[Vec<TerminalCell>] {
        &self.rows
    }

    /// Get scrollback buffer
    pub fn get_scrollback(&self) -> &VecDeque<Vec<TerminalCell>> {
        &self.scrollback
    }

    /// Take dirty rows (returns changed indices and clears tracking)
    pub fn take_dirty_rows(&mut self) -> Vec<usize> {
        let dirty: Vec<usize> = self
            .dirty_rows
            .iter()
            .enumerate()
            .filter(|(_, d)| **d)
            .map(|(i, _)| i)
            .collect();
        self.dirty_rows.fill(false);
        self.dirty_flag = false;
        dirty
    }

    /// Check if screen has changed since last read
    pub fn is_dirty(&self) -> bool {
        self.dirty_flag
    }

    /// Get which rows are dirty without clearing
    pub fn peek_dirty_rows(&self) -> Vec<usize> {
        self.dirty_rows
            .iter()
            .enumerate()
            .filter(|(_, d)| **d)
            .map(|(i, _)| i)
            .collect()
    }

    fn mark_dirty(&mut self, row: usize) {
        if row < self.dirty_rows.len() {
            self.dirty_rows[row] = true;
        }
        self.dirty_flag = true;
    }

    fn mark_all_dirty(&mut self) {
        self.dirty_rows.fill(true);
        self.dirty_flag = true;
    }

    /// Get the currently active character set
    pub fn active_charset(&self) -> CharacterSet {
        if self.charset_use_g0 {
            self.charset_g0
        } else {
            self.charset_g1
        }
    }

    /// Map a character through the active character set
    pub fn map_char(&self, c: char) -> char {
        match self.active_charset() {
            CharacterSet::Ascii => c,
            CharacterSet::DecSpecialGraphics => match c {
                '_' => ' ',
                '`' => '◆',
                'a' => '▒',
                'b' => '␉',
                'c' => '␌',
                'd' => '␍',
                'e' => '␊',
                'f' => '°',
                'g' => '±',
                'h' => '␤',
                'i' => '␋',
                'j' => '┘',
                'k' => '┐',
                'l' => '┌',
                'm' => '└',
                'n' => '┼',
                'o' => '⎺',
                'p' => '⎻',
                'q' => '─',
                'r' => '⎼',
                's' => '⎽',
                't' => '├',
                'u' => '┤',
                'v' => '┴',
                'w' => '┬',
                'x' => '│',
                'y' => '≤',
                'z' => '≥',
                '{' => 'π',
                '|' => '≠',
                '}' => '£',
                '~' => '·',
                _ => c,
            },
        }
    }

    pub fn set_charset_g0(&mut self, charset: CharacterSet) {
        self.charset_g0 = charset;
    }

    pub fn set_charset_g1(&mut self, charset: CharacterSet) {
        self.charset_g1 = charset;
    }

    pub fn shift_in(&mut self) {
        self.charset_use_g0 = true;
    }

    pub fn shift_out(&mut self) {
        self.charset_use_g0 = false;
    }

    pub fn queue_response(&mut self, response: String) {
        self.response_queue.push(response);
    }

    pub fn take_responses(&mut self) -> Vec<String> {
        std::mem::take(&mut self.response_queue)
    }

    pub fn queue_cursor_position_report(&mut self) {
        let (row, col) = if self.origin_mode {
            let row = (self.cursor.y.saturating_sub(self.scroll_region_top)) + 1;
            let col = self.cursor.x + 1;
            (row, col)
        } else {
            let row = self.cursor.y + 1;
            let col = self.cursor.x + 1;
            (row, col)
        };
        let response = format!("\x1b[{};{}R", row, col);
        self.queue_response(response);
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&TerminalCell> {
        self.rows.get(y)?.get(x)
    }

    pub fn get_render_cell(&self, x: usize, y: usize) -> Option<&TerminalCell> {
        if let Some(snapshot) = &self.sync_snapshot {
            snapshot.get(y)?.get(x)
        } else {
            self.rows.get(y)?.get(x)
        }
    }

    pub fn get_render_cursor(&self) -> &Cursor {
        if let Some(cursor) = &self.sync_cursor_snapshot {
            cursor
        } else {
            &self.cursor
        }
    }

    pub fn get_cell_mut(&mut self, x: usize, y: usize) -> Option<&mut TerminalCell> {
        self.rows.get_mut(y)?.get_mut(x)
    }

    pub fn get_scrollback_line(&self, idx: usize) -> Option<&Vec<TerminalCell>> {
        self.scrollback.get(idx)
    }

    pub fn begin_synchronized_output(&mut self) {
        if !self.synchronized_output {
            self.synchronized_output = true;
            self.sync_snapshot = Some(self.rows.clone());
            self.sync_cursor_snapshot = Some(self.cursor);
        }
    }

    pub fn end_synchronized_output(&mut self) {
        self.synchronized_output = false;
        self.sync_snapshot = None;
        self.sync_cursor_snapshot = None;
        self.generation = self.generation.wrapping_add(1);
        self.mark_all_dirty();
    }

    /// Write a character at the current cursor position
    pub fn put_char(&mut self, c: char) {
        if self.cursor.y >= self.rows_count {
            return;
        }

        match c {
            '\n' => self.linefeed(),
            '\r' => self.carriage_return(),
            '\t' => self.tab(),
            '\x08' => self.backspace(),
            c if c.is_control() => {}
            c => {
                let c = self.map_char(c);
                let char_width = if c.is_ascii() {
                    1
                } else {
                    c.width().unwrap_or(0)
                };

                if char_width == 0 {
                    return;
                }

                if self.wrap_pending && self.auto_wrap_mode {
                    self.wrap_pending = false;
                    if self.cursor.y == self.rows_count - 1 {
                        self.scroll_up(1);
                        self.cursor.x = 0;
                    } else {
                        self.cursor.x = 0;
                        self.cursor.y += 1;
                    }
                }

                let fg = self.current_fg;
                let bg = self.current_bg;
                let attrs = self.current_attrs;

                if char_width == 2 {
                    if self.cursor.x + 1 >= self.cols {
                        if self.auto_wrap_mode {
                            if self.cursor.y == self.rows_count - 1 {
                                self.scroll_up(1);
                                self.cursor.x = 0;
                            } else {
                                self.cursor.x = 0;
                                self.cursor.y += 1;
                            }
                        } else {
                            return;
                        }
                    }

                    if let Some(cell) = self.get_cell_mut(self.cursor.x, self.cursor.y) {
                        cell.c = c;
                        cell.fg = fg;
                        cell.bg = bg;
                        cell.attrs = attrs;
                        cell.wide = true;
                    }

                    if let Some(cell) = self.get_cell_mut(self.cursor.x + 1, self.cursor.y) {
                        cell.c = ' ';
                        cell.fg = fg;
                        cell.bg = bg;
                        cell.attrs = attrs;
                        cell.wide = false;
                    }

                    self.mark_dirty(self.cursor.y);
                    self.cursor.x += 2;

                    if self.cursor.x >= self.cols {
                        if self.auto_wrap_mode {
                            self.cursor.x = self.cols - 1;
                            self.wrap_pending = true;
                        } else {
                            self.cursor.x = self.cols - 1;
                        }
                    }
                } else {
                    if self.cursor.x < self.cols {
                        if let Some(cell) = self.get_cell_mut(self.cursor.x, self.cursor.y) {
                            cell.c = c;
                            cell.fg = fg;
                            cell.bg = bg;
                            cell.attrs = attrs;
                            cell.wide = false;
                        }
                        self.mark_dirty(self.cursor.y);
                        self.cursor.x += 1;

                        if self.cursor.x >= self.cols {
                            if self.auto_wrap_mode {
                                self.cursor.x = self.cols - 1;
                                self.wrap_pending = true;
                            } else {
                                self.cursor.x = self.cols - 1;
                            }
                        }
                    }
                }

                self.generation = self.generation.wrapping_add(1);
            }
        }
    }

    fn linefeed(&mut self) {
        self.wrap_pending = false;
        if self.lnm_mode {
            self.cursor.x = 0;
        }
        if self.cursor.y == self.scroll_region_bottom {
            self.scroll_up(1);
        } else if self.cursor.y < self.rows_count - 1 {
            self.cursor.y += 1;
        }
    }

    fn carriage_return(&mut self) {
        self.wrap_pending = false;
        self.cursor.x = 0;
    }

    pub fn reverse_linefeed(&mut self) {
        if self.cursor.y == self.scroll_region_top {
            self.scroll_down(1);
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
        }
    }

    pub fn next_line(&mut self) {
        self.carriage_return();
        self.linefeed();
    }

    pub fn reset(&mut self) {
        self.clear_screen();
        self.cursor = Cursor::default();
        self.current_attrs = CellAttributes::default();
        self.current_fg = Color::Default;
        self.current_bg = Color::Default;
        self.scroll_region_top = 0;
        self.scroll_region_bottom = self.rows_count.saturating_sub(1);
        self.saved_cursor = None;
        self.alt_screen = None;
        self.scrollback.clear();
        self.application_cursor_keys = false;
        self.bracketed_paste_mode = false;
        self.focus_event_mode = false;
        self.synchronized_output = false;
        self.sync_snapshot = None;
        self.sync_cursor_snapshot = None;
        self.mouse_normal_tracking = false;
        self.mouse_button_tracking = false;
        self.mouse_any_event_tracking = false;
        self.mouse_utf8_mode = false;
        self.mouse_sgr_mode = false;
        self.mouse_urxvt_mode = false;
        self.lnm_mode = false;
        self.auto_wrap_mode = true;
        self.wrap_pending = false;
        self.insert_mode = false;
        self.origin_mode = false;
        self.charset_g0 = CharacterSet::Ascii;
        self.charset_g1 = CharacterSet::Ascii;
        self.charset_use_g0 = true;
        self.response_queue.clear();
        self.mark_all_dirty();
    }

    fn tab(&mut self) {
        self.wrap_pending = false;
        for x in (self.cursor.x + 1)..self.cols {
            if self.tab_stops[x] {
                self.cursor.x = x;
                return;
            }
        }
        self.cursor.x = self.cols.saturating_sub(1);
    }

    fn backspace(&mut self) {
        self.wrap_pending = false;
        if self.cursor.x > 0 {
            self.cursor.x -= 1;
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        for _ in 0..n {
            if self.scroll_region_top < self.rows_count {
                let line = self.rows.remove(self.scroll_region_top);

                if self.alt_screen.is_none() {
                    self.scrollback.push_back(line);
                    if self.scrollback.len() > self.max_scrollback {
                        self.scrollback.pop_front();
                    }
                }

                let insert_pos = self.scroll_region_bottom.min(self.rows_count - 1);
                self.rows
                    .insert(insert_pos, vec![TerminalCell::default(); self.cols]);
            }
        }
        self.generation = self.generation.wrapping_add(1);
        self.mark_all_dirty();
    }

    pub fn scroll_down(&mut self, n: usize) {
        for _ in 0..n {
            if self.scroll_region_bottom < self.rows_count {
                self.rows.remove(self.scroll_region_bottom);
                self.rows.insert(
                    self.scroll_region_top,
                    vec![TerminalCell::default(); self.cols],
                );
            }
        }
        self.generation = self.generation.wrapping_add(1);
        self.mark_all_dirty();
    }

    pub fn clear_screen(&mut self) {
        let bg = self.current_bg;
        for row in &mut self.rows {
            for cell in row {
                cell.c = ' ';
                cell.fg = Color::Named(NamedColor::White);
                cell.bg = bg;
                cell.attrs = CellAttributes::default();
                cell.wide = false;
            }
        }
        self.generation = self.generation.wrapping_add(1);
        self.mark_all_dirty();
    }

    pub fn clear_line(&mut self) {
        let bg = self.current_bg;
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            for cell in row {
                cell.c = ' ';
                cell.fg = Color::Named(NamedColor::White);
                cell.bg = bg;
                cell.attrs = CellAttributes::default();
                cell.wide = false;
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn erase_to_eol(&mut self) {
        let bg = self.current_bg;
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            for x in self.cursor.x..self.cols {
                if let Some(cell) = row.get_mut(x) {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn erase_to_bol(&mut self) {
        let bg = self.current_bg;
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            for x in 0..=self.cursor.x {
                if let Some(cell) = row.get_mut(x) {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn erase_to_eos(&mut self) {
        self.erase_to_eol();
        let bg = self.current_bg;
        for y in (self.cursor.y + 1)..self.rows_count {
            if let Some(row) = self.rows.get_mut(y) {
                for cell in row {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.mark_dirty(y);
        }
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn erase_from_bos(&mut self) {
        let bg = self.current_bg;
        for y in 0..self.cursor.y {
            if let Some(row) = self.rows.get_mut(y) {
                for cell in row {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.mark_dirty(y);
        }
        self.erase_to_bol();
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn delete_chars(&mut self, n: usize) {
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            let start = self.cursor.x;
            let end = self.cols;
            let n = n.min(end.saturating_sub(start));

            for x in start..(end - n) {
                if let Some(src_cell) = row.get(x + n).cloned() {
                    if let Some(cell) = row.get_mut(x) {
                        *cell = src_cell;
                    }
                }
            }

            let bg = self.current_bg;
            for x in (end - n)..end {
                if let Some(cell) = row.get_mut(x) {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn insert_chars(&mut self, n: usize) {
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            let start = self.cursor.x;
            let end = self.cols;
            let n = n.min(end.saturating_sub(start));

            for x in (start + n..end).rev() {
                if let Some(src_cell) = row.get(x - n).cloned() {
                    if let Some(cell) = row.get_mut(x) {
                        *cell = src_cell;
                    }
                }
            }

            let bg = self.current_bg;
            for x in start..(start + n).min(end) {
                if let Some(cell) = row.get_mut(x) {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn erase_chars(&mut self, n: usize) {
        let bg = self.current_bg;
        if let Some(row) = self.rows.get_mut(self.cursor.y) {
            for x in self.cursor.x..(self.cursor.x + n).min(self.cols) {
                if let Some(cell) = row.get_mut(x) {
                    cell.c = ' ';
                    cell.fg = Color::Named(NamedColor::White);
                    cell.bg = bg;
                    cell.attrs = CellAttributes::default();
                    cell.wide = false;
                }
            }
            self.generation = self.generation.wrapping_add(1);
            self.mark_dirty(self.cursor.y);
        }
    }

    pub fn goto(&mut self, x: usize, y: usize) {
        self.wrap_pending = false;
        self.cursor.x = x.min(self.cols.saturating_sub(1));
        self.cursor.y = y.min(self.rows_count.saturating_sub(1));
    }

    pub fn goto_origin_aware(&mut self, x: usize, y: usize) {
        self.wrap_pending = false;
        if self.origin_mode {
            let actual_y = (self.scroll_region_top + y).min(self.scroll_region_bottom);
            self.cursor.x = x.min(self.cols.saturating_sub(1));
            self.cursor.y = actual_y;
        } else {
            self.goto(x, y);
        }
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        self.wrap_pending = false;
        let new_x = (self.cursor.x as isize + dx).max(0) as usize;
        let new_y = (self.cursor.y as isize + dy).max(0) as usize;
        self.goto(new_x, new_y);
    }

    pub fn set_origin_mode(&mut self, enabled: bool) {
        self.origin_mode = enabled;
        if enabled {
            self.cursor.x = 0;
            self.cursor.y = self.scroll_region_top;
        } else {
            self.cursor.x = 0;
            self.cursor.y = 0;
        }
    }

    pub fn save_cursor_position(&mut self) {
        self.saved_cursor = Some(SavedCursorState {
            cursor: self.cursor,
            attrs: self.current_attrs,
            fg: self.current_fg,
            bg: self.current_bg,
        });
    }

    pub fn restore_cursor_position(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.cursor = saved.cursor;
        }
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(SavedCursorState {
            cursor: self.cursor,
            attrs: self.current_attrs,
            fg: self.current_fg,
            bg: self.current_bg,
        });
    }

    pub fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor {
            self.cursor = saved.cursor;
            self.current_attrs = saved.attrs;
            self.current_fg = saved.fg;
            self.current_bg = saved.bg;
        }
    }

    pub fn use_alt_screen(&mut self) {
        if self.alt_screen.is_none() {
            self.save_cursor();
            self.alt_screen = Some(self.rows.clone());
            self.clear_screen();
            self.cursor = Cursor::default();
            self.generation = self.generation.wrapping_add(1);
            self.mark_all_dirty();
        }
    }

    pub fn use_main_screen(&mut self) {
        if let Some(mut main_screen) = self.alt_screen.take() {
            let current_cols = self.cols;
            let current_rows = self.rows_count;

            for row in &mut main_screen {
                row.resize(current_cols, TerminalCell::default());
            }

            match current_rows.cmp(&main_screen.len()) {
                std::cmp::Ordering::Greater => {
                    main_screen.resize(current_rows, vec![TerminalCell::default(); current_cols]);
                }
                std::cmp::Ordering::Less => {
                    main_screen.truncate(current_rows);
                }
                std::cmp::Ordering::Equal => {}
            }

            self.rows = main_screen;
            self.restore_cursor();
            self.cursor.x = self.cursor.x.min(current_cols.saturating_sub(1));
            self.cursor.y = self.cursor.y.min(current_rows.saturating_sub(1));
            self.scroll_region_top = 0;
            self.scroll_region_bottom = current_rows.saturating_sub(1);
            self.generation = self.generation.wrapping_add(1);
            self.mark_all_dirty();
        }
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows_count.saturating_sub(2));
        let bottom = bottom.min(self.rows_count.saturating_sub(1));

        if top < bottom {
            self.scroll_region_top = top;
            self.scroll_region_bottom = bottom;
        } else {
            self.scroll_region_top = 0;
            self.scroll_region_bottom = self.rows_count.saturating_sub(1);
        }

        if self.origin_mode {
            self.cursor.x = 0;
            self.cursor.y = self.scroll_region_top;
        } else {
            self.cursor.x = 0;
            self.cursor.y = 0;
        }
    }

    pub fn set_cursor(&mut self, x: usize, y: usize, visible: bool) {
        self.cursor.x = x.min(self.cols.saturating_sub(1));
        self.cursor.y = y.min(self.rows_count.saturating_sub(1));
        self.cursor.visible = visible;
    }
}

impl fmt::Debug for TerminalGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalGrid")
            .field("cols", &self.cols)
            .field("rows", &self.rows_count)
            .field("scrollback_lines", &self.scrollback.len())
            .field("cursor", &self.cursor)
            .finish()
    }
}
