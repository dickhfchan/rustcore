#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub struct MultipartUpload<'a> {
    pub upload_id: &'a str,
    pub bucket: &'a str,
    pub key: &'a str,
}

pub struct MultipartPart<'a> {
    pub upload_id: &'a str,
    pub part_number: u32,
    pub data: &'a [u8],
}

pub trait MultipartManager {
    fn initiate(&mut self, bucket: &str, key: &str) -> Result<String, MultipartError>;
    fn put_part(&mut self, part: MultipartPart<'_>) -> Result<(), MultipartError>;
    fn complete(&mut self, request: &MultipartUpload<'_>) -> Result<Vec<u8>, MultipartError>;
    fn abort(&mut self, request: &MultipartUpload<'_>) -> Result<(), MultipartError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipartError {
    NotFound,
    InvalidState,
}

pub struct InMemoryMultipart {
    uploads: BTreeMap<String, UploadState>,
}

struct UploadState {
    bucket: String,
    key: String,
    parts: BTreeMap<u32, Vec<u8>>,
}

impl InMemoryMultipart {
    pub fn new() -> Self {
        Self {
            uploads: BTreeMap::new(),
        }
    }
}

impl Default for InMemoryMultipart {
    fn default() -> Self {
        Self::new()
    }
}

impl MultipartManager for InMemoryMultipart {
    fn initiate(&mut self, bucket: &str, key: &str) -> Result<String, MultipartError> {
        let upload_id = format!("{}:{}:{}", bucket, key, self.uploads.len() + 1);
        let state = UploadState {
            bucket: bucket.to_string(),
            key: key.to_string(),
            parts: BTreeMap::new(),
        };
        self.uploads.insert(upload_id.clone(), state);
        Ok(upload_id)
    }

    fn put_part(&mut self, part: MultipartPart<'_>) -> Result<(), MultipartError> {
        let state = self
            .uploads
            .get_mut(part.upload_id)
            .ok_or(MultipartError::NotFound)?;
        state.parts.insert(part.part_number, part.data.to_vec());
        Ok(())
    }

    fn complete(&mut self, request: &MultipartUpload<'_>) -> Result<Vec<u8>, MultipartError> {
        let state = self
            .uploads
            .remove(request.upload_id)
            .ok_or(MultipartError::NotFound)?;
        if state.bucket != request.bucket || state.key != request.key {
            return Err(MultipartError::InvalidState);
        }
        let mut combined = Vec::new();
        for (_number, data) in state.parts {
            combined.extend_from_slice(&data);
        }
        Ok(combined)
    }

    fn abort(&mut self, request: &MultipartUpload<'_>) -> Result<(), MultipartError> {
        let state = self
            .uploads
            .remove(request.upload_id)
            .ok_or(MultipartError::NotFound)?;
        if state.bucket != request.bucket || state.key != request.key {
            return Err(MultipartError::InvalidState);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initiate_put_complete() {
        let mut manager = InMemoryMultipart::new();
        let upload_id = manager.initiate("photos", "album.zip").unwrap();
        manager
            .put_part(MultipartPart {
                upload_id: &upload_id,
                part_number: 1,
                data: b"part1",
            })
            .unwrap();
        manager
            .put_part(MultipartPart {
                upload_id: &upload_id,
                part_number: 2,
                data: b"part2",
            })
            .unwrap();
        let combined = manager
            .complete(&MultipartUpload {
                upload_id: &upload_id,
                bucket: "photos",
                key: "album.zip",
            })
            .unwrap();
        assert_eq!(combined, b"part1part2");
    }

    #[test]
    fn abort_discard_upload() {
        let mut manager = InMemoryMultipart::new();
        let upload_id = manager.initiate("docs", "report.bin").unwrap();
        manager
            .put_part(MultipartPart {
                upload_id: &upload_id,
                part_number: 1,
                data: b"data",
            })
            .unwrap();
        assert!(manager
            .abort(&MultipartUpload {
                upload_id: &upload_id,
                bucket: "docs",
                key: "report.bin",
            })
            .is_ok());
        assert!(matches!(
            manager.complete(&MultipartUpload {
                upload_id: &upload_id,
                bucket: "docs",
                key: "report.bin",
            }),
            Err(MultipartError::NotFound)
        ));
    }
}
