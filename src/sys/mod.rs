pub mod vdp;
pub mod libc;
pub mod alloc;
pub mod io;
pub mod fixed;
pub mod interrupts;
pub mod header;
pub mod z80;

pub mod prelude {
    pub use critical_section as cs;

    pub use super::vdp as vdp;
    pub use super::io;
    pub use super::z80;
    pub use super::fixed;
    pub use super::interrupts;

    pub use super::vdp::{SharedVDPAccess, ExclusiveVDPAccess};

    pub use super::header::ROMHeader;

    pub use core::sync::atomic;
    pub use core::mem;
    pub use core::cell;
    pub use core::num::NonZero;

    pub use super::with_cs;
}

use prelude::*;

use crate::sys::alloc::MDSpecAlloc;

static IS_PANICKING: atomic::AtomicBool = atomic::AtomicBool::new(false);

#[panic_handler]
pub unsafe fn panic_handler(info: &core::panic::PanicInfo) -> ! {

    if !IS_PANICKING.load(atomic::Ordering::Relaxed) {
        IS_PANICKING.store(true, atomic::Ordering::Relaxed);
        interrupts::set_vint_handler(None);
        interrupts::set_hint_handler(None);
        interrupts::set_xint_handler(None);
        use core::fmt::Write;
        
        with_cs(|cs| {
            let mut vdp = vdp::VDP::borrow_mut(cs);
            {
                let mut writer = vdp.debug_writer();
                if let Some(message) = info.message().as_str() {
                    let _ = writer.write_str("panicked: ");
                    let _ = writer.write_str(message);
                } else {
                    let _ = writer.write_fmt(format_args!("panicked: {}\n", info.message()));
                    let _ = writer.write_fmt(format_args!("at: {:?}", info.location()));
                }
                // vdp_v1::VDP::debug_alert(format_buffer.as_slice());
                // if let Some(location) = info.location() {
                //     vdp_v1::VDP::debug_alert_long(location.column());
                //     vdp_v1::VDP::debug_alert_long(location.line());
                //     vdp_v1::VDP::debug_alert(location.file());
                // }
            }
            vdp.debug_halt();
        });
    }
    extern "C" {
        fn abort() -> !;
    }

    unsafe { abort() };
}

#[global_allocator]
static ALLOCATOR: MDSpecAlloc = MDSpecAlloc;

/// Sets the 68k's interrupt mask bits to the specified constant. 
/// 
/// Requires supervisor mode, otherwise this will trap.
/// 
/// Right now, this uses a __VERY HACKY FIX__, and uses a literal hexadecimal opcode. FIXME when LLVM issue [#165077](https://github.com/llvm/llvm-project/issues/165077) is fixed.
#[inline]
pub unsafe fn move_imm_to_sr<const LEVEL: u8>() {
    // core::arch::asm!(
    //     "move.w #{lvl},%sr",
    //     lvl = const (0x2000i16 | (((LEVEL & 0x7) as i16) << 8)),
    // )

    core::arch::asm!(
        ".short 0x46FC, {lvl}",
        lvl = const (0x2000i16 | (((LEVEL & 0x7) as i16) << 8))
    );
}

#[inline]
pub unsafe fn move_from_sr() -> u16 {
    let value: u16;
    core::arch::asm!(
        ".short 0x40C7", // move.w %sr,%d7
        out("d7") value,
    );
    value
}

#[inline]
pub unsafe fn move_to_sr(value: u16) {
    core::arch::asm!(
        "move.w %d7,%sr",
        in("d7") value,
    );
}

/// Sets the 68k's interrupt mask bits to the specified constant, and stops the processor until an exception, trap, or interrupt occurs. 
/// 
/// Requires supervisor mode, otherwise this will trap.
/// 
/// Right now, this uses a __VERY HACKY FIX__, and uses a literal hexadecimal opcode. FIXME when LLVM issue [#165077](https://github.com/llvm/llvm-project/issues/165077) is fixed.
#[inline]
pub unsafe fn stop<const LEVEL: u8>() {
    // core::arch::asm!(
    //     "stop #{lvl}",
    //     lvl = const (0x2000i16 | (((LEVEL & 0x7) as i16) << 8)),
    // )

    core::arch::asm!(
        ".short 0x4E72, {lvl}", // 4E72 = stop #imm
        lvl = const (0x2000i16 | (((LEVEL & 0x7) as i16) << 8))
    )
}

/// Execute closure `f` in a critical section.
///
/// Nesting critical sections is allowed.
///
/// # Panics
///
/// This function panics if the given closure `f` panics. In this case
/// the critical section is released before unwinding.
#[inline]
pub fn with_cs<R>(f: impl FnOnce(cs::CriticalSection) -> R) -> R {
    // Helper for making sure `move_sr` is called even if `f` panics.
    struct Guard(u16);

    impl Drop for Guard {
        #[inline(always)]
        fn drop(&mut self) {
            unsafe { move_to_sr(self.0); }
        }
    }

    // unsafe { core::hint::black_box(move_from_sr()); }

    let _guard = Guard(unsafe { move_from_sr() });
    unsafe { move_imm_to_sr::<7>(); }

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

            &ALIGNED.bytes
        }
    };
}

#[macro_export]
macro_rules! include_as {
    ($align_ty:ty, $path:literal) => {
        const {
            let bytes = crate::include_bytes_aligned_as!($align_ty, $path);

            unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<$align_ty>(), bytes.len() / core::mem::size_of::<$align_ty>()) }
        }
    };
}

#[repr(C)]
pub struct FormatBuffer<const N: usize> {
    length: u8,
    buffer: mem::MaybeUninit<[u8; N]>,
}

impl<const N: usize> FormatBuffer<N> {
    const LIMIT_SIZE: () = const {
        assert!(N <= u8::MAX as usize, "length of FormatBuffer is too long!");
    };

    pub const fn new() -> Self {
        Self {
            length: 0,
            buffer: mem::MaybeUninit::uninit()
        }
    }

    #[inline(never)]
    pub const fn clear(&mut self) {
        self.length = 0;
    }

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { &self.buffer.assume_init_ref()[..self.length as usize] }
    }

    const fn uninit_data_mut(&mut self) -> &mut [u8] {
        unsafe { &mut self.buffer.assume_init_mut()[self.length as usize..] }
    }


}

impl<const N: usize> core::fmt::Write for FormatBuffer<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // vdp_v1::VDP::debug_alert_long(s.len() as u32);
        self.uninit_data_mut().get_mut(..s.len()).ok_or(core::fmt::Error)?.copy_from_slice(s.as_bytes());
        self.length += s.len() as u8;
        Ok(())
    }
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