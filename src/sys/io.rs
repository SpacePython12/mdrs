use core::{cell, ptr};

use critical_section as cs;

#[derive(Debug, Clone, Copy)]
pub struct SystemVersion(u8);

impl SystemVersion {
    #[inline]
    pub fn revision(&self) -> u8 {
        self.0 & 0xF
    }

    #[inline]
    pub fn has_fdd(&self) -> bool {
        self.0 & 0x20 != 0
    }

    #[inline]
    pub fn is_pal(&self) -> bool {
        self.0 & 0x40 != 0
    }

    #[inline]
    pub fn is_ntsc(&self) -> bool {
        !self.is_pal()
    }

    #[inline]
    pub fn is_overseas(&self) -> bool {
        self.0 & 0x80 != 0
    }
}

#[inline]
pub fn version() -> SystemVersion {
    const VERSION_REG: *const u8 = 0xA10001 as _;
    unsafe {
        SystemVersion(core::ptr::read_volatile(VERSION_REG))
    }
}


const Z80_BUS: *mut u8 = 0xA00000 as *mut _;
const Z80_BUSREQ: *mut u16 = 0xA11100 as *mut _;
const Z80_RESET: *mut u16 = 0xA11200 as *mut _;

#[inline]
pub unsafe fn assert_z80_reset() {
    core::ptr::write_volatile(Z80_RESET, 0x0000);
}

#[inline]
pub unsafe fn release_z80_reset() {
    core::ptr::write_volatile(Z80_RESET, 0x0100);
}

#[inline]
pub unsafe fn pause_z80() {
    core::ptr::write_volatile(Z80_BUSREQ, 0x0100);
}

#[inline]
pub unsafe fn unpause_z80() {
    core::ptr::write_volatile(Z80_BUSREQ, 0x0100);
}

#[inline]
pub unsafe fn is_z80_running() -> u8 {
    core::ptr::read_volatile((Z80_BUSREQ as *const u8).add(1))
}

/// A structure used to guard Z80 bus request access.
/// 
/// The Z80 is unpaused when this guard is dropped.
pub struct Z80BusGuard<'a>(core::marker::PhantomData<&'a ()>);

impl<'a> Z80BusGuard<'a> {
    #[inline(always)]
    pub unsafe fn new() -> Self {
        unsafe { pause_z80(); }
        Self(core::marker::PhantomData)
    }
}

impl<'a> Drop for Z80BusGuard<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { unpause_z80(); }
    }
}

#[inline]
pub fn with_paused_z80<T, F: FnOnce(&Z80BusGuard<'_>) -> T>(f: F) -> T {
    let guard = unsafe { Z80BusGuard::new() };
    f(&guard)
}

pub trait IOPort {
    const CTRL: *mut u8;
    const DATA: *mut u8;
    
    const SCTRL: *mut u8;
    const RXDATA: *mut u8;
    const TXDATA: *mut u8;

    fn configure(_guard: &Z80BusGuard, directions: u8) {
        unsafe { core::ptr::write_volatile(Self::CTRL, directions); }
    }

    fn read(_guard: &Z80BusGuard) -> u8 {
        unsafe { core::ptr::read_volatile(Self::DATA as *const _) }
    }

    fn write(_guard: &Z80BusGuard, value: u8) {
        unsafe { core::ptr::write_volatile(Self::DATA, value); }
    }
}

#[derive(Clone, Copy)]
pub struct Player1;

impl IOPort for Player1 {
    const CTRL: *mut u8 = 0xA10009 as *mut _;
    const DATA: *mut u8 = 0xA10003 as *mut _;

    const SCTRL: *mut u8 = 0xA10013 as *mut _;
    const RXDATA: *mut u8 = 0xA10011 as *mut _;
    const TXDATA: *mut u8 = 0xA1000F as *mut _;
}

#[derive(Clone, Copy)]
pub struct Player2;

impl IOPort for Player2 {
    const CTRL: *mut u8 = 0xA1000B as *mut _;
    const DATA: *mut u8 = 0xA10005 as *mut _;

    const SCTRL: *mut u8 = 0xA10019 as *mut _;
    const RXDATA: *mut u8 = 0xA10017 as *mut _;
    const TXDATA: *mut u8 = 0xA10015 as *mut _;
}

#[derive(Clone, Copy)]
pub struct Modem;

impl IOPort for Modem {
    const CTRL: *mut u8 = 0xA1000D as *mut _;
    const DATA: *mut u8 = 0xA10007 as *mut _;

    const SCTRL: *mut u8 = 0xA1001F as *mut _;
    const RXDATA: *mut u8 = 0xA1001D as *mut _;
    const TXDATA: *mut u8 = 0xA1001B as *mut _;
}

pub static P1_CONTROLLER: cs::Mutex<cell::Cell<ControllerState<Player1>>> = cs::Mutex::new(cell::Cell::new(ControllerState::new(Player1)));
pub static P2_CONTROLLER: cs::Mutex<cell::Cell<ControllerState<Player2>>> = cs::Mutex::new(cell::Cell::new(ControllerState::new(Player2)));

#[derive(Clone, Copy)]
pub struct ControllerState<P: IOPort>(u16, u16, P);

impl<P: IOPort> ControllerState<P> {
    pub const fn new(port: P) -> Self {
        Self(0, 0, port)
    }

    pub fn init(self) -> Self {
        with_paused_z80(|guard| {
            P::configure(guard, 0x40);
            P::write(guard, 0x40);
        });
        self
    }

    #[inline(never)]
    pub fn update(mut self) -> Self {
        self.1 = self.0;
        self.0 = with_paused_z80(|guard| {
            // 1st step
            P::write(guard, 0x40);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }
            let first = P::read(guard) as u16;

            // 2nd step
            P::write(guard, 0x00);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }
            let second = P::read(guard) as u16;

            // 3rd step
            P::write(guard, 0x40);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }

            // 4th step
            P::write(guard, 0x00);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }

            // 5th step
            P::write(guard, 0x40);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }

            // 6th step
            P::write(guard, 0x00);
            unsafe { core::arch::asm!("nop","nop","nop","nop") }
            let third = if P::read(guard) & 0xF == 0 {
                // 7th step
                P::write(guard, 0x40);
                unsafe { core::arch::asm!("nop","nop","nop","nop") }
                P::read(guard) as u16
            } else { 0 };

            !((first & 0x3F) | ((second & 0x30) << 2) | ((third & 0xF) << 8))
        });
        self
    }

    pub fn start(&self) -> bool {
        self.0 & 0x080 != 0
    }

    pub fn a(&self) -> bool {
        self.0 & 0x040 != 0
    }

    pub fn b(&self) -> bool {
        self.0 & 0x010 != 0
    }

    pub fn c(&self) -> bool {
        self.0 & 0x020 != 0
    }

    pub fn up(&self) -> bool {
        self.0 & 0x001 != 0
    }

    pub fn down(&self) -> bool {
        self.0 & 0x002 != 0
    }

    pub fn left(&self) -> bool {
        self.0 & 0x004 != 0
    }

    pub fn right(&self) -> bool {
        self.0 & 0x008 != 0
    }

    pub fn mode(&self) -> bool {
        self.0 & 0x800 != 0
    }

    pub fn x(&self) -> bool {
        self.0 & 0x400 != 0
    }

    pub fn y(&self) -> bool {
        self.0 & 0x200 != 0
    }

    pub fn z(&self) -> bool {
        self.0 & 0x100 != 0
    }
}

