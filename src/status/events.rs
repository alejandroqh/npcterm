use serde::Serialize;
use std::collections::VecDeque;

/// Terminal events that are pushed to the AI agent
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalEvent {
    CommandFinished { exit_code: i32 },
    WaitingForInput,
    Bell,
    ProcessStateChanged { old: String, new: String },
    ScreenChanged { changed_rows: Vec<usize> },
    OutputReceived { byte_count: usize },
}

/// Event queue per terminal instance
pub struct EventQueue {
    events: VecDeque<TerminalEvent>,
    max_capacity: usize,
}

impl EventQueue {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            events: VecDeque::new(),
            max_capacity,
        }
    }

    pub fn push(&mut self, event: TerminalEvent) {
        if self.events.len() >= self.max_capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Drain all events (consume)
    pub fn drain(&mut self) -> Vec<TerminalEvent> {
        self.events.drain(..).collect()
    }

    /// Peek at events without consuming
    pub fn peek(&self) -> &VecDeque<TerminalEvent> {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new(100)
    }
}
