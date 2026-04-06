use serde::Serialize;

/// Terminal process state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalState {
    Running,
    Idle,
    WaitingForInput,
    Exited,
}

/// Lightweight status response (token-optimized)
#[derive(Debug, Serialize)]
pub struct TerminalStatus {
    pub state: TerminalState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_command: Option<String>,
    pub last_lines: Vec<String>,
    pub cursor_pos: (usize, usize),
    pub cursor_visible: bool,
    pub mouse_pos: (u16, u16),
    pub dirty: bool,
    pub changed_rows: Vec<usize>,
    pub pending_events: usize,
    pub size: (usize, usize),
    pub scrollback_lines: usize,
    pub has_new_content: bool,
}

impl std::fmt::Display for TerminalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalState::Running => write!(f, "running"),
            TerminalState::Idle => write!(f, "idle"),
            TerminalState::WaitingForInput => write!(f, "waiting_for_input"),
            TerminalState::Exited => write!(f, "exited"),
        }
    }
}

impl TerminalState {
    /// Detect state from emulator info
    pub fn detect(
        is_alive: bool,
        exit_code: Option<i32>,
        last_output_ms: Option<u64>,
        foreground_is_shell: bool,
    ) -> Self {
        if !is_alive || exit_code.is_some() {
            return TerminalState::Exited;
        }

        // If the foreground process is the shell itself, likely waiting for input
        if foreground_is_shell {
            return TerminalState::WaitingForInput;
        }

        // If no output for >500ms, consider idle
        if let Some(ms) = last_output_ms {
            if ms > 500 {
                return TerminalState::Idle;
            }
        }

        TerminalState::Running
    }
}
