use std::collections::HashMap;

use serde::Serialize;

use crate::status::query::TerminalState;

use super::instance::TerminalInstance;

/// Summary info for a terminal instance
#[derive(Debug, Serialize)]
pub struct TerminalInfo {
    pub id: String,
    pub cols: usize,
    pub rows: usize,
    pub state: TerminalState,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_command: Option<String>,
}

/// Registry managing multiple terminal instances. No globals.
pub struct TerminalRegistry {
    instances: HashMap<String, TerminalInstance>,
    max_instances: usize,
    next_id: usize,
}

impl TerminalRegistry {
    const FIRST_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const SECOND_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    pub fn new(max_instances: usize) -> Self {
        Self {
            instances: HashMap::new(),
            max_instances,
            next_id: 0,
        }
    }

    fn next_short_id(&mut self) -> String {
        let second_len = Self::SECOND_CHARS.len();
        loop {
            let a = self.next_id / second_len;
            let b = self.next_id % second_len;
            self.next_id += 1;
            let mut id = String::with_capacity(2);
            id.push(Self::FIRST_CHARS[a % Self::FIRST_CHARS.len()] as char);
            id.push(Self::SECOND_CHARS[b] as char);
            if !self.instances.contains_key(&id) {
                return id;
            }
        }
    }

    pub fn create(
        &mut self,
        cols: usize,
        rows: usize,
        shell: Option<&str>,
    ) -> Result<String, String> {
        if self.instances.len() >= self.max_instances {
            return Err(format!(
                "Maximum instances ({}) reached",
                self.max_instances
            ));
        }

        let id = self.next_short_id();
        let instance = TerminalInstance::new(id.clone(), cols, rows, shell)
            .map_err(|e| format!("Failed to create terminal: {}", e))?;

        self.instances.insert(id.clone(), instance);
        Ok(id)
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&TerminalInstance> {
        self.instances.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut TerminalInstance> {
        self.instances.get_mut(id)
    }

    pub fn destroy(&mut self, id: &str) -> bool {
        self.instances.remove(id).is_some()
    }

    pub fn list(&self) -> Vec<TerminalInfo> {
        self.instances
            .values()
            .map(|inst| TerminalInfo {
                id: inst.id.clone(),
                cols: inst.cols(),
                rows: inst.rows(),
                state: inst.state(),
                created_at: inst.created_at().to_rfc3339(),
                running_command: inst.running_command(),
            })
            .collect()
    }

    pub fn tick_all(&mut self) {
        for instance in self.instances.values_mut() {
            instance.tick();
        }
    }
}

impl Default for TerminalRegistry {
    fn default() -> Self {
        Self::new(16)
    }
}
