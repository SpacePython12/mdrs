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

use alloc::vec::{self, Vec};
use fixed::types::{I8F8, I16F16, U8F8};

use crate::sys::{fixed::FixedCordicMath, io, vdp};

extern crate alloc;

pub mod sys;

const FONT_DATA: &[vdp::Tile] = include_tiles!("assets/font4bpp.bin");

const PALETTE: &[u16] = &[
    0xF000, 0xFF00, 0xF0F0, 0xF00F, 0xFFF0, 0xFF0F, 0xF0FF,
    0xF800, 0xF080, 0xF008, 0xF880, 0xF808, 0xF088, 0xF666, 0xFBBB, 0xFFFF,
];

const EARTH_PALETTE: &[u16] = include_bytes_aligned_as!(u16, "assets/earth_palette.bin");
const EARTH_TILES: &[vdp::Tile] = include_tiles!("assets/earth_tiles.bin");
const EARTH_LAYOUT: &[vdp::TileFlags] = include_bytes_aligned_as!(vdp::TileFlags, "assets/earth_layout.bin");

#[no_mangle]
pub fn main() -> ! {
    
    let mut settings = vdp::Settings::DEFAULT;
    settings.set_scroll_mode(vdp::HScrollMode::Screen, vdp::VScrollMode::Screen);
    settings.apply::<true>();

    vdp::DMACommand::new_fill(vdp::VRAMAddress::from_word_addr(0), 0x10000, 0, None).execute();

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

    vdp::DMACommand::new_transfer(
        EARTH_PALETTE, 
        vdp::Address::CRAM(32), 
        None,
    ).schedule().map_err(|_| ()).unwrap();
    vdp::DMACommand::new_transfer(
        EARTH_TILES, 
        vdp::Address::VRAM(vdp::VRAMAddress::from_tile_index(128)), 
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
            vdp::Writer::new(vdp::Address::VRAM(settings.plane_b_tile(0, y))).with_autoinc(Some(2)).write(MESSAGE_TILES.as_slice());
        }
    }

    let mut layout_buffer = EARTH_LAYOUT.to_vec();

    layout_buffer.iter_mut().for_each(|tile| {
        tile.set_tile_index(tile.tile_index() + 128);
        tile.set_palette(1);
    });

    for (i, row) in layout_buffer.as_chunks::<28>().0.iter().enumerate() {
        vdp::Writer::new(vdp::Address::VRAM(settings.plane_a_tile(6, i as u8))).with_autoinc(Some(2)).write(row);
    }
    
    let mut hscroll = 0i16;
    let mut vscroll = 0i16;
    let mut mode = vdp::HScrollMode::Screen;
    
    let mut hscroll_offset_buffer = Vec::with_capacity(224);

    let xscale = I8F8::from_num(28);
    let yscale = I8F8::from_num(32);

    hscroll_offset_buffer.extend((-112i16..112).into_iter().map(|i| {
        let x = I8F8::from_num(i) / xscale;
        let y = (x.sin() * yscale).to_num::<i16>();
        y
    }));

    loop {
        let p1 = &io::P1_CONTROLLER;

        if p1.held(io::ControllerButton::Left) {
            hscroll += 1;
        }
        if p1.held(io::ControllerButton::Right) {
            hscroll -= 1;
        }

        if p1.held(io::ControllerButton::Up) {
            vscroll -= 1;
        }
        if p1.held(io::ControllerButton::Down) {
            vscroll += 1;
        }

        if p1.pressed(io::ControllerButton::A) {
            mode = match mode {
                vdp::HScrollMode::Screen => vdp::HScrollMode::Invalid,
                vdp::HScrollMode::Invalid => vdp::HScrollMode::Rows,
                vdp::HScrollMode::Rows => vdp::HScrollMode::Lines,
                vdp::HScrollMode::Lines => vdp::HScrollMode::Screen,
            };
            settings.set_scroll_mode(mode, vdp::VScrollMode::Screen);
            settings.apply::<false>();
        }

        if mode == vdp::HScrollMode::Screen {
            vdp::Writer::new(vdp::Address::VRAM(settings.hscroll_base())).with_autoinc(2).write([hscroll, -hscroll]);
        } else {
            vdp::Writer::new(vdp::Address::VRAM(settings.hscroll_base())).with_autoinc(2).write_iter(hscroll_offset_buffer.iter().map(|offset| {
                let value = hscroll - *offset;
                [value, -value]
            }));
        }

        vdp::Writer::new(vdp::Address::VSRAM(0)).with_autoinc(2).write([vscroll, -vscroll]);

        vdp::VDP::wait_for_vblank(None);
    }
}
