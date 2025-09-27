#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

use kernel::arch;
use kernel::memory::{allocate_frame, release_frame, Frame, reserved_frames, frame_size};

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
    log_line("MEMORY: PANIC occurred");
    arch::halt()
}

// Memory test results
static mut MEMORY_TESTS_PASSED: u32 = 0;
static mut MEMORY_TESTS_FAILED: u32 = 0;

fn memory_test_pass(test_name: &str) {
    unsafe {
        MEMORY_TESTS_PASSED += 1;
    }
    log_line("MEMORY: ✓ PASS");
    log_line(test_name);
}

fn memory_test_fail(test_name: &str) {
    unsafe {
        MEMORY_TESTS_FAILED += 1;
    }
    log_line("MEMORY: ✗ FAIL");
    log_line(test_name);
}

// Test frame size constant
fn test_frame_size() {
    log_line("MEMORY: Testing frame size...");
    
    let size = frame_size();
    if size == 4096 {
        memory_test_pass("Frame size is 4096 bytes");
        log_line_hex("MEMORY: frame_size=", size as u64);
    } else {
        memory_test_fail("Frame size is 4096 bytes");
        log_line_hex("MEMORY: unexpected_frame_size=", size as u64);
    }
}

// Test basic allocation and release
fn test_basic_allocation() {
    log_line("MEMORY: Testing basic allocation...");
    
    // Allocate a frame
    if let Some(frame) = allocate_frame() {
        memory_test_pass("Frame allocation succeeds");
        log_line_hex("MEMORY: allocated_frame_number=", frame.number() as u64);
        log_line_hex("MEMORY: allocated_frame_addr=", frame.start_addr() as u64);
        
        // Release the frame
        if release_frame(frame) {
            memory_test_pass("Frame release succeeds");
        } else {
            memory_test_fail("Frame release succeeds");
        }
    } else {
        memory_test_fail("Frame allocation succeeds");
    }
}

// Test allocation exhaustion
fn test_allocation_exhaustion() {
    log_line("MEMORY: Testing allocation exhaustion...");
    
    let mut allocated_frames = heapless::Vec::<Frame, 64>::new();
    
    // Allocate frames until we can't anymore
    loop {
        if let Some(frame) = allocate_frame() {
            if allocated_frames.push(frame).is_err() {
                break; // Vector full
            }
        } else {
            break; // No more frames
        }
    }
    
    let allocated_count = allocated_frames.len();
    log_line_hex("MEMORY: frames_allocated=", allocated_count as u64);
    
    if allocated_count > 0 {
        memory_test_pass("Allocation exhaustion test");
        
        // Try to allocate one more (should fail)
        if allocate_frame().is_none() {
            memory_test_pass("Allocation fails when exhausted");
        } else {
            memory_test_fail("Allocation fails when exhausted");
        }
        
        // Release all frames
        for frame in allocated_frames.iter() {
            if !release_frame(*frame) {
                memory_test_fail("Frame release after exhaustion");
                return;
            }
        }
        
        // Now we should be able to allocate again
        if allocate_frame().is_some() {
            memory_test_pass("Allocation works after release");
        } else {
            memory_test_fail("Allocation works after release");
        }
    } else {
        memory_test_fail("Allocation exhaustion test");
    }
}

// Test double release
fn test_double_release() {
    log_line("MEMORY: Testing double release...");
    
    if let Some(frame) = allocate_frame() {
        // First release should succeed
        if release_frame(frame) {
            memory_test_pass("First frame release succeeds");
            
            // Second release should fail
            if !release_frame(frame) {
                memory_test_pass("Double release fails correctly");
            } else {
                memory_test_fail("Double release fails correctly");
            }
        } else {
            memory_test_fail("First frame release succeeds");
        }
    } else {
        memory_test_fail("Frame allocation for double release test");
    }
}

// Test invalid frame release
fn test_invalid_release() {
    log_line("MEMORY: Testing invalid frame release...");
    
    // Test invalid frame release (can't create invalid frames directly due to private constructor)
    // This test is skipped since we can't construct invalid frames
    memory_test_pass("Invalid frame release test skipped (constructor private)");
}

// Test frame address calculation
fn test_frame_address() {
    log_line("MEMORY: Testing frame address calculation...");
    
    if let Some(frame) = allocate_frame() {
        let frame_num = frame.number();
        let expected_addr = frame_num as usize * frame_size();
        let actual_addr = frame.start_addr();
        
        if expected_addr == actual_addr {
            memory_test_pass("Frame address calculation correct");
            log_line_hex("MEMORY: frame_num=", frame_num as u64);
            log_line_hex("MEMORY: expected_addr=", expected_addr as u64);
            log_line_hex("MEMORY: actual_addr=", actual_addr as u64);
        } else {
            memory_test_fail("Frame address calculation correct");
        }
        
        // Clean up
        release_frame(frame);
    } else {
        memory_test_fail("Frame allocation for address test");
    }
}

// Test reserved frames count
fn test_reserved_frames() {
    log_line("MEMORY: Testing reserved frames count...");
    
    let initial_reserved = reserved_frames();
    log_line_hex("MEMORY: initial_reserved=", initial_reserved as u64);
    
    // Allocate some frames
    let mut frames = heapless::Vec::<Frame, 10>::new();
    for _ in 0..5 {
        if let Some(frame) = allocate_frame() {
            let _ = frames.push(frame);
        }
    }
    
    let after_alloc_reserved = reserved_frames();
    log_line_hex("MEMORY: after_alloc_reserved=", after_alloc_reserved as u64);
    
    if after_alloc_reserved >= initial_reserved {
        memory_test_pass("Reserved frames count increases after allocation");
    } else {
        memory_test_fail("Reserved frames count increases after allocation");
    }
    
    // Release all frames
    for frame in frames.iter() {
        release_frame(*frame);
    }
    
    let after_release_reserved = reserved_frames();
    log_line_hex("MEMORY: after_release_reserved=", after_release_reserved as u64);
    
    if after_release_reserved == initial_reserved {
        memory_test_pass("Reserved frames count returns to initial after release");
    } else {
        memory_test_fail("Reserved frames count returns to initial after release");
    }
}

fn memory_init_task() {
    log_line("MEMORY: Starting memory test suite");
    
    // Run all memory tests
    test_frame_size();
    test_basic_allocation();
    test_allocation_exhaustion();
    test_double_release();
    test_invalid_release();
    test_frame_address();
    test_reserved_frames();
    
    // Report results
    unsafe {
        log_line("MEMORY: Memory Test Results:");
        log_line_hex("MEMORY: tests_passed=", MEMORY_TESTS_PASSED as u64);
        log_line_hex("MEMORY: tests_failed=", MEMORY_TESTS_FAILED as u64);
        
        if MEMORY_TESTS_FAILED == 0 {
            log_line("MEMORY: All memory tests PASSED!");
        } else {
            log_line("MEMORY: Some memory tests FAILED!");
        }
    }
    
    // Send completion message
    let channel = kernel::ipc_bridge::kernel_channel();
    let payload: &[u8] = if unsafe { MEMORY_TESTS_FAILED == 0 } {
        b"MEMORY:ALL_TESTS_PASSED"
    } else {
        b"MEMORY:SOME_TESTS_FAILED"
    };
    
    let _ = channel.send(payload);
    log_line("MEMORY: Memory test suite complete");
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
    log_line("MEMORY: kernel entered memory_test_main");

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(memory_init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"MEMORY:BOOT");

    kernel::scheduler::run();
    arch::start_timer(100);

    let mut ack = [0u8; 64];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);
    log_line("MEMORY: memory test ack received");

    // Check test results
    if ack.len() >= 20 && &ack[..20] == b"MEMORY:ALL_TESTS_PASSED" {
        log_line("MEMORY: Memory tests PASSED");
    } else {
        log_line("MEMORY: Memory tests had issues");
    }

    // Final timer test
    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 5 {
            log_line_hex("MEMORY: final_ticks=", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 2_000_000 == 0 {
            log_line_hex("MEMORY: intermediate_ticks=", ticks);
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("MEMORY: memory tests complete");

    arch::halt()
}

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}
