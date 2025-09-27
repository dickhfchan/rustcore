//! Bootloader handoff helpers and shared state.

use bootproto::{BootInfo, MemoryRange, PointerRange, BOOTINFO_VERSION};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

static BOOT_INFO: AtomicPtr<BootInfo> = AtomicPtr::new(ptr::null_mut());

#[no_mangle]
pub static mut RUSTCORE_BOOTINFO: BootInfo = BootInfo {
    version: BOOTINFO_VERSION,
    flags: 0,
    stack_top: 0,
    memory_map: PointerRange::empty(),
    rsdp: 0,
    bootfs: MemoryRange::empty(),
    kernel_digest: [0; 32],
};

/// Records the [`BootInfo`] structure passed in by the loader, if any.
pub fn init(boot_info: Option<&'static BootInfo>) {
    if let Some(info) = boot_info {
        BOOT_INFO.store(info as *const BootInfo as *mut BootInfo, Ordering::Release);
    }
}

/// Returns the loader-provided [`BootInfo`], when available.
pub fn boot_info() -> Option<&'static BootInfo> {
    let ptr = BOOT_INFO.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

/// Returns the staged boot filesystem extent when provided by the loader.
pub fn bootfs_range() -> Option<MemoryRange> {
    boot_info().and_then(|info| {
        if info.bootfs.is_empty() {
            None
        } else {
            Some(info.bootfs)
        }
    })
}

/// Returns the kernel image digest calculated by the loader.
pub fn kernel_digest() -> Option<[u8; 32]> {
    boot_info().map(|info| info.kernel_digest)
}
