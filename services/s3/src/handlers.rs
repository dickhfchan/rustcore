#![allow(dead_code)]

use filesystem::catalog::Catalog;
use services_s3_types::{BucketOp, ObjectOp};

pub trait BucketHandler {
    fn handle(&mut self, op: BucketOp<'_>) -> Result<(), HandlerError>;
}

pub trait ObjectHandler {
    fn handle(&mut self, op: ObjectOp<'_>) -> Result<(), HandlerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerError {
    Catalog,
    Storage,
}

/// Temporary type module used until the real request structures are defined.
pub mod services_s3_types {
    #[derive(Debug)]
    pub struct BucketOp<'a> {
        pub name: &'a str,
    }

    #[derive(Debug)]
    pub struct ObjectOp<'a> {
        pub bucket: &'a str,
        pub key: &'a str,
    }
}
