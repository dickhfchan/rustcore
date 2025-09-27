#![allow(dead_code)]

use crate::object::{ObjectError, ObjectId};

/// Placeholder for version-tracking primitives.
pub trait VersionStore {
    fn record(&mut self, object: ObjectId) -> Result<(), VersionError>;
    fn purge(&mut self, object: ObjectId) -> Result<(), VersionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionError {
    Object(ObjectError),
    Unsupported,
}
