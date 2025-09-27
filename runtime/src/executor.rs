#![allow(dead_code)]

use alloc::collections::VecDeque;

pub trait Executor {
    fn spawn(&mut self, task: Task);
    fn run(&mut self);
}

pub struct Task {
    pub func: fn(),
}

pub struct SimpleExecutor {
    queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Default for SimpleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for SimpleExecutor {
    fn spawn(&mut self, task: Task) {
        self.queue.push_back(task);
    }

    fn run(&mut self) {
        while let Some(task) = self.queue.pop_front() {
            (task.func)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SimpleExecutor, Task};
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static LOG_SIZE: usize = 4;
    static INVOCATIONS: InvocationLog = InvocationLog::new();

    struct InvocationLog {
        index: AtomicUsize,
        entries: UnsafeCell<[Option<&'static str>; LOG_SIZE]>,
    }

    unsafe impl Sync for InvocationLog {}

    impl InvocationLog {
        const fn new() -> Self {
            Self {
                index: AtomicUsize::new(0),
                entries: UnsafeCell::new([None; LOG_SIZE]),
            }
        }

        fn reset(&self) {
            self.index.store(0, Ordering::Relaxed);
            unsafe { *self.entries.get() = [None; LOG_SIZE] };
        }

        fn record(&self, name: &'static str) {
            let idx = self.index.fetch_add(1, Ordering::Relaxed);
            if idx < LOG_SIZE {
                unsafe { (*self.entries.get())[idx] = Some(name) };
            }
        }

        fn entries(&self) -> alloc::vec::Vec<&'static str> {
            let mut out = alloc::vec::Vec::new();
            let idx = self.index.load(Ordering::Relaxed);
            for i in 0..idx.min(LOG_SIZE) {
                if let Some(name) = unsafe { (*self.entries.get())[i] } {
                    out.push(name);
                }
            }
            out
        }
    }

    #[test]
    fn executor_runs_tasks_in_order() {
        INVOCATIONS.reset();
        fn first() {
            INVOCATIONS.record("first");
        }
        fn second() {
            INVOCATIONS.record("second");
        }
        let mut exec = SimpleExecutor::new();
        exec.spawn(Task { func: first });
        exec.spawn(Task { func: second });
        exec.run();
        assert_eq!(INVOCATIONS.entries(), alloc::vec!["first", "second"]);
    }
}
