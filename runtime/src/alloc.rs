#![allow(dead_code)]

/// Kernel heap statistics placeholder.
pub struct HeapStats {
    pub total: usize,
    pub used: usize,
}

pub trait Heap {
    fn allocate(&mut self, layout: core::alloc::Layout) -> Option<*mut u8>;
    fn deallocate(&mut self, ptr: *mut u8, layout: core::alloc::Layout);
    fn stats(&self) -> HeapStats;
}
