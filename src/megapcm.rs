#![allow(non_upper_case_globals)]
use super::sys::prelude::*;

// Constants for symbols in the MegaPCM sound driver.
const Z_MPCM_DriverReady: u16 = 0x1fc3;
const Z_MPCM_CommandInput: u16 = 0x1fc2;
const Z_MPCM_VolumeInput: u16 = 0x1fc4;
const Z_MPCM_SFXVolumeInput: u16 = 0x1fc5;
const Z_MPCM_PanInput: u16 = 0x1fc6;
const Z_MPCM_SFXPanInput: u16 = 0x1fc7;
const Z_MPCM_LoopId: u16 = 0x1fdd;
const Z_MPCM_ActiveSamplePitch: u16 = 0x1fdc;
const Z_MPCM_VBlankActive: u16 = 0x1fe2;
const Z_MPCM_CalibrationApplied: u16 = 0x1fe3;
const Z_MPCM_CalibrationScore_ROM: u16 = 0x1fe4;
const Z_MPCM_CalibrationScore_RAM: u16 = 0x1fe6;
const Z_MPCM_LastErrorCode: u16 = 0x1fe8;
const Z_MPCM_SampleTable: u16 = 0x1977;
const Z_MPCM_COMMAND_STOP: u8 = 0x1;
const Z_MPCM_COMMAND_PAUSE: u8 = 0x2;
const Z_MPCM_LOOP_IDLE: u8 = 0x1;
const Z_MPCM_LOOP_PAUSE: u8 = 0x2;
const Z_MPCM_LOOP_PCM: u8 = 0x10;
const Z_MPCM_LOOP_PCM_TURBO: u8 = 0x18;
const Z_MPCM_LOOP_DPCM: u8 = 0x20;
const Z_MPCM_LOOP_CALIBRATION: u8 = 0x80;
const Z_MPCM_ERROR__BAD_INTERRUPT: u8 = 0x2;
const Z_MPCM_ERROR__BAD_SAMPLE_TYPE: u8 = 0x1;
const Z_MPCM_ERROR__UNKNOWN_COMMAND: u8 = 0x80;

/// Loads the MegaPCM sound driver into the Z80's memory.
/// 
/// Implementation derived from `MegaPCM_LoadDriver`.
pub fn load_driver(cs: cs::CriticalSection) {
    static DRIVER_BINARY: &'static [u8] = include_bytes!("assets/megapcm.bin");
    z80::with_paused_z80(cs, true, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus[..DRIVER_BINARY.len() as u16].copy_bytes_from(DRIVER_BINARY);
    });

    // loop {
    //     // Waste time
    //     (0u16..0xFFF).into_iter().for_each(|i| { core::hint::black_box(i); });
        
    //     if z80::with_paused_z80(cs, false, |guard| {
    //         let bus = unsafe { z80::Z80Bus::new(guard) };
    //         bus.read_byte(Z_MPCM_DriverReady) != b'R'
    //     }) { break; }
    // }
}

/// Loads a single DPCM sample entry for the driver to see.
/// 
/// Stripped down version of `MegaPCM_LoadSampleTable`. 
/// 
/// Since we're using only one big sample, we don't need to check for file correctness or anything.
pub fn load_dpcm_sample(cs: cs::CriticalSection, data: &'static [u8], rate: u8) {
    let ptr_range = data.as_ptr_range();
    let start = ptr_range.start as usize;
    let end = ptr_range.end as usize;
    let [start_hi, start_lo]: [u16; 2] = unsafe { mem::transmute(start+start) };
    let [end_hi, end_lo]: [u16; 2] = unsafe { mem::transmute(end+end) };
    let start_bank = start_hi as u8;
    let end_bank = end_hi as u8;
    let [start_hi, start_lo]: [u8; 2] = (start_lo+1).rotate_right(1).to_ne_bytes();
    let [end_hi, end_lo]: [u8; 2] = (end_lo+1).rotate_right(1).to_ne_bytes();

    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus[Z_MPCM_SampleTable..Z_MPCM_SampleTable+9].copy_bytes_from(
            &[
                b'D', // Data type: DPCM
                0, // No flags
                rate, // Sample rate
                start_bank,
                end_bank,
                start_lo,
                start_hi,
                end_lo,
                end_hi,
            ]
        );
    });
}

pub fn start_sample(cs: cs::CriticalSection) {
    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus.write_byte(Z_MPCM_CommandInput, 0x81);
    });
}

pub fn stop_sample(cs: cs::CriticalSection) {
    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus.write_byte(Z_MPCM_CommandInput, Z_MPCM_COMMAND_STOP);
    });
}

pub fn pause_sample(cs: cs::CriticalSection) {
    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus.write_byte(Z_MPCM_CommandInput, Z_MPCM_COMMAND_PAUSE);
    });
}

pub fn unpause_sample(cs: cs::CriticalSection) {
    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus.write_byte(Z_MPCM_CommandInput, 0);
    });
}

pub fn set_volume(cs: cs::CriticalSection, volume: u8) {
    z80::with_paused_z80(cs, false, |guard| {
        let bus = unsafe { z80::Z80Bus::new(guard) };
        bus.write_byte(Z_MPCM_VolumeInput, volume);
    });
}