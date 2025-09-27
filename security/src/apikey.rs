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
