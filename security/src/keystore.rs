#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

/// Represents a hashed API key entry stored in the keystore.
pub struct ApiKeyEntry<'a> {
    pub key_id: &'a str,
    pub hash: [u8; 32],
    pub bucket: &'a str,
    pub permissions: u32,
}

pub trait KeyStore {
    fn insert(&mut self, entry: ApiKeyEntry<'_>) -> Result<(), KeyStoreError>;
    fn lookup(&self, key_id: &str) -> Result<ApiKeyEntry<'_>, KeyStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStoreError {
    NotFound,
    Storage,
}

#[derive(Default)]
pub struct InMemoryKeyStore {
    entries: Vec<StoredEntry>,
}

#[derive(Clone)]
struct StoredEntry {
    key_id: String,
    hash: [u8; 32],
    bucket: String,
    permissions: u32,
}

impl InMemoryKeyStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl KeyStore for InMemoryKeyStore {
    fn insert(&mut self, entry: ApiKeyEntry<'_>) -> Result<(), KeyStoreError> {
        if self.entries.iter().any(|e| e.key_id == entry.key_id) {
            return Err(KeyStoreError::Storage);
        }
        self.entries.push(StoredEntry {
            key_id: entry.key_id.to_owned(),
            hash: entry.hash,
            bucket: entry.bucket.to_owned(),
            permissions: entry.permissions,
        });
        Ok(())
    }

    fn lookup(&self, key_id: &str) -> Result<ApiKeyEntry<'_>, KeyStoreError> {
        let entry = self.entries.iter().find(|e| e.key_id == key_id);
        match entry {
            Some(stored) => Ok(ApiKeyEntry {
                key_id: &stored.key_id,
                hash: stored.hash,
                bucket: &stored.bucket,
                permissions: stored.permissions,
            }),
            None => Err(KeyStoreError::NotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiKeyEntry, InMemoryKeyStore, KeyStore, KeyStoreError};

    #[test]
    fn insert_and_lookup() {
        let mut store = InMemoryKeyStore::new();
        let entry = ApiKeyEntry {
            key_id: "test",
            hash: [1; 32],
            bucket: "photos",
            permissions: 0xFF,
        };
        store.insert(entry).unwrap();
        let fetched = store.lookup("test").unwrap();
        assert_eq!(fetched.bucket, "photos");
        assert_eq!(fetched.permissions, 0xFF);
    }

    #[test]
    fn duplicate_insert_fails() {
        let mut store = InMemoryKeyStore::new();
        let entry = ApiKeyEntry {
            key_id: "dup",
            hash: [0; 32],
            bucket: "bucket",
            permissions: 1,
        };
        store.insert(entry).unwrap();
        assert_eq!(store.insert(entry), Err(KeyStoreError::Storage));
    }

    #[test]
    fn lookup_missing_key() {
        let store = InMemoryKeyStore::new();
        assert_eq!(store.lookup("missing"), Err(KeyStoreError::NotFound));
    }
}
