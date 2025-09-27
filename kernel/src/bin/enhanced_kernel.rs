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

fn enhanced_init_task() {
    let channel = kernel::ipc_bridge::kernel_channel();

    log_line("ENHANCED: Starting enhanced init task");

    // Validate boot info
    if let Some(boot_info) = kernel::boot::boot_info() {
        log_line("ENHANCED: Boot info available");
        log_line("ENHANCED: Memory map length");
        log_line_hex("  memory_map_len=", boot_info.memory_map.len);
    } else {
        log_line("ENHANCED: No boot info available");
    }

    // Test timer functionality
    log_line("ENHANCED: Checking timer ticks");
    let initial_ticks = arch::timer_ticks();
    log_line("ENHANCED: Initial timer ticks");
    log_line_hex("  ticks=", initial_ticks);

    // Run the bootstrap
    let outcome = services_init::bootstrap(channel, kernel::boot::boot_info());

    // Validate bootstrap outcome
    log_line("ENHANCED: Bootstrap completed");
    if outcome.receive_error.is_none() {
        log_line("ENHANCED: Bootstrap successful");
        log_line("ENHANCED: Bootfs available");
        log_line_hex("  bootfs_length=", outcome.bootfs.length());
    } else {
        log_line("ENHANCED: Bootstrap failed");
    }

    // Test manifest validation
    log_line("ENHANCED: Manifest validation");
    if outcome.manifest.error.is_none() {
        log_line("ENHANCED: Manifest valid");
        log_line("ENHANCED: Service count");
        log_line_hex("  services=", outcome.manifest.services as u64);
    } else {
        log_line("ENHANCED: Manifest invalid");
    }

    let payload: &[u8] = if outcome.receive_error.is_none() {
        b"INIT:ENHANCED:READY"
    } else {
        b"INIT:ENHANCED:FAIL"
    };

    let _ = channel.send(payload);
    log_line("ENHANCED: Enhanced init task complete");
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
    log_line("ENHANCED: kernel entered enhanced_kernel_main");

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(enhanced_init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"ENHANCED:BOOT");

    kernel::scheduler::run();
    arch::start_timer(100);

    let mut ack = [0u8; 32];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);
    log_line("ENHANCED: init ack received");

    // Check if we got the enhanced response
    if ack.len() >= 17 && &ack[..17] == b"INIT:ENHANCED:READY" {
        log_line("ENHANCED: Enhanced test PASSED");
    } else {
        log_line("ENHANCED: Enhanced test FAILED");
    }

    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 5 {
            log_line_hex("ENHANCED: final_ticks=", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 1_000_000 == 0 {
            log_line_hex("ENHANCED: intermediate_ticks=", ticks);
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("ENHANCED: init complete");
    log_line("ENHANCED: timer ticks observed");

    if let Some((rip, cs, err)) = arch::take_last_gp_fault() {
        log_line("ENHANCED: observed GP fault");
        log_line_hex("  rip=", rip);
        log_line_hex("  cs=", cs);
        log_line_hex("  err=", err);
    }

    arch::halt()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    log_line("ENHANCED: PANIC occurred");
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
    arch::serial_write_line(prefix);
    arch::serial_write_u64_hex(value);
    arch::serial_write_byte(b'\n');
    if was_enabled {
        arch::enable_interrupts();
    }
}
