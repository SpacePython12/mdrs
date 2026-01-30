use alloc::vec::Vec;

use crate::include_bytes_aligned_as;

use super::sys::prelude::*;

pub static BAD_APPLE_PALETTES: [[u16; 16]; 4] = [
    [0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE, 0x000, 0xEEE],
    [0x000, 0x000, 0xEEE, 0xEEE, 0x000, 0x000, 0xEEE, 0xEEE, 0x000, 0x000, 0xEEE, 0xEEE, 0x000, 0x000, 0xEEE, 0xEEE],
    [0x000, 0x000, 0x000, 0x000, 0xEEE, 0xEEE, 0xEEE, 0xEEE, 0x000, 0x000, 0x000, 0x000, 0xEEE, 0xEEE, 0xEEE, 0xEEE],
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0xEEE, 0xEEE, 0xEEE, 0xEEE, 0xEEE, 0xEEE, 0xEEE, 0xEEE],
];

pub static BAD_APPLE_DBG_PAL: [u16; 16] = [
    0x222, 0x228, 0x282, 0x822, 0x828, 0x288, 0x882, 0x888, 0xAAA, 0x22C, 0x2C2, 0xC22, 0xC2C, 0x2CC, 0xCC2, 0xCCC
];

pub static BAD_APPLE_SOLIDS: [vdp::Tile; 16] = [
    [u32::from_ne_bytes([0x00; 4]); 8],
    [u32::from_ne_bytes([0x11; 4]); 8],
    [u32::from_ne_bytes([0x22; 4]); 8],
    [u32::from_ne_bytes([0x33; 4]); 8],
    [u32::from_ne_bytes([0x44; 4]); 8],
    [u32::from_ne_bytes([0x55; 4]); 8],
    [u32::from_ne_bytes([0x66; 4]); 8],
    [u32::from_ne_bytes([0x77; 4]); 8],
    [u32::from_ne_bytes([0x88; 4]); 8],
    [u32::from_ne_bytes([0x99; 4]); 8],
    [u32::from_ne_bytes([0xAA; 4]); 8],
    [u32::from_ne_bytes([0xBB; 4]); 8],
    [u32::from_ne_bytes([0xCC; 4]); 8],
    [u32::from_ne_bytes([0xDD; 4]); 8],
    [u32::from_ne_bytes([0xEE; 4]); 8],
    [u32::from_ne_bytes([0xFF; 4]); 8],
];

pub static BAD_APPLE_VIDEO: &'static [u8] = include_bytes_aligned_as!(u16, "assets/bad_apple.bin");
pub static BAD_APPLE_AUDIO: &'static [u8] = include_bytes!("assets/bad_apple_audio.bin");
pub const BAD_APPLE_AUDIO_RATE: u8 = 161;

#[repr(C)]
pub struct BAVideoHeader {
    pub frame_count: u16,
    pub shared_tiles_count: u16,
    pub first_frame_offset: u32,
}

impl BAVideoHeader {
    pub fn from_bytes<'a>(bytes: &'a [u8]) -> &'a Self {
        unsafe { core::arch::asm!("nop") }
        unsafe {
            &*core::ptr::without_provenance(core::hint::black_box(bytes).as_ptr() as usize)
        }
    }

    #[inline]
    pub fn shared_tiles(&self) -> *const u32 {
        unsafe {
            (self as *const Self).add(1).cast::<u32>()
        }
    }

    #[inline]
    pub fn first_frame(&self) -> &BAFrameHeader {
        unsafe { core::arch::asm!("nop", "nop") }
        unsafe {
            &*(self as *const Self).add(1).byte_add(self.first_frame_offset as usize).cast::<BAFrameHeader>()
        }
    }

    #[inline]
    pub fn iter(&self) -> BAFrameIter<'_> {
        BAFrameIter { current_frame: self.first_frame(), frame_count: self.frame_count }
    }
}

#[repr(C)]
pub struct BAFrameHeader {
    pub used_shared_tiles_count: u16,
    pub unique_tiles_count: u16,
    pub cmp_unique_tiles_size: u16,
    pub cmp_plane_size: u16,
    pub unique_tiles_offset: u16,
    pub plane_offset: u16,
    pub next_frame_offset: u16,
}

impl BAFrameHeader {
    #[inline]
    pub fn used_shared_tiles(&self) -> *const u16 {
        unsafe {
            (self as *const Self).add(1).cast::<u16>()
        }
    }

    #[inline]
    pub fn cmp_unique_tiles(&self) -> *const u8 {
        unsafe {
            (self as *const Self).add(1).byte_add(self.unique_tiles_offset as usize).cast::<u8>()
        }
    }

    #[inline]
    pub fn cmp_plane(&self) -> &[u8] {
        unsafe {
            let ptr = (self as *const Self).add(1).byte_add(self.plane_offset as usize).cast::<u8>();
            core::slice::from_raw_parts(ptr, self.cmp_plane_size as usize)
        }
    }

    #[inline]
    pub fn buffer_size(&self) -> usize {
        ((self.used_shared_tiles_count as usize) << 1) + ((self.unique_tiles_count as usize) << 1) + 0x1000
    }

    #[inline]
    pub const fn plane_buffer_size(&self) -> usize {
        64*32*2
    }

    #[inline]
    pub fn shared_tiles_buffer_size(&self) -> usize {
        (self.used_shared_tiles_count as usize) << 1
    }

    #[inline]
    pub fn unique_tiles_buffer_size(&self) -> usize {
        (self.unique_tiles_count as usize) << 1
    }

    #[inline]
    pub fn next_frame(&self) -> &BAFrameHeader {
        unsafe { core::arch::asm!("nop", "nop", "nop") }
        unsafe {
            &*(self as *const Self).add(1).byte_add(self.next_frame_offset as usize).cast::<BAFrameHeader>()
        }
    }
}

pub struct BAFrameIter<'a> {
    current_frame: &'a BAFrameHeader,
    frame_count: u16,
}

impl<'a> Iterator for BAFrameIter<'a> {
    type Item = &'a BAFrameHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_count > 0 {
            let frame = self.current_frame;
            self.current_frame = frame.next_frame();
            self.frame_count -= 1;
            Some(frame)
        } else { None }
    }
}

// say that again
#[repr(C)]
struct FrameBuffer {
    pub tiles: [mem::MaybeUninit<vdp::Tile>; 0x220],
    pub plane: [[mem::MaybeUninit<vdp::TileFlags>; 64]; 32],
}

const FRAMEBUFFER_COUNT: usize = 2;

static mut FRAMEBUFFERS: cell::LazyCell<[alloc::boxed::Box<FrameBuffer>; FRAMEBUFFER_COUNT]> = cell::LazyCell::new(|| {
    core::array::from_fn(|_| {
        alloc::boxed::Box::new(FrameBuffer { 
            tiles: [mem::MaybeUninit::uninit(); _], 
            plane: [[mem::MaybeUninit::uninit(); _]; _] 
        })
    })
});

// static mut FRAMEBUFFERS: [FrameBuffer; FRAMEBUFFER_COUNT] = [
//     FrameBuffer { 
//         tiles: [mem::MaybeUninit::uninit(); _], 
//         plane: [[mem::MaybeUninit::uninit(); _]; _] 
//     },
//     FrameBuffer { 
//         tiles: [mem::MaybeUninit::uninit(); _], 
//         plane: [[mem::MaybeUninit::uninit(); _]; _] 
//     }
// ];
static USING_FRAMEBUFFER: atomic::AtomicBool = atomic::AtomicBool::new(false);
static FRAMEBUFFER_INDEX: atomic::AtomicBool = atomic::AtomicBool::new(true);

#[inline(never)]
fn next_framebuffer(_cs: cs::CriticalSection) -> Option<&'static mut FrameBuffer> {
    if !USING_FRAMEBUFFER.load(atomic::Ordering::Relaxed) {
        // USING_FRAMEBUFFER.store(true, atomic::Ordering::Relaxed);

        let index = FRAMEBUFFER_INDEX.load(atomic::Ordering::Relaxed);
        let fb: &'static mut FrameBuffer = unsafe {
            #[allow(static_mut_refs)]
            &mut core::hint::black_box(&mut FRAMEBUFFERS)[index as usize]
        };

        FRAMEBUFFER_INDEX.store(!index, atomic::Ordering::Relaxed);

        return Some(fb);
    }
    None
}

pub static DMA_QUEUE: cs::Mutex<cell::RefCell<heapless::Deque<vdp::DMACommand<'static>, 8>>> = cs::Mutex::new(cell::RefCell::new(heapless::Deque::new()));

// TODO: turn this into a queue and split large dmas >4kb
pub static PLANE_DMA: cs::Mutex<cell::Cell<Option<vdp::DMACommand<'static>>>> = cs::Mutex::new(cell::Cell::new(None));
pub static STILES_DMA: cs::Mutex<cell::Cell<Option<vdp::DMACommand<'static>>>> = cs::Mutex::new(cell::Cell::new(None));
pub static UTILES_DMA: cs::Mutex<cell::Cell<Option<vdp::DMACommand<'static>>>> = cs::Mutex::new(cell::Cell::new(None));

// Dedicated assembly functions for decompressing data
extern "C" {
    fn BAVideo_DecodePlane(buffer: *mut mem::MaybeUninit<vdp::TileFlags>, frame: *const BAFrameHeader, tile_offset: usize) -> *mut mem::MaybeUninit<vdp::TileFlags>;
    fn BAVideo_DecodeSharedTiles(buffer: *mut mem::MaybeUninit<vdp::Tile>, frame: *const BAFrameHeader, video: *const BAVideoHeader) -> *mut mem::MaybeUninit<vdp::Tile>;
    fn BAVideo_DecodeUniqueTiles(buffer: *mut mem::MaybeUninit<vdp::Tile>, frame: *const BAFrameHeader) -> *mut mem::MaybeUninit<vdp::Tile>;
}

pub fn try_decode_frame(
    video: &'static BAVideoHeader,
    frame: &'static BAFrameHeader,
    plane_dst: vdp::VRAMAddress,
    mut tiles_dst: vdp::VRAMAddress,
) -> bool {
    struct NoDivChunkIter<'a, T, const N: usize>(&'a [T]);

    impl<'a, T, const N: usize> Iterator for NoDivChunkIter<'a, T, N> {
        type Item = &'a [T];
    
        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                return None;
            }
            if let Some((chunk, rest)) = self.0.split_first_chunk::<N>() {
                self.0 = rest;
                Some(chunk)
            } else {
                let rest = self.0;
                self.0 = &[];
                Some(rest)
            }
        }
    }

    if let Some(buffer) = with_cs(|cs| {
        next_framebuffer(cs)
    }) {
        unsafe {
            let plane_buffer = buffer.plane.as_flattened_mut();
            let end = BAVideo_DecodePlane(
                plane_buffer.as_mut_ptr(), 
                frame, 
                tiles_dst.tile_index() as usize
            );
            // Assert that the function did not write into uninitialized memory.
            assert!(end as *const _ <= plane_buffer.as_ptr_range().end);
            let dma = vdp::DMACommand::new_transfer(
                buffer.plane.as_flattened().assume_init_ref(), 
                vdp::Address::VRAM(plane_dst), 
                None
            );
            with_cs(|cs| {
                DMA_QUEUE.borrow_ref_mut(cs).push_back(dma).expect("DMA queue full!");
                // PLANE_DMA.borrow(cs).set(Some(dma));
            });
        }

        unsafe {
            let (shared_buffer, rest) = buffer.tiles.split_at_mut(frame.used_shared_tiles_count as usize);
            if frame.used_shared_tiles_count > 0 {
                let end = BAVideo_DecodeSharedTiles(
                    shared_buffer.as_mut_ptr(), 
                    frame, 
                    video
                );
                assert!(end as *const _ <= shared_buffer.as_ptr_range().end);
                for chunk in NoDivChunkIter::<_, 256>(shared_buffer.assume_init_ref()) {
                    let dma = vdp::DMACommand::new_transfer(
                        chunk, 
                        vdp::Address::VRAM(tiles_dst), 
                        None
                    );
                    with_cs(|cs| {
                        DMA_QUEUE.borrow_ref_mut(cs).push_back(dma).expect("DMA queue full!");
                    });
                }
            }

            tiles_dst = vdp::VRAMAddress::from_tile_index(tiles_dst.tile_index() + frame.used_shared_tiles_count);

            let (unique_buffer, extra) = rest.split_at_mut(frame.unique_tiles_count as usize);
            if frame.unique_tiles_count > 0 {
                let end = BAVideo_DecodeUniqueTiles(
                    unique_buffer.as_mut_ptr(), 
                    frame
                );
                assert!(end as *const _ <= extra.as_ptr_range().end);
                for chunk in NoDivChunkIter::<_, 256>(unique_buffer.assume_init_ref()) {
                    let dma = vdp::DMACommand::new_transfer(
                        chunk, 
                        vdp::Address::VRAM(tiles_dst), 
                        None
                    );
                    with_cs(|cs| {
                        DMA_QUEUE.borrow_ref_mut(cs).push_back(dma).expect("DMA queue full!");
                    });
                }
            }
        }
        true
    } else { false }
}