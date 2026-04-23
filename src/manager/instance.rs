use std::time::Instant;

use crate::input::keys::Key;
use crate::input::mouse::{self, MouseAction, MouseResult, MouseState};
use crate::screen::reader;
use crate::status::events::{EventQueue, TerminalEvent};
use crate::status::query::{TerminalState, TerminalStatus};
use crate::terminal::emulator::TerminalEmulator;
use crate::terminal::selection::{Position, Selection};

/// Throttle interval for `get_foreground_process_name` (expensive: spawns ps)
const PROCESS_NAME_CACHE_MS: u64 = 500;

/// A fully self-contained terminal instance (isolation for future tabs)
pub struct TerminalInstance {
    pub id: String,
    emulator: TerminalEmulator,
    mouse_state: MouseState,
    selection: Option<Selection>,
    scroll_offset: usize,
    event_queue: EventQueue,
    process_state: TerminalState,
    prev_state: TerminalState,
    created_at: chrono::DateTime<chrono::Utc>,
    /// Cached foreground process name (throttled to avoid spawning ps every tick)
    cached_process_name: Option<String>,
    last_process_name_check: Option<Instant>,
    /// Scrollback length at time of last agent read (for incremental reads)
    last_read_scrollback_len: usize,
}

impl TerminalInstance {
    pub fn new(
        id: String,
        cols: usize,
        rows: usize,
        shell: Option<&str>,
    ) -> std::io::Result<Self> {
        let emulator = TerminalEmulator::new(cols, rows, shell)?;

        Ok(Self {
            id,
            emulator,
            mouse_state: MouseState::default(),
            selection: None,
            scroll_offset: 0,
            event_queue: EventQueue::default(),
            process_state: TerminalState::Running,
            prev_state: TerminalState::Running,
            created_at: chrono::Utc::now(),
            cached_process_name: None,
            last_process_name_check: None,
            last_read_scrollback_len: 0,
        })
    }

    pub fn cols(&self) -> usize {
        self.emulator.grid.cols()
    }

    pub fn rows(&self) -> usize {
        self.emulator.grid.rows()
    }

    /// Process PTY output and update internal state. Call periodically.
    pub fn tick(&mut self) {
        let was_alive = self.process_state != TerminalState::Exited;
        let running = self.emulator.process_output();

        if self.emulator.grid.bell_pending {
            self.emulator.grid.bell_pending = false;
            self.event_queue.push(TerminalEvent::Bell);
        }

        // Only emit ScreenChanged when new dirty rows appear since last event
        if self.emulator.grid.is_dirty() {
            let dirty = self.emulator.grid.take_dirty_rows();
            if !dirty.is_empty() {
                self.event_queue
                    .push(TerminalEvent::ScreenChanged { changed_rows: dirty });
            }
        }

        // Throttled process name lookup
        self.refresh_process_name();

        let is_shell = self
            .cached_process_name
            .as_ref()
            .is_some_and(|n| matches!(n.as_str(), "bash" | "zsh" | "sh" | "fish" | "dash" | "tcsh" | "csh"));

        let last_output_ms = self
            .emulator
            .last_output_time
            .map(|t| t.elapsed().as_millis() as u64);

        let new_state = TerminalState::detect(running, self.emulator.exit_code, last_output_ms, is_shell);

        if new_state != self.prev_state {
            self.event_queue
                .push(TerminalEvent::ProcessStateChanged {
                    old: self.prev_state,
                    new: new_state,
                });

            if new_state == TerminalState::Exited && was_alive {
                self.event_queue.push(TerminalEvent::CommandFinished {
                    exit_code: self.emulator.exit_code.unwrap_or(-1),
                });
            }

            if new_state == TerminalState::WaitingForInput {
                self.event_queue.push(TerminalEvent::WaitingForInput);
            }

            self.prev_state = new_state;
        }

        self.process_state = new_state;
    }

    /// Send a keystroke and flush immediately
    pub fn send_key(&mut self, key: Key) -> std::io::Result<()> {
        self.send_key_no_flush(key)?;
        self.flush_input()
    }

    /// Send a special key without flushing (for batch operations)
    pub fn send_key_no_flush(&mut self, key: Key) -> std::io::Result<()> {
        let seq = key.to_escape_sequence(self.emulator.grid.application_cursor_keys);
        self.write_raw(&seq)
    }

    /// Write raw bytes to the terminal without flushing
    pub fn write_raw(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.emulator.write_input(data)
    }

    pub fn flush_input(&mut self) -> std::io::Result<()> {
        self.emulator.flush_input()
    }

    /// Send a mouse action
    pub fn send_mouse(&mut self, action: MouseAction) -> MouseResult {
        let has_mouse_tracking = self.emulator.grid.mouse_normal_tracking
            || self.emulator.grid.mouse_button_tracking
            || self.emulator.grid.mouse_any_event_tracking;
        let uses_sgr = self.emulator.grid.mouse_sgr_mode;

        match action {
            MouseAction::LeftClick { col, row } => {
                self.send_mouse_click(0, col, row, has_mouse_tracking && uses_sgr)
            }
            MouseAction::RightClick { col, row } => {
                self.send_mouse_click(2, col, row, has_mouse_tracking && uses_sgr)
            }
            MouseAction::DoubleClick { col, row } => {
                self.mouse_state.set(col, row);
                let mut sel = Selection::new(
                    Position::new(col, row),
                    crate::terminal::selection::SelectionType::Word,
                );
                let grid = &self.emulator.grid;
                sel.expand_to_word(|pos| {
                    grid.get_cell(pos.col as usize, pos.row as usize)
                        .map(|c| c.c)
                });
                sel.complete();
                let text = self.extract_selection_text(&sel);
                self.selection = Some(sel);
                MouseResult {
                    mouse_col: col,
                    mouse_row: row,
                    selected_text: Some(text),
                }
            }
            MouseAction::MoveTo { col, row } => {
                self.mouse_state.set(col, row);
                if has_mouse_tracking && uses_sgr && self.emulator.grid.mouse_any_event_tracking {
                    let mv = mouse::sgr_mouse_move(col, row);
                    let _ = self.emulator.write_input(&mv);
                    let _ = self.emulator.flush_input();
                }
                MouseResult { mouse_col: col, mouse_row: row, selected_text: None }
            }
            MouseAction::GetPosition => MouseResult {
                mouse_col: self.mouse_state.col,
                mouse_row: self.mouse_state.row,
                selected_text: None,
            },
            MouseAction::SetPosition { col, row } => {
                self.mouse_state.set(col, row);
                MouseResult { mouse_col: col, mouse_row: row, selected_text: None }
            }
        }
    }

    fn reset_read_baseline(&mut self) {
        self.emulator.grid.take_dirty_rows();
        self.emulator.grid.clear_read_dirty();
        self.last_read_scrollback_len = self.emulator.grid.scrollback_len();
    }

    pub fn read_screen(&mut self) -> String {
        self.reset_read_baseline();
        if self.scroll_offset > 0 {
            self.render_scrollback(true)
        } else {
            reader::read_screen_text(&self.emulator.grid)
        }
    }

    pub fn show_screen(&mut self) -> String {
        self.reset_read_baseline();
        if self.scroll_offset > 0 {
            self.render_scrollback(false)
        } else {
            reader::show_screen_text(&self.emulator.grid)
        }
    }

    /// Read only new output since the last read, capped at max_lines
    pub fn read_changes(&mut self, max_lines: usize) -> String {
        use std::fmt::Write;

        let scrollback_len = self.emulator.grid.scrollback_len();

        // Handle scrollback eviction: if our cursor is past current length, lines were evicted
        let effective_start = if self.last_read_scrollback_len > scrollback_len {
            0
        } else {
            self.last_read_scrollback_len
        };

        let new_scrollback_count = scrollback_len - effective_start;

        // Take dirty indices first (mutable borrow) before immutable scrollback borrow
        let dirty_indices = self.emulator.grid.take_read_dirty_rows();

        // Update baseline
        self.last_read_scrollback_len = scrollback_len;

        if new_scrollback_count == 0 && dirty_indices.is_empty() {
            return String::from("(no changes)\n");
        }

        let mut output = String::new();

        // Compute capped ranges before rendering (avoid rendering lines we'll discard)
        let scrollback_to_skip = new_scrollback_count.saturating_sub(max_lines);
        let scrollback_budget = new_scrollback_count - scrollback_to_skip;
        let screen_budget = max_lines.saturating_sub(scrollback_budget);

        if scrollback_budget > 0 {
            output.push_str("--- scrollback ---\n");
            if scrollback_to_skip > 0 {
                let _ = write!(output, "({scrollback_to_skip} earlier lines omitted)\n");
            }
            let render_start = effective_start + scrollback_to_skip;
            let scrollback = self.emulator.grid.get_scrollback();
            reader::append_scrollback_lines(&mut output, scrollback, render_start, scrollback_len);
        }

        if screen_budget > 0 && !dirty_indices.is_empty() {
            let screen_indices = if dirty_indices.len() > screen_budget {
                &dirty_indices[dirty_indices.len() - screen_budget..]
            } else {
                &dirty_indices[..]
            };
            output.push_str("--- screen ---\n");
            output.push_str(&reader::read_changed_rows_text(
                &self.emulator.grid,
                screen_indices,
            ));
        }

        output
    }

    pub fn read_rows(&self, start: usize, end: usize) -> String {
        reader::read_rows_text(&self.emulator.grid, start, end)
    }

    pub fn read_region(&self, col1: usize, row1: usize, col2: usize, row2: usize) -> String {
        reader::read_region_text(&self.emulator.grid, col1, row1, col2, row2)
    }

    pub fn get_status(&self, last_n: usize) -> TerminalStatus {
        let last_lines = reader::last_n_lines(&self.emulator.grid, last_n);
        let dirty_rows = if self.emulator.grid.is_dirty() {
            self.emulator.grid.peek_dirty_rows()
        } else {
            Vec::new()
        };

        TerminalStatus {
            state: self.process_state,
            exit_code: self.emulator.exit_code,
            running_command: self.cached_process_name.clone(),
            last_lines,
            cursor_pos: (self.emulator.grid.cursor.x, self.emulator.grid.cursor.y),
            cursor_visible: self.emulator.grid.cursor.visible,
            mouse_pos: (self.mouse_state.col, self.mouse_state.row),
            dirty: self.emulator.grid.is_dirty(),
            changed_rows: dirty_rows,
            pending_events: self.event_queue.len(),
            size: (self.cols(), self.rows()),
            scrollback_lines: self.emulator.grid.scrollback_len(),
            has_new_content: self.emulator.grid.has_read_dirty()
                || self.emulator.grid.scrollback_len() > self.last_read_scrollback_len,
        }
    }

    pub fn poll_events(&mut self) -> Vec<TerminalEvent> {
        self.event_queue.drain()
    }

    pub fn select_range(
        &mut self,
        start_col: usize,
        start_row: usize,
        end_col: usize,
        end_row: usize,
    ) -> String {
        let sel = Selection::from_range(
            Position::new(start_col as u16, start_row as u16),
            Position::new(end_col as u16, end_row as u16),
        );
        let text = self.extract_selection_text(&sel);
        self.selection = Some(sel);
        text
    }

    pub fn scroll_page_up(&mut self) -> usize {
        let max = self.emulator.grid.scrollback_len();
        self.scroll_offset = (self.scroll_offset + self.rows()).min(max);
        self.scroll_offset
    }

    pub fn scroll_page_down(&mut self) -> usize {
        self.scroll_offset = self.scroll_offset.saturating_sub(self.rows());
        self.scroll_offset
    }

    pub fn scroll_to_text(&mut self, text: &str) -> (usize, bool) {
        let scrollback = self.emulator.grid.get_scrollback();
        let text_lower = text.to_lowercase();

        for (i, line) in scrollback.iter().enumerate().rev() {
            let line_text: String = line.iter().map(|c| c.c).collect();
            if line_text.to_lowercase().contains(&text_lower) {
                self.scroll_offset = scrollback.len() - i;
                return (self.scroll_offset, true);
            }
        }

        for line in self.emulator.grid.get_rows() {
            let line_text: String = line.iter().map(|c| c.c).collect();
            if line_text.to_lowercase().contains(&text_lower) {
                self.scroll_offset = 0;
                return (0, true);
            }
        }

        (self.scroll_offset, false)
    }

    pub fn created_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.created_at
    }

    pub fn state(&self) -> TerminalState {
        self.process_state
    }

    pub fn running_command(&self) -> Option<String> {
        self.cached_process_name.clone()
    }

    /// Forward the terminal to an external terminal emulator
    ///
    /// Returns the name of the terminal emulator that was launched, or an error
    pub fn forward_terminal(&self) -> std::io::Result<String> {
        use crate::terminal::forwarder;

        let slave_name = self.emulator.get_slave_name()
            .map(|p| p.to_string_lossy().to_string())
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "PTY forwarding is only supported on Unix-like systems",
            ))?;

        let pid = self.emulator.get_child_pid()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Child process not available",
            ))?;

        let result = forwarder::launch(&slave_name, pid)?;
        Ok(result.terminal_name)
    }

    // --- Viewer support ---

    /// Get a reference to the grid (for viewer span compression)
    #[cfg(feature = "viewer")]
    pub fn grid(&self) -> &crate::terminal::grid::TerminalGrid {
        &self.emulator.grid
    }

    /// Take viewer-dirty row indices (clears viewer dirty state)
    #[cfg(feature = "viewer")]
    pub fn take_viewer_dirty(&mut self) -> Vec<usize> {
        self.emulator.grid.take_viewer_dirty_rows()
    }

    /// Check if any rows changed since last viewer broadcast
    #[cfg(feature = "viewer")]
    pub fn has_viewer_dirty(&self) -> bool {
        self.emulator.grid.has_viewer_dirty()
    }

    // --- Private helpers ---

    fn refresh_process_name(&mut self) {
        let should_refresh = self
            .last_process_name_check
            .map(|t| t.elapsed().as_millis() as u64 >= PROCESS_NAME_CACHE_MS)
            .unwrap_or(true);

        if should_refresh {
            self.cached_process_name = self.emulator.get_foreground_process_name();
            self.last_process_name_check = Some(Instant::now());
        }
    }

    fn send_mouse_click(&mut self, button: u8, col: u16, row: u16, send_sgr: bool) -> MouseResult {
        self.mouse_state.set(col, row);
        if send_sgr {
            let press = mouse::sgr_mouse_press(button, col, row);
            let _ = self.emulator.write_input(&press);
            let release = mouse::sgr_mouse_release(button, col, row);
            let _ = self.emulator.write_input(&release);
            let _ = self.emulator.flush_input();
        }
        MouseResult { mouse_col: col, mouse_row: row, selected_text: None }
    }

    fn extract_selection_text(&self, sel: &Selection) -> String {
        let (start, end) = sel.normalized_bounds();
        let mut text = String::new();
        let grid = &self.emulator.grid;
        let cols = self.cols();

        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col as usize } else { 0 };
            let col_end = if row == end.row { end.col as usize + 1 } else { cols };

            for col in col_start..col_end {
                if let Some(cell) = grid.get_cell(col, row as usize) {
                    if !cell.wide || cell.c != ' ' {
                        text.push(cell.c);
                    }
                }
            }

            if row != end.row {
                let trimmed = text.trim_end_matches(' ').len();
                text.truncate(trimmed);
                text.push('\n');
            }
        }

        let trimmed = text.trim_end_matches(' ').len();
        text.truncate(trimmed);
        text
    }

    fn render_scrollback(&self, with_coords: bool) -> String {
        let cols = self.cols();
        let rows = self.rows();
        let scrollback = self.emulator.grid.get_scrollback();
        let screen = self.emulator.grid.get_rows();

        let extra = if with_coords { cols + 4 } else { cols + 1 };
        let header_rows = if with_coords { 3 } else { 0 };
        let mut output = String::with_capacity(extra * (rows + header_rows));

        if with_coords {
            reader::write_column_header(&mut output, cols);
        }

        reader::render_scrollback(
            &mut output, scrollback, screen,
            self.scroll_offset, cols, rows, with_coords,
        );

        output
    }
}
