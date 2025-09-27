#![allow(dead_code)]

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
