#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    include_str!("../src/arch/x86_64/boot.S"),
    options(att_syntax)
);

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".section .note.xen.pvh,\"a\",@note",
    ".align 4",
    ".long 4f - 3f",
    ".long 6f - 5f",
    ".long 0x12",
    "3:",
    ".asciz \"Xen\"",
    "4:",
    ".align 4",
    "5:",
    ".long _start",
    "6:",
    ".align 4",
    options(att_syntax)
);

fn init_task() {
    let channel = kernel::ipc_bridge::kernel_channel();
    let outcome = services_init::bootstrap(channel, kernel::boot::boot_info());
    let payload: &[u8] = if outcome.receive_error.is_none() {
        b"INIT:READY"
    } else {
        b"INIT:FAIL"
    };
    let _ = channel.send(payload);
}

extern "C" {
    static mut RUSTCORE_BOOTINFO: BootInfo;
}

#[no_mangle]
pub extern "C" fn rustcore_entry(info: *const BootInfo) -> ! {
    let boot_info_ref: &'static BootInfo = unsafe {
        if info.is_null() {
            &*core::ptr::addr_of!(RUSTCORE_BOOTINFO)
        } else {
            &*info
        }
    };

    kernel::init(Some(boot_info_ref));
    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"BOOT");

    kernel::scheduler::run();

    let mut buf = [0u8; 16];
    let bytes = kernel::ipc_bridge::receive_bootstrap_message(&mut buf);
    if bytes.is_err() {
        failure_loop()
    } else {
        success_loop()
    }
}

fn success_loop() -> ! {
    kernel::arch::qemu::exit_success()
}

fn failure_loop() -> ! {
    kernel::arch::qemu::exit_failure()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    failure_loop()
}
