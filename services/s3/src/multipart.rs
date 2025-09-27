#![allow(dead_code)]

pub struct MultipartUpload<'a> {
    pub upload_id: &'a str,
    pub bucket: &'a str,
    pub key: &'a str,
}

pub trait MultipartManager {
    fn initiate(&mut self, request: &MultipartUpload<'_>) -> Result<(), MultipartError>;
    fn complete(&mut self, request: &MultipartUpload<'_>) -> Result<(), MultipartError>;
    fn abort(&mut self, request: &MultipartUpload<'_>) -> Result<(), MultipartError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipartError {
    NotFound,
    InvalidState,
}
