use super::prelude::*;


const BUSREQ: *mut u16 = 0xA11100 as *mut _;
const RESET: *mut u16 = 0xA11200 as *mut _;

#[inline]
pub unsafe fn assert_z80_reset() {
    RESET.write_volatile(0x000);
}

#[inline]
pub unsafe fn release_z80_reset() {
    RESET.write_volatile(0x100);
}

#[inline]
pub unsafe fn pause_z80() {
    BUSREQ.write_volatile(0x100);
}

#[inline]
pub unsafe fn unpause_z80() {
    BUSREQ.write_volatile(0x000);
}

#[inline]
pub unsafe fn is_z80_paused() -> u16 {
    BUSREQ.read_volatile()
}

/// A token used to signal that the Z80 is paused.
#[derive(Clone, Copy)]
pub struct Z80PauseGuard<'a>(core::marker::PhantomData<&'a ()>);

impl<'a> Z80PauseGuard<'a> {
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

#[inline]
pub fn with_paused_z80<R>(_cs: cs::CriticalSection, reset: bool, f: impl FnOnce(Z80PauseGuard<'_>) -> R) -> R {
    // Helper for making sure `release_z80` is called even if `f` panics.
    struct Guard(bool);

    impl Drop for Guard {
        #[inline(always)]
        fn drop(&mut self) {
            unsafe { 
                if self.0 {
                    assert_z80_reset();
                    core::arch::asm!("nop", "nop", "nop", "nop");
                    release_z80_reset();
                }
                unpause_z80(); 
            }
        }
    }

    unsafe { 
        if reset {
            release_z80_reset();
        }
        pause_z80(); 
    }
    let _guard = Guard(reset);

    unsafe { f(Z80PauseGuard::new()) }
}



#[repr(transparent)]
pub struct Z80Bus([u8; 0x10000]);

impl Z80Bus {
    #[inline]
    pub unsafe fn new<'a>(_guard: Z80PauseGuard<'a>) -> &'a mut Self {
        &mut *(0xA00000usize as *mut Self)
    }

    #[inline]
    pub fn acquire<'a>(guard: Z80PauseGuard<'a>) -> &'a mut Self {
        unsafe {
            while is_z80_paused() != 0x100 {}

            Self::new(guard)
        }
    }
}

impl core::ops::Deref for Z80Bus {
    type Target = Z80BusSlice;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self.0.as_slice()) }
    }
}

impl core::ops::DerefMut for Z80Bus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { mem::transmute(self.0.as_mut_slice()) }
    }
}

#[inline(always)]
unsafe fn copy_bytes(dst: *mut u8, src: *const u8, count: usize) {
    core::arch::asm!(
        "2:",
        "sub.l #1,{count}",
        "bcs 3f",
        "move.b ({src})+,({dst})+",
        "bra 2b",
        "3:",
        src = inout(reg_addr) src => _,
        dst = inout(reg_addr) dst => _,
        count = inout(reg_data) count => _,
    );
}

/// This represents a view of the Z80's address space. 
/// 
/// The inner slice is private in order to prevent word and long accesses, which lock up on real hardware.
#[repr(transparent)]
pub struct Z80BusSlice([u8]);

impl Z80BusSlice {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn read_byte(&self, offset: u16) -> u8 {
        unsafe { ((&self.0[offset as usize]) as *const u8).read_volatile() }
    }

    #[inline]
    pub fn write_byte(&mut self, offset: u16, value: u8) {
        unsafe { ((&mut self.0[offset as usize]) as *mut u8).write_volatile(value) }
    }

    #[inline]
    pub fn copy_bytes_to(&self, data: &mut [u8]) {
        assert_eq!(self.0.len(), data.len());

        unsafe {
            copy_bytes(data.as_mut_ptr(), self.0.as_ptr(), data.len());
        }
    }

    #[inline]
    pub fn copy_bytes_from(&mut self, data: &[u8]) {
        assert_eq!(self.0.len(), data.len());

        unsafe {
            copy_bytes(self.0.as_mut_ptr(), data.as_ptr(), data.len());
        }
        
    }
}

#[inline(always)]
fn to_range(range: impl core::ops::RangeBounds<u16>, len: usize) -> core::ops::Range<usize> {
    let start = match range.start_bound() {
        core::ops::Bound::Included(idx) => (*idx) as usize,
        core::ops::Bound::Excluded(idx) => (*idx+1) as usize,
        core::ops::Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        core::ops::Bound::Included(idx) => (*idx-1) as usize,
        core::ops::Bound::Excluded(idx) => (*idx) as usize,
        core::ops::Bound::Unbounded => len,
    };
    start..end
}

impl<Idx: core::ops::RangeBounds<u16>> core::ops::Index<Idx> for Z80BusSlice {
    type Output = Self;

    fn index(&self, index: Idx) -> &Self::Output {
        
        unsafe { mem::transmute(&self.0[to_range(index, self.0.len())]) }
    }
}

impl<Idx: core::ops::RangeBounds<u16>> core::ops::IndexMut<Idx> for Z80BusSlice {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        let len = self.0.len();
        unsafe { mem::transmute(&mut self.0[to_range(index, len)]) }
    }
}

