#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

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

pub trait MutableIndex: Index {
    fn insert(&mut self, bucket: &str, key: &str, meta: ObjectMetadata);
    fn remove(&mut self, bucket: &str, key: &str);
    fn purge_bucket(&mut self, bucket: &str);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    NotFound,
    Backend,
}

pub struct InMemoryIndex {
    buckets: BTreeMap<String, Vec<String>>,
    metadata: BTreeMap<(String, String), ObjectMetadata>,
}

impl InMemoryIndex {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }

    fn entries_for(&self, bucket: &str) -> Option<&Vec<String>> {
        self.buckets.get(bucket)
    }
}

impl Default for InMemoryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Index for InMemoryIndex {
    fn list(&self, request: &ListRequest<'_>) -> Result<ListResponse<'_>, IndexError> {
        let entries = self
            .entries_for(request.bucket)
            .ok_or(IndexError::NotFound)?;
        let prefix = request.prefix.unwrap_or("");
        let max_keys = if request.max_keys == 0 {
            usize::MAX
        } else {
            request.max_keys
        };

        let start = request
            .continuation
            .and_then(|token| entries.iter().position(|k| k == token))
            .map(|idx| idx + 1)
            .unwrap_or(0);

        let mut objects = Vec::new();
        let mut prefixes = Vec::new();
        let mut next_token = None;

        for key in entries.iter().skip(start) {
            if !key.starts_with(prefix) {
                continue;
            }

            if let Some(delimiter) = request.delimiter {
                if let Some(pos) = key[prefix.len()..].find(delimiter) {
                    let cp = &key[..prefix.len() + pos + 1];
                    if !prefixes.iter().any(|p| p == cp) {
                        prefixes.push(cp.to_string());
                    }
                    continue;
                }
            }

            let meta = self
                .metadata
                .get(&(request.bucket.to_owned(), key.clone()))
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

            if objects.len() == max_keys {
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
            common_prefixes: prefixes,
        })
    }
}

impl MutableIndex for InMemoryIndex {
    fn insert(&mut self, bucket: &str, key: &str, meta: ObjectMetadata) {
        let entries = self
            .buckets
            .entry(bucket.to_owned())
            .or_insert_with(Vec::new);
        if !entries.iter().any(|k| k == key) {
            entries.push(key.to_owned());
            entries.sort();
        }
        self.metadata
            .insert((bucket.to_owned(), key.to_owned()), meta);
    }

    fn remove(&mut self, bucket: &str, key: &str) {
        if let Some(entries) = self.buckets.get_mut(bucket) {
            entries.retain(|k| k != key);
        }
        self.metadata.remove(&(bucket.to_owned(), key.to_owned()));
    }

    fn purge_bucket(&mut self, bucket: &str) {
        self.buckets.remove(bucket);
        self.metadata.retain(|(b, _), _| b != bucket);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_and_delimiter() {
        let mut index = InMemoryIndex::new();
        index.insert(
            "photos",
            "2023/holiday/img1.jpg",
            ObjectMetadata {
                id: 1,
                size: 100,
                checksum: 0,
            },
        );
        index.insert(
            "photos",
            "2023/work/img2.jpg",
            ObjectMetadata {
                id: 2,
                size: 200,
                checksum: 0,
            },
        );

        let response = index
            .list(&ListRequest {
                bucket: "photos",
                prefix: Some("2023/"),
                delimiter: Some('/'),
                continuation: None,
                max_keys: 100,
            })
            .unwrap();

        assert!(response.objects.len() <= 1);
        assert_eq!(response.common_prefixes.len(), 2);
    }
}
