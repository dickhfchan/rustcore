#![allow(dead_code)]

/// Placeholder for an async task executor.
pub trait Executor {
    fn spawn(&self, task: fn());
    fn run(&self);
}
