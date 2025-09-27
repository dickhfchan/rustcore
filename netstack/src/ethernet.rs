#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

pub struct EthernetFrame<'a> {
    pub destination: MacAddress,
    pub source: MacAddress,
    pub ethertype: u16,
    pub payload: &'a [u8],
}

pub trait EthernetDriver {
    fn transmit(&mut self, frame: &EthernetFrame<'_>) -> Result<(), NetError>;
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, NetError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    Device,
    Malformed,
    Unsupported,
}
