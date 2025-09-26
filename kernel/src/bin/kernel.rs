#![no_std]
#![no_main]

use core::panic::PanicInfo;

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
);

fn init_task() {
    let channel = kernel::ipc_bridge::kernel_channel();

    let outcome = services_init::bootstrap(channel);
    let payload: &[u8] = if outcome.receive_error.is_none() {
        b"INIT:READY"
    } else {
        b"INIT:FAIL"
    };

    let _ = channel.send(payload);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::init();

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"BOOT");

    kernel::arch::serial_write_line("kernel: init task scheduled");
    kernel::scheduler::run();

    let mut ack = [0u8; 16];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);

    kernel::arch::serial_write_line("kernel: init ack received");

    while kernel::arch::timer_ticks() < 5 {
        core::hint::spin_loop();
    }

    kernel::arch::serial_write_line("kernel: timer ticks observed");

    kernel::arch::halt()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kernel::arch::halt()
}
