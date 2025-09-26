use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

const MAX_MESSAGES: usize = 16;
const MAX_PAYLOAD: usize = 64;

#[derive(Clone, Copy)]
pub struct Message {
    len: u8,
    payload: [u8; MAX_PAYLOAD],
}

impl Message {
    const fn empty() -> Self {
        Self {
            len: 0,
            payload: [0; MAX_PAYLOAD],
        }
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, SendError> {
        if bytes.len() > MAX_PAYLOAD {
            return Err(SendError::Oversized);
        }

        let mut message = Message::empty();
        let mut idx = 0;
        while idx < bytes.len() {
            message.payload[idx] = bytes[idx];
            idx += 1;
        }
        message.len = bytes.len() as u8;
        Ok(message)
    }

    fn write_into(&self, buffer: &mut [u8]) -> usize {
        let needed = self.len as usize;
        let copy_len = core::cmp::min(needed, buffer.len());
        let mut idx = 0;
        while idx < copy_len {
            buffer[idx] = self.payload[idx];
            idx += 1;
        }
        copy_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    Full,
    Oversized,
    Unroutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveError {
    Empty,
}

pub struct Channel {
    queue: SpinLock<MessageQueue>,
}

impl Channel {
    pub const fn new() -> Self {
        Self {
            queue: SpinLock::new(MessageQueue::new()),
        }
    }

    pub fn send(&self, bytes: &[u8]) -> Result<(), SendError> {
        let message = Message::from_slice(bytes)?;
        self.queue.lock().push(message)
    }

    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize, ReceiveError> {
        self.queue.lock().pop(buffer)
    }

    pub fn reset(&self) {
        self.queue.lock().reset();
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

struct MessageQueue {
    slots: [QueueSlot; MAX_MESSAGES],
    head: usize,
    tail: usize,
    len: usize,
}

impl MessageQueue {
    const fn new() -> Self {
        Self {
            slots: [QueueSlot::empty(); MAX_MESSAGES],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    fn push(&mut self, message: Message) -> Result<(), SendError> {
        if self.len == MAX_MESSAGES {
            return Err(SendError::Full);
        }

        self.slots[self.tail].message = message;
        self.slots[self.tail].used = true;
        self.tail = (self.tail + 1) % MAX_MESSAGES;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self, buffer: &mut [u8]) -> Result<usize, ReceiveError> {
        if self.len == 0 {
            return Err(ReceiveError::Empty);
        }

        let slot = &mut self.slots[self.head];
        self.head = (self.head + 1) % MAX_MESSAGES;
        self.len -= 1;
        slot.used = false;
        Ok(slot.message.write_into(buffer))
    }

    fn reset(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
        let mut idx = 0;
        while idx < self.slots.len() {
            self.slots[idx].used = false;
            idx += 1;
        }
    }
}

#[derive(Clone, Copy)]
struct QueueSlot {
    message: Message,
    used: bool,
}

impl QueueSlot {
    const fn empty() -> Self {
        Self {
            message: Message::empty(),
            used: false,
        }
    }
}

struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        SpinLockGuard { lock: self }
    }
}

struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}
