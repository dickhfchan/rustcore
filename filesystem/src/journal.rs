#![allow(dead_code)]

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalError {
    Storage,
    Catalog(CatalogError),
}
