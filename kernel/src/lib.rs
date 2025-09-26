#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod ipc_bridge;
pub mod memory;
pub mod scheduler;
pub mod sync;

/// Initializes core kernel subsystems in dependency order.
pub fn init() {
    arch::init();
    memory::init();
    ipc_bridge::init();
    scheduler::init();
    arch::enable_interrupts();
}
