#![cfg_attr(feature = "firmware", no_std)]
#![cfg_attr(feature = "firmware", no_main)]

//! UEFI stage-0 loader that prepares the Rustcore kernel for execution.
//!
//! The loader only builds when the `firmware` feature is enabled.  This keeps the
//! workspace build fast (the default workspace member is still the kernel) while
//! giving firmware engineers a realistic foundation to iterate on.  When the
//! feature is disabled we expose a lightweight stub so unit tests and tools that
//! depend on this crate keep compiling.

#[cfg(feature = "firmware")]
extern crate alloc;

#[cfg(feature = "firmware")]
mod firmware;

#[cfg(feature = "firmware")]
pub use firmware::efi_main;

#[cfg(not(feature = "firmware"))]
#[no_mangle]
pub extern "C" fn efi_main(_image_handle: usize, _system_table: usize) -> usize {
    0
}

#[cfg(not(feature = "firmware"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
