#[macro_use]
extern crate bitflags;
extern crate sdl2;

use std::mem;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use sdl2::audio::AudioSpecDesired;
use sdl2::render::TextureQuery;
use sdl2::pixels::Color;

mod console;
mod cpu;
mod ppu;
mod bus;
mod cartridge;
mod apu;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::env;
use std::time::{Duration, Instant};
use std::thread;

use console::Console;
use cpu::CPU;
use ppu::PPU;
use bus::{Bus, BUFFER_HEIGHT, BUFFER_WIDTH};
use cartridge::Cartridge;
use apu::APU;

const BUFFER_SCALE: usize = 3;
const WINDOW_WIDTH: usize = BUFFER_WIDTH * BUFFER_SCALE;
const WINDOW_HEIGHT: usize = BUFFER_HEIGHT * BUFFER_SCALE;
const TARGET_FRAME_RATE: u64 = 60;
const BILLION: u64 = 1_000_000_000;
const FRAME_TIME_NS: u64 = BILLION / TARGET_FRAME_RATE;

const AUDIO_SAMPLE_RATE: u32 = 44_100;

const OSD_FONT_SIZE: u16 = 14;

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
    let apu = APU::new(AUDIO_SAMPLE_RATE);

    let bus = Bus::new(cartridge, ram);

    let mut console = Console { cpu, ppu, apu, bus };

    console.reset();
    let mut buffer: Vec<u32> = vec![0; WINDOW_WIDTH * WINDOW_HEIGHT];

    let mut x_off = 0;
    let mut y_off = 0;
    let mut chr_off = 0;
    while chr_off < 4096 {
        for y in 0..8 {
            for x in 0..8 {
                // let b1 = console.bus.cartridge.chr[chr_off] >> x & 0x01;
                let plane0 = console.bus.cartridge.chr[chr_off];
                let plane1 = console.bus.cartridge.chr[chr_off + 8];

                let b0 = (plane0 >> ((7 - ((x % 8) as u8)) as usize)) & 1;
                let b1 = (plane1 >> ((7 - ((x % 8) as u8)) as usize)) & 1;

                let c;
                if b0 == 0 {
                    c = 0;
                } else {
                    c = 0xFFFFFF;
                }
                //let c = *(&console.bus.cartridge.chr[y * 8 + x + chr_off]) as u32;
                console.bus.ppu_pixels[((y_off + y) * BUFFER_WIDTH) + x + x_off] = c;
                // (0xFF << 24) | (c << 16) | (c << 8) | c;
            }
            chr_off += 2;
        }
        //chr_off += 64;
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
        samples: Some(4),
    };
    let device = audio_subsystem
        .open_queue::<i16, _>(None, &desired_spec)
        .unwrap();

    // Initialize SDL TTF
    let ttf_context = sdl2::ttf::init().unwrap();
    let mut font = ttf_context
        .load_font("assets/SourceCodePro-Regular.ttf", OSD_FONT_SIZE)
        .unwrap();
    font.set_style(sdl2::ttf::STYLE_BOLD);

    // Variables for calculating framerate
    let mut last_frame_end_time = Instant::now();
    let mut current_fps = 0;
    let mut frames_elapsed = 0;

    let mut last_timestamp = Instant::now();

    // Declare variables for calculating CPS (cycles per second)
    let mut current_cps = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        let start_time = Instant::now();

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

        // NOTE(m): Calculate how much time we need data for and
        // ask the console api to provide it.
        let timestamp = Instant::now();
        let duration = timestamp - last_timestamp;
        last_timestamp = timestamp;
        // NOTE(m): Convert to seconds following fogleman's implementation.
        // See https://doc.rust-lang.org/std/time/struct.Duration.html#method.as_secs
        let dt = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
        console.step_seconds(dt);

        // Output video
        let _ = texture.update(
            None,
            unsafe { mem::transmute(console.bus.ppu_pixels.as_slice()) },
            BUFFER_WIDTH * 4,
        );

        canvas.clear();

        // Render pixel buffer
        canvas
            .copy(
                &texture,
                None,
                Some(Rect::new(0, 0, WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)),
            )
            .unwrap();

        // Render OSD
        // NOTE(m): Platform layer DEBUG code. Nothing to do with NES emulation.
        // This should help us get rid of blocking println!() calls to output
        // debugging information to the console and causing slowdown.

        // OSD line 1
        let apu_registers = console.bus.apu_registers;
        let osd1_string = format!("APU: {:?}", apu_registers);
        let osd1_surface = font.render(&osd1_string)
            .solid(Color::RGBA(255, 0, 0, 255))
            .unwrap();
        let osd1_texture = texture_creator
            .create_texture_from_surface(&osd1_surface)
            .unwrap();
        let osd1_target_rect = Rect::new(
            0,
            WINDOW_HEIGHT as i32 - (2 * osd1_surface.height()) as i32,
            osd1_surface.width(),
            osd1_surface.height(),
        );
        canvas
            .copy(&osd1_texture, None, Some(osd1_target_rect))
            .unwrap();

        // OSD line 2
        let osd2_string = format!("FPS: {:?} | CPS: {}", current_fps, current_cps);
        let osd2_surface = font.render(&osd2_string)
            .solid(Color::RGBA(255, 0, 0, 255))
            .unwrap();
        let osd2_texture = texture_creator
            .create_texture_from_surface(&osd2_surface)
            .unwrap();
        let osd2_target_rect = Rect::new(
            0,
            WINDOW_HEIGHT as i32 - osd2_surface.height() as i32,
            osd2_surface.width(),
            osd2_surface.height(),
        );
        canvas
            .copy(&osd2_texture, None, Some(osd2_target_rect))
            .unwrap();

        canvas.present();

        // Output audio
        device.queue(&console.bus.apu_buffer);
        device.resume();

        // Calculate framerate.
        // NOTE(m): Borrowed heavily from Casey Muratori's Handmade Hero implementation.
        let frame_end_time = Instant::now();
        if (frame_end_time - last_frame_end_time) >= Duration::new(1, 0) {
            last_frame_end_time = frame_end_time;
            current_fps = frames_elapsed;
            frames_elapsed = 0;

            // // Calculate Cycles Per Second.
            // let frame_end_cycles = console.cpu.cycles;
            // current_cps = (frame_end_cycles - frame_start_cycles) * current_fps;
        }
        frames_elapsed = frames_elapsed + 1;

        // Cap framerate.
        let end_time = Instant::now();
        let time_elapsed = end_time - start_time;
        let time_elapsed: u64 =
            time_elapsed.as_secs() * BILLION + time_elapsed.subsec_nanos() as u64;
        if time_elapsed < FRAME_TIME_NS {
            thread::sleep(Duration::new(0, (FRAME_TIME_NS - time_elapsed) as u32));
        }
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
            apu: APU::new(AUDIO_SAMPLE_RATE),
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
