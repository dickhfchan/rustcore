#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerId(pub u64);

pub trait TimerService {
    fn schedule(&mut self, deadline_ticks: u64) -> TimerId;
    fn cancel(&mut self, id: TimerId) -> bool;
}
