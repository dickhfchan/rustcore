#![no_std]

mod bootfs;

pub use bootfs::{
    CapabilityIter, CapabilityList, ManifestError, ManifestSummary, ServiceDescriptor,
};

use bootfs::BootfsView;
use bootproto::BootInfo;
use ipc::channel::{Channel, ReceiveError};

pub struct BootstrapOutcome {
    pub last_message_len: usize,
    pub receive_error: Option<ReceiveError>,
    pub bootfs: BootfsView,
    pub manifest: ManifestSummary,
}

/// Entry point for the user-space init service once the microkernel has booted.
pub fn bootstrap(channel: &Channel, boot_info: Option<&BootInfo>) -> BootstrapOutcome {
    let bootfs = boot_info
        .map(|info| BootfsView::from_range(info.bootfs))
        .unwrap_or_else(BootfsView::empty);
    let manifest = bootfs.validate_manifest();

    let mut buf = [0u8; 16];
    match channel.receive(&mut buf) {
        Ok(len) => BootstrapOutcome {
            last_message_len: len,
            receive_error: None,
            bootfs,
            manifest,
        },
        Err(err) => BootstrapOutcome {
            last_message_len: 0,
            receive_error: Some(err),
            bootfs,
            manifest,
        },
    }
}
