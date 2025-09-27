#![allow(dead_code)]

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::auth::{authenticate_request, HeaderAuth};
use crate::http::{Header as HttpHeader, HttpHandler, Method, Request, Response};
use crate::log::EventLog;
use crate::multipart::{
    InMemoryMultipart, MultipartError, MultipartManager, MultipartPart, MultipartUpload,
};
use filesystem::catalog::{Catalog, CatalogError};
use filesystem::index::{IndexError, ListRequest, MutableIndex};
use security::apikey::{AuthError, StaticApiKeyValidator};
use security::keystore::KeyStore;
use storage::object::{ObjectError, ObjectStore};

pub struct S3Service<C, O, S, I, M>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
    M: MultipartManager,
{
    catalog: C,
    store: O,
    keystore: S,
    index: I,
    multipart: M,
    auth_header: &'static str,
    events: EventLog,
}

impl<C, O, S, I, M> S3Service<C, O, S, I, M>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
    M: MultipartManager,
{
    pub fn new(catalog: C, store: O, keystore: S, index: I, multipart: M) -> Self {
        Self {
            catalog,
            store,
            keystore,
            index,
            multipart,
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

    pub fn multipart_mut(&mut self) -> &mut M {
        &mut self.multipart
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

    fn bucket_exists(&self, bucket: &str) -> bool {
        let mut found = false;
        self.catalog.list_buckets(&mut |info| {
            if info.name == bucket {
                found = true;
            }
        });
        found
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
        if !self.bucket_exists(bucket) {
            return Self::response(404, b"BucketNotFound".to_vec());
        }
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
                    body.push('\n');
                }
                if !result.common_prefixes.is_empty() {
                    body.push_str("Prefixes:\n");
                    for prefix in &result.common_prefixes {
                        body.push_str(prefix);
                        body.push('\n');
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

    fn handle_create_bucket(&mut self, bucket: &str) -> Response {
        match self.catalog.create_bucket(bucket) {
            Ok(()) => {
                self.events.record(format!("CREATE_BUCKET {}", bucket));
                Self::empty_response(200)
            }
            Err(CatalogError::AlreadyExists) => {
                Self::response(409, b"BucketAlreadyExists".to_vec())
            }
            Err(CatalogError::InvalidName) => Self::response(400, b"InvalidBucketName".to_vec()),
            Err(_) => Self::response(500, b"CatalogError".to_vec()),
        }
    }

    fn handle_delete_bucket(&mut self, bucket: &str) -> Response {
        match self.catalog.delete_bucket(bucket) {
            Ok(()) => {
                self.index.purge_bucket(bucket);
                self.events.record(format!("DELETE_BUCKET {}", bucket));
                Self::empty_response(204)
            }
            Err(CatalogError::NotFound) => Self::response(404, b"NoSuchBucket".to_vec()),
            Err(_) => Self::response(500, b"CatalogError".to_vec()),
        }
    }

    fn handle_list_buckets(&mut self) -> Response {
        let mut body = String::new();
        body.push_str("Buckets:\n");
        self.catalog.list_buckets(&mut |info| {
            body.push_str(info.name);
            body.push('\n');
        });
        self.events.record("LIST_BUCKETS");
        Self::response(200, body.into_bytes())
    }

    fn handle_initiate_multipart(&mut self, bucket: &str, key: &str) -> Response {
        if !self.bucket_exists(bucket) {
            return Self::response(404, b"BucketNotFound".to_vec());
        }
        match self.multipart.initiate(bucket, key) {
            Ok(upload_id) => {
                self.events
                    .record(format!("MP_INIT {} {}/{}", upload_id, bucket, key));
                Self::response(200, format!("UploadId:{}", upload_id).into_bytes())
            }
            Err(_) => Self::response(500, b"MultipartError".to_vec()),
        }
    }

    fn handle_upload_part(&mut self, upload_id: &str, part_number: u32, data: &[u8]) -> Response {
        let part = MultipartPart {
            upload_id,
            part_number,
            data,
        };
        match self.multipart.put_part(part) {
            Ok(()) => {
                self.events.record(format!(
                    "MP_PART {} part={} size={}",
                    upload_id,
                    part_number,
                    data.len()
                ));
                Self::empty_response(200)
            }
            Err(MultipartError::NotFound) => Self::response(404, b"NoSuchUpload".to_vec()),
            Err(MultipartError::InvalidState) => {
                Self::response(409, b"InvalidUploadState".to_vec())
            }
        }
    }

    fn handle_complete_multipart(&mut self, bucket: &str, key: &str, upload_id: &str) -> Response {
        let upload = MultipartUpload {
            upload_id,
            bucket,
            key,
        };
        match self.multipart.complete(&upload) {
            Ok(data) => {
                self.events
                    .record(format!("MP_COMPLETE {} {}/{}", upload_id, bucket, key));
                self.handle_put(bucket, key, &data)
            }
            Err(MultipartError::NotFound) => Self::response(404, b"NoSuchUpload".to_vec()),
            Err(MultipartError::InvalidState) => {
                Self::response(409, b"InvalidUploadState".to_vec())
            }
        }
    }

    fn handle_abort_multipart(&mut self, bucket: &str, key: &str, upload_id: &str) -> Response {
        let upload = MultipartUpload {
            upload_id,
            bucket,
            key,
        };
        match self.multipart.abort(&upload) {
            Ok(()) => {
                self.events
                    .record(format!("MP_ABORT {} {}/{}", upload_id, bucket, key));
                Self::empty_response(204)
            }
            Err(MultipartError::NotFound) => Self::response(404, b"NoSuchUpload".to_vec()),
            Err(MultipartError::InvalidState) => {
                Self::response(409, b"InvalidUploadState".to_vec())
            }
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

    fn handle_logs(&self) -> Response {
        let snapshot = self.events.snapshot();
        let mut body = String::new();
        for entry in snapshot {
            body.push_str(&entry);
            body.push('\n');
        }
        Self::response(200, body.into_bytes())
    }
}

impl<C, O, S, I, M> HttpHandler for S3Service<C, O, S, I, M>
where
    C: Catalog,
    O: ObjectStore,
    S: KeyStore,
    I: MutableIndex,
    M: MultipartManager,
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

        if path == "_logs" {
            return match request.method {
                Method::Get => self.handle_logs(),
                _ => Self::response(405, b"MethodNotAllowed".to_vec()),
            };
        }

        if path.is_empty() {
            return match request.method {
                Method::Get => self.handle_list_buckets(),
                _ => Self::response(405, b"MethodNotAllowed".to_vec()),
            };
        }

        let mut parts = path.splitn(2, '/');
        let bucket = match parts.next() {
            Some(name) if !name.is_empty() => name,
            _ => return Self::response(400, b"MissingBucket".to_vec()),
        };
        let key = parts.next().unwrap_or("");
        let params = QueryParams::parse(query.unwrap_or(""));

        if params.uploads && matches!(request.method, Method::Post) {
            return self.handle_initiate_multipart(bucket, key);
        }

        if let Some(ref upload_id) = params.upload_id {
            return match request.method {
                Method::Put => {
                    if let Some(part_number) = params.part_number {
                        self.handle_upload_part(upload_id, part_number, &request.body)
                    } else {
                        Self::response(400, b"MissingPartNumber".to_vec())
                    }
                }
                Method::Post => self.handle_complete_multipart(bucket, key, upload_id),
                Method::Delete => self.handle_abort_multipart(bucket, key, upload_id),
                _ => Self::response(405, b"MethodNotAllowed".to_vec()),
            };
        }

        match request.method {
            Method::Put if key.is_empty() => self.handle_create_bucket(bucket),
            Method::Put => self.handle_put(bucket, key, &request.body),
            Method::Get if key.is_empty() => self.handle_list(bucket, query),
            Method::Get => self.handle_get(bucket, key),
            Method::Delete if key.is_empty() => self.handle_delete_bucket(bucket),
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
    uploads: bool,
    upload_id: Option<String>,
    part_number: Option<u32>,
}

impl QueryParams {
    fn parse(query: &str) -> Self {
        let mut params = Self {
            prefix: None,
            delimiter: None,
            continuation_token: None,
            max_keys: None,
            uploads: false,
            upload_id: None,
            part_number: None,
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
                "uploads" => params.uploads = true,
                "uploadId" => params.upload_id = Some(value.to_string()),
                "partNumber" => params.part_number = value.parse().ok(),
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

    fn new_service() -> S3Service<
        InMemoryCatalog,
        InMemoryObjectStore,
        InMemoryKeyStore,
        InMemoryIndex,
        InMemoryMultipart,
    > {
        let mut service = S3Service::new(
            InMemoryCatalog::new(),
            InMemoryObjectStore::new(),
            InMemoryKeyStore::new(),
            InMemoryIndex::new(),
            InMemoryMultipart::new(),
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

    fn new_empty_service() -> S3Service<
        InMemoryCatalog,
        InMemoryObjectStore,
        InMemoryKeyStore,
        InMemoryIndex,
        InMemoryMultipart,
    > {
        let mut service = S3Service::new(
            InMemoryCatalog::new(),
            InMemoryObjectStore::new(),
            InMemoryKeyStore::new(),
            InMemoryIndex::new(),
            InMemoryMultipart::new(),
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
    fn multipart_flow() {
        let mut service = new_service();
        let init_resp = service.handle(&make_request(
            Method::Post,
            "/photos/album.zip?uploads",
            Some("abc123"),
            &[],
        ));
        assert_eq!(init_resp.status, 200);
        let upload_id = core::str::from_utf8(&init_resp.body)
            .unwrap()
            .trim_start_matches("UploadId:")
            .trim()
            .to_string();

        let part_resp = service.handle(&make_request(
            Method::Put,
            &format!("/photos/album.zip?partNumber=1&uploadId={}", upload_id),
            Some("abc123"),
            b"chunk",
        ));
        assert_eq!(part_resp.status, 200);

        let complete_resp = service.handle(&make_request(
            Method::Post,
            &format!("/photos/album.zip?uploadId={}", upload_id),
            Some("abc123"),
            &[],
        ));
        assert_eq!(complete_resp.status, 200);

        let get_resp = service.handle(&make_request(
            Method::Get,
            "/photos/album.zip",
            Some("abc123"),
            &[],
        ));
        assert_eq!(get_resp.status, 200);
        assert_eq!(get_resp.body, b"chunk");
    }
}
