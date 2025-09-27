#![allow(dead_code)]

/// Minimal trait representing a block device interface.
pub trait BlockDevice {
    fn read(&mut self, lba: u64, buffer: &mut [u8]) -> Result<(), BlockError>;
    fn write(&mut self, lba: u64, buffer: &[u8]) -> Result<(), BlockError>;
    fn flush(&mut self) -> Result<(), BlockError> {
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

/// In-memory block device useful for testing higher levels without hardware.
pub struct MemoryBlockDevice<'a> {
    block_size: usize,
    storage: &'a mut [u8],
}

impl<'a> MemoryBlockDevice<'a> {
    pub fn new(block_size: usize, storage: &'a mut [u8]) -> Result<Self, BlockError> {
        if block_size == 0 || storage.len() % block_size != 0 {
            return Err(BlockError::Unsupported);
        }
        Ok(Self {
            block_size,
            storage,
        })
    }

    fn bounds_check(&self, lba: u64, len: usize) -> Result<usize, BlockError> {
        let offset = lba
            .checked_mul(self.block_size as u64)
            .ok_or(BlockError::OutOfRange)? as usize;
        let end = offset.checked_add(len).ok_or(BlockError::OutOfRange)?;
        if end > self.storage.len() {
            return Err(BlockError::OutOfRange);
        }
        Ok(offset)
    }
}

impl<'a> BlockDevice for MemoryBlockDevice<'a> {
    fn read(&mut self, lba: u64, buffer: &mut [u8]) -> Result<(), BlockError> {
        if buffer.is_empty() {
            return Ok(());
        }
        let offset = self.bounds_check(lba, buffer.len())?;
        buffer.copy_from_slice(&self.storage[offset..offset + buffer.len()]);
        Ok(())
    }

    fn write(&mut self, lba: u64, buffer: &[u8]) -> Result<(), BlockError> {
        if buffer.is_empty() {
            return Ok(());
        }
        let offset = self.bounds_check(lba, buffer.len())?;
        self.storage[offset..offset + buffer.len()].copy_from_slice(buffer);
        Ok(())
    }
}
