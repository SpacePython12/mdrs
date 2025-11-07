
pub mod vdp;
pub mod libc;
pub mod alloc;
pub mod io;
pub mod fixed;

use critical_section as cs;

use crate::sys::alloc::MDSpecializeAlloc;

extern "C" {
    static _data_src: u8;
    static mut _data_start: u8;
    static mut _data_end: u8;
    static mut _bss_start: u8;
    static mut _bss_end: u8;
}

#[inline]
const fn data_size() -> usize {
    unsafe { (&raw const _data_end).offset_from(&raw const _data_start) as usize }
}

#[inline]
const fn data_src_ptr() -> *const u8 {
    &raw const _data_src
}

#[inline]
const fn data_dst_ptr() -> *mut u8 {
    &raw mut _data_start
}
 
#[inline]
const fn bss_size() -> usize {
    unsafe { (&raw const _bss_end).offset_from(&raw const _bss_start) as usize }
}

#[inline]
const fn bss_dst_ptr() -> *mut u8 {
    &raw mut _bss_start
}


#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    vdp::VDP::debug_alert(info.message().as_str().unwrap_or("(panic message needs formatting)").as_bytes());
    vdp::VDP::debug_halt();
    extern "C" {
        fn abort() -> !;
    }

    unsafe { abort() };
}

/// Runs as soon as the console starts up, and before main() runs.
#[no_mangle]
pub unsafe fn _init() {
    {
        const TMSS_REG: *mut u32 = 0xA14000 as _;
        const TMSS_VAL: u32 = 0x53454741u32; // "SEGA" as a single long
        if io::version().revision() > 0 {
            core::ptr::write_volatile(TMSS_REG, TMSS_VAL);
        }
    }

    // Initalize .data segment
    core::ptr::copy_nonoverlapping(data_src_ptr(), data_dst_ptr(), data_size());

    // Zero out .bss segment
    core::ptr::write_bytes(bss_dst_ptr(), 0, bss_size());

    ALLOCATOR.init();

    with_cs::<1, 7, _>(|cs| {
        let p1 = io::P1_CONTROLLER.borrow(cs);
        let p2 = io::P2_CONTROLLER.borrow(cs);
        p1.set(p1.get().init());
        p2.set(p2.get().init());
    });
}

#[global_allocator]
static ALLOCATOR: MDSpecializeAlloc = MDSpecializeAlloc::new();

/// Sets the 68k's interrupt mask bits to the specified constant.
/// 
/// Unfortunately, due to an LLVM compiler bug, we have to use a temporary register here. See issue [#165077](https://github.com/llvm/llvm-project/issues/165077).
#[inline]
pub unsafe fn set_int_level<const LEVEL: u8>() {
    core::arch::asm!(
        "move.w #{lvl},{tmp}",
        "move.w {tmp},%sr",
        lvl = const (0x2000i16 | (((LEVEL & 0x7) as i16) << 8)),
        tmp = out(reg_data) _
    )
}

/// Execute closure `f` in a critical section.
///
/// Nesting critical sections is NOT allowed.
///
/// # Panics
///
/// This function panics if the given closure `f` panics. In this case
/// the critical section is released before unwinding.
#[inline]
pub fn with_cs<const OUTER: u8, const INNER: u8, R>(f: impl FnOnce(cs::CriticalSection) -> R) -> R {
    // Helper for making sure `release` is called even if `f` panics.
    struct Guard<const RESTORE: u8>;

    impl<const RESTORE: u8> Drop for Guard<RESTORE> {
        #[inline(always)]
        fn drop(&mut self) {
            unsafe { set_int_level::<RESTORE>(); }
        }
    }

    unsafe { set_int_level::<INNER>(); }
    let _guard = Guard::<OUTER>;

    unsafe { f(cs::CriticalSection::new()) }
}

#[repr(C)] // guarantee 'bytes' comes after '_align'
pub struct AlignedAs<Align, Bytes: ?Sized> {
    pub _align: [Align; 0],
    pub bytes: Bytes,
}

#[macro_export]
macro_rules! include_bytes_aligned_as {
    ($align_ty:ty, $path:literal) => {
        const {  // const block expression to encapsulate the static
            use $crate::sys::AlignedAs;
            
            // this assignment is made possible by CoerceUnsized
            static ALIGNED: &AlignedAs::<$align_ty, [u8]> = &AlignedAs {
                _align: [],
                bytes: *include_bytes!($path),
            };

            unsafe { core::slice::from_raw_parts(ALIGNED.bytes.as_ptr().cast::<$align_ty>(), ALIGNED.bytes.len() / core::mem::size_of::<$align_ty>()) }
        }
    };
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub struct AtomicFlag<const BIT: u8 = 0u8>(u8);

// impl<const BIT: u8> AtomicFlag<BIT> {
//     pub const fn new(value: bool) -> Self {
//         Self(value as u8)
//     }

//     #[inline]
//     unsafe fn try_lock_internal(&self) -> u8 {
//         let status: u8;
//         core::arch::asm!(
//             "bset #{i},({f})",
//             "seq {s}",
//             i = const BIT,
//             f = in(reg_addr) &self.0,
//             s = out(reg_data) status
//         );
//         status
//     }
// }