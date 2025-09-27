#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use crate::catalog::CatalogError;

/// Represents a log entry describing a catalog mutation.
pub struct JournalEntry<'a> {
    pub bucket: &'a str,
    pub key: Option<&'a str>,
    pub operation: Operation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    CreateBucket,
    DeleteBucket,
    PutObject,
    DeleteObject,
}

pub trait Journal {
    fn append(&mut self, entry: JournalEntry<'_>) -> Result<(), JournalError>;
    fn replay(&self, callback: &mut dyn FnMut(JournalEntry<'_>)) -> Result<(), JournalError>;
    fn len(&self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalError {
    Storage,
    Catalog(CatalogError),
    InvalidEntry,
}

pub struct InMemoryJournal {
    entries: Vec<StoredEntry>,
}

#[derive(Clone)]
struct StoredEntry {
    bucket: String,
    key: Option<String>,
    operation: Operation,
}

impl InMemoryJournal {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Default for InMemoryJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl Journal for InMemoryJournal {
    fn append(&mut self, entry: JournalEntry<'_>) -> Result<(), JournalError> {
        if entry.bucket.is_empty() {
            return Err(JournalError::InvalidEntry);
        }
        if matches!(
            entry.operation,
            Operation::PutObject | Operation::DeleteObject
        ) && entry.key.unwrap_or("").is_empty()
        {
            return Err(JournalError::InvalidEntry);
        }
        self.entries.push(StoredEntry {
            bucket: entry.bucket.to_owned(),
            key: entry.key.map(ToOwned::to_owned),
            operation: entry.operation,
        });
        Ok(())
    }

    fn replay(&self, callback: &mut dyn FnMut(JournalEntry<'_>)) -> Result<(), JournalError> {
        for entry in &self.entries {
            let key = entry.key.as_deref();
            callback(JournalEntry {
                bucket: &entry.bucket,
                key,
                operation: entry.operation,
            });
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_replay() {
        let mut journal = InMemoryJournal::new();
        journal
            .append(JournalEntry {
                bucket: "docs",
                key: Some("file.txt"),
                operation: Operation::PutObject,
            })
            .unwrap();
        journal
            .append(JournalEntry {
                bucket: "docs",
                key: Some("file.txt"),
                operation: Operation::DeleteObject,
            })
            .unwrap();

        let mut ops = alloc::vec::Vec::new();
        journal
            .replay(&mut |entry| ops.push(entry.operation))
            .unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], Operation::PutObject);
    }

    #[test]
    fn reject_invalid_entries() {
        let mut journal = InMemoryJournal::new();
        assert!(matches!(
            journal.append(JournalEntry {
                bucket: "",
                key: None,
                operation: Operation::CreateBucket,
            }),
            Err(JournalError::InvalidEntry)
        ));
    }
}
