#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub enum Method {
    Get,
    Put,
    Post,
    Delete,
    Head,
    Unknown,
}

pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

pub trait HttpHandler {
    fn handle(&mut self, request: &Request) -> Response;
}

pub fn parse_request(raw: &[u8]) -> Option<Request> {
    let text = core::str::from_utf8(raw).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = match parts.next()? {
        "GET" => Method::Get,
        "PUT" => Method::Put,
        "POST" => Method::Post,
        "DELETE" => Method::Delete,
        "HEAD" => Method::Head,
        _ => Method::Unknown,
    };
    let path = parts.next()?.to_string();

    let mut headers = Vec::new();
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push(Header {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            });
        }
    }

    let mut body = Vec::new();
    for line in lines {
        if !body.is_empty() {
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(line.as_bytes());
    }

    Some(Request {
        method,
        path,
        headers,
        body,
    })
}

pub fn build_response(response: &Response) -> Vec<u8> {
    let mut buffer = Vec::new();
    use alloc::fmt::Write;
    let mut status_line = alloc::string::String::new();
    let _ = write!(
        status_line,
        "HTTP/1.1 {}\r\n",
        match response.status {
            200 => "200 OK",
            403 => "403 Forbidden",
            404 => "404 Not Found",
            500 => "500 Internal Server Error",
            other => {
                let mut code = alloc::string::String::new();
                let _ = write!(code, "{}", other);
                code
            }
        }
    );
    buffer.extend_from_slice(status_line.as_bytes());
    for header in &response.headers {
        buffer.extend_from_slice(header.name.as_bytes());
        buffer.extend_from_slice(b": ");
        buffer.extend_from_slice(header.value.as_bytes());
        buffer.extend_from_slice(b"\r\n");
    }
    buffer.extend_from_slice(b"\r\n");
    buffer.extend_from_slice(&response.body);
    buffer
}

#[cfg(test)]
mod tests {
    use super::{build_response, parse_request, Method, Response};

    #[test]
    fn parse_basic_request() {
        let raw = b"GET /bucket/object HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let request = parse_request(raw).unwrap();
        assert!(matches!(request.method, Method::Get));
        assert_eq!(request.path, "/bucket/object");
        assert_eq!(request.headers.len(), 1);
    }

    #[test]
    fn build_basic_response() {
        let response = Response {
            status: 200,
            headers: alloc::vec![super::Header {
                name: "Content-Length".to_string(),
                value: "0".to_string(),
            }],
            body: alloc::vec![],
        };
        let serialized = build_response(&response);
        let text = core::str::from_utf8(&serialized).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK"));
    }
}
