#![allow(dead_code)]

use alloc::vec::Vec;

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

/// Simple loopback driver storing the last transmitted payload.
pub struct LoopbackDriver {
    buffer: Vec<u8>,
}

impl LoopbackDriver {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl Default for LoopbackDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl EthernetDriver for LoopbackDriver {
    fn transmit(&mut self, frame: &EthernetFrame<'_>) -> Result<(), NetError> {
        self.buffer.clear();
        self.buffer.extend_from_slice(frame.payload);
        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, NetError> {
        if self.buffer.is_empty() {
            return Err(NetError::Device);
        }
        let len = buffer.len().min(self.buffer.len());
        buffer[..len].copy_from_slice(&self.buffer[..len]);
        Ok(len)
    }
}
