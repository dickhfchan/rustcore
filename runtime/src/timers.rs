#![allow(dead_code)]

use alloc::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerId(pub u64);

pub trait TimerService {
    fn schedule(&mut self, deadline_ticks: u64) -> TimerId;
    fn cancel(&mut self, id: TimerId) -> bool;
    fn poll(&mut self, current_ticks: u64, ready: &mut dyn FnMut(TimerId));
}

pub struct TimerQueue {
    next_id: u64,
    timers: BTreeMap<TimerId, u64>,
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            timers: BTreeMap::new(),
        }
    }
}

impl Default for TimerQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerService for TimerQueue {
    fn schedule(&mut self, deadline_ticks: u64) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.timers.insert(id, deadline_ticks);
        id
    }

    fn cancel(&mut self, id: TimerId) -> bool {
        self.timers.remove(&id).is_some()
    }

    fn poll(&mut self, current_ticks: u64, ready: &mut dyn FnMut(TimerId)) {
        let expired: alloc::vec::Vec<_> = self
            .timers
            .iter()
            .filter(|(_, deadline)| **deadline <= current_ticks)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.timers.remove(&id);
            ready(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TimerId, TimerQueue, TimerService};

    #[test]
    fn schedule_and_poll() {
        let mut queue = TimerQueue::new();
        let id = queue.schedule(10);
        let mut fired = alloc::vec::Vec::new();
        queue.poll(5, &mut |id| fired.push(id));
        assert!(fired.is_empty());
        queue.poll(15, &mut |id| fired.push(id));
        assert_eq!(fired, alloc::vec![id]);
    }

    #[test]
    fn cancel_timer() {
        let mut queue = TimerQueue::new();
        let id = queue.schedule(20);
        assert!(queue.cancel(id));
        let mut fired = alloc::vec::Vec::new();
        queue.poll(30, &mut |id| fired.push(id));
        assert!(fired.is_empty());
    }
}
