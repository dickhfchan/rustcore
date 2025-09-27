#![allow(dead_code)]

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    NotFound,
    AlreadyExists,
    Backend,
}
