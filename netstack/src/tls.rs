#![allow(dead_code)]

use crate::ethernet::NetError;
use crate::tcp::TcpHandle;

pub struct TlsSession<'a, D: crate::ethernet::EthernetDriver> {
    handle: TcpHandle<'a, D>,
}

impl<'a, D: crate::ethernet::EthernetDriver> TlsSession<'a, D> {
    pub fn new(handle: TcpHandle<'a, D>) -> Self {
        Self { handle }
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        self.handle.send(bytes)
    }

    pub fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, NetError> {
        self.handle.recv(buffer)
    }
}
