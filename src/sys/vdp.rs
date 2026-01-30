use super::prelude::*;

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
    pub const fn from_tile_index(index: u16) -> Self {
        Self((index & 0x7FF) << 4)
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
    pub const fn tile_index(self) -> u16 {
        self.0 >> 4
    }

}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CRAMAddress(u8);

impl CRAMAddress {
    #[inline]
    pub const fn from_byte_addr(addr: u8) -> Self {
        Self(addr >> 1)
    }

    #[inline]
    pub const fn from_word_addr(addr: u8) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn byte_addr(self) -> u8 {
        self.0 << 1
    }

    #[inline]
    pub const fn word_addr(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn from_line_index(line: u8, index: u8) -> Self {
        Self::from_word_addr(((line & 0x3) << 4) | (index & 0xF))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    VRAM(VRAMAddress),
    CRAM(CRAMAddress),
    VSRAM(u8),
}

impl Address {
    #[inline]
    pub const fn byte_addr(self) -> u32 {
        match self {
            Address::VRAM(addr) => addr.byte_addr(),
            Address::CRAM(addr) => addr.byte_addr() as u32,
            Address::VSRAM(addr) => (addr << 1) as u32,
        }
    }

    #[inline]
    pub const fn word_addr(self) -> u16 {
        match self {
            Address::VRAM(addr) => addr.word_addr(),
            Address::CRAM(addr) => addr.word_addr() as u16,
            Address::VSRAM(addr) => addr as u16,
        }
    }
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
    pub fn from_raw_value(value: u8) -> Self {
        match value & 0x80 {
            0x00 => WindowClip::Before(value & 0x1F),
            0x80 => WindowClip::After(value & 0x1F),
            _ => unreachable!()
        }
    }

    pub fn raw_value(self) -> u8 {
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
    Invalid = 0b01,
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
    Size128x32 = 0b00_00_00_11,
    Size32x64 = 0b00_01_00_00,
    Size64x64 = 0b00_01_00_01,
    Size32x128 = 0b00_11_00_00,
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
pub struct TileFlags(pub u16);

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

unsafe impl bytemuck::TransparentWrapper<u16> for TileFlags {}
unsafe impl bytemuck::NoUninit for TileFlags {}
unsafe impl bytemuck::AnyBitPattern for TileFlags {}
unsafe impl bytemuck::Zeroable for TileFlags {}

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

unsafe impl bytemuck::NoUninit for Sprite {}
unsafe impl bytemuck::AnyBitPattern for Sprite {}
unsafe impl bytemuck::Zeroable for Sprite {}

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
    #[derive(Clone, Copy, PartialEq, Eq, Default)]
    pub struct Mode: u32 {
        /// If set, the video signal breaks somehow.
        const DISABLE_DISPLAY = 1 << 0;
        /// If set, the H/V counter is latched on external interrupts.
        const STOP_HV_ON_XINT = 1 << 1;
        /// If set, all 3 color bits are used. If clear, only the least significant bit is used.
        const FULL_COLOR_MODE = 1 << 2;
        /// If set, horizontal interrupts are enabled.
        const ENABLE_HINT = 1 << 4;
        /// If set, the leftmost 8 pixels are blanked.
        const BLANK_LEFT_COLUMN = 1 << 5;
        /// If set, mode 5 (normal MD operation) is enabled. If clear, mode 4 is enabled.
        const ENABLE_MODE5 = 1 << 10;
        /// If set, V30 mode is active.
        const V30_MODE = 1 << 11;
        /// If set, DMA transfers are enabled.
        const ENABLE_DMA = 1 << 12;
        /// If set, vertical interrupts are enabled.
        const ENABLE_VINT = 1 << 13;
        /// If set, display is enabled. If clear, display is filled with background color.
        const ENABLE_DISPLAY = 1 << 14;
        /// If set, extended VRAM mode is enabled. 
        /// 
        /// This breaks VDP operation on consoles with only 64 kb of VRAM, because the extra memory is addressed with the lowest address line.
        const EXTENDED_VRAM = 1 << 15;
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

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Register {
    ModeSet0 = 0x00,
    ModeSet1 = 0x01,
    PlaneAAddr = 0x02,
    WindowAddr = 0x03,
    PlaneBAddr = 0x04,
    SpriteAddr = 0x05,
    SpriteBase = 0x06,
    BackgroundColor = 0x07,
    HIntCounter = 0x0A,
    ModeSet2 = 0x0B,
    ModeSet3 = 0x0C,
    HScrollAddr = 0x0D,
    PlaneBase = 0x0E,
    AutoInc = 0x0F,
    PlaneSize = 0x10,
    WindowHClip = 0x11,
    WindowVClip = 0x12,
    DMALenL = 0x13,
    DMALenH = 0x14,
    DMAAddrL = 0x15,
    DMAAddrM = 0x16,
    DMAAddrH = 0x17,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WordCmd(pub u16);

impl WordCmd {
    #[inline]
    pub const fn set_reg(reg: Register, val: u8) -> Self {
        Self(0x8000 | ((((reg as u8) & 0x1F) as u16) << 8) | (val as u16))
    }
}

impl core::fmt::Debug for WordCmd {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{:04X}]", self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LongCmd(pub u32);

impl LongCmd {
    #[inline]
    pub const fn write_addr(addr: Address, dma: bool, copy: bool) -> Self {
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
    pub const fn read_addr(addr: Address, dma: bool, copy: bool) -> Self {
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

    #[inline]
    pub const fn merge(cmds: [WordCmd; 2]) -> Self {
        unsafe { mem::transmute(cmds) }
    }

    #[inline]
    pub const fn split(self) -> [WordCmd; 2] {
        unsafe { mem::transmute(self) }
    }
}

impl core::fmt::Debug for LongCmd {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let split = self.split();
        write!(f, "[{:04X}|{:04X}]", split[0].0, split[1].0)
    }
}

#[repr(align(4))]
union DataPort {
    single: u16,
    pair: [u16; 2]
}

#[repr(align(4))]
union CtrlPort {
    status: Status,
    reg_cmd: WordCmd,
    reg_cmds: [WordCmd; 2],
    addr_cmd: LongCmd,
}

#[repr(C)]
pub(super) struct VDPPort {
    data: DataPort,
    ctrl: CtrlPort,
    hvctr: HVCounter,
}

impl VDPPort {
    const BASE_ADDR: usize = 0xC00000;

    pub unsafe fn steal_mut() -> &'static mut Self {
        &mut *core::hint::black_box(Self::BASE_ADDR as *mut Self)
    }

    pub unsafe fn steal() -> &'static Self {
        &*core::hint::black_box(Self::BASE_ADDR as *const Self)
    }

    pub unsafe fn steal_mut_ptr() ->  *mut Self {
        core::hint::black_box(Self::BASE_ADDR as *mut Self)
    }

    pub unsafe fn steal_ptr() -> *const Self {
        core::hint::black_box(Self::BASE_ADDR as *const Self)
    }

    pub fn status(&self) -> Status {
        unsafe { (&raw const self.ctrl.status).read_volatile() }
    }

    pub fn hv_counter(&self) -> HVCounter {
        unsafe { (&raw const self.hvctr).read_volatile() }
    }

    pub unsafe fn execute_word(&mut self, cmd: WordCmd) {
        (&raw mut self.ctrl.reg_cmd).write_volatile(cmd);
    }

    pub unsafe fn execute_word_pair(&mut self, cmds: [WordCmd; 2]) {
        (&raw mut self.ctrl.reg_cmds).write_volatile(cmds);
    }

    pub unsafe fn execute_long(&mut self, cmd: LongCmd) {
        (&raw mut self.ctrl.addr_cmd).write_volatile(cmd);
    }

    pub unsafe fn execute_long_dma(&mut self, cmd: LongCmd) {
        let scratch = mem::MaybeUninit::new(cmd);
        (&raw mut self.ctrl.addr_cmd).write_volatile(scratch.as_ptr().read_volatile());
    }

    pub unsafe fn write_word(&mut self, word: u16) {
        (&raw mut self.data.single).write_volatile(word);
    }

    pub unsafe fn write_word_pair(&mut self, pair: [u16; 2]) {
        (&raw mut self.data.pair).write_volatile(pair);
    }

    pub unsafe fn read_word(&mut self) -> u16 {
        (&raw mut self.data.single).read_volatile()
    }

    pub unsafe fn read_word_pair(&mut self) -> [u16; 2] {
        (&raw mut self.data.pair).read_volatile()
    }
}

pub struct VDPSettings {
    pub mode: Mode,
    pub plane_a_addr: VRAMAddress,
    pub plane_b_addr: VRAMAddress,
    pub window_addr: VRAMAddress,
    pub sprites_addr: VRAMAddress,
    pub hscroll_addr: VRAMAddress,
    pub plane_size: PlaneSize,
    pub window_h_clip: WindowClip,
    pub window_v_clip: WindowClip,
    pub background_color: CRAMAddress,
    pub hint_interval: u8, 
}

#[derive(Default)]
struct VDPState {
    mode: [atomic::AtomicU8; 4],
    plane_a_addr: atomic::AtomicU8,
    plane_b_addr: atomic::AtomicU8,
    window_addr:  atomic::AtomicU8,
    sprites_addr: atomic::AtomicU8,
    hscroll_addr: atomic::AtomicU8,
    plane_size: atomic::AtomicU8,
    window_h_clip: atomic::AtomicU8,
    window_v_clip: atomic::AtomicU8,
    background_color: atomic::AtomicU8,
    hint_interval: atomic::AtomicU8, 
}

impl VDPState {
    #[inline]
    pub fn from_settings(settings: VDPSettings) -> Self {
        let this = Self::default();
        this.replace_mode(settings.mode);
        this.set_plane_a_addr(settings.plane_a_addr);
        this.set_plane_b_addr(settings.plane_b_addr);
        this.set_sprites_addr(settings.sprites_addr);
        this.set_window_addr(settings.window_addr);
        this.set_hscroll_addr(settings.hscroll_addr);
        this.set_plane_size(settings.plane_size);
        this.set_window_h_clip(settings.window_h_clip);
        this.set_window_v_clip(settings.window_v_clip);
        this.set_background_color(settings.background_color);
        this.set_hint_interval(settings.hint_interval);

        this
    }

    #[inline]
    pub fn into_settings(self) -> VDPSettings {
        VDPSettings {
            mode: self.mode(),
            sprites_addr: self.sprites_addr(),
            plane_a_addr: self.plane_a_addr(),
            plane_b_addr: self.plane_b_addr(),
            window_addr: self.window_addr(),
            hscroll_addr: self.hscroll_addr(),
            plane_size: self.plane_size(),
            window_h_clip: self.window_h_clip(),
            window_v_clip: self.window_v_clip(),
            background_color: self.background_color(),
            hint_interval: self.hint_interval(),
        }
    }

    #[inline]
    pub fn mode(&self) -> Mode {
        Mode::from_bits_retain(u32::from_be_bytes(self.mode.each_ref().map(|byte| byte.load(atomic::Ordering::Relaxed))))
    }

    #[inline]
    pub fn mode_bytes(&self) -> [u8; 4] {
        self.mode.each_ref().map(|byte| byte.load(atomic::Ordering::Relaxed))
    }

    #[inline]
    pub fn replace_mode(&self, mode: Mode) {
        let bits = mode.bits().to_be_bytes();
        bits.into_iter().zip(self.mode.iter()).for_each(|(val, dst): (u8, &atomic::AtomicU8)| {
            dst.store(val, atomic::Ordering::Relaxed);
        });
    }

    #[inline]
    pub fn plane_a_addr(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.plane_a_addr.load(atomic::Ordering::Relaxed) as u16) << 9)
    }

    #[inline]
    pub fn set_plane_a_addr(&self, addr: VRAMAddress) {
        self.plane_a_addr.store(((addr.word_addr() >> 9) as u8) & 0x78, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn plane_b_addr(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.plane_b_addr.load(atomic::Ordering::Relaxed) as u16) << 12)
    } 

    #[inline]
    pub fn set_plane_b_addr(&self, addr: VRAMAddress) {
        self.plane_b_addr.store(((addr.word_addr() >> 12) as u8) & 0xF, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn sprites_addr(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.sprites_addr.load(atomic::Ordering::Relaxed) as u16) << 8)
    }

    #[inline]
    pub fn set_sprites_addr(&self, addr: VRAMAddress) {
        self.sprites_addr.store(((addr.word_addr() >> 8) as u8) & 0xFF, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn window_addr(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.window_addr.load(atomic::Ordering::Relaxed) as u16) << 9)
    }

    #[inline]
    pub fn set_window_addr(&self, addr: VRAMAddress) {
        self.window_addr.store(((addr.word_addr() >> 9) as u8) & 0x7E, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn hscroll_addr(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.hscroll_addr.load(atomic::Ordering::Relaxed) as u16) << 9)
    }

    #[inline]
    pub fn set_hscroll_addr(&self, addr: VRAMAddress) {
        self.hscroll_addr.store(((addr.word_addr() >> 9) as u8) & 0x7F, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn plane_size(&self) -> PlaneSize {
        unsafe { core::mem::transmute(self.plane_size.load(atomic::Ordering::Relaxed)) }
    }

    #[inline]
    pub fn set_plane_size(&self, size: PlaneSize) {
        self.plane_size.store(size as u8, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn window_h_clip(&self) -> WindowClip {
        WindowClip::from_raw_value(self.window_h_clip.load(atomic::Ordering::Relaxed))
    }

    #[inline]
    pub fn set_window_h_clip(&self, clip: WindowClip) {
        self.window_h_clip.store(clip.raw_value(), atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn window_v_clip(&self) -> WindowClip {
        WindowClip::from_raw_value(self.window_v_clip.load(atomic::Ordering::Relaxed))
    }

    #[inline]
    pub fn set_window_v_clip(&self, clip: WindowClip) {
        self.window_v_clip.store(clip.raw_value(), atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn background_color(&self) -> CRAMAddress {
        CRAMAddress::from_word_addr(self.background_color.load(atomic::Ordering::Relaxed))
    }

    #[inline]
    pub fn set_background_color(&self, color: CRAMAddress) {
        self.background_color.store(color.word_addr(), atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn hint_interval(&self) -> u8 {
        self.hint_interval.load(atomic::Ordering::Relaxed)
    }

    #[inline]
    pub fn set_hint_interval(&self, interval: u8) {
        self.hint_interval.store(interval, atomic::Ordering::Relaxed);
    }

}

static STATE: cs::Mutex<cell::OnceCell<cell::RefCell<VDPState>>> = cs::Mutex::new(cell::OnceCell::new());

// cell::RefCell::new(VDPState { 
//     mode: Mode::from_bits_retain(0x81007404),
//     plane_a_addr: 0x30,
//     plane_b_addr: 0x07,
//     window_addr: 0x34,
//     sprites_addr: 0x78,
//     hscroll_addr: 0x3D,
//     window_h_clip: WindowClip::Before(0),
//     window_v_clip: WindowClip::Before(0),
//     plane_size: PlaneSize::Size64x32,
//     background_color: 0u8,
//     hint_interval: 0xFF,
// })

pub struct VDP;

impl VDP {
    #[inline]
    pub fn init(cs: cs::CriticalSection, settings: VDPSettings) -> Result<(), VDPSettings> {
        if io::version().revision() > 0 {
            const TMSS_REG: *mut u32 = 0xA14000 as _;
            const TMSS_VAL: u32 = 0x53454741u32; // "SEGA" as a single long
            unsafe {
                TMSS_REG.write_volatile(TMSS_VAL);   
            }
        }

        STATE.borrow(cs).set(core::cell::RefCell::new(VDPState::from_settings(settings))).map_err(|state| state.into_inner().into_settings())?;

        Self::borrow_mut(cs).apply_state();

        Ok(())
    }

    #[inline]
    pub fn borrow<'cs>(cs: cs::CriticalSection<'cs>) -> VDPRef<'cs> {
        VDPRef {
            state: STATE.borrow(cs).get().expect("VDP wasn't initialized!").borrow(),
            port: unsafe { VDPPort::steal_ptr() },
            phantom: core::marker::PhantomData
        }
    }

    #[inline]
    pub fn borrow_mut<'cs>(cs: cs::CriticalSection<'cs>) -> VDPRefMut<'cs> {
        VDPRefMut {
            state: STATE.borrow(cs).get().expect("VDP wasn't initialized!").borrow_mut(),
            port: unsafe { VDPPort::steal_mut_ptr() },
            phantom: core::marker::PhantomData
        }
    }
}

pub struct VDPRef<'cs> {
    port: *const VDPPort,
    state: core::cell::Ref<'cs, VDPState>,
    phantom: core::marker::PhantomData<&'cs VDPPort>,
}

pub struct VDPRefMut<'cs> {
    port: *mut VDPPort,
    state: core::cell::RefMut<'cs, VDPState>,
    phantom: core::marker::PhantomData<&'cs mut VDPPort>,
}

#[allow(private_interfaces)]
mod sealed_access {
    use super::*;

    pub trait SealedShared {
        fn port(&self) -> &VDPPort;
        fn state(&self) -> &VDPState;
    }

    pub trait SealedExclusive: SealedShared {
        fn port_mut(&self) -> &mut VDPPort;
        fn state_mut(&mut self) -> &mut VDPState;
    }

    impl<'cs> SealedShared for VDPRef<'cs> {
        fn port(&self) -> &VDPPort {
            unsafe { &*self.port }
        }
    
        fn state(&self) -> &VDPState {
            &self.state
        }
    }

    impl<'cs> SealedShared for VDPRefMut<'cs> {
        fn port(&self) -> &VDPPort {
            unsafe { &*self.port }
        }
    
        fn state(&self) -> &VDPState {
            &self.state
        }
    }

    impl<'cs> SealedExclusive for VDPRefMut<'cs> {
        fn port_mut(&self) -> &mut VDPPort {
            unsafe { &mut *self.port }
        }

        fn state_mut(&mut self) -> &mut VDPState {
            &mut self.state
        }
    }
}

pub trait SharedVDPAccess: sealed_access::SealedShared {
    #[inline]
    fn mode(&self) -> Mode {
        self.state().mode()
    }

    #[inline]
    fn plane_a_addr(&self) -> VRAMAddress {
        self.state().plane_a_addr()
    }

    #[inline]
    fn plane_b_addr(&self) -> VRAMAddress {
        self.state().plane_b_addr()
    }

    #[inline]
    fn sprites_addr(&self) -> VRAMAddress {
        self.state().sprites_addr()
    }

    #[inline]
    fn window_addr(&self) -> VRAMAddress {
        self.state().window_addr()
    }

    #[inline]
    fn hscroll_addr(&self) -> VRAMAddress {
        self.state().hscroll_addr()
    }

    #[inline]
    fn window_h_clip(&self) -> WindowClip {
        self.state().window_h_clip()
    }

    #[inline]
    fn window_v_clip(&self) -> WindowClip {
        self.state().window_v_clip()
    }

    #[inline]
    fn plane_size(&self) -> PlaneSize {
        self.state().plane_size()
    }

    #[inline]
    fn background_color(&self) -> CRAMAddress {
        self.state().background_color()
    }

    #[inline]
    fn hint_interval(&self) -> u8 {
        self.state().hint_interval()
    }

    #[inline]
    fn status(&self) -> Status {
        self.port().status()
    }

    #[inline]
    fn hv_counter(&self) -> HVCounter {
        self.port().hv_counter()
    }

    #[inline]
    fn plane_a_tile_addr(&self, x: u8, y: u8) -> vdp::VRAMAddress {
        self.state().plane_size().tile_offset_from(self.state().plane_a_addr(), x, y)
    }

    #[inline]
    fn plane_b_tile_addr(&self, x: u8, y: u8) -> vdp::VRAMAddress {
        self.state().plane_size().tile_offset_from(self.state().plane_b_addr(), x, y)
    }

    #[inline]
    fn window_tile_addr(&self, x: u8, y: u8) -> vdp::VRAMAddress {
        self.state().plane_size().tile_offset_from(self.state().window_addr(), x, y)
    }
}

impl<T: sealed_access::SealedShared> SharedVDPAccess for T {} 

pub trait ExclusiveVDPAccess: sealed_access::SealedExclusive + sealed_access::SealedShared + SharedVDPAccess {
    #[inline(never)]
    fn apply_state(&mut self) {
        unsafe {
            core::arch::asm!("nop");

            // Mode registers
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::ModeSet0, self.state().mode[3].load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::ModeSet1, self.state().mode[2].load(atomic::Ordering::Relaxed)),
            ]);
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::ModeSet2, self.state().mode[1].load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::ModeSet3, self.state().mode[0].load(atomic::Ordering::Relaxed)),
            ]);

            // Base addresses and plane size
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::PlaneAAddr, self.state().plane_a_addr.load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::PlaneBAddr, self.state().plane_b_addr.load(atomic::Ordering::Relaxed)),
            ]);
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::SpriteAddr, self.state().sprites_addr.load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::HScrollAddr, self.state().hscroll_addr.load(atomic::Ordering::Relaxed)),
            ]);
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::WindowAddr, self.state().window_addr.load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::PlaneSize, self.state().plane_size.load(atomic::Ordering::Relaxed)),
            ]);

            // Window clip
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::WindowHClip, self.state().window_h_clip.load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::WindowVClip, self.state().window_v_clip.load(atomic::Ordering::Relaxed)),
            ]);

            // Background color and horizontal interrupt interval
            self.port_mut().execute_word_pair([
                WordCmd::set_reg(Register::BackgroundColor, self.state().background_color.load(atomic::Ordering::Relaxed)),
                WordCmd::set_reg(Register::HIntCounter, self.state().hint_interval.load(atomic::Ordering::Relaxed)),
            ]);

            core::arch::asm!("nop");
        }
    }

    #[inline]
    fn set_mode(&mut self, mode: Mode) {
        const REGS: [Register; 4] = [Register::ModeSet3, Register::ModeSet2, Register::ModeSet1, Register::ModeSet0];
        let mode_bytes = mode.bits().to_be_bytes();
        self.state().mode.iter().zip(mode_bytes).zip(REGS).for_each(|((mode, value), reg): ((&atomic::AtomicU8, u8), Register)| {
            if mode.load(atomic::Ordering::Relaxed) ^ value != 0 {
                mode.store(value, atomic::Ordering::Relaxed);
                unsafe { self.port_mut().execute_word(WordCmd::set_reg(reg, value)); }
            }
        });
        // if self.state().mode_bytes() != mode_bytes {
        //     const MODE_MASK: u32 = 0xFF_0F_FC_37; // The bits that actually do stuff.

        //     let mask = (self.state().mode ^ mode) & Mode::from_bits_retain(MODE_MASK);

        //     self.state_mut().mode ^= mask;

        //     let mask_bytes = mask.bits().to_be_bytes();
        //     let mode_bytes = self.state().mode.bits().to_be_bytes();
        //     const REGS: [Register; 4] = [Register::ModeSet3, Register::ModeSet2, Register::ModeSet1, Register::ModeSet0];

        //     mask_bytes.iter().copied().enumerate().filter_map(|(i, mask)| (mask != 0).then(|| (REGS[i], mode_bytes[i]))).for_each(|(reg, value)| {
        //         unsafe { self.port_mut().execute_word(WordCmd::set_reg(reg, value)); }
        //     });
        // }
    }

    #[inline]
    fn set_plane_a_addr(&mut self, addr: VRAMAddress) {
        self.state_mut().set_plane_a_addr(addr);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::PlaneAAddr, self.state().plane_a_addr.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_plane_b_addr(&mut self, addr: VRAMAddress) {
        self.state_mut().set_plane_b_addr(addr);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::PlaneBAddr, self.state().plane_b_addr.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_sprites_addr(&mut self, addr: VRAMAddress) {
        self.state_mut().set_sprites_addr(addr);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::SpriteAddr, self.state().sprites_addr.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_window_addr(&mut self, addr: VRAMAddress) {
        self.state_mut().set_window_addr(addr);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::WindowAddr, self.state().window_addr.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_hscroll_addr(&mut self, addr: VRAMAddress) {
        self.state_mut().set_hscroll_addr(addr);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::HScrollAddr, self.state().hscroll_addr.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_window_h_clip(&mut self, clip: WindowClip) {
        self.state_mut().set_window_h_clip(clip);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::WindowHClip, self.state().window_h_clip.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_window_v_clip(&mut self, clip: WindowClip) {
        self.state_mut().set_window_v_clip(clip);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::WindowVClip, self.state().window_v_clip.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_plane_size(&mut self, size: PlaneSize) {
        self.state_mut().set_plane_size(size);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::PlaneSize, self.state().plane_size.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_background_color(&mut self, color: CRAMAddress) {
        self.state_mut().set_background_color(color);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::BackgroundColor, self.state().background_color.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn set_hint_interval(&mut self, interval: u8) {
        self.state_mut().set_hint_interval(interval);
        unsafe { 
            self.port_mut().execute_word(
                WordCmd::set_reg(Register::HIntCounter, self.state().hint_interval.load(atomic::Ordering::Relaxed))
            ); 
        }
    }

    #[inline]
    fn stream(&mut self, address: Address, autoinc: impl Into<Option<u8>>) -> Stream<'_, Self> {
        Stream::new(self, address, autoinc)
    }

    #[inline]
    fn debug_writer(&mut self) -> DebugMessageWriter<'_, Self> {
        DebugMessageWriter(self)
    }

    #[inline]
    fn debug_halt(&mut self) {
        unsafe { self.port_mut().execute_word(WordCmd::set_reg(mem::transmute(29u8), 0)); }
    }
}

impl<T: sealed_access::SealedExclusive> ExclusiveVDPAccess for T {} 

pub struct Stream<'a, A: ExclusiveVDPAccess + ?Sized> {
    access: &'a mut A
}

impl<'a, A: ExclusiveVDPAccess + ?Sized> Stream<'a, A> {
    #[inline]
    fn new(access: &'a mut A, address: Address, autoinc: impl Into<Option<u8>>) -> Self {
        unsafe {
            if let Some(autoinc) = autoinc.into() {
                access.port_mut().execute_word(WordCmd::set_reg(Register::AutoInc, autoinc));
            }
            access.port_mut().execute_long(LongCmd::write_addr(address, false, false));
        }
        Self {
            access
        }
    }

    #[inline]
    pub fn write_word(&mut self, word: u16) {
        unsafe { self.access.port_mut().write_word(word); }
    }

    #[inline]
    pub fn write_word_pair(&mut self, words: [u16; 2]) {
        unsafe { self.access.port_mut().write_word_pair(words); }
    }

    #[inline]
    pub fn read_word(&mut self) -> u16 {
        unsafe { self.access.port_mut().read_word() }
    }

    #[inline]
    pub fn read_word_pair(&mut self) -> [u16; 2]{
        unsafe { self.access.port_mut().read_word_pair() }
    }

    #[inline]
    pub fn write_data<T: StreamData + ?Sized>(&mut self, data: impl AsRef<T>) {
        data.as_ref().into_stream(self);
    }

    #[inline]
    pub fn write_data_iter<T: StreamData>(&mut self, iter: impl IntoIterator<Item = T>) {
        for data in iter {
            data.into_stream(self);
        }
    }

    #[deprecated]
    pub fn write_words_iter(self, iter: impl IntoIterator<Item = u16>) {
        let mut iter = iter.into_iter();
        unsafe {
            loop {
                match (iter.next(), iter.next()) {
                    (None, None) => {
                        break;
                    },
                    (Some(word), None) => {
                        self.access.port_mut().write_word(word);
                        break;
                    },
                    (Some(high), Some(low)) => {
                        self.access.port_mut().write_word_pair([high, low]);
                    },
                    (None, Some(_)) => unreachable!(),
                }
            }
        }
    }
}

#[inline(always)]
const fn can_use_pair_copy<T: Sized + Copy + bytemuck::NoUninit>() -> bool {
    let size = size_of::<T>();
    let align = align_of::<T>();
    assert!(size & 1 != 0);
    assert!(align & 1 != 0);
    size & 2 == 0
}

pub trait StreamData: 'static {
    fn into_stream(&self, stream: &mut Stream<impl ExclusiveVDPAccess + ?Sized>);
}

impl<T: Copy + Sized + bytemuck::NoUninit + 'static> StreamData for T {
    fn into_stream(&self, stream: &mut Stream<impl ExclusiveVDPAccess + ?Sized>) {
        if const {
            let size = size_of::<T>();
            let align = align_of::<T>();
            assert!(size & 1 == 0);
            assert!(align & 1 == 0);
            size & 2 == 0
        } {
            let slice = bytemuck::cast_slice::<_, [u16; 2]>(core::slice::from_ref(self));
            for pair in slice {
                stream.write_word_pair(*pair);
            }
        } else {
            let slice = bytemuck::cast_slice::<_, u16>(core::slice::from_ref(self));
            let (pairs, extras) = slice.as_chunks::<2>();
            for pair in pairs {
                stream.write_word_pair(*pair);
            }
            for extra in extras {
                stream.write_word(*extra);
            }
        }
    }
}

impl<T: Copy + Sized + bytemuck::NoUninit> StreamData for [T] {
    fn into_stream(&self, stream: &mut Stream<impl ExclusiveVDPAccess + ?Sized>) {
        if const {
            let size = size_of::<T>();
            let align = align_of::<T>();
            assert!(size & 1 == 0);
            assert!(align & 1 == 0);
            size & 2 == 0
        } {
            let slice = bytemuck::cast_slice::<_, [u16; 2]>(self);
            for pair in slice {
                stream.write_word_pair(*pair);
            }
        } else {
            let slice = bytemuck::cast_slice::<_, u16>(self);
            let (pairs, extras) = slice.as_chunks::<2>();
            for pair in pairs {
                stream.write_word_pair(*pair);
            }
            for extra in extras {
                stream.write_word(*extra);
            }
        }
    }
}

pub struct DMATransfer<P: core::ops::Deref> {
    cmd: DMACommand<'static>,
    data: core::pin::Pin<P>,
}

impl<P: core::ops::Deref> DMATransfer<P> {
    pub fn new<T: StreamData + ?Sized + 'static>(
        data: core::pin::Pin<P>,
        dest: Address,
        autoinc: impl Into<Option<u8>>,
    ) -> Self where P: core::ops::Deref<Target = T> {
        Self { 
            cmd: unsafe {
                let data_ref = data.as_ref();
                DMACommand::new_transfer(
                    &*(&raw const *data_ref), 
                    dest, 
                    autoinc
                )
            }, 
            data
        }
    }

    pub fn data(&self) -> &P::Target {
        self.data.as_ref().get_ref()
    }

    pub fn into_parts<'a>(self) -> (DMACommand<'a>, core::pin::Pin<P>) where P: 'a {
        (self.cmd, self.data)
    }

    #[inline]
    pub fn try_execute<A: ExclusiveVDPAccess>(self, access: &mut A) -> Result<core::pin::Pin<P>, Self> {
        match self.cmd.try_execute(access) {
            Ok(()) => Ok(self.data),
            Err(_) => Err(self),
        }
    }

    #[inline]
    pub fn execute<A: ExclusiveVDPAccess>(self, access: &mut A) -> core::pin::Pin<P> {
        self.cmd.execute(access);
        self.data
    }
}


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DMACommand<'a> {
    cmds: [LongCmd; 4],
    phantom: core::marker::PhantomData<&'a ()>
}

impl<'a> DMACommand<'a> {
    #[inline(always)]
    pub fn new_transfer<T: StreamData + ?Sized>(
        data: &'a T,
        dest: Address,
        autoinc: impl Into<Option<u8>>,
    ) -> Self {
        let autoinc = autoinc.into().unwrap_or(2);
        let len = (size_of_val(data) >> 1) as u16;
        let addr = ((data as *const T).addr() >> 1) as u32;

        Self {
            cmds: [
                LongCmd::merge([
                    WordCmd::set_reg(Register::DMALenH, (len >> 8) as u8), 
                    WordCmd::set_reg(Register::DMALenL, len as u8)
                ]),
                LongCmd::merge([
                    WordCmd::set_reg(Register::AutoInc, autoinc), 
                    WordCmd::set_reg(Register::DMAAddrH, (addr >> 16) as u8)
                ]),
                LongCmd::merge([
                    WordCmd::set_reg(Register::DMAAddrM, (addr >> 8) as u8), 
                    WordCmd::set_reg(Register::DMAAddrL, addr as u8)
                ]),
                LongCmd::write_addr(dest, true, false)
            ],
            phantom: core::marker::PhantomData
        }
    }

    #[inline]
    pub fn length(&self) -> u16 {
        let [hi, lo] = self.cmds[0].split();
        (((hi.0 as u8) as u16) << 8) | ((lo.0 as u8) as u16)
    }

    #[inline]
    pub fn try_execute<A: ExclusiveVDPAccess>(self, access: &mut A) -> Result<(), Self> {
        if access.status().contains(Status::DMA_ACTIVE) {
            return Err(self);
        }
        unsafe {
            access.port_mut().execute_long(self.cmds[0]);
            access.port_mut().execute_long(self.cmds[1]);
            access.port_mut().execute_long(self.cmds[2]);
            if self.cmds[3].split()[0] == WordCmd(0) {
                access.port_mut().write_word(self.cmds[3].split()[1].0);
            } else {
                access.port_mut().execute_long(self.cmds[3]);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn execute<A: ExclusiveVDPAccess>(mut self, access: &mut A) {
        while let Err(this) = self.try_execute(access) {
            self = this;
        }
    }
}

impl DMACommand<'static> {
    #[inline(always)]
    pub fn new_fill(
        dst: VRAMAddress,
        len: usize,
        val: u8,
        autoinc: impl Into<Option<u8>>,
    ) -> Self {
        let autoinc = autoinc.into().unwrap_or(1);
        let len = len as u16;
        Self {
            cmds: [
                LongCmd::merge([
                    WordCmd::set_reg(Register::DMALenH, (len >> 8) as u8), 
                    WordCmd::set_reg(Register::DMALenL, len as u8)
                ]),
                LongCmd::merge([
                    WordCmd::set_reg(Register::AutoInc, autoinc), 
                    WordCmd::set_reg(Register::DMAAddrH, 0x80)
                ]),
                LongCmd::write_addr(Address::VRAM(dst), true, false),
                LongCmd::merge([
                    WordCmd(0), 
                    WordCmd((val as u16) << 8)
                ]),
            ],
            phantom: core::marker::PhantomData
        }
    }

    #[inline(always)]
    pub fn new_copy(
        src: VRAMAddress,
        dst: VRAMAddress,
        len: usize,
        autoinc: impl Into<Option<u8>>,
    ) -> Self {
        let autoinc = autoinc.into().unwrap_or(1);
        let addr = src.word_addr();
        let len = (len >> 1) as u16;
        Self {
            cmds: [
                LongCmd::merge([
                    WordCmd::set_reg(Register::DMALenH, (len >> 8) as u8), 
                    WordCmd::set_reg(Register::DMALenL, len as u8)
                ]),
                LongCmd::merge([
                    WordCmd::set_reg(Register::AutoInc, autoinc), 
                    WordCmd::set_reg(Register::DMAAddrH, 0xC0)
                ]),
                LongCmd::merge([
                    WordCmd::set_reg(Register::DMAAddrM, (addr >> 8) as u8), 
                    WordCmd::set_reg(Register::DMAAddrL, addr as u8)
                ]),
                LongCmd::write_addr(Address::VRAM(dst), true, true)
            ],
            phantom: core::marker::PhantomData
        }
    }
}

pub struct DebugMessageWriter<'a, A: ExclusiveVDPAccess + ?Sized>(&'a mut A);

impl<'a, A: ExclusiveVDPAccess + ?Sized> DebugMessageWriter<'a, A> {
    #[inline(always)]
    fn write_byte(&mut self, val: Option<NonZero<u8>>) {
        unsafe { self.0.port_mut().execute_word(WordCmd::set_reg(mem::transmute(30u8), val.map_or(0, NonZero::get))); }
    }
}

impl<'a, A: ExclusiveVDPAccess + ?Sized> Drop for DebugMessageWriter<'a, A> {
    fn drop(&mut self) {
        self.write_byte(None);
    }
}

impl<'a, A: ExclusiveVDPAccess + ?Sized> core::fmt::Write for DebugMessageWriter<'a, A> {
    #[inline(never)]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(match byte {
                0x00..0x20 => None,
                byte => NonZero::new(byte)
            });
        }
        Ok(())
    }
}

pub struct TileMessageWriter<'a, A: ExclusiveVDPAccess + ?Sized>(Stream<'a, A>, TileFlags);
