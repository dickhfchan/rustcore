#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::catalog::{Catalog, CatalogError};
use storage::object::ObjectMetadata;

/// Listing parameters for S3-style object listings.
pub struct ListRequest<'a> {
    pub bucket: &'a str,
    pub prefix: Option<&'a str>,
    pub delimiter: Option<char>,
    pub continuation: Option<&'a str>,
    pub max_keys: usize,
}

pub struct ListResponse<'a> {
    pub objects: Vec<ListObject<'a>>,
    pub next_token: Option<String>,
    pub common_prefixes: Vec<String>,
}

pub struct ListObject<'a> {
    pub key: &'a str,
    pub size: u64,
}

pub trait Index {
    fn list(&self, request: &ListRequest<'_>) -> Result<ListResponse<'_>, IndexError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    NotFound,
    Backend,
}

pub struct InMemoryIndex<'a, C: Catalog> {
    catalog: &'a C,
    buckets: BTreeMap<String, Vec<String>>,
    metadata: BTreeMap<(String, String), ObjectMetadata>,
}

impl<'a, C: Catalog> InMemoryIndex<'a, C> {
    pub fn new(catalog: &'a C) -> Self {
        Self {
            catalog,
            buckets: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, bucket: &str, key: &str, meta: ObjectMetadata) {
        let entries = self
            .buckets
            .entry(bucket.to_owned())
            .or_insert_with(Vec::new);
        if !entries.contains(&key.to_owned()) {
            entries.push(key.to_owned());
            entries.sort();
        }
        self.metadata
            .insert((bucket.to_owned(), key.to_owned()), meta);
    }
}

impl<'a, C: Catalog> Index for InMemoryIndex<'a, C> {
    fn list(&self, request: &ListRequest<'_>) -> Result<ListResponse<'_>, IndexError> {
        let entries = self
            .buckets
            .get(request.bucket)
            .ok_or(IndexError::NotFound)?;

        let prefix = request.prefix.unwrap_or("");
        let start_index = request
            .continuation
            .and_then(|token| entries.iter().position(|k| k == token))
            .map(|idx| idx + 1)
            .unwrap_or(0);

        let mut objects = Vec::new();
        let mut common_prefixes = Vec::new();
        let mut next_token = None;
        for key in entries.iter().skip(start_index) {
            if !key.starts_with(prefix) {
                continue;
            }

            if let Some(delimiter) = request.delimiter {
                if let Some(pos) = key[prefix.len()..].find(delimiter) {
                    let prefix = &key[..prefix.len() + pos + 1];
                    if !common_prefixes.iter().any(|existing| existing == prefix) {
                        common_prefixes.push(prefix.to_owned());
                    }
                    continue;
                }
            }

            let meta = self
                .metadata
                .get(&(request.bucket.to_owned(), key.to_owned()))
                .copied()
                .unwrap_or(ObjectMetadata {
                    id: 0,
                    size: 0,
                    checksum: 0,
                });

            objects.push(ListObject {
                key,
                size: meta.size,
            });

            if request.max_keys > 0 && objects.len() >= request.max_keys {
                let idx = entries
                    .iter()
                    .position(|k| k == key)
                    .unwrap_or(entries.len());
                if idx + 1 < entries.len() {
                    next_token = Some(entries[idx].clone());
                }
                break;
            }
        }

        Ok(ListResponse {
            objects,
            next_token,
            common_prefixes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Catalog, InMemoryCatalog};
    use storage::object::ObjectStore;

    #[test]
    fn list_with_prefix() {
        let mut catalog = InMemoryCatalog::new();
        catalog.create_bucket("photos").unwrap();
        let mut index = InMemoryIndex::new(&catalog);
        index.insert(
            "photos",
            "2023/vacation/img1.jpg",
            ObjectMetadata {
                id: 1,
                size: 10,
                checksum: 0,
            },
        );
        index.insert(
            "photos",
            "2023/work/img2.jpg",
            ObjectMetadata {
                id: 2,
                size: 20,
                checksum: 0,
            },
        );

        let response = index
            .list(&ListRequest {
                bucket: "photos",
                prefix: Some("2023/vacation"),
                delimiter: None,
                continuation: None,
                max_keys: 100,
            })
            .unwrap();
        assert_eq!(response.objects.len(), 1);
        assert_eq!(response.objects[0].key, "2023/vacation/img1.jpg");
    }
}
