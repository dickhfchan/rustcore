#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::auth::{authenticate_request, HeaderAuth};
use crate::http::{Header as HttpHeader, HttpHandler, Method, Request, Response};
use crate::log::EventLog;
use filesystem::catalog::{Catalog, CatalogError};
use filesystem::index::{IndexError, ListRequest, MutableIndex};
use security::apikey::{AuthError, StaticApiKeyValidator};
use security::keystore::KeyStore;
use storage::object::{ObjectError, ObjectStore};

pub struct S3Service<C, O, S, I>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
{
    catalog: C,
    store: O,
    keystore: S,
    index: I,
    auth_header: &'static str,
    events: EventLog,
}

impl<C, O, S, I> S3Service<C, O, S, I>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
{
    pub fn new(catalog: C, store: O, keystore: S, index: I) -> Self {
        Self {
            catalog,
            store,
            keystore,
            index,
            auth_header: "x-api-key",
            events: EventLog::default(),
        }
    }

    pub fn catalog_mut(&mut self) -> &mut C {
        &mut self.catalog
    }

    pub fn keystore_mut(&mut self) -> &mut S {
        &mut self.keystore
    }

    pub fn index_mut(&mut self) -> &mut I {
        &mut self.index
    }

    #[cfg(test)]
    pub fn events(&self) -> &[String] {
        self.events.entries()
    }

    fn response(status: u16, body: Vec<u8>) -> Response {
        Response {
            status,
            headers: vec![HttpHeader {
                name: "Content-Length".to_string(),
                value: body.len().to_string(),
            }],
            body,
        }
    }

    fn empty_response(status: u16) -> Response {
        Self::response(status, Vec::new())
    }

    fn storage_key(bucket: &str, key: &str) -> String {
        format!("{}/{}", bucket, key)
    }

    fn handle_put(&mut self, bucket: &str, key: &str, body: &[u8]) -> Response {
        if key.is_empty() {
            return Self::response(400, b"MissingObjectKey".to_vec());
        }
        let storage_key = Self::storage_key(bucket, key);
        match self.store.put(&storage_key, body) {
            Ok(meta) => match self.catalog.put_object(bucket, key, meta) {
                Ok(()) => {
                    self.index.insert(bucket, key, meta);
                    self.events
                        .record(format!("PUT {}/{} size={}", bucket, key, body.len()));
                    Self::empty_response(200)
                }
                Err(CatalogError::NotFound) => {
                    let _ = self.store.delete(&storage_key);
                    Self::response(404, b"BucketNotFound".to_vec())
                }
                Err(CatalogError::InvalidName) => {
                    let _ = self.store.delete(&storage_key);
                    Self::response(400, b"InvalidName".to_vec())
                }
                Err(_) => {
                    let _ = self.store.delete(&storage_key);
                    Self::response(500, b"CatalogError".to_vec())
                }
            },
            Err(ObjectError::InvalidKey) => Self::response(400, b"InvalidKey".to_vec()),
            Err(ObjectError::Backend(_)) => Self::response(500, b"StorageError".to_vec()),
            Err(ObjectError::NotFound) => Self::response(404, b"NotFound".to_vec()),
        }
    }

    fn handle_get(&mut self, bucket: &str, key: &str) -> Response {
        let meta = match self.catalog.object_metadata(bucket, key) {
            Ok(meta) => meta,
            Err(_) => return Self::response(404, b"NoSuchKey".to_vec()),
        };
        let storage_key = Self::storage_key(bucket, key);
        let mut buffer = vec![0u8; meta.size as usize];
        match self.store.get(&storage_key, &mut buffer) {
            Ok(_) => Self::response(200, buffer),
            Err(ObjectError::NotFound) => Self::response(404, b"NoSuchKey".to_vec()),
            Err(_) => Self::response(500, b"StorageError".to_vec()),
        }
    }

    fn handle_delete(&mut self, bucket: &str, key: &str) -> Response {
        if self.catalog.remove_object(bucket, key).is_err() {
            return Self::response(404, b"NoSuchKey".to_vec());
        }
        self.index.remove(bucket, key);
        let storage_key = Self::storage_key(bucket, key);
        match self.store.delete(&storage_key) {
            Ok(()) => Self::empty_response(204),
            Err(ObjectError::NotFound) => Self::response(404, b"NoSuchKey".to_vec()),
            Err(_) => Self::response(500, b"StorageError".to_vec()),
        }
    }

    fn handle_list(&mut self, bucket: &str, query: Option<&str>) -> Response {
        let params = QueryParams::parse(query.unwrap_or(""));
        let list_req = ListRequest {
            bucket,
            prefix: params.prefix.as_deref(),
            delimiter: params.delimiter,
            continuation: params.continuation_token.as_deref(),
            max_keys: params.max_keys.unwrap_or(1000),
        };
        match self.index.list(&list_req) {
            Ok(result) => {
                let mut body = String::new();
                body.push_str("Objects:\n");
                for obj in &result.objects {
                    body.push_str(obj.key);
                    body.push_str("\n");
                }
                if !result.common_prefixes.is_empty() {
                    body.push_str("Prefixes:\n");
                    for prefix in &result.common_prefixes {
                        body.push_str(prefix);
                        body.push_str("\n");
                    }
                }
                if let Some(token) = result.next_token {
                    body.push_str("NextToken:");
                    body.push_str(&token);
                }
                self.events
                    .record(format!("LIST {} prefix={:?}", bucket, list_req.prefix));
                Self::response(200, body.into_bytes())
            }
            Err(IndexError::NotFound) => Self::response(404, b"BucketNotFound".to_vec()),
            Err(_) => Self::response(500, b"IndexError".to_vec()),
        }
    }

    fn auth_error_response(err: AuthError) -> Response {
        let body = match err {
            AuthError::Missing => b"MissingApiKey".to_vec(),
            AuthError::Invalid => b"InvalidApiKey".to_vec(),
            AuthError::Store(_) => b"InvalidApiKey".to_vec(),
        };
        Self::response(403, body)
    }
}

impl<C, O, S, I> HttpHandler for S3Service<C, O, S, I>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
{
    fn handle(&mut self, request: &Request) -> Response {
        let validator = StaticApiKeyValidator::new(&self.keystore);
        if let Err(err) = authenticate_request(request, &validator, &HeaderAuth, self.auth_header) {
            return Self::auth_error_response(err);
        }

        let trimmed = request.path.trim_start_matches('/');
        let (path, query) = match trimmed.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (trimmed, None),
        };

        let mut parts = path.splitn(2, '/');
        let bucket = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => return Self::response(400, b"MissingBucket".to_vec()),
        };
        let key = parts.next().unwrap_or("");

        match request.method {
            Method::Put => self.handle_put(bucket, key, &request.body),
            Method::Get if key.is_empty() => self.handle_list(bucket, query),
            Method::Get => self.handle_get(bucket, key),
            Method::Delete => self.handle_delete(bucket, key),
            _ => Self::response(405, b"MethodNotAllowed".to_vec()),
        }
    }
}

struct QueryParams {
    prefix: Option<String>,
    delimiter: Option<char>,
    continuation_token: Option<String>,
    max_keys: Option<usize>,
}

impl QueryParams {
    fn parse(query: &str) -> Self {
        let mut params = Self {
            prefix: None,
            delimiter: None,
            continuation_token: None,
            max_keys: None,
        };
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (name, value) = match pair.split_once('=') {
                Some(split) => split,
                None => (pair, ""),
            };
            match name {
                "prefix" => params.prefix = Some(value.replace('%', "")),
                "delimiter" => params.delimiter = value.chars().next(),
                "continuation-token" => params.continuation_token = Some(value.to_string()),
                "max-keys" => params.max_keys = value.parse().ok(),
                _ => {}
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Header, Method};
    use filesystem::catalog::InMemoryCatalog;
    use filesystem::index::InMemoryIndex;
    use security::keystore::{ApiKeyEntry, InMemoryKeyStore};
    use storage::object::InMemoryObjectStore;

    fn make_request(method: Method, path: &str, key: Option<&str>, body: &[u8]) -> Request {
        let mut headers = Vec::new();
        if let Some(value) = key {
            headers.push(Header {
                name: "x-api-key".to_string(),
                value: value.to_string(),
            });
        }
        Request {
            method,
            path: path.to_string(),
            headers,
            body: body.to_vec(),
        }
    }

    fn new_service(
    ) -> S3Service<InMemoryCatalog, InMemoryObjectStore, InMemoryKeyStore, InMemoryIndex> {
        let mut service = S3Service::new(
            InMemoryCatalog::new(),
            InMemoryObjectStore::new(),
            InMemoryKeyStore::new(),
            InMemoryIndex::new(),
        );
        service.catalog_mut().create_bucket("photos").unwrap();
        service
            .keystore_mut()
            .insert(ApiKeyEntry {
                key_id: "abc123",
                hash: [0; 32],
                bucket: "photos",
                permissions: 1,
            })
            .unwrap();
        service
    }

    fn new_empty_service(
    ) -> S3Service<InMemoryCatalog, InMemoryObjectStore, InMemoryKeyStore, InMemoryIndex> {
        let mut service = S3Service::new(
            InMemoryCatalog::new(),
            InMemoryObjectStore::new(),
            InMemoryKeyStore::new(),
            InMemoryIndex::new(),
        );
        service
            .keystore_mut()
            .insert(ApiKeyEntry {
                key_id: "abc123",
                hash: [0; 32],
                bucket: "photos",
                permissions: 1,
            })
            .unwrap();
        service
    }

    #[test]
    fn put_get_delete_cycle() {
        let mut service = new_service();
        let put_resp = service.handle(&make_request(
            Method::Put,
            "/photos/cat.jpg",
            Some("abc123"),
            b"meow",
        ));
        assert_eq!(put_resp.status, 200);

        let list_resp = service.handle(&make_request(
            Method::Get,
            "/photos?prefix=",
            Some("abc123"),
            &[],
        ));
        assert_eq!(list_resp.status, 200);
        let body = core::str::from_utf8(&list_resp.body).unwrap();
        assert!(body.contains("cat.jpg"));

        let get_resp = service.handle(&make_request(
            Method::Get,
            "/photos/cat.jpg",
            Some("abc123"),
            &[],
        ));
        assert_eq!(get_resp.status, 200);
        assert_eq!(get_resp.body, b"meow");

        let del_resp = service.handle(&make_request(
            Method::Delete,
            "/photos/cat.jpg",
            Some("abc123"),
            &[],
        ));
        assert_eq!(del_resp.status, 204);

        let get_missing = service.handle(&make_request(
            Method::Get,
            "/photos/cat.jpg",
            Some("abc123"),
            &[],
        ));
        assert_eq!(get_missing.status, 404);
    }

    #[test]
    fn list_with_prefix_and_delimiter() {
        let mut service = new_service();
        service.handle(&make_request(
            Method::Put,
            "/photos/2023/holiday/img1.jpg",
            Some("abc123"),
            b"1",
        ));
        service.handle(&make_request(
            Method::Put,
            "/photos/2023/work/img2.jpg",
            Some("abc123"),
            b"2",
        ));

        let resp = service.handle(&make_request(
            Method::Get,
            "/photos?prefix=2023/&delimiter=/",
            Some("abc123"),
            &[],
        ));
        assert_eq!(resp.status, 200);
        let body = core::str::from_utf8(&resp.body).unwrap();
        assert!(body.contains("Prefixes"));
    }

    #[test]
    fn put_missing_bucket_fails() {
        let mut service = new_empty_service();
        let resp = service.handle(&make_request(
            Method::Put,
            "/photos/item",
            Some("abc123"),
            b"data",
        ));
        assert_eq!(resp.status, 404);
        assert!(service.events().is_empty());
    }

    #[test]
    fn put_with_empty_key_is_rejected() {
        let mut service = new_service();
        let resp = service.handle(&make_request(
            Method::Put,
            "/photos/",
            Some("abc123"),
            b"data",
        ));
        assert_eq!(resp.status, 400);
    }
}
