#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

use kernel::arch;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("../arch/x86_64/boot.S"), options(att_syntax));

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

    let outcome = services_init::bootstrap(channel, kernel::boot::boot_info());
    let payload: &[u8] = if outcome.receive_error.is_none() {
        b"INIT:READY"
    } else {
        b"INIT:FAIL"
    };

    let _ = channel.send(payload);
}

#[no_mangle]
pub extern "C" fn rustcore_entry(boot_info_ptr: *const BootInfo) -> ! {
    let boot_info_ref: &'static BootInfo = unsafe {
        if boot_info_ptr.is_null() {
            &*core::ptr::addr_of!(kernel::boot::RUSTCORE_BOOTINFO)
        } else {
            &*boot_info_ptr
        }
    };

    kernel::init(Some(boot_info_ref));
    log_line("kernel: entered kernel_main");

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"BOOT");

    kernel::scheduler::run();
    arch::start_timer(100);

    let mut ack = [0u8; 16];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);
    log_line("kernel: init ack received");

    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 5 {
            log_line_hex("ticks", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 1_000_000 == 0 {
            log_line_hex("ticks", ticks);
        }
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

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}

fn log_line_hex(prefix: &str, value: u64) {
    let was_enabled = arch::interrupts_enabled();
    if was_enabled {
        arch::disable_interrupts();
    }
    arch::serial_write_bytes(prefix.as_bytes());
    arch::serial_write_byte(b' ');
    arch::serial_write_byte(b'0');
    arch::serial_write_byte(b'x');
    arch::serial_write_u64_hex(value);
    arch::serial_write_byte(b'\n');
    if was_enabled {
        arch::enable_interrupts();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kernel::arch::halt()
}
