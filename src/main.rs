#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![feature(ptr_metadata)]
#![feature(bigint_helper_methods)]
#![feature(likely_unlikely)]
#![feature(const_option_ops)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(const_ops)]
#![feature(slice_ptr_get)]
#![feature(allocator_api)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(core_io_borrowed_buf)]
#![feature(const_array)]
#![feature(const_index)]
#![feature(const_cmp)]
#![feature(int_format_into)]
#![feature(slice_from_ptr_range)]

use crate::sys::{prelude::*};

extern crate alloc;

pub mod sys;
pub mod megapcm;
pub mod ba_video;

use alloc::vec::Vec;
use ba_video::*;

define_rom_header! {
    crate::sys::header::ROMHeader::new()
        .with_system(b"SEGA GENESIS") // Sorry, I'm American.
        .with_copyright(b"(C)SPPY 2026.JAN")
        .with_domestic_title(b"BAD APPLE!! 4MB DEMO")
        .with_overseas_title(b"BAD APPLE!! 4MB DEMO")
        .with_device_support(b"J6")
        .with_note(b"Bad Apple!! feat. nomico")
}

const TEXT_BASE_TILE: vdp::TileFlags = vdp::TileFlags::for_tile(0x450, 2).with_priority(true);
static FONT_DATA: &[vdp::Tile] = include_as!(vdp::Tile, "assets/font4bpp.bin");

static TEXT_PALETTE: &[u16] = &[
    0x0000, 0xFF00, 0xF0F0, 0xF00F, 0xFFF0, 0xFF0F, 0xF0FF,
    0xF800, 0xF080, 0xF008, 0xF880, 0xF808, 0xF088, 0x0666, 0x0BBB, 0x0888,
];

static FRAME_COUNTER: atomic::AtomicU16 = atomic::AtomicU16::new(0);
static CYCLE_COUNTER: atomic::AtomicU8 = atomic::AtomicU8::new(0);
static VSCROLL_BASE: atomic::AtomicI16 = atomic::AtomicI16::new(0);
static NO_PEND_DMA: atomic::AtomicBool = atomic::AtomicBool::new(false);
static PAUSED: atomic::AtomicBool = atomic::AtomicBool::new(false);
static USE_DEBUG_PAL: atomic::AtomicBool = atomic::AtomicBool::new(false);

static CTRL_P1: io::ControllerState<io::Player1> = io::ControllerState::new(io::Player1);
static CTRL_P2: io::ControllerState<io::Player2> = io::ControllerState::new(io::Player2);

const fn jitter_hscroll<const SIGN: bool>(line: usize) -> [i16; 2] {
    if SIGN {
        [(line & 1) as i16, 0]
    } else {
        [(!line & 1) as i16, 0]
    }
}

const HSCROLL_LINES: [[[i16; 2]; 8]; 2] = [
    core::array::from_fn(jitter_hscroll::<false>),
    core::array::from_fn(jitter_hscroll::<true>),
];

#[allow(static_mut_refs)]
extern "C" fn vint_handler() {
    with_cs(|cs| {
        CTRL_P1.update(cs);
        CTRL_P2.update(cs);

        if CTRL_P1.pressed(io::Button::C) || CTRL_P2.pressed(io::Button::C) {
            USE_DEBUG_PAL.store(!USE_DEBUG_PAL.load(atomic::Ordering::Relaxed), atomic::Ordering::Relaxed);
        }

        if CTRL_P1.pressed(io::Button::Start) || CTRL_P2.pressed(io::Button::Start) {
            let paused = PAUSED.load(atomic::Ordering::Relaxed);
            if paused {
                megapcm::unpause_sample(cs);
            } else {
                megapcm::pause_sample(cs);
            }
            PAUSED.store(!paused, atomic::Ordering::Relaxed);
        }

        if PAUSED.load(atomic::Ordering::Relaxed) { return; }
        
        let mut vdp = vdp::VDP::borrow_mut(cs);

        let cycle_index = CYCLE_COUNTER.load(atomic::Ordering::Acquire);
        if cycle_index <= 7 {

            if cycle_index & 1 == 0 {
                let palette = if USE_DEBUG_PAL.load(atomic::Ordering::Relaxed) {
                    &BAD_APPLE_DBG_PAL
                } else {
                    &BAD_APPLE_PALETTES[((cycle_index >> 1) & 0x3) as usize]
                };
                vdp.stream(vdp::Address::CRAM(vdp::CRAMAddress::from_line_index(0, 0)), None)
                    .write_data(palette.as_slice());
            }

            let frame_count = FRAME_COUNTER.load(atomic::Ordering::Relaxed);
            {
                let addr = vdp.plane_b_tile_addr(0, 1);
                print(
                    &mut vdp,
                    TEXT_BASE_TILE, 
                    addr, 
                    cycle_index.format_into(&mut core::fmt::NumBuffer::new())
                );
            }
            {
                let addr = vdp.plane_b_tile_addr(2, 1);
                print(
                    &mut vdp,
                    TEXT_BASE_TILE, 
                    addr, 
                    frame_count.format_into(&mut core::fmt::NumBuffer::new())
                );
            }
            FRAME_COUNTER.store(frame_count+1, atomic::Ordering::Relaxed);
        }
        CYCLE_COUNTER.store(cycle_index+1, atomic::Ordering::Release);

        let vscroll_base = VSCROLL_BASE.load(atomic::Ordering::Relaxed);
        vdp.stream(vdp::Address::VSRAM(0), None)
            .write_data([vscroll_base]);
        vdp.stream(vdp::Address::VRAM(vdp::VRAMAddress::from_tile_index(0x7A0)), None)
            .write_data(HSCROLL_LINES[(cycle_index & 1) as usize]);

        let mut dma_count = 0u8;
        let mut dma_size = None;
        while vdp.status().intersects(vdp::Status::VBLANK_ACTIVE) {
            if let Some(dma) = DMA_QUEUE.borrow_ref_mut(cs).pop_front() {
                NO_PEND_DMA.store(false, atomic::Ordering::Relaxed);
                let size = dma.length();
                dma_count += 1;
                *dma_size.get_or_insert(0u16) += size;
                dma.execute(&mut vdp);
                break;
            } else {
                NO_PEND_DMA.store(true, atomic::Ordering::Relaxed);
                break;
            };
        }

        {
            let addr = vdp.plane_b_tile_addr(0, 27);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                "DMA len:       "
            );
        }

        if let Some(size) = dma_size {
            
            let addr = vdp.plane_b_tile_addr(9, 27);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                (size as u16).format_into(&mut core::fmt::NumBuffer::new())
            );
        }
    });
}

/// Stretch the screen vertically by 2x.
#[unsafe(naked)]
extern "C" fn ba_hint_handler() {
    core::arch::naked_asm!(
        "move.l #{vdp_base},%a0", // Load VDP base address
        "move.l #{vscroll_base},%a1",
        "move.b (8,%a0),%d0", // Fetch vertical component of H/V counter
        "lsr.b #1,%d0", // Now in units of 2 scanlines (b/c 2x scale)
        "neg.b %d0", // Negate it
        "ext.w %d0", // Sign extend to word
        "add.w (%a1),%d0", // Add vscroll base offset (note: this IS an atomic load on vscroll_base)
        "move.w %d0,%d1", 
        // ".short 0x4841", // Machine code for "swap %d1"
        // "move.w %d0,%d1", // %d1 now contains two copies of %d0
        "move.l #0x40000010,(4,%a0)", // Set VDP address to start of VSRAM
        "move.w %d1,(%a0)", // Write both words to VSRAM
        "rts",
        vdp_base = const 0xC00000usize,
        vscroll_base = sym VSCROLL_BASE
    )
}

#[inline(never)]
fn print(vdp: &mut vdp::VDPRefMut, base: vdp::TileFlags, addr: vdp::VRAMAddress, text: impl AsRef<[u8]>) {
    vdp.stream(vdp::Address::VRAM(addr), None).write_data_iter( text.as_ref().iter().map(|ch| {
        vdp::TileFlags(base.0 + *ch as u16)
    }));
}

const FRAMERATES: &[&str] = &[
    "FPS: 30.0 ",
    "FPS: 26.7 ",
    "FPS: 24.0 ",
    "FPS: 21.8 ",
    "FPS: 20.0 ",
    "FPS: 18.5 ",
    "FPS: 17.1 ",
    "FPS: 16.0 ",
    "FPS: 15.0 ",
    "FPS: 14.1 ",
    "FPS: 13.3 ",
    "FPS: 12.6 ",
    "FPS: 12.0 ",
    "FPS: 11.4 ",
    "FPS: 10.9 ",
    "FPS: 10.4 ",
    "FPS: 10.0 ",
];

#[no_mangle]
pub fn main() {

    with_cs(|cs| {
        vdp::VDP::init(cs, vdp::VDPSettings {
            mode: (
                vdp::Mode::FULL_COLOR_MODE |
                vdp::Mode::ENABLE_MODE5 |
                vdp::Mode::ENABLE_DMA |
                vdp::Mode::ENABLE_VINT |
                vdp::Mode::ENABLE_DISPLAY |
                vdp::Mode::LINE_SCROLL |
                // vdp::Mode::ROW_SCROLL |
                vdp::Mode::ENABLE_DISPLAY |
                vdp::Mode::H40_MODE
            ),
            sprites_addr: vdp::VRAMAddress::from_byte_addr(0xF000),
            plane_a_addr: vdp::VRAMAddress::from_byte_addr(0xC000),
            plane_b_addr: vdp::VRAMAddress::from_byte_addr(0xE000),
            window_addr: vdp::VRAMAddress::from_byte_addr(0xD000),
            hscroll_addr: vdp::VRAMAddress::from_byte_addr(0xF400),
            plane_size: vdp::PlaneSize::Size64x64,
            window_h_clip: vdp::WindowClip::Before(0),
            window_v_clip: vdp::WindowClip::Before(0),
            background_color: vdp::CRAMAddress::from_line_index(0, 0),
            hint_interval: 0xFF,
        }).map_err(|_| ()).unwrap();

        CTRL_P1.init(cs);
        CTRL_P2.init(cs);

        let mut vdp = vdp::VDP::borrow_mut(cs);
        vdp::DMACommand::new_fill(vdp::VRAMAddress::from_word_addr(0), 0x10000, 0, None).execute(&mut vdp);

        megapcm::load_driver(cs);
        megapcm::load_dpcm_sample(cs, BAD_APPLE_AUDIO, BAD_APPLE_AUDIO_RATE);
        megapcm::set_volume(cs, 15);

        while vdp.status().contains(vdp::Status::DMA_ACTIVE) {
            interrupts::wait_for_vint();
        }
    });
    
    with_cs(|cs| {
        let mut vdp = vdp::VDP::borrow_mut(cs);

        vdp::DMACommand::new_transfer(
            &BAD_APPLE_SOLIDS, 
            vdp::Address::VRAM(vdp::VRAMAddress::from_tile_index(0)), 
            None
        ).execute(&mut vdp);
        vdp::DMACommand::new_transfer(
            FONT_DATA, 
            vdp::Address::VRAM(vdp::VRAMAddress::from_tile_index(TEXT_BASE_TILE.tile_index())), 
            None
        ).execute(&mut vdp);
        vdp::DMACommand::new_transfer(
            &BAD_APPLE_DBG_PAL, 
            vdp::Address::CRAM(vdp::CRAMAddress::from_line_index(1, 0)), 
            None
        ).execute(&mut vdp);
        vdp::DMACommand::new_transfer(
            TEXT_PALETTE, 
            vdp::Address::CRAM(vdp::CRAMAddress::from_line_index(2, 0)), 
            None
        ).execute(&mut vdp);

        megapcm::start_sample(cs);
    });

    
    interrupts::wait_for_vint();

    interrupts::set_vint_handler(Some(vint_handler));

    let video = BAVideoHeader::from_bytes(BAD_APPLE_VIDEO);
    let frames = video.iter();

    let mut odd_frame = false;

    let mut total_dropped_frames = 0u8;

    for frame in frames {

        with_cs(|cs| {
            let mut vdp = vdp::VDP::borrow_mut(cs);

            let addr = vdp.plane_b_tile_addr(0, 25);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                frame.used_shared_tiles_count.format_into(&mut core::fmt::NumBuffer::new())
            );

            let addr = vdp.plane_b_tile_addr(0, 26);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                frame.unique_tiles_count.format_into(&mut core::fmt::NumBuffer::new())
            );
        });

        let (tile_offset, plane_offset, vscroll_base) = if odd_frame {
            (0x230, 0x680, 0x100)
        } else {
            (0x10, 0x600, 0x000)
        };


        try_decode_frame(
            video,
            frame,
            vdp::VRAMAddress::from_tile_index(plane_offset),
            vdp::VRAMAddress::from_tile_index(tile_offset)
        );

        while !NO_PEND_DMA.load(atomic::Ordering::Relaxed) {
            interrupts::wait_for_vint();
        }

        while CYCLE_COUNTER.load(atomic::Ordering::Relaxed) < 8 {
            interrupts::wait_for_vint();
        }
        with_cs(|cs| {
            let mut vdp = vdp::VDP::borrow_mut(cs);

            let dropped_frames = CYCLE_COUNTER.load(atomic::Ordering::Relaxed)-8;
            total_dropped_frames += dropped_frames;

            let addr = vdp.plane_b_tile_addr(0, 0);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                FRAMERATES[dropped_frames as usize]
            );

            let addr = vdp.plane_b_tile_addr(0, 2);
            print(
                &mut vdp,
                TEXT_BASE_TILE, 
                addr, 
                total_dropped_frames.format_into(&mut core::fmt::NumBuffer::new())
            );
        });

        CYCLE_COUNTER.store(0, atomic::Ordering::Relaxed);
        VSCROLL_BASE.store(vscroll_base, atomic::Ordering::Relaxed);
        odd_frame = !odd_frame;

    }
    
    interrupts::set_vint_handler(None);

}
