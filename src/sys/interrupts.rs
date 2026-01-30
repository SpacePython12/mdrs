use super::prelude::*;

#[repr(C)]
pub struct VectorTable {
    pub stack_pointer: *const (),
    pub entrypoint: unsafe extern "C" fn(),
    pub bus_error: unsafe extern "C" fn(),
    pub addr_error: unsafe extern "C" fn(),
    pub ill_instr: unsafe extern "C" fn(),
    pub zero_divide: unsafe extern "C" fn(),
    pub chk_instr: unsafe extern "C" fn(),
    pub trapv_instr: unsafe extern "C" fn(),
    pub privilege: unsafe extern "C" fn(),
    pub trace: unsafe extern "C" fn(),
    pub line_1010: unsafe extern "C" fn(),
    pub line_1111: unsafe extern "C" fn(),
    unused0: [unsafe extern "C" fn(); 12],
    pub irqs: [unsafe extern "C" fn(); 8],
    pub traps: [unsafe extern "C" fn(); 16],
    unused1: [unsafe extern "C" fn(); 16],
}

#[used]
#[link_section = ".boot.vtable"]
pub static VECTOR_TABLE: VectorTable = VectorTable {
    stack_pointer: 0x01000000 as *mut _,
    entrypoint,
    bus_error: trap,
    addr_error: addr_err,
    ill_instr: trap,
    zero_divide: trap,
    chk_instr: trap,
    trapv_instr: trap,
    privilege: trap,
    trace: trap,
    line_1010: trap,
    line_1111: trap,
    unused0: [trap; _],
    irqs: [
        default_irq,
        default_irq,
        xint_irq,
        default_irq,
        hint_irq,
        default_irq,
        vint_irq,
        default_irq,
    ],
    traps: [default_irq; _],
    unused1: [trap; _],
};

unsafe impl Sync for VectorTable {}

#[unsafe(naked)]
unsafe extern "C" fn trap() {
    core::arch::naked_asm!(
        "2:",
        "bra 2b",
        // ".short 0x4E72, 0x2700", // stop #0x2700
    )
}

#[unsafe(naked)]
unsafe extern "C" fn default_irq() {
    core::arch::naked_asm!(
        ".short 0x4E73", // rte
    )
}

#[unsafe(naked)]
unsafe extern "C" fn addr_err() {
    core::arch::naked_asm!(
        ".short 0x46FC, 0x2700", // Disable interrupts for the time being
        "movem.l %d0/%a0,-(%sp)", // Save the registers we'll be using
        "move.l 18(%sp),%d0", // Load the errant PC value from the stack frame
        "btst #0,%d0", // Is the source PC on an odd address?
        "bne 2f", // If it isn't, it probably isn't a long branch.
        "jsr {trap}",
        "2:",
        "sub.l #1,%d0", // Align the value so it doesn't cause another error
        "move.l %d0,%a0", // Move to a0 so we can use it for addressing
        "move.w (%a0)+,%d0", // Load the opcode into d0
        "and.w #0xF0FF,%d0", // Mask out the condition code
        "cmpi.w #0x60FF,%d0", // Is the opcode bra.l?
        "beq 2f", // If it isn't, trap because it probably wasn't valid anyways
        "jsr {trap}",
        "2:",
        "move.l  (%a0),%d0", // Load 32-bit offset into d0
        "adda.l  %d0,%a0", // Offset a0 with branch offset in d0
        "move.l  %a0,18(%sp)", // Store the newly offseted PC value so we return to it when we rte
        "movem.l (%sp)+,%d0/%a0", // Restore the clobbered registers
        "lea     8(%sp),%sp", // Set up rte...
        ".short 0x46FC, 0x2100", // Re-enable interrupts
        ".short 0x4E73", // ...and return back to branch!
        trap = sym trap,
    )
}

#[inline]
pub fn set_vint_handler(handler: Option<extern "C" fn()>) {
    VINT_HANDLER_PRESENT.store(false, atomic::Ordering::Relaxed);
    if let Some(handler) = handler {
        // We use volatile reads to force the compiler to not optimize or reorder things.
        #[allow(static_mut_refs)]
        unsafe { (VINT_HANDLER.as_mut_ptr()).write_volatile(handler); }
        VINT_HANDLER_PRESENT.store(true, atomic::Ordering::Relaxed);
    }
}

#[inline(never)] 
pub fn wait_for_vint() {
    VINT_FLAG.store(true, atomic::Ordering::Relaxed);
    while VINT_FLAG.load(atomic::Ordering::Relaxed) {
        unsafe { super::stop::<1>(); }
    }
}

pub(super) static VINT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static VINT_HANDLER_PRESENT: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static mut VINT_HANDLER: mem::MaybeUninit<extern "C" fn()> = mem::MaybeUninit::uninit();
#[unsafe(naked)]
pub(super) unsafe extern "C" fn vint_irq() {
    core::arch::naked_asm!(
        "movem.l %d0-%d1/%a0-%a1,-(%sp)",
        "move.b {handler_flag},%d0",
        "cmpi.b #0,%d0",
        "beq 2f",
        "move.l {handler},%a0",
        "jsr (%a0)",
        "2:",
        "move.b #0,{flag}",
        "movem.l (%sp)+,%d0-%d1/%a0-%a1",
        ".short 0x4E73", // rte
        handler_flag = sym VINT_HANDLER_PRESENT,
        handler = sym VINT_HANDLER,
        flag = sym VINT_FLAG,
    )
}

#[inline]
pub fn set_hint_handler(handler: Option<extern "C" fn()>) {
    HINT_HANDLER_PRESENT.store(false, atomic::Ordering::Relaxed);
    if let Some(handler) = handler {
        // We use volatile reads to force the compiler to not optimize or reorder things.
        #[allow(static_mut_refs)]
        unsafe { (HINT_HANDLER.as_mut_ptr()).write_volatile(handler); }
        HINT_HANDLER_PRESENT.store(true, atomic::Ordering::Relaxed);
    }
}

pub(super) static HINT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static HINT_HANDLER_PRESENT: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static mut HINT_HANDLER: mem::MaybeUninit<extern "C" fn()> = mem::MaybeUninit::uninit();
#[unsafe(naked)]
pub(super) unsafe extern "C" fn hint_irq() {
    core::arch::naked_asm!(
        "movem.l %d0-%d1/%a0-%a1,-(%sp)",
        "move.b {handler_flag},%d0",
        "cmpi.b #0,%d0",
        "beq 2f",
        "move.l {handler},%a0",
        "jsr (%a0)",
        "2:",
        "movem.l (%sp)+,%d0-%d1/%a0-%a1",
        ".short 0x4E73", // rte
        handler_flag = sym HINT_HANDLER_PRESENT,
        handler = sym HINT_HANDLER,
    )
}

#[inline]
pub fn set_xint_handler(handler: Option<extern "C" fn()>) {
    XINT_HANDLER_PRESENT.store(false, atomic::Ordering::Relaxed);
    if let Some(handler) = handler {
        // We use volatile reads to force the compiler to not optimize or reorder things.
        #[allow(static_mut_refs)]
        unsafe { (XINT_HANDLER.as_mut_ptr()).write_volatile(handler); }
        XINT_HANDLER_PRESENT.store(true, atomic::Ordering::Relaxed);
    }
}

pub(super) static XINT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static XINT_HANDLER_PRESENT: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub(super) static mut XINT_HANDLER: mem::MaybeUninit<extern "C" fn()> = mem::MaybeUninit::uninit();
#[unsafe(naked)]
pub(super) unsafe extern "C" fn xint_irq() {
    core::arch::naked_asm!(
        "movem.l %d0-%d1/%a0-%a1,-(%sp)",
        "move.b {handler_flag},%d0",
        "cmpi.b #0,%d0",
        "beq 2f",
        "move.l {handler},%a0",
        "jsr (%a0)",
        "2:",
        "movem.l (%sp)+,%d0-%d1/%a0-%a1",
        ".short 0x4E73", // rte
        handler_flag = sym XINT_HANDLER_PRESENT,
        handler = sym XINT_HANDLER,
    )
}

#[unsafe(naked)]
unsafe extern "C" fn entrypoint() {
    core::arch::naked_asm!(
        ".short 0x46FC, 0x2100", // move #0x2100,%sr
        "jsr {ram_init}",
        "jsr {main}",
        "jsr {trap}",
        ram_init = sym ram_init,
        main = sym crate::main,
        trap = sym trap,

    )
}

unsafe extern "C" fn ram_init() {
    extern "C" {
        static _data_src: u8;
        static mut _data_start: u8;
        static mut _data_end: u8;
        static mut _bss_start: u8;
        static mut _bss_end: u8;
    }

    (&raw mut _data_start).copy_from_nonoverlapping(&raw const _data_src, (&raw mut _data_end).offset_from_unsigned(&raw mut _data_start));
    (&raw mut _bss_start).write_bytes(0, (&raw mut _bss_end).offset_from_unsigned(&raw mut _bss_start));

    super::alloc::MDSpecAlloc::init();
}

#[no_mangle]
#[unsafe(naked)]
unsafe extern "C" fn abort() {
    core::arch::naked_asm!(
        ".short 0x4AFC",
    )
}