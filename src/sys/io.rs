use critical_section as cs;

#[repr(transparent)]
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
        SystemVersion(VERSION_REG.read_volatile())
    }
}

pub trait IOPort {
    const CTRL: *mut u8;
    const DATA: *mut u8;
    
    const SCTRL: *mut u8;
    const RXDATA: *mut u8;
    const TXDATA: *mut u8;

    fn configure(directions: u8) {
        unsafe { Self::CTRL.write_volatile(directions); }
    }

    fn read() -> u8 {
        unsafe { Self::DATA.read_volatile() }
    }

    fn write(value: u8) {
        unsafe { Self::DATA.write_volatile(value); }
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

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Button {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    B = 4,
    C = 5,
    A = 6,
    Start = 7,
    Z = 8,
    Y = 9,
    X = 10,
    Mode = 11,
}

pub struct ControllerState<P: IOPort>(core::sync::atomic::AtomicU16, core::sync::atomic::AtomicU16, P);

impl<P: IOPort> ControllerState<P> {
    pub const fn new(port: P) -> Self {
        Self(
            core::sync::atomic::AtomicU16::new(0), 
            core::sync::atomic::AtomicU16::new(0), 
            port
        )
    }

    pub fn init(&self, _cs: cs::CriticalSection) {
        P::configure(0x40);
        P::write(0x40);
    }

    #[inline(never)]
    pub fn update(&self, _cs: cs::CriticalSection) {
        let new = {
            // 1st step
            P::write(0x40);
            unsafe { core::arch::asm!("nop","nop") }
            let first = P::read() as u16;

            // 2nd step
            P::write(0x00);
            unsafe { core::arch::asm!("nop","nop") }
            let second = P::read() as u16;

            // 3rd step
            P::write(0x40);
            unsafe { core::arch::asm!("nop","nop") }

            // 4th step
            P::write(0x00);
            unsafe { core::arch::asm!("nop","nop") }

            // 5th step
            P::write(0x40);
            unsafe { core::arch::asm!("nop","nop") }

            // 6th step
            P::write(0x00);
            unsafe { core::arch::asm!("nop","nop") }
            let third = if P::read() & 0xF == 0 {
                // 7th step
                P::write(0x40);
                unsafe { core::arch::asm!("nop","nop","nop","nop") }
                P::read() as u16
            } else { 0 };

            !((first & 0x3F) | ((second & 0x30) << 2) | ((third & 0xF) << 8))
        };
        let old = self.0.load(core::sync::atomic::Ordering::Acquire);
        let change = old ^ new;
        self.1.store(change, core::sync::atomic::Ordering::Relaxed);
        self.0.store(new, core::sync::atomic::Ordering::Release);
    }

    pub fn held(&self, button: Button) -> bool {
        let mask = 1u16 << button as u8;
        let state = self.0.load(core::sync::atomic::Ordering::Relaxed);
        state & mask != 0
    }

    pub fn pressed(&self, button: Button) -> bool {
        let mask = 1u16 << button as u8;
        let state = self.0.load(core::sync::atomic::Ordering::Relaxed);
        let change = self.1.load(core::sync::atomic::Ordering::Relaxed);
        (state & change) & mask != 0
    }

    pub fn released(&self, button: Button) -> bool {
        let mask = 1u16 << button as u8;
        let state = self.0.load(core::sync::atomic::Ordering::Relaxed);
        let change = self.1.load(core::sync::atomic::Ordering::Relaxed);
        (!state & change) & mask != 0
    }
}

