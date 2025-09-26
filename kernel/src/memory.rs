use crate::sync::SpinLock;

const FRAME_SIZE_BYTES: usize = 4096;
const TOTAL_FRAMES: usize = 128;
const BOOT_RESERVED_FRAMES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame(u16);

impl Frame {
    pub const fn number(&self) -> u16 {
        self.0
    }

    pub const fn start_addr(&self) -> usize {
        self.0 as usize * FRAME_SIZE_BYTES
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameState {
    Free,
    Reserved,
}

struct FrameAllocator {
    map: [FrameState; TOTAL_FRAMES],
    next_search_idx: usize,
}

impl FrameAllocator {
    const fn new() -> Self {
        Self {
            map: [FrameState::Free; TOTAL_FRAMES],
            next_search_idx: 0,
        }
    }

    fn reserve_range(&mut self, start: usize, len: usize) {
        for idx in start..(start + len).min(TOTAL_FRAMES) {
            self.map[idx] = FrameState::Reserved;
        }
    }

    fn allocate_frame(&mut self) -> Option<Frame> {
        for offset in 0..TOTAL_FRAMES {
            let idx = (self.next_search_idx + offset) % TOTAL_FRAMES;
            if self.map[idx] == FrameState::Free {
                self.map[idx] = FrameState::Reserved;
                self.next_search_idx = (idx + 1) % TOTAL_FRAMES;
                return Some(Frame(idx as u16));
            }
        }
        None
    }

    fn release_frame(&mut self, frame: Frame) -> bool {
        let idx = frame.number() as usize;
        if idx >= TOTAL_FRAMES {
            return false;
        }

        match self.map[idx] {
            FrameState::Free => false,
            FrameState::Reserved => {
                self.map[idx] = FrameState::Free;
                true
            }
        }
    }

    fn reserved_frames(&self) -> usize {
        self.map
            .iter()
            .filter(|state| matches!(state, FrameState::Reserved))
            .count()
    }
}

static FRAME_ALLOCATOR: SpinLock<FrameAllocator> = SpinLock::new(FrameAllocator::new());

/// Initializes the physical memory allocator with a simple frame map.
pub fn init() {
    let mut allocator = FRAME_ALLOCATOR.lock();
    allocator.reserve_range(0, BOOT_RESERVED_FRAMES);
}

/// Attempts to allocate a 4 KiB frame.
pub fn allocate_frame() -> Option<Frame> {
    FRAME_ALLOCATOR.lock().allocate_frame()
}

/// Returns a frame back to the allocator; returns `false` if the frame was not in use.
pub fn release_frame(frame: Frame) -> bool {
    FRAME_ALLOCATOR.lock().release_frame(frame)
}

/// Returns the number of frames the allocator currently marks as reserved.
pub fn reserved_frames() -> usize {
    FRAME_ALLOCATOR.lock().reserved_frames()
}

/// Exposes the frame size to callers that need to compute addresses.
pub const fn frame_size() -> usize {
    FRAME_SIZE_BYTES
}
