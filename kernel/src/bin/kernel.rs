#![no_std]
#![no_main]

use core::panic::PanicInfo;

use kernel::arch;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("../arch/x86_64/boot.S"));

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

fn log_line(message: &str) {
    let was_enabled = arch::interrupts_enabled();
    if was_enabled {
        arch::disable_interrupts();
    }
    arch::serial_write_line(message);
    if was_enabled {
        arch::enable_interrupts();
    }
}

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
pub extern "C" fn kernel_main() -> ! {
    log_line("kernel: entered kernel_main");
    kernel::init();

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"BOOT");

    kernel::scheduler::run();
    kernel::arch::unmask_timer_irq();

    let mut ack = [0u8; 16];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);

    while kernel::arch::timer_ticks() < 5 {
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    arch::serial_write_line("kernel: init complete");
    arch::serial_write_line("kernel: timer ticks observed");

    if let Some((rip, cs, err)) = arch::take_last_gp_fault() {
        arch::serial_write_line("kernel: observed GP fault");
        log_line_hex("  rip=", rip);
        log_line_hex("  cs=", cs);
        log_line_hex("  err=", err);
    }

    arch::halt()
}

fn log_line_hex(prefix: &str, value: u64) {
    let was_enabled = arch::interrupts_enabled();
    if was_enabled {
        arch::disable_interrupts();
    }
    let mut buf = [0u8; 18];
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut idx = 0;
    while idx < 16 {
        let shift = (15 - idx) * 4;
        buf[2 + idx] = HEX[((value >> shift) & 0xF) as usize];
        idx += 1;
    }
    buf[0] = b'0';
    buf[1] = b'x';
    buf[17] = b'\n';
    arch::serial_write_bytes(prefix.as_bytes());
    arch::serial_write_byte(b' ');
    arch::serial_write_bytes(&buf[..18]);
    if was_enabled {
        arch::enable_interrupts();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kernel::arch::halt()
}
