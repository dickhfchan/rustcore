#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod boot;
pub mod ipc_bridge;
pub mod memory;
pub mod scheduler;
pub mod sync;

/// Initializes core kernel subsystems in dependency order.
pub fn init(boot_info: Option<&'static bootproto::BootInfo>) {
    boot::init(boot_info);
    arch::init();
    memory::init(boot_info);
    ipc_bridge::init();
    scheduler::init();
    arch::enable_interrupts();
}
