#![allow(dead_code)]

use security::apikey::{ApiKeyValidator, AuthError};

pub struct ApiKeyHeader<'a> {
    pub value: &'a str,
}

pub trait AuthLayer<V: ApiKeyValidator> {
    fn authenticate(
        &self,
        validator: &V,
        header: Option<ApiKeyHeader<'_>>,
    ) -> Result<(), AuthError>;
}

pub struct HeaderAuth;

impl<V: ApiKeyValidator> AuthLayer<V> for HeaderAuth {
    fn authenticate(
        &self,
        validator: &V,
        header: Option<ApiKeyHeader<'_>>,
    ) -> Result<(), AuthError> {
        let header = header.ok_or(AuthError::Missing)?;
        let _entry = validator.validate(header.value)?;
        Ok(())
    }
}
