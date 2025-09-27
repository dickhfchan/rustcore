#![allow(dead_code)]

use crate::ethernet::NetError;
use crate::ip::Ipv4Addr;

pub struct TcpEndpoint {
    pub local: (Ipv4Addr, u16),
    pub remote: (Ipv4Addr, u16),
}

pub trait TcpStack {
    fn connect(&mut self, remote: (Ipv4Addr, u16)) -> Result<TcpHandle, NetError>;
    fn listen(&mut self, local_port: u16) -> Result<TcpListener, NetError>;
}

pub struct TcpHandle;

impl TcpHandle {
    pub fn send(&mut self, _bytes: &[u8]) -> Result<(), NetError> {
        Ok(())
    }

    pub fn recv(&mut self, _buffer: &mut [u8]) -> Result<usize, NetError> {
        Ok(0)
    }
}

pub struct TcpListener;

impl TcpListener {
    pub fn accept(&mut self) -> Result<TcpHandle, NetError> {
        Err(NetError::Unsupported)
    }
}
