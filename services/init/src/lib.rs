#![no_std]

use ipc::channel::{Channel, ReceiveError};

pub struct BootstrapOutcome {
    pub last_message_len: usize,
    pub receive_error: Option<ReceiveError>,
}

/// Entry point for the user-space init service once the microkernel has booted.
pub fn bootstrap(channel: &Channel) -> BootstrapOutcome {
    let mut buf = [0u8; 16];
    match channel.receive(&mut buf) {
        Ok(len) => BootstrapOutcome {
            last_message_len: len,
            receive_error: None,
        },
        Err(err) => BootstrapOutcome {
            last_message_len: 0,
            receive_error: Some(err),
        },
    }
}
