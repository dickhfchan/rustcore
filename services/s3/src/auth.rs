#![allow(dead_code)]

use security::apikey::{ApiKeyValidator, AuthError};

use crate::http::Request;

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

pub fn authenticate_request<V: ApiKeyValidator, L: AuthLayer<V>>(
    request: &Request,
    validator: &V,
    layer: &L,
    header_name: &str,
) -> Result<(), AuthError> {
    let header = request
        .header(header_name)
        .map(|value| ApiKeyHeader { value });
    layer.authenticate(validator, header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Header, Method, Request};
    use alloc::vec;
    use security::apikey::StaticApiKeyValidator;
    use security::keystore::{ApiKeyEntry, InMemoryKeyStore, KeyStore};

    fn make_request(header_value: Option<&str>) -> Request {
        let mut headers = alloc::vec::Vec::new();
        if let Some(value) = header_value {
            headers.push(Header {
                name: "x-api-key".to_string(),
                value: value.to_string(),
            });
        }
        Request {
            method: Method::Get,
            path: "/".to_string(),
            headers,
            body: alloc::vec![],
        }
    }

    #[test]
    fn authenticates_with_valid_key() {
        let mut store = InMemoryKeyStore::new();
        store
            .insert(ApiKeyEntry {
                key_id: "key-1",
                hash: [0; 32],
                bucket: "default",
                permissions: 1,
            })
            .unwrap();
        let validator = StaticApiKeyValidator::new(&store);
        let request = make_request(Some("key-1"));
        assert!(authenticate_request(&request, &validator, &HeaderAuth, "x-api-key").is_ok());
    }

    #[test]
    fn missing_key_is_error() {
        let store = InMemoryKeyStore::new();
        let validator = StaticApiKeyValidator::new(&store);
        let request = make_request(None);
        assert!(matches!(
            authenticate_request(&request, &validator, &HeaderAuth, "x-api-key"),
            Err(AuthError::Missing)
        ));
    }
}
