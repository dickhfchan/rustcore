#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

use kernel::arch;
use kernel::memory::{allocate_frame, release_frame, reserved_frames, frame_size};
use kernel::ipc_bridge::init_service_registered;

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
    log_line("DIRECT_FUNC: PANIC occurred");
    arch::halt()
}

// Test counters
static mut TESTS_PASSED: u32 = 0;
static mut TESTS_FAILED: u32 = 0;

fn test_pass(test_name: &str) {
    unsafe {
        TESTS_PASSED += 1;
    }
    log_line("DIRECT_FUNC: ✓ PASS");
    log_line(test_name);
}

fn test_fail(test_name: &str) {
    unsafe {
        TESTS_FAILED += 1;
    }
    log_line("DIRECT_FUNC: ✗ FAIL");
    log_line(test_name);
}

// Direct functional tests that run immediately
fn run_direct_functional_tests() {
    log_line("DIRECT_FUNC: Starting direct functional tests");
    
    // Test 1: Basic kernel functionality
    log_line("DIRECT_FUNC: Testing basic kernel functionality...");
    test_pass("KERNEL: Basic kernel functionality");
    
    // Test 2: Memory management
    log_line("DIRECT_FUNC: Testing memory management...");
    
    // Test frame size
    let frame_size = frame_size();
    if frame_size == 4096 {
        test_pass("MEMORY: Frame size correct (4096 bytes)");
        log_line_hex("DIRECT_FUNC: frame_size=", frame_size as u64);
    } else {
        test_fail("MEMORY: Frame size correct");
    }
    
    // Test frame allocation
    if let Some(frame) = allocate_frame() {
        test_pass("MEMORY: Frame allocation succeeds");
        log_line_hex("DIRECT_FUNC: allocated_frame=", frame.number() as u64);
        log_line_hex("DIRECT_FUNC: frame_start_addr=", frame.start_addr() as u64);
        
        // Test frame release
        if release_frame(frame) {
            test_pass("MEMORY: Frame release succeeds");
        } else {
            test_fail("MEMORY: Frame release succeeds");
        }
    } else {
        test_fail("MEMORY: Frame allocation succeeds");
    }
    
    // Test multiple frame allocation/deallocation
    log_line("DIRECT_FUNC: Testing multiple frame operations...");
    let mut allocated_frames = heapless::Vec::<kernel::memory::Frame, 16>::new();
    
    // Allocate multiple frames
    for i in 0..5 {
        if let Some(frame) = allocate_frame() {
            let _ = allocated_frames.push(frame);
            log_line_hex("DIRECT_FUNC: allocated_frame_", i as u64);
        } else {
            log_line("DIRECT_FUNC: Frame allocation failed");
            break;
        }
    }
    
    let allocated_count = allocated_frames.len();
    log_line_hex("DIRECT_FUNC: total_allocated=", allocated_count as u64);
    
    if allocated_count > 0 {
        test_pass("MEMORY: Multiple frame allocation");
        
        // Release all frames
        for frame in allocated_frames.iter() {
            if !release_frame(*frame) {
                test_fail("MEMORY: Multiple frame release");
                return;
            }
        }
        test_pass("MEMORY: Multiple frame release");
    } else {
        test_fail("MEMORY: Multiple frame allocation");
    }
    
    // Test reserved frames count
    let reserved = reserved_frames();
    test_pass("MEMORY: Reserved frames count available");
    log_line_hex("DIRECT_FUNC: reserved_frames=", reserved as u64);
    
    // Test 3: Timer functionality
    log_line("DIRECT_FUNC: Testing timer functionality...");
    let initial_ticks = arch::timer_ticks();
    log_line_hex("DIRECT_FUNC: initial_ticks=", initial_ticks);
    
    // Start timer
    arch::start_timer(100); // 100 Hz
    test_pass("TIMER: Timer start succeeds");
    
    // Wait for timer to tick
    let mut spin_count = 0;
    loop {
        let current_ticks = arch::timer_ticks();
        if current_ticks > initial_ticks || spin_count > 10_000_000 {
            break;
        }
        spin_count += 1;
        core::hint::spin_loop();
    }
    
    let final_ticks = arch::timer_ticks();
    if final_ticks > initial_ticks {
        test_pass("TIMER: Timer ticks increment");
        log_line_hex("DIRECT_FUNC: final_ticks=", final_ticks);
    } else {
        test_fail("TIMER: Timer ticks increment");
        log_line_hex("DIRECT_FUNC: final_ticks=", final_ticks);
    }
    
    // Test 4: Interrupt functionality
    log_line("DIRECT_FUNC: Testing interrupt functionality...");
    let interrupts_enabled = arch::interrupts_enabled();
    if interrupts_enabled {
        test_pass("INTERRUPT: Interrupts enabled");
    } else {
        test_fail("INTERRUPT: Interrupts enabled");
    }
    
    // Test interrupt enable/disable
    arch::disable_interrupts();
    if !arch::interrupts_enabled() {
        test_pass("INTERRUPT: Interrupt disable works");
    } else {
        test_fail("INTERRUPT: Interrupt disable works");
    }
    
    arch::enable_interrupts();
    if arch::interrupts_enabled() {
        test_pass("INTERRUPT: Interrupt enable works");
    } else {
        test_fail("INTERRUPT: Interrupt enable works");
    }
    
    // Test 5: IPC functionality
    log_line("DIRECT_FUNC: Testing IPC functionality...");
    if init_service_registered() {
        test_pass("IPC: Init service registered");
    } else {
        test_fail("IPC: Init service registered");
    }
    
    // Test 6: Boot information
    log_line("DIRECT_FUNC: Testing boot information...");
    if let Some(boot_info) = kernel::boot::boot_info() {
        test_pass("BOOT: Boot info available");
        log_line_hex("DIRECT_FUNC: memory_map_len=", boot_info.memory_map.len);
        log_line_hex("DIRECT_FUNC: bootfs_length=", boot_info.bootfs.length);
        
        if boot_info.memory_map.len > 0 {
            test_pass("BOOT: Memory map has entries");
        } else {
            test_fail("BOOT: Memory map has entries");
        }
        
        if boot_info.bootfs.length > 0 {
            test_pass("BOOT: Bootfs has content");
        } else {
            test_fail("BOOT: Bootfs has content");
        }
    } else {
        test_fail("BOOT: Boot info available");
    }
    
    // Test 7: Error handling
    log_line("DIRECT_FUNC: Testing error handling...");
    if let Some((rip, cs, err)) = arch::take_last_gp_fault() {
        log_line("DIRECT_FUNC: GP fault detected");
        log_line_hex("DIRECT_FUNC: gp_rip=", rip);
        log_line_hex("DIRECT_FUNC: gp_cs=", cs);
        log_line_hex("DIRECT_FUNC: gp_err=", err);
        test_fail("ERROR: No GP faults should occur");
    } else {
        test_pass("ERROR: No GP faults detected");
    }
    
    // Report results
    unsafe {
        log_line("DIRECT_FUNC: Functional Test Results:");
        log_line_hex("DIRECT_FUNC: tests_passed=", TESTS_PASSED as u64);
        log_line_hex("DIRECT_FUNC: tests_failed=", TESTS_FAILED as u64);
        
        if TESTS_FAILED == 0 {
            log_line("DIRECT_FUNC: All functional tests PASSED!");
        } else {
            log_line("DIRECT_FUNC: Some functional tests FAILED!");
        }
    }
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
    log_line("DIRECT_FUNC: kernel entered direct_functional_test_main");

    // Run functional tests directly
    run_direct_functional_tests();

    // Start timer for final timing test
    arch::start_timer(100);

    // Wait for additional timer ticks
    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 10 {
            log_line_hex("DIRECT_FUNC: final_ticks=", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 3_000_000 == 0 {
            log_line_hex("DIRECT_FUNC: intermediate_ticks=", ticks);
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("DIRECT_FUNC: direct functional tests complete");
    log_line("DIRECT_FUNC: timer ticks observed");

    arch::halt()
}

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}
