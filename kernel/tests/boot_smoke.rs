#![no_std]
#![no_main]

use core::panic::PanicInfo;

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
    loop {
        core::hint::spin_loop();
    }
}

fn failure_loop() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    failure_loop()
}
