use core::arch::asm;

/// Initializes CPU-specific facilities: paging, descriptor tables, and default traps.
pub fn init() {
    disable_interrupts();

    unsafe {
        serial::init();
        serial::write_bytes(b"arch: serial ready\n");
        pic::init();
        serial::write_bytes(b"arch: pic init\n");
        paging::init();
        serial::write_bytes(b"arch: paging init\n");
        descriptor::init();
        serial::write_bytes(b"arch: descriptor init\n");
        interrupts::init();
        serial::write_bytes(b"arch: idt init\n");
        pit::start_periodic(100); // 100 Hz tick for scheduler bookkeeping
        serial::write_bytes(b"arch: pit started\n");
    }

    enable_simd();
}

/// Enables maskable interrupts.
pub fn enable_interrupts() {
    unsafe {
        // SAFETY: `sti` depends on a well-formed IDT which `init` installs.
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Disables maskable interrupts.
pub fn disable_interrupts() {
    unsafe {
        // SAFETY: `cli` simply clears the IF flag; callers are responsible for progress.
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Returns true when the CPU interrupt flag is set.
pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {0}",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }
    rflags & (1 << 9) != 0
}

/// Halts the CPU until the next hardware event.
pub fn halt() -> ! {
    loop {
        unsafe {
            // SAFETY: `hlt` is valid once interrupts are configured.
            asm!("hlt", options(nomem, nostack));
        }
    }
}

mod descriptor {
    use core::{arch::asm, mem::size_of, ptr};

    #[repr(C, packed)]
    struct DescriptorTablePointer {
        limit: u16,
        base: u64,
    }

    #[repr(C, align(16))]
    struct TaskStateSegment {
        _reserved1: u32,
        rsp: [u64; 3],
        _reserved2: u64,
        ist: [u64; 7],
        _reserved3: u64,
        _reserved4: u16,
        iomap_base: u16,
    }

    impl TaskStateSegment {
        const fn new() -> Self {
            Self {
                _reserved1: 0,
                rsp: [0; 3],
                _reserved2: 0,
                ist: [0; 7],
                _reserved3: 0,
                _reserved4: 0,
                iomap_base: size_of::<Self>() as u16,
            }
        }
    }

    // Null, code, data, TSS (low), TSS (high).
    const GDT_ENTRIES: usize = 5;

    static mut GDT: [u64; GDT_ENTRIES] = [0; GDT_ENTRIES];
    static mut TSS: TaskStateSegment = TaskStateSegment::new();

    pub(super) const KERNEL_CODE_SELECTOR: u16 = 0x08;
    pub(super) const KERNEL_DATA_SELECTOR: u16 = 0x10;
    const TSS_SELECTOR: u16 = 0x18;

    #[allow(static_mut_refs)]
    pub(super) unsafe fn init() {
        GDT[0] = 0;
        GDT[1] = gdt_entry(0x00af9a000000ffff);
        GDT[2] = gdt_entry(0x00af92000000ffff);

        let (tss_low, tss_high) = tss_descriptor(ptr::addr_of!(TSS));
        GDT[3] = tss_low;
        GDT[4] = tss_high;

        let descriptor = DescriptorTablePointer {
            limit: (GDT_ENTRIES * size_of::<u64>() - 1) as u16,
            base: ptr::addr_of!(GDT[0]) as u64,
        };

        // SAFETY: Pointer references the statically allocated GDT.
        asm!("lgdt [{0}]", in(reg) &descriptor, options(readonly, nostack));

        reload_segment_selectors();
        load_tss();
    }

    const fn gdt_entry(raw: u64) -> u64 {
        raw
    }

    fn tss_descriptor(tss: *const TaskStateSegment) -> (u64, u64) {
        let base = tss as u64;
        let limit = (size_of::<TaskStateSegment>() - 1) as u64;

        let lower = (limit & 0xFFFF)
            | ((base & 0xFFFF) << 16)
            | ((base >> 16 & 0xFF) << 32)
            | (0x89u64 << 40)
            | ((limit >> 16 & 0xF) << 48)
            | ((base >> 24 & 0xFF) << 56);

        let upper = base >> 32;
        (lower, upper)
    }

    unsafe fn reload_segment_selectors() {
        asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            sel = const KERNEL_CODE_SELECTOR as u64,
            tmp = lateout(reg) _,
            options(nostack)
        );

        asm!(
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            in("ax") KERNEL_DATA_SELECTOR,
            options(nomem, preserves_flags)
        );
    }

    unsafe fn load_tss() {
        asm!("ltr {0:x}", in(reg) TSS_SELECTOR, options(nostack));
    }
}

mod interrupts {
    use core::{arch::asm, cell::UnsafeCell, mem::size_of, ptr};
    use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

    use core::num::NonZeroUsize;

    use super::{descriptor::KERNEL_CODE_SELECTOR, pic, serial};

    #[repr(C, packed)]
    struct DescriptorTablePointer {
        limit: u16,
        base: u64,
    }

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct IdtEntry {
        offset_low: u16,
        selector: u16,
        options: u16,
        offset_mid: u16,
        offset_high: u32,
        reserved: u32,
    }

    impl IdtEntry {
        const fn missing() -> Self {
            Self {
                offset_low: 0,
                selector: 0,
                options: 0,
                offset_mid: 0,
                offset_high: 0,
                reserved: 0,
            }
        }

        fn with_handler(handler: HandlerFunc, dpl: u16) -> Self {
            let ptr = handler as usize as u64;
            let mut options: u16 = 0x8E00;
            options |= (dpl & 0x3) << 13;

            Self {
                offset_low: ptr as u16,
                selector: KERNEL_CODE_SELECTOR,
                options,
                offset_mid: (ptr >> 16) as u16,
                offset_high: (ptr >> 32) as u32,
                reserved: 0,
            }
        }

        fn with_handler_err(handler: HandlerFuncWithErr, dpl: u16) -> Self {
            let ptr = handler as usize as u64;
            let mut options: u16 = 0x8E00;
            options |= (dpl & 0x3) << 13;

            Self {
                offset_low: ptr as u16,
                selector: KERNEL_CODE_SELECTOR,
                options,
                offset_mid: (ptr >> 16) as u16,
                offset_high: (ptr >> 32) as u32,
                reserved: 0,
            }
        }
    }

    #[repr(C, align(16))]
    struct Idt {
        entries: [IdtEntry; 256],
    }

    impl Idt {
        const fn new() -> Self {
            Self {
                entries: [IdtEntry::missing(); 256],
            }
        }

        fn set_handler(&mut self, vector: InterruptVector, handler: HandlerFunc, dpl: u16) {
            self.entries[vector as usize] = IdtEntry::with_handler(handler, dpl);
        }

        fn set_handler_err(&mut self, vector: InterruptVector, handler: HandlerFuncWithErr, dpl: u16) {
            self.entries[vector as usize] = IdtEntry::with_handler_err(handler, dpl);
        }
    }

    static mut IDT: Idt = Idt::new();

    static TIMER_CALLBACK: AtomicUsize = AtomicUsize::new(0);
    static IPC_CALLBACK: AtomicUsize = AtomicUsize::new(0);

    struct TickCell(UnsafeCell<u64>);
    unsafe impl Sync for TickCell {}

    static TIMER_TICKS: TickCell = TickCell(UnsafeCell::new(0));

    pub type HandlerFunc = extern "x86-interrupt" fn(&mut InterruptStackFrame);
    pub type HandlerFuncWithErr = extern "x86-interrupt" fn(&mut InterruptStackFrame, u64);

    #[repr(C)]
    pub struct InterruptStackFrame {
        pub instruction_pointer: u64,
        pub code_segment: u64,
        pub cpu_flags: u64,
        pub stack_pointer: u64,
        pub stack_segment: u64,
    }

    #[repr(u8)]
    pub enum InterruptVector {
        GeneralProtection = 13,
        Timer = 32,
        PrimarySpurious = 0x27,
        SecondarySpurious = 0x2F,
        Ipc = 0x80,
    }

    #[allow(static_mut_refs)]
    pub(super) unsafe fn init() {
        IDT.set_handler(InterruptVector::Timer, timer_trap, 0);
        IDT.set_handler(InterruptVector::Ipc, ipc_trap, 3);
        IDT.set_handler_err(InterruptVector::GeneralProtection, general_protection_fault, 0);
        IDT.set_handler(InterruptVector::PrimarySpurious, spurious_trap, 0);
        IDT.set_handler(InterruptVector::SecondarySpurious, spurious_trap, 0);

        let descriptor = DescriptorTablePointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: ptr::addr_of!(IDT) as u64,
        };

        asm!("lidt [{0}]", in(reg) &descriptor, options(readonly, nostack));
    }

    extern "x86-interrupt" fn timer_trap(_frame: &mut InterruptStackFrame) {
        unsafe {
            let ptr = TIMER_TICKS.0.get();
            ptr.write(ptr.read().wrapping_add(1));
        }

        if let Some(func) = load_callback(&TIMER_CALLBACK) {
            func();
        }

        acknowledge(InterruptVector::Timer);
    }

    extern "x86-interrupt" fn ipc_trap(_frame: &mut InterruptStackFrame) {
        if let Some(func) = load_callback(&IPC_CALLBACK) {
            func();
        }

        acknowledge(InterruptVector::Ipc);
    }

    static GP_FAULT_RIP: AtomicU64 = AtomicU64::new(0);
    static GP_FAULT_CS: AtomicU64 = AtomicU64::new(0);
    static GP_FAULT_ERR: AtomicU64 = AtomicU64::new(0);
    static GP_FAULT_VALID: AtomicBool = AtomicBool::new(false);

    extern "x86-interrupt" fn general_protection_fault(
        frame: &mut InterruptStackFrame,
        error_code: u64,
    ) {
        GP_FAULT_RIP.store(frame.instruction_pointer, Ordering::Relaxed);
        GP_FAULT_CS.store(frame.code_segment, Ordering::Relaxed);
        GP_FAULT_ERR.store(error_code, Ordering::Relaxed);
        GP_FAULT_VALID.store(true, Ordering::Release);

        super::disable_interrupts();
        serial::write_bytes(b"general protection fault\n");
        serial::write_bytes(b"  rip=");
        serial::write_u64_hex(frame.instruction_pointer);
        serial::write_byte(b'\n');
        serial::write_bytes(b"  cs=");
        serial::write_u64_hex(frame.code_segment);
        serial::write_byte(b'\n');
        serial::write_bytes(b"  err=");
        serial::write_u64_hex(error_code);
        serial::write_byte(b'\n');

        // Skip the faulting instruction so we can continue execution and surface the context.
        frame.instruction_pointer = frame.instruction_pointer.wrapping_add(5);

        super::enable_interrupts();
    }

    pub fn take_last_gp_fault() -> Option<(u64, u64, u64)> {
        if GP_FAULT_VALID.swap(false, Ordering::AcqRel) {
            Some((
                GP_FAULT_RIP.load(Ordering::Acquire),
                GP_FAULT_CS.load(Ordering::Acquire),
                GP_FAULT_ERR.load(Ordering::Acquire),
            ))
        } else {
            None
        }
    }

    extern "x86-interrupt" fn spurious_trap(_frame: &mut InterruptStackFrame) {
        pic::end_of_interrupt(0);
    }

    fn acknowledge(vector: InterruptVector) {
        match vector {
            InterruptVector::Timer => pic::end_of_interrupt(0),
            InterruptVector::Ipc => {},
            InterruptVector::GeneralProtection => {},
            InterruptVector::PrimarySpurious => {},
            InterruptVector::SecondarySpurious => {},
        }
    }

    pub fn register_timer_handler(callback: fn()) {
        store_callback(&TIMER_CALLBACK, callback);
    }

    pub fn register_ipc_handler(callback: fn()) {
        store_callback(&IPC_CALLBACK, callback);
    }

    pub fn timer_ticks() -> u64 {
        let was_enabled = super::interrupts_enabled();
        if was_enabled {
            super::disable_interrupts();
        }
        let value = unsafe { *TIMER_TICKS.0.get() };
        if was_enabled {
            super::enable_interrupts();
        }
        value
    }

    fn store_callback(slot: &AtomicUsize, callback: fn()) {
        slot.store(callback as usize, Ordering::SeqCst);
    }

    fn load_callback(slot: &AtomicUsize) -> Option<fn()> {
        NonZeroUsize::new(slot.load(Ordering::SeqCst)).map(|nz| unsafe { core::mem::transmute(nz.get()) })
    }
}

mod paging {
    use core::{arch::asm, ptr};

    #[repr(align(4096))]
    struct PageTable {
        entries: [u64; 512],
    }

    impl PageTable {
        const fn new() -> Self {
            Self { entries: [0; 512] }
        }
    }

    static mut PML4: PageTable = PageTable::new();
    static mut PDP: PageTable = PageTable::new();
    static mut PD: PageTable = PageTable::new();

    const PRESENT: u64 = 1 << 0;
    const WRITABLE: u64 = 1 << 1;
    const HUGE: u64 = 1 << 7;

    #[allow(static_mut_refs)]
    pub(super) unsafe fn init() {
        PML4.entries[0] = (ptr::addr_of!(PDP) as u64) | PRESENT | WRITABLE;
        PDP.entries[0] = (ptr::addr_of!(PD) as u64) | PRESENT | WRITABLE;

        let page_size: u64 = 2 * 1024 * 1024;
        let mut idx: usize = 0;
        while idx < PD.entries.len() {
            PD.entries[idx] = (idx as u64 * page_size) | PRESENT | WRITABLE | HUGE;
            idx += 1;
        }

        let root_table = ptr::addr_of!(PML4) as u64;
        asm!("mov cr3, {0}", in(reg) root_table, options(nostack, preserves_flags));
    }
}

mod io {
    use core::arch::asm;

    #[inline]
    pub unsafe fn out_u8(port: u16, value: u8) {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }

    #[inline]
    pub unsafe fn in_u8(port: u16) -> u8 {
        let value: u8;
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }

    #[inline]
    pub unsafe fn io_wait() {
        asm!("out 0x80, al", in("al") 0_u8, options(nomem, nostack, preserves_flags));
    }
}

mod serial {
    use super::io;

    const COM1: u16 = 0x3F8;

    pub(super) unsafe fn init() {
        io::out_u8(COM1 + 1, 0x00); // Disable interrupts
        io::out_u8(COM1 + 3, 0x80); // Enable DLAB
        io::out_u8(COM1, 0x03); // Divisor low (38400 baud)
        io::out_u8(COM1 + 1, 0x00); // Divisor high
        io::out_u8(COM1 + 3, 0x03); // 8 bits, no parity, one stop
        io::out_u8(COM1 + 2, 0xC7); // Enable FIFO, clear, 14-byte threshold
        io::out_u8(COM1 + 4, 0x0B); // IRQs enabled, RTS/DSR set
    }

    pub(super) fn write_bytes(bytes: &[u8]) {
        for &byte in bytes {
            write_byte(byte);
        }
    }

    pub(super) fn write_byte(byte: u8) {
        unsafe {
            while (io::in_u8(COM1 + 5) & 0x20) == 0 {}
            io::out_u8(COM1, byte);
        }
    }

    pub(super) fn write_u64_hex(value: u64) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut buf = [0u8; 16];
        let mut idx = 0;
        while idx < 16 {
            let shift = (15 - idx) * 4;
            let nibble = ((value >> shift) & 0xF) as usize;
            buf[idx] = HEX[nibble];
            idx += 1;
        }
        write_bytes(&buf);
    }
}

mod pic {
    use super::io;

    const PIC1_CMD: u16 = 0x20;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_CMD: u16 = 0xA0;
    const PIC2_DATA: u16 = 0xA1;

    const ICW1_INIT: u8 = 0x10;
    const ICW1_ICW4: u8 = 0x01;
    const ICW4_8086: u8 = 0x01;
    const PIC1_OFFSET: u8 = 0x20;
    const PIC2_OFFSET: u8 = 0x28;
    const PIC_EOI: u8 = 0x20;

    #[allow(static_mut_refs)]
    pub(super) unsafe fn init() {
        // Start initialization sequence.
        io::out_u8(PIC1_CMD, ICW1_INIT | ICW1_ICW4);
        io::io_wait();
        io::out_u8(PIC2_CMD, ICW1_INIT | ICW1_ICW4);
        io::io_wait();

        // Remap vector offsets.
        io::out_u8(PIC1_DATA, PIC1_OFFSET);
        io::io_wait();
        io::out_u8(PIC2_DATA, PIC2_OFFSET);
        io::io_wait();

        // Tell Master PIC about the slave at IRQ2, and vice versa.
        io::out_u8(PIC1_DATA, 0x04);
        io::io_wait();
        io::out_u8(PIC2_DATA, 0x02);
        io::io_wait();

        // Set environment info.
        io::out_u8(PIC1_DATA, ICW4_8086);
        io::io_wait();
        io::out_u8(PIC2_DATA, ICW4_8086);
        io::io_wait();

        // Mask all IRQ lines initially.
        io::out_u8(PIC1_DATA, 0xFF);
        io::out_u8(PIC2_DATA, 0xFF);
    }

    pub(super) fn end_of_interrupt(irq: u8) {
        unsafe {
            if irq >= 8 {
                io::out_u8(PIC2_CMD, PIC_EOI);
            }
            io::out_u8(PIC1_CMD, PIC_EOI);
        }
    }

    pub(super) fn unmask_irq(irq: u8) {
        unsafe {
            if irq < 8 {
                let mut mask = io::in_u8(PIC1_DATA);
                mask &= !(1 << irq);
                io::out_u8(PIC1_DATA, mask);
            } else {
                let mut mask = io::in_u8(PIC2_DATA);
                mask &= !(1 << (irq - 8));
                io::out_u8(PIC2_DATA, mask);
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn mask_irq(irq: u8) {
        unsafe {
            if irq < 8 {
                let mut mask = io::in_u8(PIC1_DATA);
                mask |= 1 << irq;
                io::out_u8(PIC1_DATA, mask);
            } else {
                let mut mask = io::in_u8(PIC2_DATA);
                mask |= 1 << (irq - 8);
                io::out_u8(PIC2_DATA, mask);
            }
        }
    }
}

mod pit {
    use super::io;

    const PIT_FREQUENCY_HZ: u64 = 1_193_182;
    const PIT_CHANNEL0: u16 = 0x40;
    const PIT_COMMAND: u16 = 0x43;

    pub(super) unsafe fn start_periodic(hz: u32) {
        let hz = hz.clamp(19, 1000); // keep divisor within 16-bit range and reasonable tick
        let divisor = (PIT_FREQUENCY_HZ / hz as u64) as u16;

        program_counter(divisor);
    }

    unsafe fn program_counter(divisor: u16) {
        // Channel 0, access low/high, mode 3 (square wave).
        io::out_u8(PIT_COMMAND, 0x36);
        io::out_u8(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        io::out_u8(PIT_CHANNEL0, (divisor >> 8) as u8);
    }
}

pub use interrupts::{
    register_ipc_handler, register_timer_handler, timer_ticks, InterruptStackFrame, InterruptVector,
};

pub fn take_last_gp_fault() -> Option<(u64, u64, u64)> {
    interrupts::take_last_gp_fault()
}

pub fn unmask_timer_irq() {
    pic::unmask_irq(0)
}

pub fn serial_write_line(message: &str) {
    serial::write_bytes(message.as_bytes());
    serial::write_byte(b'\n');
}

pub fn serial_write_bytes(bytes: &[u8]) {
    serial::write_bytes(bytes);
}

pub fn serial_write_byte(byte: u8) {
    serial::write_byte(byte);
}

fn enable_simd() {
    unsafe {
        let mut cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, preserves_flags));
        cr0 |= (1 << 1) | (1 << 5); // MP, NE
        asm!("mov cr0, {}", in(reg) cr0, options(nomem, preserves_flags));

        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, preserves_flags));
        cr4 |= (1 << 5) | (1 << 9) | (1 << 10); // ensure PAE, OSFXSR, OSXMMEXCPT
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, preserves_flags));

        asm!("fninit", options(nomem, preserves_flags));

        let mut cr0_after: u64;
        let mut cr4_after: u64;
        asm!("mov {}, cr0", out(reg) cr0_after, options(nomem, preserves_flags));
        asm!("mov {}, cr4", out(reg) cr4_after, options(nomem, preserves_flags));
        serial::write_bytes(b"arch: cr0=");
        serial::write_u64_hex(cr0_after);
        serial::write_byte(b'\n');
        serial::write_bytes(b"arch: cr4=");
        serial::write_u64_hex(cr4_after);
        serial::write_byte(b'\n');
    }
}
