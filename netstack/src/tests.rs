#![cfg(test)]

use super::ethernet::{LoopbackDriver, MacAddress};
use super::ip::{Ipv4Addr, LoopbackIp};
use super::tcp::TcpHandle;

#[test]
fn loopback_flow() {
    let driver = LoopbackDriver::new();
    let mut ip = LoopbackIp::new(
        driver,
        MacAddress([0, 1, 2, 3, 4, 5]),
        Ipv4Addr([127, 0, 0, 1]),
    );
    let mut handle = TcpHandle::new(&mut ip, (Ipv4Addr([127, 0, 0, 1]), 80));
    let data = b"hello";
    handle.send(data).unwrap();
    let mut buffer = [0u8; 16];
    let len = handle.recv(&mut buffer).unwrap();
    assert_eq!(&buffer[..len], data);
}
