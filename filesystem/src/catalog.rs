#![allow(dead_code)]

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;

use storage::object::ObjectMetadata;

/// Minimal bucket descriptor.
pub struct BucketInfo<'a> {
    pub name: &'a str,
    pub object_count: u64,
}

pub trait Catalog {
    fn create_bucket(&mut self, name: &str) -> Result<(), CatalogError>;
    fn delete_bucket(&mut self, name: &str) -> Result<(), CatalogError>;
    fn list_buckets(&self, sink: &mut dyn FnMut(BucketInfo<'_>));
    fn put_object(
        &mut self,
        bucket: &str,
        key: &str,
        meta: ObjectMetadata,
    ) -> Result<(), CatalogError>;
    fn remove_object(&mut self, bucket: &str, key: &str) -> Result<(), CatalogError>;
    fn object_metadata(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, CatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    NotFound,
    AlreadyExists,
    InvalidName,
    Backend,
}

#[derive(Default)]
pub struct InMemoryCatalog {
    buckets: BTreeMap<String, Bucket>,
}

struct Bucket {
    objects: BTreeMap<String, ObjectMetadata>,
}

impl Default for Bucket {
    fn default() -> Self {
        Self {
            objects: BTreeMap::new(),
        }
    }
}

impl InMemoryCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    fn bucket_mut(&mut self, name: &str) -> Result<&mut Bucket, CatalogError> {
        self.buckets.get_mut(name).ok_or(CatalogError::NotFound)
    }

    fn bucket(&self, name: &str) -> Result<&Bucket, CatalogError> {
        self.buckets.get(name).ok_or(CatalogError::NotFound)
    }
}

impl Catalog for InMemoryCatalog {
    fn create_bucket(&mut self, name: &str) -> Result<(), CatalogError> {
        if name.is_empty() {
            return Err(CatalogError::InvalidName);
        }
        if self.buckets.contains_key(name) {
            return Err(CatalogError::AlreadyExists);
        }
        self.buckets.insert(name.to_owned(), Bucket::default());
        Ok(())
    }

    fn delete_bucket(&mut self, name: &str) -> Result<(), CatalogError> {
        self.buckets
            .remove(name)
            .map(|_| ())
            .ok_or(CatalogError::NotFound)
    }

    fn list_buckets(&self, sink: &mut dyn FnMut(BucketInfo<'_>)) {
        for (name, bucket) in &self.buckets {
            sink(BucketInfo {
                name,
                object_count: bucket.objects.len() as u64,
            });
        }
    }

    fn put_object(
        &mut self,
        bucket: &str,
        key: &str,
        meta: ObjectMetadata,
    ) -> Result<(), CatalogError> {
        if key.is_empty() {
            return Err(CatalogError::InvalidName);
        }
        let bucket = self.bucket_mut(bucket)?;
        bucket.objects.insert(key.to_owned(), meta);
        Ok(())
    }

    fn remove_object(&mut self, bucket: &str, key: &str) -> Result<(), CatalogError> {
        let bucket = self.bucket_mut(bucket)?;
        bucket
            .objects
            .remove(key)
            .map(|_| ())
            .ok_or(CatalogError::NotFound)
    }

    fn object_metadata(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, CatalogError> {
        let bucket = self.bucket(bucket)?;
        bucket
            .objects
            .get(key)
            .copied()
            .ok_or(CatalogError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list_bucket() {
        let mut catalog = InMemoryCatalog::new();
        catalog.create_bucket("photos").unwrap();

        let mut buckets = alloc::vec::Vec::new();
        catalog.list_buckets(&mut |info| buckets.push(info.name.to_owned()));
        assert_eq!(buckets, alloc::vec!["photos".to_string()]);
    }

    #[test]
    fn put_and_fetch_object() {
        let mut catalog = InMemoryCatalog::new();
        catalog.create_bucket("docs").unwrap();
        let meta = ObjectMetadata {
            id: 7,
            size: 128,
            checksum: 0,
        };
        catalog.put_object("docs", "file.txt", meta).unwrap();

        let stored = catalog.object_metadata("docs", "file.txt").unwrap();
        assert_eq!(stored.size, 128);
    }
}
