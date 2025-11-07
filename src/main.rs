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

use core::num::NonZero;

use fixed::types::{I8F8, I16F16};

use crate::sys::{io, vdp};

extern crate alloc;

pub mod sys;

const FONT_DATA: &[vdp::Tile] = include_tiles!("assets/font4bpp.bin");

const PALETTE: &[u16] = &[
    0xF000, 0xFF00, 0xF0F0, 0xF00F, 0xFFF0, 0xFF0F, 0xF0FF,
    0xF800, 0xF080, 0xF008, 0xF880, 0xF808, 0xF088, 0xF666, 0xFBBB, 0xFFFF,
];

#[no_mangle]
pub fn main() -> ! {
    
    let mut settings = vdp::Settings::DEFAULT;
    settings.set_scroll_mode(vdp::HScrollMode::Screen, vdp::VScrollMode::Screen);
    settings.apply::<true>();

    vdp::DMACommand::new_fill(vdp::VRAMAddress::from_word_addr(0), 0x10000, 0, None).schedule().map_err(|_| ()).unwrap();

    vdp::VDP::wait_for_vblank(None);

    vdp::DMACommand::new_transfer(
        PALETTE, 
        vdp::Address::CRAM(0), 
        None,
    ).schedule().map_err(|_| ()).unwrap();
    vdp::DMACommand::new_transfer(
        FONT_DATA, 
        vdp::Address::VRAM(vdp::VRAMAddress::from_tile_index(0)), 
        None,
    ).schedule().map_err(|_| ()).unwrap();

    vdp::VDP::wait_for_vblank(None);

    {
        const MESSAGE: &'static [u8] = b"Hello World from Rust on a Sega Genesis!";
        const MESSAGE_LEN: usize = const { MESSAGE.len() };
        const MESSAGE_TILES: [vdp::TileFlags; 40] = core::hint::black_box(const {
            let mut tiles = const { [core::mem::MaybeUninit::<vdp::TileFlags>::uninit(); MESSAGE_LEN] };
            let mut i = 0usize;
            while i < MESSAGE_LEN {
                tiles[i].write(vdp::TileFlags::for_tile(MESSAGE[i] as u16, 0));
                i += 1;
            }
            unsafe { core::mem::MaybeUninit::array_assume_init(tiles) }
        });

        for y in 0..32u8 {
            vdp::Writer::new(vdp::Address::VRAM(settings.plane_a_tile(0, y))).with_autoinc(Some(2)).write(MESSAGE_TILES.as_slice());
        }
    }

    let mut hscroll = 0i16;
    let mut vscroll = 0i16;

    loop {
        let p1 = core::hint::black_box(sys::with_cs::<1, 7, _>(|cs| core::hint::black_box(io::P1_CONTROLLER.borrow(cs).get())));

        if p1.left() {
            hscroll += 1;
        }
        if p1.right() {
            hscroll -= 1;
        }

        if p1.up() {
            vscroll -= 1;
        }
        if p1.down() {
            vscroll += 1;
        }

        vdp::Writer::new(vdp::Address::VRAM(settings.hscroll_base())).with_autoinc(2).write([hscroll, hscroll]);

        vdp::Writer::new(vdp::Address::VSRAM(0)).with_autoinc(2).write([vscroll, vscroll]);

        vdp::VDP::wait_for_vblank(None);
    }
}
