use std::fmt;

/// Represents a key that can be sent to the terminal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    Ctrl(char),
    Alt(char),
}

impl Key {
    /// Parse a key name string from MCP tool arguments
    pub fn from_str(s: &str) -> Result<Self, String> {
        // Single character
        if s.len() == 1 {
            return Ok(Key::Char(s.chars().next().unwrap()));
        }

        // Named keys (case-insensitive)
        match s.to_lowercase().as_str() {
            "enter" | "return" => Ok(Key::Enter),
            "tab" => Ok(Key::Tab),
            "escape" | "esc" => Ok(Key::Escape),
            "backspace" | "bs" => Ok(Key::Backspace),
            "delete" | "del" => Ok(Key::Delete),
            "up" => Ok(Key::Up),
            "down" => Ok(Key::Down),
            "left" => Ok(Key::Left),
            "right" => Ok(Key::Right),
            "home" => Ok(Key::Home),
            "end" => Ok(Key::End),
            "pageup" | "page_up" | "pgup" => Ok(Key::PageUp),
            "pagedown" | "page_down" | "pgdn" => Ok(Key::PageDown),
            "space" => Ok(Key::Char(' ')),
            _ => {
                // Ctrl+X
                if let Some(rest) = s.strip_prefix("Ctrl+").or_else(|| s.strip_prefix("ctrl+")) {
                    if rest.len() == 1 {
                        let c = rest.chars().next().unwrap().to_ascii_lowercase();
                        if c.is_ascii_lowercase() {
                            return Ok(Key::Ctrl(c));
                        }
                    }
                    return Err(format!("Invalid Ctrl combo: {}", s));
                }

                // Alt+X
                if let Some(rest) = s.strip_prefix("Alt+").or_else(|| s.strip_prefix("alt+")) {
                    if rest.len() == 1 {
                        return Ok(Key::Alt(rest.chars().next().unwrap()));
                    }
                    return Err(format!("Invalid Alt combo: {}", s));
                }

                // Function keys F1-F12
                if let Some(rest) = s.strip_prefix('F').or_else(|| s.strip_prefix('f')) {
                    if let Ok(n) = rest.parse::<u8>() {
                        if (1..=12).contains(&n) {
                            return Ok(Key::F(n));
                        }
                    }
                }

                Err(format!("Unknown key: {}", s))
            }
        }
    }

    /// Convert to escape sequence bytes to send to PTY
    pub fn to_escape_sequence(&self, application_cursor_keys: bool) -> Vec<u8> {
        match self {
            Key::Char(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
            Key::Enter => vec![b'\r'],
            Key::Tab => vec![b'\t'],
            Key::Escape => vec![0x1b],
            Key::Backspace => vec![0x7f],
            Key::Delete => b"\x1b[3~".to_vec(),
            Key::Up => {
                if application_cursor_keys {
                    b"\x1bOA".to_vec()
                } else {
                    b"\x1b[A".to_vec()
                }
            }
            Key::Down => {
                if application_cursor_keys {
                    b"\x1bOB".to_vec()
                } else {
                    b"\x1b[B".to_vec()
                }
            }
            Key::Right => {
                if application_cursor_keys {
                    b"\x1bOC".to_vec()
                } else {
                    b"\x1b[C".to_vec()
                }
            }
            Key::Left => {
                if application_cursor_keys {
                    b"\x1bOD".to_vec()
                } else {
                    b"\x1b[D".to_vec()
                }
            }
            Key::Home => b"\x1b[H".to_vec(),
            Key::End => b"\x1b[F".to_vec(),
            Key::PageUp => b"\x1b[5~".to_vec(),
            Key::PageDown => b"\x1b[6~".to_vec(),
            Key::F(n) => match n {
                1 => b"\x1bOP".to_vec(),
                2 => b"\x1bOQ".to_vec(),
                3 => b"\x1bOR".to_vec(),
                4 => b"\x1bOS".to_vec(),
                5 => b"\x1b[15~".to_vec(),
                6 => b"\x1b[17~".to_vec(),
                7 => b"\x1b[18~".to_vec(),
                8 => b"\x1b[19~".to_vec(),
                9 => b"\x1b[20~".to_vec(),
                10 => b"\x1b[21~".to_vec(),
                11 => b"\x1b[23~".to_vec(),
                12 => b"\x1b[24~".to_vec(),
                _ => vec![],
            },
            Key::Ctrl(c) => {
                // Ctrl+A = 0x01, Ctrl+B = 0x02, ..., Ctrl+Z = 0x1A
                let code = (*c as u8).wrapping_sub(b'a').wrapping_add(1);
                vec![code]
            }
            Key::Alt(c) => {
                // Alt+key = ESC followed by key
                let mut buf = vec![0x1b];
                let mut char_buf = [0u8; 4];
                let s = c.encode_utf8(&mut char_buf);
                buf.extend_from_slice(s.as_bytes());
                buf
            }
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{}", c),
            Key::Enter => write!(f, "Enter"),
            Key::Tab => write!(f, "Tab"),
            Key::Escape => write!(f, "Escape"),
            Key::Backspace => write!(f, "Backspace"),
            Key::Delete => write!(f, "Delete"),
            Key::Up => write!(f, "Up"),
            Key::Down => write!(f, "Down"),
            Key::Left => write!(f, "Left"),
            Key::Right => write!(f, "Right"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PageUp"),
            Key::PageDown => write!(f, "PageDown"),
            Key::F(n) => write!(f, "F{}", n),
            Key::Ctrl(c) => write!(f, "Ctrl+{}", c),
            Key::Alt(c) => write!(f, "Alt+{}", c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_char() {
        assert_eq!(Key::from_str("a").unwrap(), Key::Char('a'));
        assert_eq!(Key::from_str("Z").unwrap(), Key::Char('Z'));
    }

    #[test]
    fn test_parse_named() {
        assert_eq!(Key::from_str("Enter").unwrap(), Key::Enter);
        assert_eq!(Key::from_str("enter").unwrap(), Key::Enter);
        assert_eq!(Key::from_str("Tab").unwrap(), Key::Tab);
        assert_eq!(Key::from_str("Escape").unwrap(), Key::Escape);
        assert_eq!(Key::from_str("Backspace").unwrap(), Key::Backspace);
        assert_eq!(Key::from_str("Delete").unwrap(), Key::Delete);
        assert_eq!(Key::from_str("Up").unwrap(), Key::Up);
        assert_eq!(Key::from_str("space").unwrap(), Key::Char(' '));
    }

    #[test]
    fn test_parse_ctrl() {
        assert_eq!(Key::from_str("Ctrl+c").unwrap(), Key::Ctrl('c'));
        assert_eq!(Key::from_str("ctrl+z").unwrap(), Key::Ctrl('z'));
    }

    #[test]
    fn test_parse_alt() {
        assert_eq!(Key::from_str("Alt+x").unwrap(), Key::Alt('x'));
    }

    #[test]
    fn test_parse_function_keys() {
        assert_eq!(Key::from_str("F1").unwrap(), Key::F(1));
        assert_eq!(Key::from_str("F12").unwrap(), Key::F(12));
    }

    #[test]
    fn test_escape_sequences() {
        assert_eq!(Key::Enter.to_escape_sequence(false), vec![b'\r']);
        assert_eq!(Key::Ctrl('c').to_escape_sequence(false), vec![0x03]);
        assert_eq!(Key::Ctrl('d').to_escape_sequence(false), vec![0x04]);
        assert_eq!(Key::Up.to_escape_sequence(false), b"\x1b[A".to_vec());
        assert_eq!(Key::Up.to_escape_sequence(true), b"\x1bOA".to_vec());
        assert_eq!(Key::F(1).to_escape_sequence(false), b"\x1bOP".to_vec());
    }
}
