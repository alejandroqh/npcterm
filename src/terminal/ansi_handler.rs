use super::cell::{CharacterSet, Color, CursorShape, NamedColor};
use super::grid::TerminalGrid;
use vte::{Params, Perform};

/// ANSI escape sequence handler that implements the VTE Perform trait
pub struct AnsiHandler<'a> {
    pub grid: &'a mut TerminalGrid,
}

impl<'a> AnsiHandler<'a> {
    pub fn new(grid: &'a mut TerminalGrid) -> Self {
        Self { grid }
    }

    fn parse_param_with_default(param: Option<&[u16]>, default: u16) -> u16 {
        param
            .and_then(|p| p.first())
            .copied()
            .map(|v| if v == 0 { default } else { v })
            .unwrap_or(default)
    }

    fn handle_sgr(&mut self, params: &Params) {
        if params.is_empty() {
            self.grid.current_attrs = Default::default();
            self.grid.current_fg = Color::Default;
            self.grid.current_bg = Color::Default;
            return;
        }

        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            match param[0] {
                0 => {
                    self.grid.current_attrs = Default::default();
                    self.grid.current_fg = Color::Default;
                    self.grid.current_bg = Color::Default;
                }
                1 => self.grid.current_attrs.bold = true,
                2 => self.grid.current_attrs.dim = true,
                3 => self.grid.current_attrs.italic = true,
                4 => self.grid.current_attrs.underline = true,
                5 => self.grid.current_attrs.blink = true,
                7 => self.grid.current_attrs.reverse = true,
                8 => self.grid.current_attrs.hidden = true,
                9 => self.grid.current_attrs.strikethrough = true,
                22 => {
                    self.grid.current_attrs.bold = false;
                    self.grid.current_attrs.dim = false;
                }
                23 => self.grid.current_attrs.italic = false,
                24 => self.grid.current_attrs.underline = false,
                25 => self.grid.current_attrs.blink = false,
                27 => self.grid.current_attrs.reverse = false,
                28 => self.grid.current_attrs.hidden = false,
                29 => self.grid.current_attrs.strikethrough = false,
                // Foreground colors
                30 => self.grid.current_fg = Color::Named(NamedColor::Black),
                31 => self.grid.current_fg = Color::Named(NamedColor::Red),
                32 => self.grid.current_fg = Color::Named(NamedColor::Green),
                33 => self.grid.current_fg = Color::Named(NamedColor::Yellow),
                34 => self.grid.current_fg = Color::Named(NamedColor::Blue),
                35 => self.grid.current_fg = Color::Named(NamedColor::Magenta),
                36 => self.grid.current_fg = Color::Named(NamedColor::Cyan),
                37 => self.grid.current_fg = Color::Named(NamedColor::White),
                38 => {
                    if let Some(next_param) = iter.next() {
                        match next_param[0] {
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (iter.next(), iter.next(), iter.next())
                                {
                                    self.grid.current_fg =
                                        Color::Rgb(r[0] as u8, g[0] as u8, b[0] as u8);
                                }
                            }
                            5 => {
                                if let Some(idx) = iter.next() {
                                    self.grid.current_fg = Color::Indexed(idx[0] as u8);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.grid.current_fg = Color::Default,
                // Background colors
                40 => self.grid.current_bg = Color::Named(NamedColor::Black),
                41 => self.grid.current_bg = Color::Named(NamedColor::Red),
                42 => self.grid.current_bg = Color::Named(NamedColor::Green),
                43 => self.grid.current_bg = Color::Named(NamedColor::Yellow),
                44 => self.grid.current_bg = Color::Named(NamedColor::Blue),
                45 => self.grid.current_bg = Color::Named(NamedColor::Magenta),
                46 => self.grid.current_bg = Color::Named(NamedColor::Cyan),
                47 => self.grid.current_bg = Color::Named(NamedColor::White),
                48 => {
                    if let Some(next_param) = iter.next() {
                        match next_param[0] {
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (iter.next(), iter.next(), iter.next())
                                {
                                    self.grid.current_bg =
                                        Color::Rgb(r[0] as u8, g[0] as u8, b[0] as u8);
                                }
                            }
                            5 => {
                                if let Some(idx) = iter.next() {
                                    self.grid.current_bg = Color::Indexed(idx[0] as u8);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.grid.current_bg = Color::Default,
                // Bright foreground
                90 => self.grid.current_fg = Color::Named(NamedColor::BrightBlack),
                91 => self.grid.current_fg = Color::Named(NamedColor::BrightRed),
                92 => self.grid.current_fg = Color::Named(NamedColor::BrightGreen),
                93 => self.grid.current_fg = Color::Named(NamedColor::BrightYellow),
                94 => self.grid.current_fg = Color::Named(NamedColor::BrightBlue),
                95 => self.grid.current_fg = Color::Named(NamedColor::BrightMagenta),
                96 => self.grid.current_fg = Color::Named(NamedColor::BrightCyan),
                97 => self.grid.current_fg = Color::Named(NamedColor::BrightWhite),
                // Bright background
                100 => self.grid.current_bg = Color::Named(NamedColor::BrightBlack),
                101 => self.grid.current_bg = Color::Named(NamedColor::BrightRed),
                102 => self.grid.current_bg = Color::Named(NamedColor::BrightGreen),
                103 => self.grid.current_bg = Color::Named(NamedColor::BrightYellow),
                104 => self.grid.current_bg = Color::Named(NamedColor::BrightBlue),
                105 => self.grid.current_bg = Color::Named(NamedColor::BrightMagenta),
                106 => self.grid.current_bg = Color::Named(NamedColor::BrightCyan),
                107 => self.grid.current_bg = Color::Named(NamedColor::BrightWhite),
                _ => {}
            }
        }
    }
}

impl Perform for AnsiHandler<'_> {
    fn print(&mut self, c: char) {
        self.grid.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.grid.put_char('\n'),
            b'\r' => self.grid.put_char('\r'),
            b'\t' => self.grid.put_char('\t'),
            b'\x08' => self.grid.put_char('\x08'),
            b'\x07' => {
                // Bell — set pending flag for event system
                self.grid.bell_pending = true;
            }
            b'\x0b' => self.grid.put_char('\n'),
            b'\x0c' => {
                self.grid.clear_screen();
                self.grid.goto(0, 0);
            }
            b'\x0e' => self.grid.shift_out(),
            b'\x0f' => self.grid.shift_in(),
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        if ignore {
            return;
        }

        match (c, intermediates) {
            ('A', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(0, -(n as isize));
            }
            ('B', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(0, n as isize);
            }
            ('C', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(n as isize, 0);
            }
            ('D', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(-(n as isize), 0);
            }
            ('c', []) => {
                self.grid.queue_response("\x1b[?62;0c".to_string());
            }
            ('c', [b'>']) => {
                self.grid.queue_response("\x1b[>0;0;0c".to_string());
            }
            ('E', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(0, n as isize);
                self.grid.cursor.x = 0;
            }
            ('F', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.move_cursor(0, -(n as isize));
                self.grid.cursor.x = 0;
            }
            ('G', []) => {
                let col = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.cursor.x = col
                    .saturating_sub(1)
                    .min(self.grid.cols().saturating_sub(1));
            }
            ('H', []) | ('f', []) => {
                let mut iter = params.iter();
                let row = Self::parse_param_with_default(iter.next(), 1) as usize;
                let col = Self::parse_param_with_default(iter.next(), 1) as usize;
                self.grid
                    .goto_origin_aware(col.saturating_sub(1), row.saturating_sub(1));
            }
            ('J', []) => {
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match mode {
                    0 => self.grid.erase_to_eos(),
                    1 => self.grid.erase_from_bos(),
                    2 | 3 => self.grid.clear_screen(),
                    _ => {}
                }
            }
            ('K', []) => {
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match mode {
                    0 => self.grid.erase_to_eol(),
                    1 => self.grid.erase_to_bol(),
                    2 => self.grid.clear_line(),
                    _ => {}
                }
            }
            ('P', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.delete_chars(n);
            }
            ('@', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.insert_chars(n);
            }
            ('X', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.erase_chars(n);
            }
            ('L', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.scroll_down(n);
            }
            ('M', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.scroll_up(n);
            }
            ('S', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.scroll_up(n);
            }
            ('T', []) => {
                let n = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                self.grid.scroll_down(n);
            }
            ('d', []) => {
                let row = Self::parse_param_with_default(params.iter().next(), 1) as usize;
                let y = row.saturating_sub(1);
                if self.grid.origin_mode {
                    let scroll_top = self.grid.scroll_region_top();
                    let scroll_bottom = self.grid.scroll_region_bottom();
                    self.grid.cursor.y = (scroll_top + y).min(scroll_bottom);
                } else {
                    self.grid.cursor.y = y.min(self.grid.rows().saturating_sub(1));
                }
            }
            ('h', []) => {
                for param in params.iter() {
                    match param[0] {
                        4 => self.grid.insert_mode = true,
                        20 => self.grid.lnm_mode = true,
                        _ => {}
                    }
                }
            }
            ('h', [b'?']) => {
                for param in params.iter() {
                    match param[0] {
                        1 => self.grid.application_cursor_keys = true,
                        6 => self.grid.set_origin_mode(true),
                        7 => self.grid.auto_wrap_mode = true,
                        25 => self.grid.cursor.visible = true,
                        1000 => self.grid.mouse_normal_tracking = true,
                        1002 => self.grid.mouse_button_tracking = true,
                        1003 => self.grid.mouse_any_event_tracking = true,
                        1004 => self.grid.focus_event_mode = true,
                        1005 => self.grid.mouse_utf8_mode = true,
                        1006 => self.grid.mouse_sgr_mode = true,
                        1015 => self.grid.mouse_urxvt_mode = true,
                        47 => self.grid.use_alt_screen(),
                        1047 => self.grid.use_alt_screen(),
                        1048 => self.grid.save_cursor(),
                        1049 => self.grid.use_alt_screen(),
                        2004 => self.grid.bracketed_paste_mode = true,
                        2026 => self.grid.begin_synchronized_output(),
                        _ => {}
                    }
                }
            }
            ('l', []) => {
                for param in params.iter() {
                    match param[0] {
                        4 => self.grid.insert_mode = false,
                        20 => self.grid.lnm_mode = false,
                        _ => {}
                    }
                }
            }
            ('l', [b'?']) => {
                for param in params.iter() {
                    match param[0] {
                        1 => self.grid.application_cursor_keys = false,
                        6 => self.grid.set_origin_mode(false),
                        7 => self.grid.auto_wrap_mode = false,
                        25 => self.grid.cursor.visible = false,
                        1000 => self.grid.mouse_normal_tracking = false,
                        1002 => self.grid.mouse_button_tracking = false,
                        1003 => self.grid.mouse_any_event_tracking = false,
                        1004 => self.grid.focus_event_mode = false,
                        1005 => self.grid.mouse_utf8_mode = false,
                        1006 => self.grid.mouse_sgr_mode = false,
                        1015 => self.grid.mouse_urxvt_mode = false,
                        47 => self.grid.use_main_screen(),
                        1047 => self.grid.use_main_screen(),
                        1048 => self.grid.restore_cursor(),
                        1049 => self.grid.use_main_screen(),
                        2004 => self.grid.bracketed_paste_mode = false,
                        2026 => self.grid.end_synchronized_output(),
                        _ => {}
                    }
                }
            }
            ('n', []) => {
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match mode {
                    5 => self.grid.queue_response("\x1b[0n".to_string()),
                    6 => self.grid.queue_cursor_position_report(),
                    _ => {}
                }
            }
            ('n', [b'?']) => {
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match mode {
                    6 => {
                        let (row, col) = if self.grid.origin_mode {
                            let row = (self
                                .grid
                                .cursor
                                .y
                                .saturating_sub(self.grid.scroll_region_top()))
                                + 1;
                            let col = self.grid.cursor.x + 1;
                            (row, col)
                        } else {
                            let row = self.grid.cursor.y + 1;
                            let col = self.grid.cursor.x + 1;
                            (row, col)
                        };
                        let response = format!("\x1b[?{};{}R", row, col);
                        self.grid.queue_response(response);
                    }
                    15 => self.grid.queue_response("\x1b[?13n".to_string()),
                    25 => self.grid.queue_response("\x1b[?21n".to_string()),
                    26 => self.grid.queue_response("\x1b[?27;1n".to_string()),
                    _ => {}
                }
            }
            ('m', []) => {
                self.handle_sgr(params);
            }
            ('r', []) => {
                let mut iter = params.iter();
                let top = Self::parse_param_with_default(iter.next(), 1) as usize;
                let bottom_default = self.grid.rows() as u16;
                let bottom = Self::parse_param_with_default(iter.next(), bottom_default) as usize;
                self.grid
                    .set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
            }
            ('s', []) => self.grid.save_cursor_position(),
            ('u', []) => self.grid.restore_cursor_position(),
            ('q', [b' ']) => {
                let shape = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                self.grid.cursor.shape = match shape {
                    1 | 2 => CursorShape::Block,
                    3 | 4 => CursorShape::Underline,
                    5 | 6 => CursorShape::Bar,
                    _ => CursorShape::Block,
                };
            }
            ('t', []) => {
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                match mode {
                    18 => {
                        let response = format!("\x1b[8;{};{}t", self.grid.rows(), self.grid.cols());
                        self.grid.queue_response(response);
                    }
                    19 => {
                        let response = format!("\x1b[9;{};{}t", self.grid.rows(), self.grid.cols());
                        self.grid.queue_response(response);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (byte, intermediates) {
            (b'D', []) => self.grid.put_char('\n'),
            (b'M', []) => self.grid.reverse_linefeed(),
            (b'E', []) => self.grid.next_line(),
            (b'7', []) => self.grid.save_cursor(),
            (b'8', []) => self.grid.restore_cursor(),
            (b'c', []) => self.grid.reset(),
            (b'H', []) => {}
            (b'=', []) => {}
            (b'>', []) => {}
            (b'\\', []) => {}
            (b'0', [b'(']) => self.grid.set_charset_g0(CharacterSet::DecSpecialGraphics),
            (b'B', [b'(']) => self.grid.set_charset_g0(CharacterSet::Ascii),
            (b'A', [b'(']) => self.grid.set_charset_g0(CharacterSet::Ascii),
            (b'0', [b')']) => self.grid.set_charset_g1(CharacterSet::DecSpecialGraphics),
            (b'B', [b')']) => self.grid.set_charset_g1(CharacterSet::Ascii),
            (b'A', [b')']) => self.grid.set_charset_g1(CharacterSet::Ascii),
            _ => {}
        }
    }
}
