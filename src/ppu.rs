use Bus;

const PALETTE: [u32; 64] = [
    0x666666, 0x002A88, 0x1412A7, 0x3B00A4, 0x5C007E, 0x6E0040, 0x6C0600, 0x561D00,
    0x333500, 0x0B4800, 0x005200, 0x004F08, 0x00404D, 0x000000, 0x000000, 0x000000,
    0xADADAD, 0x155FD9, 0x4240FF, 0x7527FE, 0xA01ACC, 0xB71E7B, 0xB53120, 0x994E00,
    0x6B6D00, 0x388700, 0x0C9300, 0x008F32, 0x007C8D, 0x000000, 0x000000, 0x000000,
    0xFFFEFF, 0x64B0FF, 0x9290FF, 0xC676FF, 0xF36AFF, 0xFE6ECC, 0xFE8170, 0xEA9E22,
    0xBCBE00, 0x88D800, 0x5CE430, 0x45E082, 0x48CDDE, 0x4F4F4F, 0x000000, 0x000000,
    0xFFFEFF, 0xC0DFFF, 0xD3D2FF, 0xE8C8FF, 0xFBC2FF, 0xFEC4EA, 0xFECCC5, 0xF7D8A5,
    0xE4E594, 0xCFEF96, 0xBDF4AB, 0xB3F3CC, 0xB5EBF2, 0xB8B8B8, 0x000000, 0x000000,
];

pub struct PPU {
    // Cycle Counters
    pub cycle: u32,
    pub scan_line: u32,
    pub frame: u64,

    // Registers
    pub even_odd: bool,

    pub show_background: bool,

    pub tile_data: u64,
    pub palette_data: [u8; 32],
}

impl PPU {
    pub fn new() -> PPU {
        PPU {
            cycle: 0,
            scan_line: 0,
            frame: 0,
            even_odd: false,
            show_background: true,
            tile_data: 0,
            palette_data: [0; 32],
        }
    }

    pub fn fetch_tile_data(&self) -> u32 {
        (self.tile_data >> 32) as u32
    }

    pub fn background_pixel(&mut self) -> u8 {
        if self.show_background {
            let data = self.fetch_tile_data();
            (data & 0x0F) as u8
        } else {
            0
        }
    }

    pub fn read_palette(&self, address: u16) -> u8 {
        let address = if address >= 16 && address % 4 == 0 {
            address - 16
        } else {
            address
        };
        self.palette_data[address as usize]
    }

    pub fn render_pixel(&mut self, bus: &mut Bus) {
        let x = self.cycle - 1;
        let y = self.scan_line;
        let background = self.background_pixel();
        let color = background;
        let c = PALETTE[self.read_palette(color as u16 % 64) as usize];
        bus.ppu_back_buffer[((y * 256) + x) as usize] = c;
    }

    pub fn tick(&mut self) {
        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scan_line += 1;
            if self.scan_line > 261 {
                self.scan_line = 0;
                self.frame += 1;
                self.even_odd = !self.even_odd;
            }
        }
    }

    pub fn step(&mut self, bus: &mut Bus) {
        self.tick();
        let pre_line = self.scan_line == 261;
        let visible_line = self.scan_line < 240;
        let render_line = pre_line || visible_line;

        if visible_line  {
            self.render_pixel(bus);
        }
    }
}
