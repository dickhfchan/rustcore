use crate::sync::SpinLock;

const MAX_TASKS: usize = 16;

type TaskEntry = fn();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Completed,
}

#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    pub id: TaskId,
    pub entry: TaskEntry,
    pub state: TaskState,
}

impl TaskControlBlock {
    const fn new(id: TaskId, entry: TaskEntry, state: TaskState) -> Self {
        Self { id, entry, state }
    }
}

struct ReadyQueue {
    slots: [Option<TaskControlBlock>; MAX_TASKS],
    head: usize,
    tail: usize,
    next_id: u16,
}

impl ReadyQueue {
    const fn new() -> Self {
        Self {
            slots: [None; MAX_TASKS],
            head: 0,
            tail: 0,
            next_id: 0,
        }
    }

    fn push(&mut self, entry: TaskEntry) -> Option<TaskId> {
        let next_tail = (self.tail + 1) % MAX_TASKS;
        if next_tail == self.head {
            return None;
        }

        let id = TaskId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);

        self.slots[self.tail] = Some(TaskControlBlock::new(id, entry, TaskState::Ready));
        self.tail = next_tail;
        Some(id)
    }

    fn pop(&mut self) -> Option<TaskControlBlock> {
        if self.head == self.tail {
            return None;
        }

        let mut slot = self.slots[self.head].take();
        self.head = (self.head + 1) % MAX_TASKS;
        if let Some(task) = slot.as_mut() {
            task.state = TaskState::Running;
        }
        slot
    }

    fn reset(&mut self) {
        self.slots = [None; MAX_TASKS];
        self.head = 0;
        self.tail = 0;
    }
}

static READY_QUEUE: SpinLock<ReadyQueue> = SpinLock::new(ReadyQueue::new());

/// Boots the scheduler with a clean run queue.
pub fn init() {
    READY_QUEUE.lock().reset();
}

/// Registers a task entry point with the scheduler.
pub fn register(entry: TaskEntry) -> Option<TaskId> {
    READY_QUEUE.lock().push(entry)
}

/// Runs tasks in FIFO order until the queue drains.
pub fn run() {
    loop {
        let task = {
            let mut queue = READY_QUEUE.lock();
            queue.pop()
        };

        match task {
            Some(tcb) => (tcb.entry)(),
            None => break,
        }
    }
}

/// Fetches the next runnable task, marking it as running.
pub fn next_task() -> Option<TaskControlBlock> {
    READY_QUEUE.lock().pop()
}
