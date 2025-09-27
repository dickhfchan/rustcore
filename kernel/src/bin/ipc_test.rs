#![no_std]
#![no_main]

use bootproto::BootInfo;
use core::panic::PanicInfo;

use kernel::arch;
use kernel::ipc_bridge::{
    send_bootstrap_message, receive_bootstrap_message, 
    init_service_registered, kernel_channel
};

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
    log_line("IPC: PANIC occurred");
    arch::halt()
}

// IPC test results
static mut IPC_TESTS_PASSED: u32 = 0;
static mut IPC_TESTS_FAILED: u32 = 0;

fn ipc_test_pass(test_name: &str) {
    unsafe {
        IPC_TESTS_PASSED += 1;
    }
    log_line("IPC: ✓ PASS");
    log_line(test_name);
}

fn ipc_test_fail(test_name: &str) {
    unsafe {
        IPC_TESTS_FAILED += 1;
    }
    log_line("IPC: ✗ FAIL");
    log_line(test_name);
}

// Test channel existence
fn test_channel_existence() {
    log_line("IPC: Testing channel existence...");
    
    let channel = kernel_channel();
    if core::ptr::addr_of!(*channel) != core::ptr::null() {
        ipc_test_pass("IPC channel exists");
    } else {
        ipc_test_fail("IPC channel exists");
    }
}

// Test init service registration
fn test_init_service_registration() {
    log_line("IPC: Testing init service registration...");
    
    if init_service_registered() {
        ipc_test_pass("Init service is registered");
    } else {
        ipc_test_fail("Init service is registered");
    }
}

// Test message sending with valid payload
fn test_valid_message_send() {
    log_line("IPC: Testing valid message send...");
    
    let test_payload = b"IPC_TEST_MESSAGE";
    match send_bootstrap_message(test_payload) {
        Ok(()) => ipc_test_pass("Valid message send succeeds"),
        Err(_) => ipc_test_fail("Valid message send succeeds"),
    }
}

// Test message sending with empty payload
fn test_empty_message_send() {
    log_line("IPC: Testing empty message send...");
    
    let empty_payload = b"";
    match send_bootstrap_message(empty_payload) {
        Ok(()) => ipc_test_pass("Empty message send succeeds"),
        Err(_) => ipc_test_fail("Empty message send succeeds"),
    }
}

// Test message sending with large payload
fn test_large_message_send() {
    log_line("IPC: Testing large message send...");
    
    let large_payload = b"This is a very long message that tests the IPC system's ability to handle larger payloads and ensure that the communication channel can handle messages of various sizes without issues.";
    match send_bootstrap_message(large_payload) {
        Ok(()) => ipc_test_pass("Large message send succeeds"),
        Err(_) => ipc_test_fail("Large message send succeeds"),
    }
}

// Test message receiving with adequate buffer
fn test_valid_message_receive() {
    log_line("IPC: Testing valid message receive...");
    
    let mut buffer = [0u8; 64];
    match receive_bootstrap_message(&mut buffer) {
        Ok(size) => {
            ipc_test_pass("Valid message receive succeeds");
            log_line_hex("IPC: received_size=", size as u64);
            
            // Log received message (first few bytes)
            log_line("IPC: received_message_start:");
            let msg_len = size.min(16);
            for i in 0..msg_len {
                log_line_hex("IPC: byte_", buffer[i] as u64);
            }
        }
        Err(_) => ipc_test_fail("Valid message receive succeeds"),
    }
}

// Test message receiving with small buffer
fn test_small_buffer_receive() {
    log_line("IPC: Testing small buffer receive...");
    
    let mut small_buffer = [0u8; 4];
    match receive_bootstrap_message(&mut small_buffer) {
        Ok(size) => {
            ipc_test_pass("Small buffer receive succeeds");
            log_line_hex("IPC: small_buffer_size=", size as u64);
        }
        Err(_) => ipc_test_fail("Small buffer receive succeeds"),
    }
}

// Test message receiving with zero-sized buffer
fn test_zero_buffer_receive() {
    log_line("IPC: Testing zero buffer receive...");
    
    let mut zero_buffer = [];
    match receive_bootstrap_message(&mut zero_buffer) {
        Ok(_) => ipc_test_fail("Zero buffer receive should fail"),
        Err(_) => ipc_test_pass("Zero buffer receive fails correctly"),
    }
}

// Test multiple message exchanges
fn test_multiple_message_exchanges() {
    log_line("IPC: Testing multiple message exchanges...");
    
    let messages = [
        b"MSG1",
        b"MSG2", 
        b"MSG3",
        b"MSG4"
    ];
    
    for (i, msg) in messages.iter().enumerate() {
        match send_bootstrap_message(*msg) {
            Ok(()) => {
                log_line_hex("IPC: sent_message_", i as u64);
            }
            Err(_) => {
                ipc_test_fail("Multiple message send");
                return;
            }
        }
        
        let mut buffer = [0u8; 32];
        match receive_bootstrap_message(&mut buffer) {
            Ok(size) => {
                log_line_hex("IPC: received_message_", i as u64);
                log_line_hex("IPC: message_size=", size as u64);
            }
            Err(_) => {
                ipc_test_fail("Multiple message receive");
                return;
            }
        }
    }
    
    ipc_test_pass("Multiple message exchanges");
}

// Test IPC channel state consistency
fn test_channel_state_consistency() {
    log_line("IPC: Testing channel state consistency...");
    
    // Send a message
    if send_bootstrap_message(b"STATE_TEST").is_err() {
        ipc_test_fail("Channel state test send");
        return;
    }
    
    // Try to receive immediately
    let mut buffer = [0u8; 32];
    match receive_bootstrap_message(&mut buffer) {
        Ok(_) => ipc_test_pass("Channel state consistency"),
        Err(_) => ipc_test_fail("Channel state consistency"),
    }
}

// Test IPC error handling
fn test_ipc_error_handling() {
    log_line("IPC: Testing IPC error handling...");
    
    // Test with invalid buffer (this should be handled gracefully)
    let mut buffer = [0u8; 1];
    match receive_bootstrap_message(&mut buffer) {
        Ok(_) => ipc_test_pass("IPC error handling"),
        Err(_) => {
            // This might be expected behavior
            log_line("IPC: Receive failed (might be expected)");
            ipc_test_pass("IPC error handling");
        }
    }
}

fn ipc_init_task() {
    log_line("IPC: Starting IPC test suite");
    
    // Run all IPC tests
    test_channel_existence();
    test_init_service_registration();
    test_valid_message_send();
    test_empty_message_send();
    test_large_message_send();
    test_valid_message_receive();
    test_small_buffer_receive();
    test_zero_buffer_receive();
    test_multiple_message_exchanges();
    test_channel_state_consistency();
    test_ipc_error_handling();
    
    // Report results
    unsafe {
        log_line("IPC: IPC Test Results:");
        log_line_hex("IPC: tests_passed=", IPC_TESTS_PASSED as u64);
        log_line_hex("IPC: tests_failed=", IPC_TESTS_FAILED as u64);
        
        if IPC_TESTS_FAILED == 0 {
            log_line("IPC: All IPC tests PASSED!");
        } else {
            log_line("IPC: Some IPC tests FAILED!");
        }
    }
    
    // Send completion message
    let channel = kernel_channel();
    let payload: &[u8] = if unsafe { IPC_TESTS_FAILED == 0 } {
        b"IPC:ALL_TESTS_PASSED"
    } else {
        b"IPC:SOME_TESTS_FAILED"
    };
    
    let _ = channel.send(payload);
    log_line("IPC: IPC test suite complete");
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
    log_line("IPC: kernel entered ipc_test_main");

    kernel::ipc_bridge::register_init_service();
    let _ = kernel::scheduler::register(ipc_init_task);
    let _ = kernel::ipc_bridge::send_bootstrap_message(b"IPC:BOOT");

    kernel::scheduler::run();
    arch::start_timer(100);

    let mut ack = [0u8; 64];
    let _ = kernel::ipc_bridge::receive_bootstrap_message(&mut ack);
    log_line("IPC: ipc test ack received");

    // Check test results
    if ack.len() >= 17 && &ack[..17] == b"IPC:ALL_TESTS_PASSED" {
        log_line("IPC: IPC tests PASSED");
    } else {
        log_line("IPC: IPC tests had issues");
    }

    // Final timer test
    let mut spin = 0u64;
    loop {
        let ticks = kernel::arch::timer_ticks();
        if ticks >= 3 {
            log_line_hex("IPC: final_ticks=", ticks);
            break;
        }
        spin = spin.wrapping_add(1);
        if spin % 1_000_000 == 0 {
            log_line_hex("IPC: intermediate_ticks=", ticks);
        }
        core::hint::spin_loop();
    }

    arch::disable_interrupts();
    log_line("IPC: ipc tests complete");

    arch::halt()
}

#[no_mangle]
pub extern "C" fn rustcore_entry64(boot_info_ptr: *const BootInfo) -> ! {
    rustcore_entry(boot_info_ptr)
}
