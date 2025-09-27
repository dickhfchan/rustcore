#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::block::BlockError;

/// Identifier assigned to each stored object.
pub type ObjectId = u128;

/// Metadata stub describing an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Simple in-memory object store useful for early integration testing.
pub struct InMemoryObjectStore {
    next_id: ObjectId,
    objects: BTreeMap<String, Vec<u8>>,
}

impl InMemoryObjectStore {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            objects: BTreeMap::new(),
        }
    }

    fn allocate_id(&mut self) -> ObjectId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }
}

impl Default for InMemoryObjectStore {
    fn default() -> Self {
        Self {
            next_id: 1,
            objects: BTreeMap::new(),
        }
    }
}

impl ObjectStore for InMemoryObjectStore {
    fn put(&mut self, key: &str, data: &[u8]) -> Result<ObjectMetadata, ObjectError> {
        if key.is_empty() {
            return Err(ObjectError::InvalidKey);
        }
        let id = self.allocate_id();
        self.objects.insert(key.to_string(), data.to_vec());
        Ok(ObjectMetadata {
            id,
            size: data.len() as u64,
            checksum: 0,
        })
    }

    fn get(&self, key: &str, buffer: &mut [u8]) -> Result<ObjectMetadata, ObjectError> {
        let data = self.objects.get(key).ok_or(ObjectError::NotFound)?;
        if buffer.len() < data.len() {
            return Err(ObjectError::Backend(BlockError::OutOfRange));
        }
        buffer[..data.len()].copy_from_slice(data);
        Ok(ObjectMetadata {
            id: 0,
            size: data.len() as u64,
            checksum: 0,
        })
    }

    fn delete(&mut self, key: &str) -> Result<(), ObjectError> {
        self.objects
            .remove(key)
            .map(|_| ())
            .ok_or(ObjectError::NotFound)
    }
}
