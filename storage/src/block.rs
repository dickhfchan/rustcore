#![allow(dead_code)]

/// Minimal trait representing a block device interface.
pub trait BlockDevice {
    fn read(&self, lba: u64, buffer: &mut [u8]) -> Result<(), BlockError>;
    fn write(&self, lba: u64, buffer: &[u8]) -> Result<(), BlockError>;
    fn flush(&self) -> Result<(), BlockError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    OutOfRange,
    Io,
    Unsupported,
}

/// Stub builder that future drivers can implement.
pub struct BlockDeviceBuilder;

impl BlockDeviceBuilder {
    pub const fn new() -> Self {
        Self
    }
}
