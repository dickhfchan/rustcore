#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

pub struct EventLog {
    entries: Vec<String>,
    capacity: usize,
}

impl EventLog {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&mut self, event: impl AsRef<str>) {
        if self.entries.len() == self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(event.as_ref().to_string());
    }

    #[cfg(test)]
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}
