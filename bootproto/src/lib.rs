#![no_std]

//! Bootloader-to-kernel handoff structures shared between the Rustcore kernel
//! and any stage-0 loader. The format is intentionally plain C so a loader can
//! populate the fields without depending on Rust support libraries.

use core::marker::PhantomData;

/// Increment the version each time the layout of [`BootInfo`] changes.
pub const BOOTINFO_VERSION: u16 = 1;

/// Describes the execution environment prepared by the boot loader.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BootInfo {
    /// Format version for compatibility checks.
    pub version: u16,
    /// Reserved for future flags (must be zero for now).
    pub flags: u16,
    /// Physical address of the loader's stack top when control transferred.
    pub stack_top: u64,
    /// Bounds of the bootloader-provided memory map.
    pub memory_map: PointerRange<MemoryRegion>,
    /// ACPI RSDP physical address when available.
    pub rsdp: u64,
    /// Location of the boot filesystem payload.
    pub bootfs: MemoryRange,
    /// SHA-256 digest of the kernel image.
    pub kernel_digest: Sha256Digest,
}

impl BootInfo {
    /// Returns `true` when the structure is considered compatible with the
    /// current kernel expectations.
    pub fn is_compatible(&self) -> bool {
        self.version == BOOTINFO_VERSION
    }
}

/// A pointer + length pair that describes an array of `T` in physical memory.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PointerRange<T> {
    pub base: u64,
    pub len: u64,
    #[doc(hidden)]
    pub marker: PhantomData<T>,
}

impl<T> PointerRange<T> {
    /// A helper to construct an empty pointer range.
    pub const fn empty() -> Self {
        Self {
            base: 0,
            len: 0,
            marker: PhantomData,
        }
    }

    /// Returns true if no elements are described.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Describes a physical extent in bytes.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MemoryRange {
    pub base: u64,
    pub length: u64,
}

impl MemoryRange {
    /// Constructs an empty range.
    pub const fn empty() -> Self {
        Self { base: 0, length: 0 }
    }

    /// Returns true if this range contains no bytes.
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// Entry in the physical memory map provided by firmware.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub kind: MemoryRegionKind,
}

/// Memory classifications understood by the kernel during early boot.
#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MemoryRegionKind {
    UsableRam = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    Mmio = 5,
}

/// SHA-256 digest placeholder used by the loader to attest the kernel image.
pub type Sha256Digest = [u8; 32];
