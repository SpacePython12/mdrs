use core::num::NonZero;
use core::ptr;
use core::mem;
use core::cell;

use critical_section as cs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VRAMAddress(u16);

impl VRAMAddress {
    #[inline]
    pub const fn from_byte_addr(addr: u32) -> Self {
        Self((addr >> 1) as u16)
    }

    #[inline]
    pub const fn from_word_addr(addr: u16) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn byte_addr(self) -> u32 {
        (self.0 << 1) as u32
    }

    #[inline]
    pub const fn word_addr(self) -> u16 {
        self.0
    }

    #[inline]
    pub const fn from_tile_index(index: u16) -> Self {
        Self((index & 0x7FF) << 5)
    }
}

// impl const From<u16> for VRAMAddress {
//     fn from(value: u16) -> Self {
//         Self(value)
//     }
// }

// impl const core::ops::Not for VRAMAddress {
//     type Output = Self;

//     #[inline]
//     fn not(self) -> Self::Output {
//         Self(!self.0)
//     }
// }

// impl const core::ops::BitAnd<u16> for VRAMAddress {
//     type Output = Self;

//     #[inline]
//     fn bitand(self, rhs: u16) -> Self::Output {
//         Self(self.0 & (rhs >> 1))
//     }
// }

// impl const core::ops::BitAndAssign<u16> for VRAMAddress {
//     #[inline]
//     fn bitand_assign(&mut self, rhs: u16) {
//         self.0 &= rhs
//     }
// }

// impl const core::ops::BitOr<u16> for VRAMAddress {
//     type Output = Self;

//     #[inline]
//     fn bitor(self, rhs: u16) -> Self::Output {
//         Self(self.0 | rhs)
//     }
// }

// impl const core::ops::BitOrAssign<u16> for VRAMAddress {
//     #[inline]
//     fn bitor_assign(&mut self, rhs: u16) {
//         self.0 |= rhs
//     }
// }

// impl const core::ops::BitXor<u16> for VRAMAddress {
//     type Output = Self;

//     #[inline]
//     fn bitxor(self, rhs: u16) -> Self::Output {
//         Self(self.0 ^ rhs)
//     }
// }

// impl const core::ops::BitXorAssign<u16> for VRAMAddress {
//     #[inline]
//     fn bitxor_assign(&mut self, rhs: u16) {
//         self.0 ^= rhs
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    VRAM(VRAMAddress),
    CRAM(u8),
    VSRAM(u8),
}

impl Address {
    #[inline]
    pub const fn byte_addr(self) -> u32 {
        match self {
            Address::VRAM(addr) => addr.byte_addr(),
            Address::CRAM(addr) => addr as u32,
            Address::VSRAM(addr) => addr as u32,
        }
    }

    #[inline]
    pub const fn word_addr(self) -> u16 {
        match self {
            Address::VRAM(addr) => addr.word_addr(),
            Address::CRAM(addr) => (addr >> 1) as u16,
            Address::VSRAM(addr) => (addr >> 1) as u16,
        }
    }

    #[inline]
    pub fn cram_line(line: u8) -> Self {
        Self::CRAM((line & 0x3) << 4)
    }

    // pub fn vram_plane_a_loc(x: u8, y: u8) -> Self {
    //     let settings = Settings::current();
    //     let width = settings.plane_width;
    //     let height = settings.plane_height;
    //     let addr = settings.plane_a_base.0 + ()
    // }
}

/// A struct representing where the window is drawn instead of plane A for an axis.
///
/// For example x: After(10), would make the window render to the right of tile 10 onwards.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WindowClip {
    Before(u8),
    After(u8),
}

impl Default for WindowClip {
    fn default() -> Self {
        Self::Before(0)
    }
}

impl WindowClip {
    fn raw_value(self) -> u8 {
        match self {
            WindowClip::Before(v) => v & 0x1f,
            WindowClip::After(v) => 0x80 | (v & 0x1f),
        }
    }
}

/// This enumeration is for configuring how vertical scrolling works.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VScrollMode {
    #[default]
    Screen = 0,
    Columns = 1,
}

/// This enumeration is for configuring how horizontal scrolling works.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HScrollMode {
    #[default]
    Screen = 0b00,
    Rows = 0b10,
    Lines = 0b11,
}

/// The interlacing rendering mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterlaceMode {
    #[default]
    None = 0b00,
    Interlace = 0b01,
    DoubleRes = 0b11,
}

/// An enumeration of valid plane sizes in tiles.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaneSize {
    #[default]
    Size32x32 = 0b00_00_00_00,
    Size64x32 = 0b00_00_00_01,
    Size128x32 = 0b00_00_00_10,
    Size32x64 = 0b00_01_00_00,
    Size64x64 = 0b00_01_00_10,
    Size32x128 = 0b00_10_00_00,
}

impl PlaneSize {
    #[inline]
    pub const fn width_shift(&self) -> u8 {
        match self {
            PlaneSize::Size32x32 |
            PlaneSize::Size32x64 |
            PlaneSize::Size32x128 => 5,
            PlaneSize::Size64x32 |
            PlaneSize::Size64x64 => 6,
            PlaneSize::Size128x32 => 7,
        }
    }

    #[inline]
    pub const fn height_shift(&self) -> u8 {
        match self {
            PlaneSize::Size32x32 |
            PlaneSize::Size64x32 |
            PlaneSize::Size128x32 => 5,
            PlaneSize::Size32x64 |
            PlaneSize::Size64x64 => 6,
            PlaneSize::Size32x128 => 7,
        }
    }

    #[inline]
    pub const fn width_tiles(&self) -> u8 {
        1u8 << self.width_shift()
    }

    #[inline]
    pub const fn height_tiles(&self) -> u8 {
        1u8 << self.height_shift()
    }

    #[inline]
    pub const fn x_mask(&self) -> u8 {
        self.width_tiles()-1
    }

    #[inline]
    pub const fn y_mask(&self) -> u8 {
        self.height_tiles()-1
    }

    #[inline]
    pub const fn pitch_shift(&self) -> u8 {
        self.width_shift()
    }

    #[inline]
    pub const fn tile_offset(&self, x: u8, y: u8) -> u16 {
        let x = (x & self.x_mask()) as u16;
        let y = (y & self.y_mask()) as u16;
        (y << self.pitch_shift()) + x
    }

    #[inline]
    pub const fn tile_offset_from(&self, base: VRAMAddress, x: u8, y: u8) -> VRAMAddress {
        VRAMAddress(base.0 + self.tile_offset(x, y))
    }
}

/// A struct representing the display flags of a single tile.
///
/// This is shared between sprite definitions and tiles rendered on one of the 3
/// render planes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TileFlags(u16);

impl TileFlags {
    const PRIORITY_FLAG: u16 = 0x8000;
    const H_FLIP_FLAG: u16 = 0x0800;
    const V_FLIP_FLAG: u16 = 0x1000;
    const TILE_INDEX_MASK: u16 = 0x07FF;
    const PALETTE_SHIFT: u8 = 13;
    const PALETTE_MASK: u16 = const { 3u16 << Self::PALETTE_SHIFT };

    pub const ZEROED: Self = Self(0);

    /// Create a new flag set for a given tile index.
    pub const fn for_tile(tile_index: u16, palette: u8) -> Self {
        Self::ZEROED
            .with_tile_index(tile_index)
            .with_palette(palette)
    }

    /// Get the tile index these flags refer to.
    pub const fn tile_index(&self) -> u16 { 
        self.0 & Self::TILE_INDEX_MASK
    }

    /// Set the tile index for these flags.
    pub const fn set_tile_index(&mut self, tile_index: u16) {
        self.0 = (self.0 & !Self::TILE_INDEX_MASK) | (tile_index & Self::TILE_INDEX_MASK);
    }

    pub const fn with_tile_index(mut self, tile_index: u16) -> Self {
        self.set_tile_index(tile_index);
        self
    }

    /// Get the palette index these flags use.
    pub const fn palette(&self) -> u8 {
        ((self.0 >> Self::PALETTE_SHIFT) & 3) as u8
    }

    /// Set the palette used by these flags.
    pub const fn set_palette(&mut self, palette: u8) {
        self.0 = (self.0 & !Self::PALETTE_MASK) | (((palette & 3) as u16) << Self::PALETTE_SHIFT);
    }

    pub const fn with_palette(mut self, palette: u8) -> Self {
        self.set_palette(palette);
        self
    }

    /// Returns true if this tile will be rendered with priority.
    pub const fn priority(&self) -> bool { 
        (self.0 as i16) < 0
    }

    /// Configure whether these flags render tiles with priority.
    pub const fn set_priority(&mut self, priority: bool)  {
        if priority {
            self.0 |= Self::PRIORITY_FLAG;
        } else {
            self.0 &= !Self::PRIORITY_FLAG;
        }
    }
    
    pub const fn with_priority(mut self, priority: bool) -> Self {
        self.set_priority(priority);
        self
    }

    /// Returns true if this tile is flipped horizontally.
    pub const fn flip_h(&self) -> bool { 
        (self.0 & Self::H_FLIP_FLAG) != 0
    }

    /// Set whether these flags will render horizontally flipped tiles.
    pub const fn set_flip_h(&mut self, flip: bool) {
        if flip {
            self.0 |= Self::H_FLIP_FLAG
        } else {
            self.0 &= !Self::H_FLIP_FLAG
        }
    }

    pub const fn with_flip_h(mut self, flip: bool) -> Self {
        self.set_flip_h(flip);
        self
    }

    /// Returns true if this tile is flipped vertically.
    pub const fn flip_v(&self) -> bool { 
        (self.0 & Self::V_FLIP_FLAG) != 0 
    }

    /// Set whether these flags will render vertically flipped tiles.
    pub const fn set_flip_v(&mut self, flip: bool) {
        if flip {
            self.0 |= Self::V_FLIP_FLAG
        } else {
            self.0 &= !Self::V_FLIP_FLAG
        }
    }

    pub const fn with_flip_v(mut self, flip: bool) -> Self {
        self.set_flip_v(flip);
        self
    }
}

impl From<TileFlags> for u16 {
    fn from(value: TileFlags) -> Self {
        value.0
    }
}

impl From<u16> for TileFlags {
    fn from(value: u16) -> Self {
        TileFlags(value)
    }
}

/// A typedef for tile contents.
pub type Tile = [u32; 8];

// #[macro_export]
// macro_rules! include_tiles {
//     ($path:literal) => {
//         include_bytes_aligned_as!($crate::sys::vdp::Tile, $path)
//     };
// }

/// An enumeration of valid sprite sizes in tiles.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default)]
pub enum SpriteSize {
    #[default]
    Size1x1 = 0b00_00,
    Size2x1 = 0b01_00,
    Size3x1 = 0b10_00,
    Size4x1 = 0b11_00,
    Size1x2 = 0b00_01,
    Size2x2 = 0b01_01,
    Size3x2 = 0b10_01,
    Size4x2 = 0b11_01,
    Size1x3 = 0b00_10,
    Size2x3 = 0b01_10,
    Size3x3 = 0b10_10,
    Size4x3 = 0b11_10,
    Size1x4 = 0b00_11,
    Size2x4 = 0b01_11,
    Size3x4 = 0b10_11,
    Size4x4 = 0b11_11,
}

impl SpriteSize {
    /// Get the `SpriteSize` given the width and height of the sprite in tiles.
    pub fn for_size(w: u8, h: u8) -> SpriteSize {
        unsafe { mem::transmute(((w & 0x3) - 1) << 2 | ((h & 0x3) - 1)) }
    }

    pub fn width(&self) -> u8 {
        match self {
            Self::Size1x1 |
            Self::Size1x2 |
            Self::Size1x3 |
            Self::Size1x4 => 1,
            Self::Size2x1 |
            Self::Size2x2 |
            Self::Size2x3 |
            Self::Size2x4 => 2,
            Self::Size3x1 |
            Self::Size3x2 |
            Self::Size3x3 |
            Self::Size3x4 => 3,
            Self::Size4x1 |
            Self::Size4x2 |
            Self::Size4x3 |
            Self::Size4x4 => 4,
        }
    }

    pub fn height(&self) -> u8 {
        match self {
            Self::Size1x1 |
            Self::Size2x1 |
            Self::Size3x1 |
            Self::Size4x1 => 1,
            Self::Size1x2 |
            Self::Size2x2 |
            Self::Size3x2 |
            Self::Size4x2 => 2,
            Self::Size1x3 |
            Self::Size2x3 |
            Self::Size3x3 |
            Self::Size4x3 => 3,
            Self::Size1x4 |
            Self::Size2x4 |
            Self::Size3x4 |
            Self::Size4x4 => 4,
        }
    }
}

/// A representation of the hardware sprites supported by the Mega Drive VDP.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Sprite {
    pub y: u16,
    pub size: SpriteSize,
    pub link: u8,
    pub flags: TileFlags,
    pub x: u16,
}

impl Sprite {
    pub const ZEROED: Self = Self {
        y: 0,
        size: SpriteSize::Size1x1,
        link: 0,
        flags: TileFlags::ZEROED,
        x: 0,
    };

    /// Create a new sprite with the given rendering flags.
    pub const fn with_flags(flags: TileFlags, size: SpriteSize) -> Self {
        Sprite {
            y: 0,
            size,
            link: 0,
            flags,
            x: 0,
        }
    }

    /// Fetch the rendering flags for this sprite.
    pub const fn flags(&self) -> TileFlags { self.flags }

    /// Get a mutable reference to this sprite's rendering flags.
    pub const fn flags_mut(&mut self) -> &mut TileFlags { &mut self.flags }

    /// Set the rendering flags for this sprite.
    pub const fn set_flags(&mut self, flags: TileFlags) { self.flags = flags; }
}

impl core::ops::Deref for Sprite {
    type Target = TileFlags;

    fn deref(&self) -> &Self::Target {
        &self.flags
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct Status: u16 {
        /// If set, this is a PAL system. If clear, this is an NTSC system.
        const IS_PAL = 1 << 0;
        /// If set, DMA is currently in progress.
        const DMA_ACTIVE = 1 << 1;
        /// If set, horizontal blank is currently in progress.
        const HBLANK_ACTIVE = 1 << 2;
        /// If set, vertical blank is currently in progress.
        const VBLANK_ACTIVE = 1 << 3;
        /// If set, odd frame displayed in interlaced mode. If clear, even frame displayed in interlaced mode. 
        const ODD_FRAME = 1 << 4;
        /// If set, any two sprites on the current scanline have non-transparent pixels overlapping.
        /// 
        /// This is a holdover from the Master System's TMS9918 chip, and should be ignored.
        const SPRITE_OVERLAP = 1 << 5;
        /// If set, the sprite limit has been hit on the current scanline (>16 in H32 mode, >20 in H40 mode).
        const SPRITE_LIMIT = 1 << 6;
        /// If set, vertical interrupt is occuring.
        const VINT_OCCURRED = 1 << 7;
        /// If set, the VDP's FIFO buffer is completely full. 
        /// 
        /// Any subsequent writes to the control port will freeze the 68k until the VDP can process the next command.
        const FIFO_FULL = 1 << 8;
        /// If set, the VDP's FIFO buffer is completely empty. 
        const FIFO_EMPTY = 1 << 9;

    }
}

bitflags::bitflags! {
    pub struct Mode: u32 {
        /// If set, the H/V counter is latched on external interrupts.
        const STOP_HV_ON_XINT = 1 << 1;
        /// If set, horizontal interrupts are enabled.
        const ENABLE_HINT = 1 << 4;
        /// If set, V30 mode is active.
        const V30_MODE = 1 << 11;
        /// If set, DMA transfers are enabled.
        const ENABLE_DMA = 1 << 12;
        /// If set, vertical interrupts are enabled.
        const ENABLE_VINT = 1 << 13;
        /// If set, display is enabled. If clear, display is filled with background color
        const ENABLE_DISPLAY = 1 << 14;
        /// If set, all 8 hscroll values in a group are used. If clear, every 8th scroll value is used for each row of tiles.
        const LINE_SCROLL = 1 << 16;
        /// If set, each row has its own set of 8 hscroll values. If clear, they share the first 8 hscroll values.
        const ROW_SCROLL = 1 << 17;
        /// If set, each column (2 tiles) has its own vscroll value. If clear, they all share the first vscroll value.
        const COLUMN_SCROLL = 1 << 18;
        /// If set, external interrupts are enabled.
        const ENABLE_XINT = 1 << 19;
        /// If set, H40 mode is active.
        const H40_MODE = (1 << 24) | (1 << 31);
        /// If set, interlace mode is active.
        /// 
        /// The type of interlace mode is determined by the double interlace flag.
        const ENABLE_INTERLACE = 1 << 25;
        /// If set, double interlace mode is active. If clear, normal interlace mode is active.
        /// 
        /// This has no effect if interlace mode is disabled.
        const DOUBLE_INTERLACE = 1 << 26;
        /// If set, shadow/highlight mode is active.
        const SHADOW_HIGHLIGHT = 1 << 27;
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HVCounter {
    pub v: u8,
    pub h: u8,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SetRegCmd(pub u16);

impl SetRegCmd {
    #[inline]
    pub const fn new(reg: u8, val: u8) -> Self {
        Self(0x8000 | (((reg & 0x1F) as u16) << 8) | (val as u16))
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SetAddrCmd(pub u32);

impl SetAddrCmd {
    #[inline]
    pub const fn new_write(addr: Address, dma: bool, copy: bool) -> Self {
        let mut ctrl = match addr {
            Address::VRAM(_) => 0x40000000,
            Address::CRAM(_) => 0xC0000000,
            Address::VSRAM(_) => 0x40000010,
        };
        let addr = addr.byte_addr();
        if dma {
            ctrl |= 0x80;
        }
        if copy {
            ctrl |= 0x40;
        }
        Self(((addr & 0x1C000) >> 14) | ((addr & 0x3FFF) << 16) | ctrl)
    }

    #[inline]
    pub const fn new_read(addr: Address, dma: bool, copy: bool) -> Self {
        let mut ctrl = match addr {
            Address::VRAM(_) => 0x00000000,
            Address::CRAM(_) => 0x00000020,
            Address::VSRAM(_) => 0x00000010,
        };
        let addr = addr.byte_addr();
        if dma {
            ctrl |= 0x80;
        }
        if copy {
            ctrl |= 0x40;
        }
        Self(((addr & 0x1C000) >> 14) | ((addr & 0x3FFF) << 16) | ctrl)
    }
}

#[repr(align(4))]
union DataPort {
    pub word: u16,
    pub long: u32,
    pub pair: [u16; 2]
}

#[repr(align(4))]
union CtrlPort {
    status: Status,
    reg_cmd: SetRegCmd,
    reg_cmds: [SetRegCmd; 2],
    addr_cmd: SetAddrCmd,
}

#[repr(C)]
pub struct VDPIO {
    data: DataPort,
    ctrl: CtrlPort,
    hvctr: HVCounter,
}

impl VDPIO {

    pub unsafe fn steal() -> &'static mut Self {
        &mut *(0xC00000usize as *mut Self)
    }

    pub fn status(&mut self) -> Status {
        unsafe { ptr::read_volatile(&raw const self.ctrl.status) }
    }

    pub fn hv_counter(&self) -> HVCounter {
        unsafe { ptr::read_volatile(&raw const self.hvctr) }
    }

    pub unsafe fn set_register(&mut self, cmd: SetRegCmd) {
        ptr::write_volatile(&raw mut self.ctrl.reg_cmd, cmd);
    }

    pub unsafe fn set_registers(&mut self, cmds: [SetRegCmd; 2]) {
        ptr::write_volatile(&raw mut self.ctrl.reg_cmds, cmds);
    }

    pub unsafe fn set_address(&mut self, cmd: SetAddrCmd) {
        ptr::write_volatile(&raw mut self.ctrl.addr_cmd, cmd);
    }

    pub unsafe fn set_dma_address(&mut self, cmd: SetAddrCmd) {
        let scratch = mem::MaybeUninit::new(cmd);
        ptr::write_volatile(&raw mut self.ctrl.addr_cmd, ptr::read_volatile(scratch.as_ptr()));
    }

    pub unsafe fn write_word(&mut self, word: u16) {
        ptr::write_volatile(&raw mut self.data.word, word);
    }

    pub unsafe fn write_long(&mut self, long: u32) {
        ptr::write_volatile(&raw mut self.data.long, long);
    }

    pub unsafe fn write_words(&mut self, words: [u16; 2]) {
        ptr::write_volatile(&raw mut self.data.pair, words);
    }

    pub unsafe fn read_word(&mut self) -> u16 {
        ptr::read_volatile(&raw mut self.data.word)
    }

    pub unsafe fn read_long(&mut self) -> u32 {
        ptr::read_volatile(&raw mut self.data.long)
    }

    pub unsafe fn read_words(&mut self) -> [u16; 2] {
        ptr::read_volatile(&raw mut self.data.pair)
    }
}

pub struct VDPState {
    mode: Mode,
    sprites_base: u8,
    plane_a_base: u8,
    plane_b_base: u8,
    window_base: u8,
    hscroll_base: u8,
    plane_size: PlaneSize,
    window_x_clip: WindowClip,
    window_y_clip: WindowClip,
    background_color: u8,
    hint_interval: u8, 
}


