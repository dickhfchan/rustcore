#![allow(dead_code)]

pub enum Method {
    Get,
    Put,
    Post,
    Delete,
    Head,
}

pub struct Request<'a> {
    pub method: Method,
    pub path: &'a str,
    pub headers: &'a [Header<'a>],
    pub body: &'a [u8],
}

pub struct Response<'a> {
    pub status: u16,
    pub headers: &'a [Header<'a>],
    pub body: &'a [u8],
}

pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

pub trait HttpHandler {
    fn handle(&mut self, request: &Request<'_>) -> Response<'_>;
}
