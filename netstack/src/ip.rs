#![allow(dead_code)]

use alloc::vec::Vec;

use crate::ethernet::{EthernetDriver, EthernetFrame, MacAddress, NetError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Addr(pub [u8; 4]);

pub struct LoopbackIp<D: EthernetDriver> {
    driver: D,
    mac: MacAddress,
    addr: Ipv4Addr,
}

impl<D: EthernetDriver> LoopbackIp<D> {
    pub fn new(driver: D, mac: MacAddress, addr: Ipv4Addr) -> Self {
        Self { driver, mac, addr }
    }

    pub fn send(&mut self, payload: &[u8]) -> Result<(), NetError> {
        let frame = EthernetFrame {
            destination: self.mac,
            source: self.mac,
            ethertype: 0x0800,
            payload,
        };
        self.driver.transmit(&frame)
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, NetError> {
        self.driver.receive(buffer)
    }
}
