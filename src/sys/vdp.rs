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
    FullScroll = 0,
    DoubleCellScroll = 1,
}

/// This enumeration is for configuring how horizontal scrolling works.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HScrollMode {
    #[default]
    FullScroll = 0b00,
    CellScroll = 0b10,
    LineScroll = 0b11,
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

#[macro_export]
macro_rules! include_tiles {
    ($path:literal) => {
        include_bytes_aligned_as!($crate::sys::vdp::Tile, $path)
    };
}

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

impl core::ops::DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.flags
    }
}

pub trait VRAMData: Send + Sync + 'static {
    fn as_words(&self) -> &[u16];

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        let (pairs, single) = self.as_words().as_chunks::<2>();
        (
            unsafe { core::slice::from_raw_parts(pairs.as_ptr() as *const [u16; 2], pairs.len()) },
            single.first()
        )
    }
}

impl VRAMData for u16 {
    #[inline]
    fn as_words(&self) -> &[u16] {
        core::slice::from_ref(self)
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (
            unsafe { core::slice::from_raw_parts((&raw const *self).cast::<[u16; 2]>(), 0) },
            Some(self)
        )
    }
}

impl VRAMData for [u16] {
    #[inline]
    fn as_words(&self) -> &[u16] {
        self
    }
}

impl VRAMData for i16 {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts((&raw const *self).cast::<u16>(), 1) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (
            unsafe { core::slice::from_raw_parts((&raw const *self).cast::<[u16; 2]>(), 0) },
            Some(unsafe { &*(&raw const *self).cast::<u16>() })
        )
    }
}

impl VRAMData for [i16] {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u16>(), self.len()) }
    }
}

impl VRAMData for TileFlags {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts((&raw const *self).cast::<u16>(), 1) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (
            unsafe { core::slice::from_raw_parts((&raw const *self).cast::<[u16; 2]>(), 0) },
            Some(unsafe { &*(&raw const *self).cast::<u16>() })
        )
    }
}

impl VRAMData for [TileFlags] {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u16>(), self.len()) }
    }
}

impl VRAMData for Tile {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts((&raw const *self).cast::<u16>(), 16) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (unsafe { core::slice::from_raw_parts((&raw const *self).cast::<[u16; 2]>(), 8) }, None)
    }
}

impl VRAMData for [Tile] {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u16>(), self.len() << 4) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<[u16; 2]>(), self.len() << 3) }, None)
    }
}

impl VRAMData for Sprite {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts((&raw const *self).cast::<u16>(), 4) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (unsafe { core::slice::from_raw_parts((&raw const *self).cast::<[u16; 2]>(), 2) }, None)
    }
}

impl VRAMData for [Sprite] {
    #[inline]
    fn as_words(&self) -> &[u16] {
        unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u16>(), self.len() << 2) }
    }

    #[inline]
    fn as_word_pairs(&self) -> (&[[u16; 2]], Option<&u16>) {
        (unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<[u16; 2]>(), self.len() << 1) }, None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status(u16);

impl Status {
    #[inline]
    pub fn is_pal(&self) -> bool {
        self.0 & 0x1 != 0
    }

    #[inline]
    pub fn dma_in_progress(&self) -> bool {
        self.0 & 0x2 != 0
    }

    #[inline]
    pub fn in_hblank(&self) -> bool {
        self.0 & 0x4 != 0
    }

    #[inline]
    pub fn in_vblank(&self) -> bool {
        self.0 & 0x8 != 0
    }

    #[inline]
    pub fn odd_interlace_frame(&self) -> bool {
        self.0 & 0x10 != 0
    }

    #[inline]
    pub fn sprite_collision(&self) -> bool {
        self.0 & 0x20 != 0
    }

    #[inline]
    pub fn sprite_limit_hit(&self) -> bool {
        self.0 & 0x40 != 0
    }

    #[inline]
    pub fn vint_occurred(&self) -> bool {
        self.0 & 0x80 != 0
    }

    #[inline]
    pub fn fifo_full(&self) -> bool {
        self.0 & 0x100 != 0
    }

    #[inline]
    pub fn fifo_empty(&self) -> bool {
        self.0 & 0x200 != 0
    }
}

macro_rules! flag_u32 {
    ($flag:expr,$value:expr) => {
        if $value { $flag } else { 0 }
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    mode: u32,
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

impl Default for Settings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Settings {
    pub const DEFAULT: Self = Self {
        mode: 0x81007404,
        plane_a_base: 0x30,
        plane_b_base: 0x07,
        window_base: 0x34,
        sprites_base: 0x78,
        hscroll_base: 0x3D,
        window_x_clip: WindowClip::Before(0),
        window_y_clip: WindowClip::Before(0),
        plane_size: PlaneSize::Size64x32,
        background_color: 0u8,
        hint_interval: 0xFF,
    };

    #[inline]
    pub(super) fn clear() {
        Self::DEFAULT.apply::<true>();
    }


    #[inline]
    pub fn current() -> Self {
        super::with_cs::<1, 7, _>(|cs| {
            GLOBAL_SETTINGS.borrow(cs).get()
        })
    }

    #[inline(never)]
    pub fn apply<const FORCE: bool>(self) {
        super::with_cs::<1, 7, _>(|cs| {
            let orig = GLOBAL_SETTINGS.borrow(cs).get();
        
            if FORCE || self.mode != orig.mode {
                const MODE_MASK: u32 = 0xFF_0F_FC_37; // The bits that actually do stuff.
    
                let mask = (self.mode ^ orig.mode) & MODE_MASK;
    
                if FORCE || mask & 0xFF != 0 {
                    VDP::set_register(0, self.mode as u8);
                }
        
                if FORCE || mask & 0xFF00 != 0 {
                    VDP::set_register(1, (self.mode >> 8) as u8);
                }
        
                if FORCE || mask & 0xFF0000 != 0 {
                    VDP::set_register(11, (self.mode >> 16) as u8);
                }
        
                if FORCE || mask & 0xFF000000 != 0 {
                    VDP::set_register(12, (self.mode >> 24) as u8);
                }
            }
    
            if FORCE || self.plane_a_base != orig.plane_a_base {
                VDP::set_register(2, self.plane_a_base);
            }
    
            if FORCE || self.plane_b_base != orig.plane_b_base {
                VDP::set_register(4, self.plane_b_base);
            }
    
            if FORCE || self.sprites_base != orig.sprites_base {
                VDP::set_register(5, self.sprites_base);
            }
    
            if FORCE || self.window_base != orig.window_base {
                VDP::set_register(3, self.window_base);
            }
    
            if FORCE || self.hscroll_base != orig.hscroll_base {
                VDP::set_register(13, self.hscroll_base);
            }
    
            if FORCE || self.plane_size != orig.plane_size {
                VDP::set_register(16, self.plane_size as u8);
            }
    
            if FORCE || self.window_x_clip != orig.window_x_clip {
                VDP::set_register(17, self.window_x_clip.raw_value());
            }
    
            if FORCE || self.window_y_clip != orig.window_y_clip {
                VDP::set_register(18, self.window_y_clip.raw_value());
            }
    
            if FORCE || self.background_color != orig.background_color {
                VDP::set_register(7, self.background_color);
            }
    
            if FORCE || self.hint_interval != orig.hint_interval {
                VDP::set_register(10, self.hint_interval);
            }
    
            GLOBAL_SETTINGS.borrow(cs).set(self);
        })
    }

    #[inline]
    pub fn modify_mode(&mut self, mode: u32, mask: u32) {
        self.mode = (self.mode & !mask) | (mode & mask)
    }

    #[inline]
    pub fn set_scroll_mode(&mut self, hscroll: HScrollMode, vscroll: VScrollMode) {
        self.modify_mode(((hscroll as u32) << 16) | ((vscroll as u32) << 18), 0x30000);
    }

    #[inline]
    pub fn set_interlace_mode(&mut self, mode: InterlaceMode) {
        self.modify_mode((mode as u32) << 25, 0x6000000);
    }

    #[inline] 
    pub fn set_background_color(&mut self, line: u8, index: u8) {
        self.background_color = ((line & 0x3) << 4) | (index & 0xF);
    }

    #[inline]
    pub fn enable_display(&mut self, enable: bool) {
        self.modify_mode(flag_u32!(0x4000, enable), 0x4000);
    }

    // #[inline]
    // pub fn enable_mode5(&mut self, enable: bool) {
    //     self.modify_mode(flag_u32!(0x400, enable), 0x400);
    // }

    #[inline]
    pub fn enable_interrupts(&mut self, vint: bool, hint: bool, xint: bool) {
        self.modify_mode(
            flag_u32!(0x2000, vint) | flag_u32!(0x10, hint) | flag_u32!(0x80000, xint), 
            0x82010
        );
    }

    #[inline]
    pub fn stop_hv_on_xint(&mut self, stop: bool) {
        self.modify_mode(flag_u32!(0x2, stop), 0x2);
    }

    #[inline]
    pub fn enable_dma(&mut self, enable: bool) {
        self.modify_mode(flag_u32!(0x1000, enable), 0x1000);
    }

    #[inline]
    pub fn enable_h40(&mut self, enable: bool) {
        self.modify_mode(flag_u32!(0x81000000, enable), 0x81000000);
    }

    #[inline]
    pub fn enable_v30(&mut self, enable: bool) {
        self.modify_mode(flag_u32!(0x800, enable), 0x800);
    }

    #[inline]
    pub fn enable_shadow_highlight(&mut self, enable: bool) {
        self.modify_mode(flag_u32!(0x8000000, enable), 0x8000000);
    }

    #[inline]
    pub fn set_hint_interval(&mut self, interval: u8) {
        self.hint_interval = interval;
    }

    #[inline]
    pub fn set_plane_a_base(&mut self, addr: VRAMAddress) {
        self.plane_a_base = ((addr.word_addr() >> 9) as u8) & 0x78;
    }

    #[inline]
    pub fn plane_a_base(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.plane_a_base as u16) << 9)
    }

    #[inline]
    pub fn set_plane_b_base(&mut self, addr: VRAMAddress) {
        self.plane_b_base = ((addr.word_addr() >> 12) as u8) & 0xF;
    }

    #[inline]
    pub fn plane_b_base(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.plane_b_base as u16) << 12)
    } 

    #[inline]
    pub fn set_sprites_base(&mut self, addr: VRAMAddress) {
        self.sprites_base = ((addr.word_addr() >> 8) as u8) & 0xFF;
    }

    #[inline]
    pub fn sprites_base(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.sprites_base as u16) << 8)
    }

    #[inline]
    pub fn set_window_base(&mut self, addr: VRAMAddress) {
        self.window_base = ((addr.word_addr() >> 9) as u8) & 0x7E;
    }

    #[inline]
    pub fn window_base(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.window_base as u16) << 9)
    }

    #[inline]
    pub fn set_hscroll_base(&mut self, addr: VRAMAddress) {
        self.hscroll_base = ((addr.word_addr() >> 9) as u8) & 0x7F;
    }

    #[inline]
    pub fn hscroll_base(&self) -> VRAMAddress {
        VRAMAddress::from_word_addr((self.hscroll_base as u16) << 9)
    }

    #[inline]
    pub fn set_plane_size(&mut self, size: PlaneSize) {
        self.plane_size = size;
    }

    #[inline]
    pub fn plane_size(&self) -> PlaneSize {
        self.plane_size
    }

    #[inline]
    pub fn set_window_clip(&mut self, x_clip: WindowClip, y_clip: WindowClip) {
        self.window_x_clip = x_clip;
        self.window_y_clip = y_clip;
    }

    #[inline] 
    pub fn window_x_clip(&self) -> WindowClip {
        self.window_x_clip
    }

    #[inline] 
    pub fn window_y_clip(&self) -> WindowClip {
        self.window_y_clip
    }

    #[inline]
    pub fn plane_a_tile(&self, x: u8, y: u8) -> VRAMAddress {
        self.plane_size.tile_offset_from(self.plane_a_base(), x, y)
    }

    #[inline]
    pub fn plane_b_tile(&self, x: u8, y: u8) -> VRAMAddress {
        self.plane_size.tile_offset_from(self.plane_b_base(), x, y)
    }

    #[inline]
    pub fn window_tile(&self, x: u8, y: u8) -> VRAMAddress {
        self.plane_size.tile_offset_from(self.window_base(), x, y)
    }
}

static GLOBAL_SETTINGS: cs::Mutex<cell::Cell<Settings>> = cs::Mutex::new(cell::Cell::new(Settings::DEFAULT));

const VDP_DATA_PORT: *mut () = 0xC00000 as _;
const VDP_CTRL_PORT: *mut () = 0xC00004 as _;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WordCmd(pub u16);

impl WordCmd {
    const NULL: Self = Self(0);

    #[inline]
    pub const fn set_reg(reg: u8, val: u8) -> Self {
        Self(0x8000 | (((reg & 0x1F) as u16) << 8) | (val as u16))
    }

    #[inline]
    pub fn execute(self) {
        unsafe {
            // core::arch::asm!(
            //     "move.w {cmd},({port})",
            //     cmd = in(reg_data) self.0,
            //     port = in(reg_addr) VDP_CTRL_PORT,
            // );
            ptr::write_volatile(VDP_CTRL_PORT as *mut u16, self.0);
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LongCmd(pub u32);

impl LongCmd {

    #[inline]
    pub const fn set_addr_w(addr: Address, dma: bool, copy: bool) -> Self {
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
    pub const fn set_addr_r(addr: Address, dma: bool, copy: bool) -> Self {
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
    pub const fn from_words(first: WordCmd, second: WordCmd) -> Self {
        Self(((first.0 as u32) << 16) | (second.0 as u32))
    }

    #[inline]
    pub fn execute(self) {
        unsafe {
            // core::arch::asm!(
            //     "move.l {cmd},({port})",
            //     cmd = in(reg_data) self.0,
            //     port = in(reg_addr) VDP_CTRL_PORT,
            // );
            ptr::write_volatile(VDP_CTRL_PORT as *mut u32, self.0);
        }
    }

    #[inline]
    pub fn execute_dma(self) {
        unsafe {
            // core::arch::asm!(
            //     "move.l {cmd},{scratch}",
            //     "move.l {scratch},({port})",
            //     cmd = in(reg_data) self.0,
            //     port = in(reg_addr) VDP_CTRL_PORT,
            //     scratch = sym SCRATCH,
            // );
            ptr::write_volatile(&raw mut LONG_CMD_SCRATCH, mem::MaybeUninit::new(self.0));
            ptr::write_volatile(VDP_CTRL_PORT as *mut u32, ptr::read_volatile(&raw const LONG_CMD_SCRATCH).assume_init());
        }
    }
}

static mut LONG_CMD_SCRATCH: mem::MaybeUninit<u32> = const { mem::MaybeUninit::uninit() };

#[derive(Clone)]
pub struct Writer(Address, Option<u8>);

impl Writer {
    #[inline]
    pub const fn new(addr: Address) -> Self {
        Self(addr, None)
    }

    #[inline]
    pub fn with_autoinc(mut self, autoinc: impl Into<Option<u8>>) -> Self {
        self.1 = autoinc.into();
        self
    }

    #[inline]
    fn begin(&self) {
        if let Some(autoinc) = self.1 {
            WordCmd::set_reg(0xF, autoinc).execute();
        }

        LongCmd::set_addr_w(self.0, false, false).execute();
    }

    #[inline]
    pub fn write<T: VRAMData + ?Sized>(self, data: impl AsRef<T>) {
        self.begin();
        unsafe {
            let (pairs, extra) = data.as_ref().as_word_pairs();
            for &pair in pairs {
                ptr::write_volatile(VDP_DATA_PORT as *mut [u16; 2], pair);
            }
            if let Some(&extra) = extra {
                ptr::write_volatile(VDP_DATA_PORT as *mut u16, extra);
            }
        }
    }

    #[inline]
    pub fn write_iter<T: VRAMData + ?Sized>(self, iter: impl IntoIterator<Item = impl AsRef<T>>) {
        self.begin();
        let mut iter = iter.into_iter();
        unsafe {
            let mut last_extra: Option<u16> = None;

            while let Some(data) = iter.next() {
                let (pairs, extra) = data.as_ref().as_word_pairs();
                if !pairs.is_empty() {
                    if let Some(last_extra) = last_extra.take() {
                        ptr::write_volatile(VDP_DATA_PORT as *mut u16, last_extra);
                    }
                }
                for &pair in pairs {
                    ptr::write_volatile(VDP_DATA_PORT as *mut [u16; 2], pair);
                }
                if let Some(&extra) = extra {
                    if let Some(last_extra) = last_extra.take() {
                        ptr::write_volatile(VDP_DATA_PORT as *mut [u16; 2], [last_extra, extra]);
                    } else {
                        last_extra.replace(extra);
                    }
                }
            }
        }
    }
}

pub struct VDP;

impl VDP {

    

    #[inline]
    #[deprecated]
    fn set_register(reg: u8, val: u8) {
        WordCmd::set_reg(reg, val).execute();
    }

    #[inline]
    #[deprecated]
    fn set_register_double(rega: u8, vala: u8, regb: u8, valb: u8) {
        LongCmd::from_words(WordCmd::set_reg(rega, vala), WordCmd::set_reg(regb, valb)).execute();
    }

    #[inline]
    #[deprecated]
    fn set_address_inner(addr: Address, dma: bool, copy: bool) {
        let cmd = LongCmd::set_addr_w(addr, dma, copy);
        if dma {
            cmd.execute_dma();
        } else {
            cmd.execute();
        }
    }

    #[inline]
    pub fn wait_for_vblank(handler: Option<fn(cs::CriticalSection)>) {
        fn null_handler(_cs: cs::CriticalSection) {}
        unsafe {
            Self::set_vint_handler(handler.unwrap_or(null_handler));
            Self::vint_wait();
        }
    }

    #[inline]
    unsafe fn set_vint_handler(handler: fn(cs::CriticalSection)) {
        // We use volatile reads to force the compiler to not optimize or reorder things.
        ptr::write_volatile(&raw mut VINT_HANDLER, Some(handler));
    }

    #[inline(never)]
    unsafe fn vint_wait() {
        while ptr::read_volatile(&raw const VINT_HANDLER).is_some() {
            core::hint::spin_loop();
        }
    }

    #[inline]
    pub fn status() -> Status {
        Status(unsafe {
            ptr::read_volatile(VDP_CTRL_PORT as *mut u16)
        })
    }

    #[inline]
    #[deprecated]
    pub fn write_data(data: u16) {
        unsafe {
            ptr::write_volatile(VDP_DATA_PORT as *mut u16, data);
        }
    }

    #[inline]
    pub fn write_tile_flags(tiles: &[TileFlags], addr: VRAMAddress, autoinc: Option<NonZero<u8>>) {
        if let Some(inc) = autoinc {
            WordCmd::set_reg(0xF, inc.get()).execute();
        }
        LongCmd::set_addr_w(Address::VRAM(addr), false, false).execute();
        let (pairs, single) = tiles.as_chunks::<2>();
        let mut i = 0u16;
        while i < pairs.len() as u16  {
            unsafe {
                ptr::write_volatile(VDP_DATA_PORT as *mut [TileFlags; 2], pairs[i as usize]);
            }
            i += 1
        }
        if let Some(single) = single.get(0) {
            unsafe {
                ptr::write_volatile(VDP_DATA_PORT as *mut TileFlags, *single);
            }
        }
    }

    #[inline]
    #[deprecated]
    pub fn set_address(addr: Address) {
        LongCmd::set_addr_w(addr, false, false).execute();
    }

    #[inline]
    #[deprecated]
    pub fn set_autoinc(inc: u8) {
        WordCmd::set_reg(15, inc).execute();
    }

    #[inline]
    pub fn debug_alert(message: &[u8]) {
        let (pairs, singles) = message.as_chunks::<2>();
        for pair in pairs {
            LongCmd::from_words(WordCmd::set_reg(30, pair[0]), WordCmd::set_reg(30, pair[1])).execute();
        }

        if let Some(single) = singles.get(0) {
            LongCmd::from_words(WordCmd::set_reg(30, *single), WordCmd::set_reg(30, 0)).execute();
        } else {
            WordCmd::set_reg(30, 0).execute();
        }
    }

    #[inline]
    pub fn debug_halt() {
        WordCmd::set_reg(29, 0).execute();
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DMACommand {
    cmds: [LongCmd; 4],
}

impl DMACommand {
    #[inline]
    pub fn new_transfer<T: VRAMData>(
        src: &[T],
        dst: Address,
        autoinc: Option<NonZero<u8>>,
    ) -> Self {
        let autoinc = autoinc.map_or(2, NonZero::get);
        let addr = (src.as_ptr().addr() >> 1) as u32;
        let len = ((src.len() * mem::size_of::<T>()) >> 1) as u16;
        let cmds = [
            LongCmd::from_words(WordCmd::set_reg(0x0F, autoinc), WordCmd::set_reg(0x17, (addr >> 16) as u8)),
            LongCmd::from_words(WordCmd::set_reg(0x16, (addr >> 8) as u8), WordCmd::set_reg(0x15, addr as u8)),
            LongCmd::from_words(WordCmd::set_reg(0x14, (len >> 8) as u8), WordCmd::set_reg(0x13, len as u8)),
            LongCmd::set_addr_w(dst, true, false)
        ];
        Self {
            cmds,
        }
    }

    #[inline]
    pub fn new_fill(
        dst: VRAMAddress,
        len: usize,
        val: u8,
        autoinc: Option<NonZero<u8>>,
    ) -> Self {
        let autoinc = autoinc.map_or(1, NonZero::get);
        let len = len as u16;
        let cmds = [
            LongCmd::from_words(WordCmd::set_reg(0x0F, autoinc), WordCmd::set_reg(0x17, 0x80)),
            LongCmd::from_words(WordCmd::set_reg(0x14, (len >> 8) as u8), WordCmd::set_reg(0x13, len as u8)),
            LongCmd::set_addr_w(Address::VRAM(dst), true, false),
            LongCmd::from_words(WordCmd::NULL, WordCmd((val as u16) << 8))
        ];
        Self {
            cmds
        }
    }

    #[inline]
    pub fn new_copy(
        src: VRAMAddress,
        dst: VRAMAddress,
        len: usize,
        autoinc: Option<NonZero<u8>>,
    ) -> Self {
        let autoinc = autoinc.map_or(1, NonZero::get);
        let addr = src.word_addr();
        let len = (len >> 1) as u16;
        let cmds = [
            LongCmd::from_words(WordCmd::set_reg(0x0F, autoinc), WordCmd::set_reg(0x17, 0xC0)),
            LongCmd::from_words(WordCmd::set_reg(0x16, (addr >> 8) as u8), WordCmd::set_reg(0x15, addr as u8)),
            LongCmd::from_words(WordCmd::set_reg(0x14, (len >> 8) as u8), WordCmd::set_reg(0x13, len as u8)),
            LongCmd::set_addr_w(Address::VRAM(dst), true, true)
        ];
        Self {
            cmds
        }
    }

    #[inline]
    pub fn schedule(self) -> Result<(), Self> {
        super::with_cs::<1, 7, _>(|cs| {
            DMA_QUEUE.borrow_ref_mut(cs).push_back(self)
        })
    }

    #[inline]
    pub fn execute(self) {
        unsafe {
            core::arch::asm!(
                "move.l ({cmds}),({ctrl})",
                "move.l (4,{cmds}),({ctrl})",
                "move.l (8,{cmds}),({ctrl})",
                "cmpi.w #0,(12,{cmds})",
                "beq  2f",
                "move.l (12,{cmds}),{scratch}",
                "move.l {scratch},({ctrl})",
                "bra  3f",
                "2:",
                "move.w (14,{cmds}),(-4,{ctrl})",
                "3:",
                cmds = in(reg_addr) &raw const self,
                ctrl = in(reg_addr) VDP_CTRL_PORT as *mut u32,
                scratch = sym LONG_CMD_SCRATCH,
            )
        }
    }
}

#[repr(C)]
struct DmaQueue<const N: usize> {
    head: u8,
    tail: u8,
    full: bool,
    data: [mem::MaybeUninit<DMACommand>; N]
}

impl<const N: usize> DmaQueue<N> {
    pub const INIT: Self = Self {
        head: 0,
        tail: 0,
        full: false,
        data: const { [mem::MaybeUninit::uninit(); N] },
    };

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head == self.tail && !self.full
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.full
    }

    #[inline]
    pub fn increment(&self, i: u8) -> u8 {
        unsafe {
            let out: u8;
            core::arch::asm!(
                "add.b  #1,{i}",
                "cmpi.b  #{N},{i}",
                "bne    2f",
                "move.b #0,{i}",
                "2:",
                i = inout(reg_data) i => out,
                N = const N,
            );
            out
        }
    }

    #[inline]
    pub fn decrement(&self, i: u8) -> u8 {
        unsafe {
            let out: u8;
            core::arch::asm!(
                "sub.b  #1,{i}",
                "bcc    2f",
                "move.b #{Nm1},{i}",
                "2:",
                i = inout(reg_data) i => out,
                Nm1 = const N-1,
            );
            out
        }
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<DMACommand> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.pop_front_unchecked() })
        }
    }

    #[inline]
    pub fn pop_back(&mut self) -> Option<DMACommand> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.pop_back_unchecked() })
        }
    }

    #[inline]
    pub fn push_front(&mut self, value: DMACommand) -> Result<(), DMACommand> {
        if self.is_full() {
            Err(value)
        } else {
            unsafe { self.push_front_unchecked(value) }
            Ok(())
        }
    }

    #[inline]
    pub fn push_back(&mut self, value: DMACommand) -> Result<(), DMACommand> {
        if self.is_full() {
            Err(value)
        } else {
            unsafe { self.push_back_unchecked(value) }
            Ok(())
        }
    }

    #[inline]
    pub unsafe fn pop_front_unchecked(&mut self) -> DMACommand {
        let index = self.head as usize;
        self.full = false;
        self.head = self.increment(self.head);
        self.data.get_unchecked_mut(index).assume_init_read()
    }

    #[inline]
    pub unsafe fn pop_back_unchecked(&mut self) -> DMACommand {
        self.full = false;
        self.tail = self.decrement(self.tail);
        self.data.get_unchecked_mut(self.tail as usize).assume_init_read()
    }

    #[inline]
    pub unsafe fn push_front_unchecked(&mut self, value: DMACommand) {
        let index = self.decrement(self.head) as usize;
        self.data.get_unchecked_mut(index).write(value);
        self.head = index as u8;
        if self.head == self.tail {
            self.full = true;
        }
    }

    #[inline]
    pub unsafe fn push_back_unchecked(&mut self, value: DMACommand) {
        self.data.get_unchecked_mut(self.tail as usize).write(value);
        self.tail = self.increment(self.tail);
        if self.head == self.tail {
            self.full = true;
        }
    }
}

static DMA_QUEUE: cs::Mutex<cell::RefCell<DmaQueue<32>>> = cs::Mutex::new(cell::RefCell::new(DmaQueue::INIT));

#[repr(C)]
struct VIntData {
    data: Option<ptr::NonNull<()>>,
    vtable: mem::MaybeUninit<ptr::DynMetadata<dyn FnOnce(cs::CriticalSection)>>
}

/// The static storage for the vertical interrupt handler. Should this be bounded by some kind of mutex? Yes. Do I care right now? No.
static mut VINT_HANDLER: Option<fn(cs::CriticalSection)> = None;

static mut HINT_HANDLER: Option<fn()> = None;

/// The vertical interrupt handler. 
/// 
/// This is called whenever the electron beam finishes the last scanline, and has entered the vertical blanking period.
#[no_mangle]
unsafe fn _vblank() {
    while !VDP::status().in_vblank() {
        core::hint::spin_loop();
    }

    super::with_cs::<1, 7, _>(|cs| {
        {
            let p1 = super::io::P1_CONTROLLER.borrow(cs);
            let p2 = super::io::P2_CONTROLLER.borrow(cs);
            p1.set(p1.get().update());
            p2.set(p2.get().update());
        }

        if VDP::status().dma_in_progress() {
            return;
        }

        let handler = ptr::read_volatile(&raw const VINT_HANDLER); // Read the handler pointer
        if let Some(handler) = handler {

            handler(cs);
            
            // Set handler to null to indicate vblank has happened
            ptr::write_volatile(&raw mut VINT_HANDLER, None);
        }
        let mut queue = DMA_QUEUE.borrow_ref_mut(cs);
        'queue_loop: loop {
            loop {
                let status = VDP::status();
                if !status.in_vblank() {
                    break 'queue_loop;
                }
                if !status.dma_in_progress() {
                    break;
                }
                core::arch::asm!("nop","nop","nop","nop"); // Waste a bunch of time
            }
            if let Some(cmd) = queue.pop_front() {
                cmd.execute();
            } else {
                break;
            }
        }
    });
}

#[no_mangle]
unsafe fn _hblank() {
    let handler = ptr::read_volatile(&raw const HINT_HANDLER);
    if let Some(handler) = handler {
        handler();
    }
}

#[no_mangle]
unsafe fn _extint() {
    
}