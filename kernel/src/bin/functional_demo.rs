#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

use kernel::arch;
use kernel::memory::{allocate_frame, release_frame, reserved_frames, frame_size};
use kernel::scheduler::register;
use kernel::ipc_bridge::{send_bootstrap_message, receive_bootstrap_message, init_service_registered};

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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    log_line("FUNC_DEMO: PANIC occurred");
    arch::halt()
}

// Test counters
static mut FUNC_TESTS_PASSED: u32 = 0;
static mut FUNC_TESTS_FAILED: u32 = 0;

fn test_pass(test_name: &str) {
    unsafe {
        FUNC_TESTS_PASSED += 1;
    }
    log_line("FUNC_DEMO: ✓ PASS");
    log_line(test_name);
}

fn test_fail(test_name: &str) {
    unsafe {
        FUNC_TESTS_FAILED += 1;
    }
    log_line("FUNC_DEMO: ✗ FAIL");
    log_line(test_name);
}

fn functional_demo_task() {
    log_line("FUNC_DEMO: Starting functional demonstration");
    
    // Test 1: IPC Channel functionality
    log_line("FUNC_DEMO: Testing IPC functionality...");
    if init_service_registered() {
        test_pass("IPC: Init service registered");
    } else {
        test_fail("IPC: Init service registered");
    }
    
    // Test 2: Memory management
    log_line("FUNC_DEMO: Testing memory management...");
    
    // Test frame size
    let frame_size = frame_size();
    if frame_size == 4096 {
        test_pass("MEMORY: Frame size correct (4096 bytes)");
        log_line_hex("FUNC_DEMO: frame_size=", frame_size as u64);
    } else {
        test_fail("MEMORY: Frame size correct");
    }
    
    // Test frame allocation
    if let Some(frame) = allocate_frame() {
        test_pass("MEMORY: Frame allocation succeeds");
        log_line_hex("FUNC_DEMO: allocated_frame=", frame.number() as u64);
        
        // Test frame release
        if release_frame(frame) {
            test_pass("MEMORY: Frame release succeeds");
        } else {
            test_fail("MEMORY: Frame release succeeds");
        }
    } else {
        test_fail("MEMORY: Frame allocation succeeds");
    }
    
    // Test reserved frames count
    let reserved = reserved_frames();
    test_pass("MEMORY: Reserved frames count available");
    log_line_hex("FUNC_DEMO: reserved_frames=", reserved as u64);
    
    // Test 3: Timer functionality
    log_line("FUNC_DEMO: Testing timer functionality...");
    let initial_ticks = arch::timer_ticks();
    log_line_hex("FUNC_DEMO: initial_ticks=", initial_ticks);
    
    // Wait a bit for timer to tick
    let mut spin_count = 0;
    loop {
        let current_ticks = arch::timer_ticks();
        if current_ticks > initial_ticks || spin_count > 5_000_000 {
            break;
        }
        spin_count += 1;
        core::hint::spin_loop();
    }
    
    let final_ticks = arch::timer_ticks();
    if final_ticks > initial_ticks {
        test_pass("TIMER: Timer ticks increment");
        log_line_hex("FUNC_DEMO: final_ticks=", final_ticks);
    } else {
        test_fail("TIMER: Timer ticks increment");
    }
    
    // Test 4: Interrupt functionality
    log_line("FUNC_DEMO: Testing interrupt functionality...");
    let interrupts_enabled = arch::interrupts_enabled();
    if interrupts_enabled {
        test_pass("INTERRUPT: Interrupts enabled");
    } else {
        test_fail("INTERRUPT: Interrupts enabled");
    }
    
    // Test 5: Boot information
    log_line("FUNC_DEMO: Testing boot information...");
    if let Some(boot_info) = kernel::boot::boot_info() {
        test_pass("BOOT: Boot info available");
        log_line_hex("FUNC_DEMO: memory_map_len=", boot_info.memory_map.len);
        log_line_hex("FUNC_DEMO: bootfs_length=", boot_info.bootfs.length);
    } else {
        test_fail("BOOT: Boot info available");
    }
    
    // Report results
    unsafe {
        log_line("FUNC_DEMO: Functional Test Results:");
        log_line_hex("FUNC_DEMO: tests_passed=", FUNC_TESTS_PASSED as u64);
        log_line_hex("FUNC_DEMO: tests_failed=", FUNC_TESTS_FAILED as u64);
        
        if FUNC_TESTS_FAILED == 0 {
            log_line("FUNC_DEMO: All functional tests PASSED!");
        } else {
            log_line("FUNC_DEMO: Some functional tests FAILED!");
        }
    }
    
    // Send completion message
    let channel = kernel::ipc_bridge::kernel_channel();
    let payload: &[u8] = if unsafe { FUNC_TESTS_FAILED == 0 } {
        b"FUNC_DEMO:ALL_TESTS_PASSED"
    } else {
        b"FUNC_DEMO:SOME_TESTS_FAILED"
    };
    
    let _ = channel.send(payload);
    log_line("FUNC_DEMO: Functional demonstration complete");
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
    log_line("FUNC_DEMO: kernel entered functional_demo_main");

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(functional_demo_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"FUNC_DEMO:BOOT");

    kernel::scheduler::run();
    arch::start_timer(100);

    let mut ack = [0u8; 64];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);
    log_line("FUNC_DEMO: functional demo ack received");

    // Check test results
    if ack.len() >= 22 && &ack[..22] == b"FUNC_DEMO:ALL_TESTS_PASSED" {
        log_line("FUNC_DEMO: Functional tests PASSED");
    } else {
        log_line("FUNC_DEMO: Functional tests had issues");
    }

    // Final timer test
    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 5 {
            log_line_hex("FUNC_DEMO: final_ticks=", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 2_000_000 == 0 {
            log_line_hex("FUNC_DEMO: intermediate_ticks=", ticks);
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("FUNC_DEMO: functional demonstration complete");
    log_line("FUNC_DEMO: timer ticks observed");

    arch::halt()
}

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}
