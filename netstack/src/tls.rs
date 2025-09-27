#![allow(dead_code)]

use crate::ethernet::NetError;
use crate::tcp::TcpHandle;

pub struct TlsSession {
    handle: TcpHandle,
}

impl TlsSession {
    pub fn new(handle: TcpHandle) -> Self {
        Self { handle }
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        self.handle.send(bytes)
    }

    pub fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, NetError> {
        self.handle.recv(buffer)
    }
}
