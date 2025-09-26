use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::arch;

/// A simple lock that guards data by disabling interrupts on the current core.
pub struct SpinLock<T> {
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new lock around the provided value.
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Acquires the lock by disabling interrupts until the guard is dropped.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let interrupts_were_enabled = arch::interrupts_enabled();
        if interrupts_were_enabled {
            arch::disable_interrupts();
        }

        SpinLockGuard {
            lock: self,
            interrupts_were_enabled,
        }
    }
}

/// Guard returned by [`SpinLock::lock`].
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    interrupts_were_enabled: bool,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        if self.interrupts_were_enabled {
            arch::enable_interrupts();
        }
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

/// Executes the provided closure while holding the lock.
pub fn with_lock<T, R, F>(lock: &SpinLock<T>, f: F) -> R
where
    F: FnOnce(&mut T) -> R,
{
    let mut guard = lock.lock();
    f(&mut guard)
}
