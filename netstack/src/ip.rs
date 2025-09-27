#![allow(dead_code)]

use crate::ethernet::NetError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Addr(pub [u8; 4]);

pub trait IpStack {
    fn send(&mut self, dst: Ipv4Addr, protocol: u8, payload: &[u8]) -> Result<(), NetError>;
    fn poll(&mut self) -> Result<(), NetError>;
}
