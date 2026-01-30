
#[repr(C)]
pub struct ROMHeader {
    pub system: [u8; 16],
    pub copyright: [u8; 16],
    pub domestic_title: [u8; 48],
    pub overseas_title: [u8; 48],
    pub serial_number: [u8; 14],
    pub rom_checksum: u16,
    pub device_support: [u8; 16],
    pub rom_start: usize,
    pub rom_end: usize,
    pub ram_start: usize,
    pub ram_end: usize,
    pub extra_memory: ExtraMemory,
    pub modem_support: [u8; 12],
    pub note: [u8; 40],
    pub region_support: [u8; 16]
}

#[macro_export]
macro_rules! define_rom_header {
    ($val:expr) => {
        #[used]
        #[link_section = ".boot.header"]
        pub static ROM_HEADER: crate::sys::header::ROMHeader = const {
            $val
        };
    };
}

const fn pad_spaces<const N: usize>(s: &[u8]) -> [u8; N] {
    assert!(s.len() <= N, "ROM header field is too big");
    let mut result = [0x20u8; N];
    result[..s.len()].copy_from_slice(s);
    result
}

impl ROMHeader {
    pub const DEFAULT: Self = Self {
        system: pad_spaces(b"SEGA"),
        copyright: pad_spaces(b"(C)"),
        domestic_title: pad_spaces(b""),
        overseas_title: pad_spaces(b""),
        serial_number: pad_spaces(b"GM 00000000-00"),
        rom_checksum: 0x0000,
        device_support: pad_spaces(b""),
        rom_start: 0x000000,
        rom_end: 0x3FFFFF,
        ram_start: 0xFF0000,
        ram_end: 0xFFFFFF,
        extra_memory: ExtraMemory { 
            magic: pad_spaces(b""), 
            kind: 0x20, 
            unknown: 0x20, 
            start: 0x20202020, 
            end: 0x20202020, 
        },
        modem_support: pad_spaces(b""),
        note: pad_spaces(b""),
        region_support: pad_spaces(b"JUE"),
    };

    pub const fn new() -> Self {
        Self::DEFAULT
    }

    pub const fn with_system(mut self, value: &[u8]) -> Self {
        self.system = pad_spaces(value);
        assert!(&self.system[..4] == b"SEGA", "system field of ROM header must start with 'SEGA' in order for TMSS to recognize the program!");
        self
    }

    pub const fn with_copyright(mut self, value: &[u8]) -> Self {
        self.copyright = pad_spaces(value);
        self
    }

    pub const fn with_domestic_title(mut self, value: &[u8]) -> Self {
        self.domestic_title = pad_spaces(value);
        self
    }

    pub const fn with_overseas_title(mut self, value: &[u8]) -> Self {
        self.overseas_title = pad_spaces(value);
        self
    }

    pub const fn with_serial_number(mut self, value: &[u8]) -> Self {
        self.serial_number = pad_spaces(value);
        self
    }

    pub const fn with_device_support(mut self, value: &[u8]) -> Self {
        self.device_support = pad_spaces(value);
        self
    }

    pub const fn with_modem_support(mut self, value: &[u8]) -> Self {
        self.modem_support = pad_spaces(value);
        self
    }

    pub const fn with_region_support(mut self, value: &[u8]) -> Self {
        self.region_support = pad_spaces(value);
        self
    }

    pub const fn with_note(mut self, value: &[u8]) -> Self {
        self.note = pad_spaces(value);
        self
    }

}

#[repr(C)]
pub struct ExtraMemory {
    pub magic: [u8; 2],
    pub kind: u8,
    pub unknown: u8,
    pub start: usize,
    pub end: usize,
}
