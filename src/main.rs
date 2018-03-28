#[macro_use]
extern crate bitflags;
extern crate sdl2;

use std::mem;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use sdl2::audio::AudioSpecDesired;

mod cpu;
mod ppu;
mod bus;
mod cartridge;
mod apu;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::env;

use cpu::CPU;
use ppu::PPU;
use bus::{Bus, BUFFER_WIDTH, BUFFER_HEIGHT};
use cartridge::Cartridge;
use apu::{APU, AUDIO_SAMPLE_RATE};

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

pub struct Console {
    pub cpu: CPU,
    pub ppu: PPU,
    pub apu: APU,
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
        self.apu.step(&mut self.bus);
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
    let apu = APU::new();

    let bus = Bus::new(cartridge, ram);

    let mut console = Console { cpu, ppu, apu, bus };

    console.reset();
    let mut buffer: Vec<u32> = vec![0; WINDOW_WIDTH * WINDOW_HEIGHT];

    let mut x_off = 0;
    let mut y_off = 0;
    let mut chr_off = 0;
    while chr_off < 8192 {
        for y in 0..8 {
            for x in 0..8 {
                let c = *(&console.bus.cartridge.chr[y * 8 + x + chr_off]) as u32;
                console.bus.ppu_pixels[((y_off + y) * BUFFER_WIDTH) + x + x_off] =
                    (0xFF << 24) | (c << 16) | (c << 8) | c;
            }
        }
        chr_off += 64;
        x_off += 10;
        if x_off > BUFFER_WIDTH - 10 {
            x_off = 0;
            y_off += 10;
        }
    }

    // Initialize SDL
    let sdl_context = sdl2::init().unwrap();

    // Initialize SDL Video
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Emunes", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::ARGB8888,
            BUFFER_WIDTH as u32,
            BUFFER_HEIGHT as u32,
        )
        .unwrap();

    // Initialize SDL Audio
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(AUDIO_SAMPLE_RATE as i32),
        channels: Some(2),
        samples: Some(4)
    };

    let device = audio_subsystem.open_queue::<i16, _>(None, &desired_spec).unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // The rest of the game loop goes here...
        console.step();

        // Output video
        let _ = texture.update(
            None,
            unsafe { mem::transmute(console.bus.ppu_pixels.as_slice()) },
            BUFFER_WIDTH * 4,
        );

        canvas.clear();
        canvas
            .copy(
                &texture,
                None,
                Some(Rect::new(0, 0, WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)),
            )
            .unwrap();
        canvas.present();

        // Output audio
        device.queue(&console.bus.apu_buffer);
        device.resume();

    }
    // for y in 0..256 {
    //     for x in 0..256 {
    //         let offset = y*pitch + x*3;
    //         buffer[offset] = x as u8;
    //         buffer[offset + 1] = y as u8;
    //         buffer[offset + 2] = 0;
    //     }
    // }}).unwrap();
    //println!("{}", console.log_string());
    //window.update_with_buffer(&buffer).unwrap();
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
            apu: APU::new(),
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
