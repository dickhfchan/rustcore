#![no_std]

pub mod channel;

/// Performs any global IPC initialization required before the scheduler starts.
pub fn init(channel: &channel::Channel) {
    channel.reset();
}
