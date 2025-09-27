#![allow(dead_code)]

use crate::keystore::{ApiKeyEntry, KeyStore, KeyStoreError};

pub trait ApiKeyValidator {
    fn validate(&self, key: &str) -> Result<ApiKeyEntry<'_>, AuthError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    Missing,
    Invalid,
    Store(KeyStoreError),
}

pub struct StaticApiKeyValidator<'a, S: KeyStore> {
    store: &'a S,
}

impl<'a, S: KeyStore> StaticApiKeyValidator<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }
}

impl<'a, S: KeyStore> ApiKeyValidator for StaticApiKeyValidator<'a, S> {
    fn validate(&self, key: &str) -> Result<ApiKeyEntry<'_>, AuthError> {
        self.store.lookup(key).map_err(AuthError::Store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::{InMemoryKeyStore, KeyStore};

    #[test]
    fn validates_existing_key() {
        let mut store = InMemoryKeyStore::new();
        store
            .insert(ApiKeyEntry {
                key_id: "key1",
                hash: [0xAB; 32],
                bucket: "photos",
                permissions: 0x1,
            })
            .unwrap();
        let validator = StaticApiKeyValidator::new(&store);
        let entry = validator.validate("key1").unwrap();
        assert_eq!(entry.bucket, "photos");
    }

    #[test]
    fn missing_key_returns_error() {
        let store = InMemoryKeyStore::new();
        let validator = StaticApiKeyValidator::new(&store);
        assert!(matches!(
            validator.validate("missing"),
            Err(AuthError::Store(KeyStoreError::NotFound))
        ));
    }
}
