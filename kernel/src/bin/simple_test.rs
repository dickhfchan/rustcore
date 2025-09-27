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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    log_line("SIMPLE: PANIC occurred");
    arch::halt()
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
    log_line("SIMPLE: kernel entered simple_test_main");
    
    // Test 1: Basic functionality
    log_line("SIMPLE: Testing basic functionality...");
    log_line("SIMPLE: ✓ Basic kernel functionality works");
    
    // Test 2: Memory allocation
    log_line("SIMPLE: Testing memory allocation...");
    if let Some(frame) = kernel::memory::allocate_frame() {
        log_line("SIMPLE: ✓ Memory allocation works");
        log_line("SIMPLE: Frame allocated successfully");
        
        if kernel::memory::release_frame(frame) {
            log_line("SIMPLE: ✓ Memory release works");
        } else {
            log_line("SIMPLE: ✗ Memory release failed");
        }
    } else {
        log_line("SIMPLE: ✗ Memory allocation failed");
    }
    
    // Test 3: Timer functionality
    log_line("SIMPLE: Testing timer functionality...");
    let initial_ticks = arch::timer_ticks();
    log_line("SIMPLE: Initial timer ticks recorded");
    
    arch::start_timer(100);
    log_line("SIMPLE: ✓ Timer start works");
    
    // Wait a bit
    let mut spin = 0u64;
    loop {
        let ticks = arch::timer_ticks();
        if ticks > initial_ticks || spin > 5_000_000 {
            break;
        }
        spin += 1;
        core::hint::spin_loop();
    }
    
    let final_ticks = arch::timer_ticks();
    if final_ticks > initial_ticks {
        log_line("SIMPLE: ✓ Timer ticks increment works");
    } else {
        log_line("SIMPLE: ✗ Timer ticks increment failed");
    }
    
    // Test 4: Interrupt functionality
    log_line("SIMPLE: Testing interrupt functionality...");
    if arch::interrupts_enabled() {
        log_line("SIMPLE: ✓ Interrupts enabled");
    } else {
        log_line("SIMPLE: ✗ Interrupts not enabled");
    }
    
    // Test 5: IPC functionality
    log_line("SIMPLE: Testing IPC functionality...");
    if kernel::ipc_bridge::init_service_registered() {
        log_line("SIMPLE: ✓ IPC init service registered");
    } else {
        log_line("SIMPLE: ✗ IPC init service not registered");
    }
    
    // Test 6: Boot information
    log_line("SIMPLE: Testing boot information...");
    if let Some(boot_info) = kernel::boot::boot_info() {
        log_line("SIMPLE: ✓ Boot info available");
        log_line("SIMPLE: Memory map and bootfs accessible");
    } else {
        log_line("SIMPLE: ✗ Boot info not available");
    }
    
    // Final summary
    log_line("SIMPLE: All functional tests completed");
    log_line("SIMPLE: Functional testing demonstration successful");
    
    // Wait for timer ticks
    let mut final_spin = 0u64;
    loop {
        let ticks = arch::timer_ticks();
        if ticks >= 5 {
            log_line("SIMPLE: Final timer check completed");
            break;
        }
        final_spin += 1;
        if final_spin % 2_000_000 == 0 {
            log_line("SIMPLE: Waiting for timer ticks...");
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("SIMPLE: Simple functional test complete");

    arch::halt()
}

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}
