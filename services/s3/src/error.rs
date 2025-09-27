#![allow(dead_code)]

pub enum S3Error<'a> {
    AccessDenied,
    NoSuchBucket(&'a str),
    NoSuchKey(&'a str),
    Internal,
}

impl<'a> S3Error<'a> {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::AccessDenied => 403,
            Self::NoSuchBucket(_) | Self::NoSuchKey(_) => 404,
            Self::Internal => 500,
        }
    }
}
