#![allow(dead_code)]

/// Listing parameters for S3-style object listings.
pub struct ListRequest<'a> {
    pub bucket: &'a str,
    pub prefix: Option<&'a str>,
    pub delimiter: Option<char>,
    pub continuation: Option<&'a str>,
    pub max_keys: usize,
}

pub struct ListResponse<'a> {
    pub objects: &'a [ListObject<'a>],
    pub next_token: Option<&'a str>,
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
