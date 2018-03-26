#[macro_use]
extern crate bitflags;
extern crate minifb;

mod cpu;
mod ppu;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::env;

use cpu::CPU;
use ppu::PPU;

use minifb::{Key, Window, WindowOptions};

const BUFFER_WIDTH: usize = 256;
const BUFFER_HEIGHT: usize = 240;
const BUFFER_SCALE: usize = 3;
const WINDOW_WIDTH: usize = BUFFER_WIDTH * BUFFER_SCALE;
const WINDOW_HEIGHT: usize = BUFFER_HEIGHT * BUFFER_SCALE;

pub trait BitReader {
    fn read_u8(&mut self) -> Result<u8, io::Error>;

    fn read_u32_be(&mut self) -> Result<u32, io::Error>;
    fn read_u32_le(&mut self) -> Result<u32, io::Error>;
}

impl BitReader for File {
    fn read_u8(&mut self) -> Result<u8, io::Error> {
        let mut buffer = [0; 1];
        self.read(&mut buffer)?;
        Ok(buffer[0])
    }

    fn read_u32_be(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];

        self.read(&mut buffer)?;

        Ok(
            buffer[3] as u32 + ((buffer[2] as u32) << 8) + ((buffer[1] as u32) << 16)
                + ((buffer[0] as u32) << 24),
        )
    }

    fn read_u32_le(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];

        self.read(&mut buffer)?;

        Ok(
            buffer[0] as u32 + ((buffer[1] as u32) << 8) + ((buffer[2] as u32) << 16)
                + ((buffer[3] as u32) << 24),
        )
    }
}

pub struct RomHeader {
    pub magic: u32,
    pub prg_count: u8,
    pub chr_count: u8,
    pub control1: u8,
    pub control2: u8,
    pub ram_count: u8,
}

pub struct Cartridge {
    pub prg: Vec<u8>,
    pub chr: Vec<u8>,
    pub sram: Vec<u8>,
    pub mapper_type: u8,
    pub mirror_mode: u8,
    pub battery_present: bool,
}

pub trait Mapper {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn step(&self);
}

pub struct Mapper2<'a> {
    cartridge: &'a mut Cartridge,
}

impl<'a> Mapper2<'a> {
    pub fn new(cartridge: &mut Cartridge) -> Mapper2 {
        Mapper2 { cartridge }
    }
}

impl<'a> Mapper for Mapper2<'a> {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000...0x2000 => self.cartridge.chr[address as usize],
            0x2001...0xC000 => self.cartridge.prg[address as usize],
            _ => panic!("Invalid mapper read {:?}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000...0x2000 => self.cartridge.chr[address as usize] = value,
            0x2001...0xC000 => self.cartridge.prg[address as usize] = value,
            _ => panic!("Invalid mapper write {:?}", address),
        }
    }

    fn step(&self) {}
}

pub fn new_mapper(mapper_type: u8, cartridge: &mut Cartridge) -> Mapper2 {
    match mapper_type {
        0 => Mapper2::new(cartridge),
        _ => panic!("Invalid mapper_type {:?}", mapper_type),
    }
}

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

pub struct Console {
    pub cpu: CPU,
    pub ppu: PPU,
    pub bus: Bus,
}

impl Console {
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus)
    }

    pub fn log_string(&mut self) -> String {
        self.cpu.log_string(&self.bus)
    }

    pub fn step(&mut self) -> u32 {
        let cpu_cycles = self.cpu.step(&mut self.bus);
        let ppu_cycles = cpu_cycles * 3;
        for _ in 0..ppu_cycles {
            self.ppu.step(&mut self.bus);
        }
        cpu_cycles
    }
}

fn read_rom(path: &str) -> Result<Cartridge, io::Error> {
    let mut fp = File::open(path)?;
    let magic = fp.read_u32_be()?;
    if magic != 0x4e45531a {
        panic!(
            "Not an INES ROM file: magic number mismatch (got: 0x{:08X})",
            magic
        );
    }

    let prg_rom_size = fp.read_u8()? as usize;
    let chr_rom_size = fp.read_u8()? as usize;
    let flags_1 = fp.read_u8()?;
    let flags_2 = fp.read_u8()?;
    let prg_ram_size = fp.read_u8()?;
    // Skip padding
    fp.seek(SeekFrom::Current(7))?;

    println!("prg_rom_size: {}", prg_rom_size);
    println!("chr_rom_size: {}", chr_rom_size);
    println!("flags_1: {}", flags_1);
    println!("flags_2: {}", flags_2);
    println!("prg_ram_size: {}", prg_ram_size);

    let mapper_type = (flags_1 >> 4) | (flags_2 >> 4) << 4;
    println!("mapper: {}", mapper_type);

    let mirror_mode = (flags_1 & 1) | ((flags_1 >> 3) & 1) << 1;
    println!("mirror_mode: {}", mirror_mode);

    let battery = (flags_1 >> 1) & 1;

    // Read trainer data (need to skip this)
    if flags_1 & 4 == 4 {
        let mut trainer = [0; 0x512];
        fp.read(&mut trainer)?;
    }

    // Read PRG ROM banks
    let mut prg: Vec<u8> = Vec::new();
    prg.resize(prg_rom_size * 16384, 0);
    fp.read(&mut prg)?;

    // Read CHR ROM banks
    let mut chr: Vec<u8> = Vec::new();
    chr.resize(chr_rom_size * 8192, 0);
    fp.read(&mut chr)?;

    println!("prg len {}", prg.len());
    println!("chr len {}", chr.len());

    // If no CHR rom is available make some
    if chr_rom_size == 0 {
        chr.resize(8192, 0);
    }

    let mut sram: Vec<u8> = Vec::new();
    sram.resize(8192, 0);

    Ok(Cartridge {
        prg,
        chr,
        sram,
        mapper_type,
        mirror_mode,
        battery_present: battery == 1,
    })
}

fn usage() {
    println!("Usage: emunes romfile.nes");
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        usage();
        std::process::exit(1);
    }

    let filename = &args[1];
    let cartridge = read_rom(&filename).unwrap();

    //let mapper = new_mapper(cartridge.mapper_type, cartridge);
    let mut ram: Vec<u8> = Vec::new();
    ram.resize(2048, 0);

    let cpu = CPU::new();
    let ppu = PPU::new();

    let bus = Bus::new(cartridge, ram);


    let mut console = Console { cpu, ppu, bus };

    console.reset();
    let mut buffer: Vec<u32> = vec![0; WINDOW_WIDTH * WINDOW_HEIGHT];


    let mut x_off = 0;
    let mut y_off = 0;
    let mut chr_off = 0;
    while chr_off < 1024 {
        for y in 0..8 {
            for x in 0..8 {
                let c = *(&console.bus.cartridge.chr[y * 8 + x + chr_off]) as u32;
                console.bus.ppu_pixels[((y_off + y) * BUFFER_WIDTH) + x + x_off]= (0xFF << 24) | (c << 16) | (c << 8) | c;
            }
        }
        chr_off += 64;
        x_off += 10;
        if x_off > BUFFER_WIDTH - 10 {
            x_off = 0;
            y_off += 10;
        }
    }

    loop {

        let mut window = Window::new(
            "Emunes",
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            WindowOptions::default(),
        ).unwrap_or_else(|e| {
            panic!("{}", e);
        });

        while window.is_open() && !window.is_key_down(Key::Escape) {
            console.step();
            //println!("{}", console.log_string());
            for y in 0..BUFFER_HEIGHT {
                for x in 0..BUFFER_WIDTH {
                    {
                        let c = &console.bus.ppu_pixels[(y * BUFFER_WIDTH) + x];
                        for dy in 0..BUFFER_SCALE {
                            for dx in 0..BUFFER_SCALE {
                                buffer[(y * BUFFER_SCALE + dy) * WINDOW_WIDTH
                                           + (x * BUFFER_SCALE + dx)] = *c;
                            }
                        }
                    }
                }
            }
            window.update_with_buffer(&buffer).unwrap();
        }

        //     let mut v = 0;
        //     let mut x = 0;
        //     let mut y = 0;
        //     let offx = 0;
        //     let offy = 0;

        //     while v < console.bus.cartridge.chr.len() {
        //         x += 1;
        //         if x >= WIDTH {
        //             x = 0;
        //             y += 1;
        //         }
        //         // if x > 8 {
        //         //     x = 0;
        //         //     y += 1;
        //         //     if y > 8 {
        //         //         offx += 16;
        //         //         y = 0;
        //         //         offy += 16;
        //         //         if offy > 32 {break;}
        //         //     }
        //         // }
        //         let c = console.bus.cartridge.chr[v] as u32;
        //         buffer[(y + offy) * WIDTH + (x + offx)] = c; //  (c << 24) | (c << 16) | (c << 8) | c;
        //         buffer[(y + offy) * WIDTH + (x + offx)] = (c << 24) | (c << 16) | (c << 8) | c;
        //         //buffer[(y + offy) * WIDTH + (x + offx)] =  (c << 24) | (c << 16) | (c << 8) | c;
        //         v += 1;
        //     }

        //     // let mut x = 0;
        //     // for i in buffer.iter_mut() {
        //     //     let c = vram[x % 0x2000] as u32;
        //     //     *i =  (c << 24) | (c << 16) | (c << 8) | c;
        //     //     //*i = (c << 8) | c;
        //     //     //*i = c;
        //     //     //*i = 0; // write something more funny here!
        //     //     x += 1;
        //     // }

        //     // We unwrap here as we want this code to exit if it fails.
        //     // Real applications may want to handle this in a different way

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;
    use std::io::BufReader;

    #[test]
    fn it_runs_nestest() {
        let cartridge = read_rom("testroms/nestest.nes").unwrap();

        //let mapper = new_mapper(cartridge.mapper_type, cartridge);
        let mut ram: Vec<u8> = Vec::new();
        ram.resize(2048, 0);

        let cpu = CPU::new();

        let bus = Bus::new(cartridge, ram);

        let mut console = Console {
            cpu,
            ppu: PPU::new(),
            bus,
        };

        console.reset();
        // This is specifically for the nestest log.
        // Setting the PC to 0xC000 will run automated tests.
        // This allows it to be compared to the nestest.log.
        // See http://www.qmtpro.com/~nes/misc/nestest.txt for more info.
        console.cpu.pc = 0xC000;

        let f = File::open("testroms/nestest.log").unwrap();
        let mut reader = BufReader::new(f);
        let mut history: Vec<String> = Vec::new();

        let mut i = 0;
        loop {
            let mut expected = String::new();
            reader.read_line(&mut expected).unwrap();
            let expected = expected.trim_right().to_owned();
            if expected.len() == 0 {
                break;
            }
            let actual = console.log_string();
            assert_eq!(expected, actual);
            //println!("{}", expected);
            if actual != expected {
                println!("Processor state does not match the test logs:");
                let min = (i as i32) - 10;
                let min = if min < 0 { 0 } else { min };
                for j in min..i {
                    println!("  {}", history[j as usize]);
                }
                println!("Line {}:", i + 1);
                println!("  {}\n* {}", expected, actual);
                break;
            }
            console.step();
            history.push(expected.clone());
            i += 1;
        }
        let result = console.bus.read_16(0x02);
        println!("Done. Result code = {:04X}", result);
        // See testroms/nestest.txt to see what error codes we can get.
        assert_eq!(result, 0x0000);
    }
}
