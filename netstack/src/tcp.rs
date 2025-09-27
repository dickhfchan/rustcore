#![allow(dead_code)]

use crate::ethernet::NetError;
use crate::ip::{Ipv4Addr, LoopbackIp};

pub struct TcpEndpoint {
    pub local: (Ipv4Addr, u16),
    pub remote: (Ipv4Addr, u16),
}

pub struct TcpHandle<'a, D: crate::ethernet::EthernetDriver> {
    ip: &'a mut LoopbackIp<D>,
    remote: (Ipv4Addr, u16),
}

impl<'a, D: crate::ethernet::EthernetDriver> TcpHandle<'a, D> {
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        self.ip.send(bytes)
    }

    pub fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, NetError> {
        self.ip.receive(buffer)
    }
}

impl<'a, D: crate::ethernet::EthernetDriver> TcpHandle<'a, D> {
    pub fn new(ip: &'a mut LoopbackIp<D>, remote: (Ipv4Addr, u16)) -> Self {
        Self { ip, remote }
    }
}
