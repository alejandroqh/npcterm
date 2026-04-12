use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::VecDeque;

/// A single agent interaction entry (tool call)
#[derive(Debug, Clone, Serialize)]
pub struct InteractionEntry {
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    pub params: serde_json::Value,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Ring-buffer log of agent interactions
pub struct InteractionLog {
    entries: VecDeque<InteractionEntry>,
    max_capacity: usize,
}

impl InteractionLog {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_capacity,
        }
    }

    pub fn push(&mut self, entry: InteractionEntry) {
        if self.entries.len() >= self.max_capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn recent(&self, n: usize) -> Vec<InteractionEntry> {
        let skip = self.entries.len().saturating_sub(n);
        self.entries.iter().skip(skip).cloned().collect()
    }
}

impl Default for InteractionLog {
    fn default() -> Self {
        Self::new(500)
    }
}
