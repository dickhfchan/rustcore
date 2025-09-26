use core::sync::atomic::{AtomicBool, Ordering};

use ipc::channel::{Channel, ReceiveError, SendError};

use crate::sync::SpinLock;

static KERNEL_CHANNEL: Channel = Channel::new();
static ROUTING_TABLE: SpinLock<RoutingTable> = SpinLock::new(RoutingTable::new());
static IPC_INTERRUPT_PENDING: AtomicBool = AtomicBool::new(false);

struct RoutingTable {
    init_registered: bool,
}

impl RoutingTable {
    const fn new() -> Self {
        Self {
            init_registered: false,
        }
    }

    fn mark_init_registered(&mut self) {
        self.init_registered = true;
    }

    fn clear(&mut self) {
        self.init_registered = false;
    }

    fn init_registered(&self) -> bool {
        self.init_registered
    }
}

/// Initializes kernel-side IPC routing tables.
pub fn init() {
    crate::arch::register_ipc_handler(handle_ipc_trap);
    ipc::init(&KERNEL_CHANNEL);
    ROUTING_TABLE.lock().clear();
    IPC_INTERRUPT_PENDING.store(false, Ordering::Release);
}

/// Exposes the kernel-managed channel to the rest of the system.
pub fn kernel_channel() -> &'static Channel {
    &KERNEL_CHANNEL
}

/// Marks the init service as subscribed to the bootstrap channel.
pub fn register_init_service() {
    ROUTING_TABLE.lock().mark_init_registered();
}

/// Returns whether the init service has registered for startup messages.
pub fn init_service_registered() -> bool {
    ROUTING_TABLE.lock().init_registered()
}

/// Sends a bootstrap message to the init service if it is connected.
pub fn send_bootstrap_message(payload: &[u8]) -> Result<(), SendError> {
    if init_service_registered() {
        KERNEL_CHANNEL.send(payload)
    } else {
        Err(SendError::Unroutable)
    }
}

/// Attempts to receive a message from the kernel channel.
pub fn receive_bootstrap_message(buffer: &mut [u8]) -> Result<usize, ReceiveError> {
    KERNEL_CHANNEL.receive(buffer)
}

/// Returns whether an IPC interrupt was raised since the last check.
pub fn take_ipc_interrupt() -> bool {
    IPC_INTERRUPT_PENDING.swap(false, Ordering::AcqRel)
}

fn handle_ipc_trap() {
    IPC_INTERRUPT_PENDING.store(true, Ordering::Release);
}
