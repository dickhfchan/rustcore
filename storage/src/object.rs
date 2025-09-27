#![allow(dead_code)]

use crate::block::BlockError;

/// Identifier assigned to each stored object.
pub type ObjectId = u128;

/// Metadata stub describing an object.
pub struct ObjectMetadata {
    pub id: ObjectId,
    pub size: u64,
    pub checksum: u64,
}

/// Storage backend trait that higher layers can target.
pub trait ObjectStore {
    fn put(&mut self, key: &str, data: &[u8]) -> Result<ObjectMetadata, ObjectError>;
    fn get(&self, key: &str, buffer: &mut [u8]) -> Result<ObjectMetadata, ObjectError>;
    fn delete(&mut self, key: &str) -> Result<(), ObjectError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectError {
    NotFound,
    Backend(BlockError),
    InvalidKey,
}
