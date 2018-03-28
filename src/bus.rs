use cartridge::Cartridge;

pub const BUFFER_WIDTH: usize = 256;
pub const BUFFER_HEIGHT: usize = 240;

pub struct Bus {
    pub cartridge: Cartridge,
    pub ram: Vec<u8>,
    pub apu_registers: [u8; 22],
    pub ppu_name_table: [u8; 2048],
    pub ppu_palette: [u8; 32],
    pub ppu_oam: [u8; 256],
    pub ppu_pixels: Vec<u32>,
}

impl Bus {
    pub fn new(cartridge: Cartridge, ram: Vec<u8>) -> Bus {
        Bus {
            cartridge,
            ram,
            apu_registers: [0; 22],
            ppu_name_table: [0; 2048],
            ppu_palette: [0; 32],
            ppu_oam: [0; 256],
            ppu_pixels: vec![0; BUFFER_WIDTH * BUFFER_HEIGHT],
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000...0x1FFF => self.ram[(address % 0x2000) as usize],
            0x2000...0x3FFF => 0xCC, // TODO: self.ppu.read_register(0x2000 + address % 8)
            0x4000...0x4013 => 0xFF, // TODO: read from APU registers
            0x4014 => 0xCC,          // TODO: self.ppu.read_register(address)
            0x4015 => 0xFF,          // TODO: self.apu.read_register(address)
            0x4016 => 0xFF,          // TODO: self.controller1.read()
            0x4017 => 0xFF,          // TODO: self.controller2.read()
            0x4018...0x5FFF => 0xFF, // TODO: I/O registers
            0x6000...0xFFFF => self.mapper_read(address),
            _ => panic!("Invalid bus memory read at address {:04X}", address),
        }
    }

    pub fn read_16(&self, address: u16) -> u16 {
        (self.read(address + 1) as u16) << 8 | self.read(address) as u16
    }

    pub fn read_16_bug(&self, address: u16) -> u16 {
        let address_plus_one = (address & 0xFF00) | (address as u8).wrapping_add(1) as u16;
        let lo = self.read(address);
        let hi = self.read(address_plus_one);
        (hi as u16) << 8 | lo as u16
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000...0x1FFF => self.ram[(address % 2048) as usize] = value,
            0x4000...0x4013 | 0x4015 => {
                //println!("APU {:04X} ({}) = {}", address, (address - 0x4000), value);
                self.apu_registers[(address - 0x4000) as usize] = value;
            }
            _ => {}
        }
    }

    pub fn mapper_read(&self, address: u16) -> u8 {
        match address {
            0x0000...0x2000 => self.cartridge.chr[address as usize],
            0x6000...0x7FFF => 0xCC, // TODO: self.cartridge.sram[address - 0x6000]
            0x8000...0xBFFF => self.cartridge.prg[(address - 0x8000) as usize],
            0xC000...0xFFFF => self.cartridge.prg[(address - 0xC000) as usize],
            _ => panic!("Invalid bus mapper read at address {}", address),
        }
    }

    pub fn ppu_read(&self, address: u16) -> u8 {
        let address = address % 0x4000;
        match address {
            0x0000...0x1FFF => self.mapper_read(address),
            0x2000...0x3F00 => {
                let mode = self.cartridge.mirror_mode;
                // FIXME: this is wrong
                self.ppu_name_table[address as usize]
            }
            0x3F00...0x4000 => self.ppu_palette[(address % 32) as usize],
            _ => panic!("Invalid bus PPU read at address {}", address),
        }
    }
}
