#[macro_use]
extern crate bitflags;

extern crate minifb;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::io::prelude::*;
use std::num::Wrapping;

use std::io::BufReader;

use minifb::{Key, Window, WindowOptions};

const ADDRESS_MODE_ABSOLUTE: u8 = 1;
const ADDRESS_MODE_ABSOLUTE_X: u8 = 2;
const ADDRESS_MODE_ABSOLUTE_Y: u8 = 3;
const ADDRESS_MODE_ACCUMULATOR: u8 = 4;
const ADDRESS_MODE_IMMEDIATE: u8 = 5;
const ADDRESS_MODE_IMPLIED: u8 = 6;
const ADDRESS_MODE_INDEXED_INDIRECT: u8 = 7;
const ADDRESS_MODE_INDIRECT: u8 = 8;
const ADDRESS_MODE_INDIRECT_INDEXED: u8 = 9;
const ADDRESS_MODE_RELATIVE: u8 = 10;
const ADDRESS_MODE_ZERO_PAGE: u8 = 11;
const ADDRESS_MODE_ZERO_PAGE_X: u8 = 12;
const ADDRESS_MODE_ZERO_PAGE_Y: u8 = 13;

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

const INSTRUCTION_MODES: [u8; 256] = [
    6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2,
    2, 2, 2, 1, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3,
    6, 3, 2, 2, 2, 2, 6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12,
    12, 6, 3, 6, 3, 2, 2, 2, 2, 6, 7, 6, 7, 11, 11, 11, 11, 6, 5, 4, 5, 8, 1, 1, 1, 10, 9, 6, 9,
    12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6, 5, 1, 1, 1, 1, 10,
    9, 6, 9, 12, 12, 13, 13, 6, 3, 6, 3, 2, 2, 3, 3, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6, 5, 1, 1,
    1, 1, 10, 9, 6, 9, 12, 12, 13, 13, 6, 3, 6, 3, 2, 2, 3, 3, 5, 7, 5, 7, 11, 11, 11, 11, 6, 5, 6,
    5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2, 5, 7, 5, 7, 11, 11, 11, 11,
    6, 5, 6, 5, 1, 1, 1, 1, 10, 9, 6, 9, 12, 12, 12, 12, 6, 3, 6, 3, 2, 2, 2, 2,
];

const INSTRUCTION_SIZES: [u8; 256] = [
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    3, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 0, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 0, 3, 0, 0,
    2, 2, 2, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0, 2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
];

const INSTRUCTION_CYCLES: [u8; 256] = [
    7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

const INSTRUCTION_PAGE_CYCLES: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
];


const INSTRUCTION_NAMES: &'static [&'static str] = &[
    "BRK", "ORA", "KIL", "SLO", "NOP", "ORA", "ASL", "SLO", "PHP", "ORA", "ASL", "ANC", "NOP",
    "ORA", "ASL", "SLO", "BPL", "ORA", "KIL", "SLO", "NOP", "ORA", "ASL", "SLO", "CLC", "ORA",
    "NOP", "SLO", "NOP", "ORA", "ASL", "SLO", "JSR", "AND", "KIL", "RLA", "BIT", "AND", "ROL",
    "RLA", "PLP", "AND", "ROL", "ANC", "BIT", "AND", "ROL", "RLA", "BMI", "AND", "KIL", "RLA",
    "NOP", "AND", "ROL", "RLA", "SEC", "AND", "NOP", "RLA", "NOP", "AND", "ROL", "RLA", "RTI",
    "EOR", "KIL", "SRE", "NOP", "EOR", "LSR", "SRE", "PHA", "EOR", "LSR", "ALR", "JMP", "EOR",
    "LSR", "SRE", "BVC", "EOR", "KIL", "SRE", "NOP", "EOR", "LSR", "SRE", "CLI", "EOR", "NOP",
    "SRE", "NOP", "EOR", "LSR", "SRE", "RTS", "ADC", "KIL", "RRA", "NOP", "ADC", "ROR", "RRA",
    "PLA", "ADC", "ROR", "ARR", "JMP", "ADC", "ROR", "RRA", "BVS", "ADC", "KIL", "RRA", "NOP",
    "ADC", "ROR", "RRA", "SEI", "ADC", "NOP", "RRA", "NOP", "ADC", "ROR", "RRA", "NOP", "STA",
    "NOP", "SAX", "STY", "STA", "STX", "SAX", "DEY", "NOP", "TXA", "XAA", "STY", "STA", "STX",
    "SAX", "BCC", "STA", "KIL", "AHX", "STY", "STA", "STX", "SAX", "TYA", "STA", "TXS", "TAS",
    "SHY", "STA", "SHX", "AHX", "LDY", "LDA", "LDX", "LAX", "LDY", "LDA", "LDX", "LAX", "TAY",
    "LDA", "TAX", "LAX", "LDY", "LDA", "LDX", "LAX", "BCS", "LDA", "KIL", "LAX", "LDY", "LDA",
    "LDX", "LAX", "CLV", "LDA", "TSX", "LAS", "LDY", "LDA", "LDX", "LAX", "CPY", "CMP", "NOP",
    "DCP", "CPY", "CMP", "DEC", "DCP", "INY", "CMP", "DEX", "AXS", "CPY", "CMP", "DEC", "DCP",
    "BNE", "CMP", "KIL", "DCP", "NOP", "CMP", "DEC", "DCP", "CLD", "CMP", "NOP", "DCP", "NOP",
    "CMP", "DEC", "DCP", "CPX", "SBC", "NOP", "ISC", "CPX", "SBC", "INC", "ISC", "INX", "SBC",
    "NOP", "SBC", "CPX", "SBC", "INC", "ISC", "BEQ", "SBC", "KIL", "ISC", "NOP", "SBC", "INC",
    "ISC", "SED", "SBC", "NOP", "ISC", "NOP", "SBC", "INC", "ISC",
];

pub trait BitReader {
    fn read_u8(&mut self) -> Result<u8, io::Error>;

    fn read_u32_be(&mut self) -> Result<u32, io::Error>;
    fn read_u32_le(&mut self) -> Result<u32, io::Error>;
}

impl BitReader for File {
    fn read_u8(&mut self) -> Result<u8, io::Error> {
        let mut buffer = [0; 1];
        try!(self.read(&mut buffer));
        Ok(buffer[0])
    }

    fn read_u32_be(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];

        try!(self.read(&mut buffer));

        Ok(
            buffer[3] as u32 + ((buffer[2] as u32) << 8) + ((buffer[1] as u32) << 16)
                + ((buffer[0] as u32) << 24),
        )
    }

    fn read_u32_le(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];

        try!(self.read(&mut buffer));

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

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        const CARRY             = 1 << 0;
        const ZERO              = 1 << 1;
        const INTERRUPT_DISABLE = 1 << 2;
        const DECIMAL_MODE      = 1 << 3;
        const BREAK             = 1 << 4;
        const UNUSED            = 1 << 5;
        const OVERFLOW          = 1 << 6;
        const NEGATIVE          = 1 << 7;
    }
}

fn pages_differ(a: u16, b: u16) -> bool {
    a & 0xFF00 != b & 0xFF00
}

pub struct CPU {
    pub cycles: u64,
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub flags: Flags,
}

// http://wiki.nesdev.com/w/index.php/CPU_unofficial_opcodes
// http://6502.org/tutorials/6502opcodes.html
impl CPU {
    pub fn new() -> CPU {
        CPU {
            cycles: 0,
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            flags: Default::default(),
        }
    }

    pub fn reset(&mut self, bus: &mut Bus) {
        self.pc = bus.read_16(0xFFFC);
        println!("RESET PC: {:04X}", self.pc);
        self.sp = 0xFD;
        self.flags = Flags::UNUSED | Flags::INTERRUPT_DISABLE;
    }

    /// Set the zero flag if the value is 0.
    pub fn set_z_flag(&mut self, v: u8) {
        self.flags.set(Flags::ZERO, v == 0);
    }

    /// Set the negative flag if the value is negative (high bit is set).
    pub fn set_n_flag(&mut self, v: u8) {
        self.flags.set(Flags::NEGATIVE, (v & 0x80) != 0);
    }

    /// Set the zero flag and the negative flag.
    pub fn set_zn_flag(&mut self, v: u8) {
        self.set_z_flag(v);
        self.set_n_flag(v);
    }

    pub fn push(&mut self, bus: &mut Bus, v: u8) {
        let sp = self.sp as u16;
        bus.write(0x100 + sp, v);
        self.sp -= 1;
    }

    pub fn pull(&mut self, bus: &Bus) -> u8 {
        self.sp += 1;
        let sp = self.sp as u16;
        bus.read(0x100 + sp)
    }

    pub fn push_16(&mut self, mut bus: &mut Bus, v: u16) {
        let hi = (v >> 8) as u8;
        let lo = (v & 0xFF) as u8;
        self.push(&mut bus, hi);
        self.push(&mut bus, lo);
    }

    pub fn pull_16(&mut self, bus: &Bus) -> u16 {
        let lo = self.pull(&bus);
        let hi = self.pull(&bus);
        (hi as u16) << 8 | lo as u16
    }

    pub fn get_address(&mut self, bus: &Bus, opcode: u8, side_effects: bool) -> u16 {
        let address_mode = INSTRUCTION_MODES[opcode as usize];
        let mut page_crossed = false;
        let address = match address_mode {
            ADDRESS_MODE_ABSOLUTE => bus.read_16(self.pc + 1),
            ADDRESS_MODE_ACCUMULATOR => 0,
            ADDRESS_MODE_IMMEDIATE => self.pc + 1,
            ADDRESS_MODE_IMPLIED => 0,
            ADDRESS_MODE_INDEXED_INDIRECT => bus.read_16_bug((bus.read(self.pc + 1).wrapping_add(self.x)) as u16),
            ADDRESS_MODE_INDIRECT => bus.read_16_bug(bus.read_16(self.pc + 1)),
            ADDRESS_MODE_INDIRECT_INDEXED => {
                let address = bus.read_16_bug((bus.read(self.pc + 1) as u16)).wrapping_add(self.y as u16);
                page_crossed = pages_differ(address.wrapping_sub(self.y as u16), address);
                address
            }
            ADDRESS_MODE_RELATIVE => {
                let offset = bus.read(self.pc + 1) as u16;
                if offset < 0x80 {
                    self.pc + 2 + offset
                } else {
                    self.pc + 2 + offset - 0x100
                }
            }
            ADDRESS_MODE_ZERO_PAGE => bus.read(self.pc + 1) as u16,
            _ => panic!("Invalid address mode {}", address_mode),
        };

        if side_effects && page_crossed {
            self.cycles += INSTRUCTION_PAGE_CYCLES[opcode as usize] as u64;
        }
        address
    }

    pub fn branch_to(&mut self, address: u16) {
        self.pc = address;
        self.cycles += 1;
        if pages_differ(self.pc, address) {
            self.cycles += 1;
        }
    }

    pub fn compare(&mut self, a: u8, b: u8) {
        self.set_zn_flag(a.wrapping_sub(b));
        self.flags.set(Flags::CARRY, a >= b);
    }

    pub fn log_string(&mut self, bus: &Bus) -> String {
        let opcode = bus.read(self.pc);
        let arg1 = bus.read(self.pc + 1);
        let arg2 = bus.read(self.pc + 2);
        let name = INSTRUCTION_NAMES[opcode as usize];
        let opcode_size = INSTRUCTION_SIZES[opcode as usize];
        let opcode_string = match opcode_size {
            1 => format!("{:02X}      ", opcode),
            2 => format!("{:02X} {:02X}   ", opcode, arg1),
            3 => format!("{:02X} {:02X} {:02X}", opcode, arg1, arg2),
            _ => panic!(
                "Invalid instruction size {:02X} size {}",
                opcode, opcode_size
            ),
        };
        let address_mode = INSTRUCTION_MODES[opcode as usize];
        let address = self.get_address(&bus, opcode, false);
        let value = bus.read(address);
        let mut address_string = match address_mode {
            ADDRESS_MODE_ABSOLUTE => format!("${:04X} = {:02X}", address, value),
            ADDRESS_MODE_ACCUMULATOR => "A".to_owned(),
            ADDRESS_MODE_RELATIVE => format!("${:04X}", address),
            ADDRESS_MODE_IMMEDIATE => format!("#${:02X}", arg1),
            ADDRESS_MODE_INDEXED_INDIRECT => format!("(${:02X},X) @ {:02X} = {:04X} = {:02X}", arg1, (arg1.wrapping_add(self.x)), address, value),
            ADDRESS_MODE_INDIRECT => format!("(${:02X}{:02X}) = {:04X}", arg2, arg1, address),
            ADDRESS_MODE_INDIRECT_INDEXED => format!("(${:02X}),Y = {:04X} @ {:04X} = {:02X}", arg1, bus.read_16_bug(arg1 as u16), address, value),

            ADDRESS_MODE_IMPLIED => "".to_owned(),
            ADDRESS_MODE_ZERO_PAGE => format!("${:02X} = {:02X}", arg1, value),
            _ => format!("??? opcode {:02X} mode {}", opcode, address_mode)
        };

        // Jump instructions don't show the value at the address.
        if address_mode == ADDRESS_MODE_ABSOLUTE && name.starts_with("J") {
            address_string = format!("${:04X}", address);
        }

        let cycles = (self.cycles * 3) % 341;

        format!(
            "{:04X}  {}  {} {:27} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{:3}",
            self.pc,
            opcode_string,
            name,
            address_string,
            self.a,
            self.x,
            self.y,
            self.flags,
            self.sp,
            cycles
        )
    }

    pub fn log(&mut self, bus: &Bus) {
        println!("{}", self.log_string(&bus));
    }

    pub fn step(&mut self, mut bus: &mut Bus) {
        let opcode = bus.read(self.pc);
        let address_mode = INSTRUCTION_MODES[opcode as usize];
        let address = self.get_address(&bus, opcode, true);

        //println!("Address: {:04X} mode {:?}", address, address_mode);
        //
        self.pc += INSTRUCTION_SIZES[opcode as usize] as u16;
        self.cycles += INSTRUCTION_CYCLES[opcode as usize] as u64;
        // Reference: https://wiki.nesdev.com/w/index.php/CPU_unofficial_opcodes
        match opcode {
            //// Control Instructions ////

            // PHP - Push Processor Status
            0x08 => {
                let flags = self.flags.bits();
                self.push(&mut bus, flags | 0x10);
            }

            // BPL - Branch If Positive
            0x10 => {
                if !self.flags.intersects(Flags::NEGATIVE) {
                    self.branch_to(address);
                }
            }

            // CLC - Clear Carry Flag
            0x18 => {
                self.flags.remove(Flags::CARRY);
            }

            // JSR - Jump to Subroutine
            0x20 => {
                let pc = self.pc;
                self.push_16(&mut bus, pc - 1);
                self.pc = address;
            }

            // BIT - Bit Test
            0x24 | 0x2C => {
                let v = bus.read(address);
                let a = self.a;
                self.flags.set(Flags::OVERFLOW, ((v >> 6) & 1) > 0);
                self.set_z_flag(v & a);
                self.set_n_flag(v);
            }

            // PLP - Pull Processor Status
            0x28 => {
                let flags = self.pull(&bus) & 0xEF | 0x20;
                self.flags = Flags::from_bits(flags).unwrap();
            }

            // BMI - Branch on Minus
            0x30 => {
                if self.flags.intersects(Flags::NEGATIVE) {
                    self.branch_to(address);
                }
            }

            // SEC - Set Carry Flag
            0x38 => {
                self.flags |= Flags::CARRY;
            }

            // RTI - Return from Interrupt
            0x40 => {
                let flags = self.pull(&bus) & 0xEF | 0x20;
                self.flags = Flags::from_bits(flags).unwrap();
                self.pc = self.pull_16(&bus);
            }

            // PHA - Push Accumulator
            0x48 => {
                let a = self.a;
                self.push(&mut bus, a);
            }

            // JMP - Jump
            0x4C | 0x6C => {
                self.pc = address;
            }

            // BVC - Branch on Overflow Clear
            0x50 => {
                if !self.flags.intersects(Flags::OVERFLOW) {
                    self.branch_to(address);
                }
            }

            // CLI - Clear Interrupt
            0x58 => {
                self.flags.remove(Flags::INTERRUPT_DISABLE);
            }

            // RTS - Return from Subroutine
            0x60 => {
                self.pc = self.pull_16(&bus) + 1;
            }

            // PLA - Pull Accumulator
            0x68 => {
                self.a = self.pull(&bus);
                let a = self.a;
                self.set_zn_flag(a);
            }

            // BVS - Branch on Overflow Set
            0x70 => {
                if self.flags.intersects(Flags::OVERFLOW) {
                    self.branch_to(address);
                }
            }

            // SEI - Set Interrupt
            0x78 => {
                self.flags |= Flags::INTERRUPT_DISABLE;
            }

            // STY - Store Y Register
            0x84 | 0x8C | 0x94 => {
                bus.write(address, self.y)
            }

            // DEY - Decrement Y Register
            0x88 => {
                self.y = self.y.wrapping_sub(1);
                let y = self.y;
                self.set_zn_flag(y);
            }

            // BCC - Branch on Carry Clear
            0x90 => {
                if !self.flags.intersects(Flags::CARRY) {
                    self.branch_to(address);
                }
            }

            // TYA - Transfer Y to A
            0x98 => {
                self.a = self.y;
                let a = self.a;
                self.set_zn_flag(a);
            }

            // LDY - Load Y Register
            0xA0 | 0xA4 | 0xAC | 0xB4 | 0xBC => {
                self.y = bus.read(address);
                let y = self.y;
                self.set_zn_flag(y);
            }

            // TAY - Transfer A to Y
            0xA8 => {
                self.y = self.a;
                let y = self.y;
                self.set_zn_flag(y);
            }

            // BCS - Branch on Carry Set
            0xB0 => {
                if self.flags.intersects(Flags::CARRY) {
                    self.branch_to(address);
                }
            }

            // CLV - Clear Overflow Flag
            0xB8 => {
                self.flags.remove(Flags::OVERFLOW);
            }

            // CPY - Compare Y Register
            0xC0 | 0xC4 | 0xCC => {
                let v = bus.read(address);
                let y = self.y;
                self.compare(y, v);
            }

            // INY - Increment Y Register
            0xC8 => {
                self.y = self.y.wrapping_add(1);
                let y = self.y;
                self.set_zn_flag(y);
            }

            // BNE - Branch on Not Equal
            0xD0 => {
                if !self.flags.intersects(Flags::ZERO) {
                    self.branch_to(address);
                }
            }

            // CLD - Clear Decimal Flag
            0xD8 => {
                self.flags.remove(Flags::DECIMAL_MODE);
            }

            // CPX - Compare X Register
            0xE0 | 0xE4 | 0xEC => {
                let v = bus.read(address);
                let x = self.x;
                self.compare(x, v);
            }

            // INX - Increment X Register
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                let x = self.x;
                self.set_zn_flag(x);
            }

            // BEQ - Branch on Equal
            0xF0 => {
                if self.flags.intersects(Flags::ZERO) {
                    self.branch_to(address);
                }
            }

            // SED - Set Decimal Flag
            0xF8 => {
                self.flags |= Flags::DECIMAL_MODE;
            }

            //// ALU Operations ////

            // ORA - Bitwise OR with Accumulator
            0x01 | 0x05 | 0x09 | 0x0D | 0x11 | 0x15 | 0x19 | 0x1D => {
                self.a = self.a | bus.read(address);
                let a = self.a;
                self.set_zn_flag(a);
            }

            // AND - Bitwise AND with Accumulator
            0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D => {
                self.a = self.a & bus.read(address);
                let a = self.a;
                self.set_zn_flag(a);
            }

            // EOR - Bitwise Exclusive OR
            0x41 | 0x45 | 0x49 | 0x4D | 0x51 | 0x55 | 0x59 | 0x5D => {
                self.a = self.a ^ bus.read(address);
                let a = self.a;
                self.set_zn_flag(a);
            }

            // ADC - Add with Carry
            0x61 | 0x65 | 0x69 | 0x6D | 0x71 | 0x75 | 0x79 | 0x7D => {
                let a = self.a;
                let b: u8 = bus.read(address);
                let c: u8 = if self.flags.intersects(Flags::CARRY) { 1 } else { 0 };
                self.a = a.wrapping_add(b).wrapping_add(c);
                let _a = self.a;
                self.set_zn_flag(_a);
                self.flags.set(Flags::CARRY, a as i32 + b as i32 + c as i32 > 0xFF);
                self.flags.set(Flags::OVERFLOW, (a ^ b) & 0x80 == 0 && (a ^ _a) & 0x80 != 0);
            }

            // STA - Store Accumulator
            0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => {
                bus.write(address, self.a);
            }

            // LDA - Load Accumulator
            0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD  => {
                self.a = bus.read(address);
                let a = self.a;
                self.set_zn_flag(a);
            }

            // CMP - Compare Accumulator
            0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => {
                let v = bus.read(address);
                let a = self.a;
                self.compare(a, v);
            }

            // SBC - Subtract with Carry
            0xE1 | 0xE5 | 0xE9 | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => {
                let a = self.a;
                let b: u8 = bus.read(address);
                let c: u8 = if self.flags.intersects(Flags::CARRY) { 1 } else { 0 };
                self.a = a.wrapping_sub(b).wrapping_sub(1 - c);
                let _a = self.a;
                self.set_zn_flag(_a);
                self.flags.set(Flags::CARRY, (a as i32) - (b as i32) - ((1 - c) as i32) >= 0);
                self.flags.set(Flags::OVERFLOW, (a ^ b) & 0x80 != 0 && (a ^ _a) & 0x80 != 0);
            }

            //// Read-Modify-Write Operations ////

            // ASL - Arithmetic Shift Left
            0x06 | 0x0A | 0x0E | 0x16 | 0x1E => {
                if address_mode == ADDRESS_MODE_ACCUMULATOR {
                    self.flags.set(Flags::CARRY, ((self.a >> 7) & 1) > 0);
                    self.a <<= 1;
                    let a = self.a;
                    self.set_zn_flag(a);
                } else {
                    let mut v = bus.read(address);
                    self.flags.set(Flags::CARRY, ((v >> 7) & 1) > 0);
                    v <<= 1;
                    bus.write(address, v);
                    self.set_zn_flag(v);
                }
            }

            // ROL - Rotate Left
            0x26 | 0x2A | 0x2E | 0x36 | 0x3E => {
                let c: u8 = if self.flags.intersects(Flags::CARRY) { 1 } else { 0 };
                if address_mode == ADDRESS_MODE_ACCUMULATOR {
                    self.flags.set(Flags::CARRY, ((self.a >> 7) & 1) > 0);
                    self.a = (self.a << 1) | c;
                    let a = self.a;
                    self.set_zn_flag(a);
                } else {
                    let mut v = bus.read(address);
                    self.flags.set(Flags::CARRY, ((v >> 7) & 1) > 0);
                    v  = (v << 1) | c;
                    bus.write(address, v);
                    self.set_zn_flag(v);
                }
            }

            // LSR - Logical Shift Right
            0x46 | 0x4A | 0x4E | 0x56 | 0x5E => {
                if address_mode == ADDRESS_MODE_ACCUMULATOR {
                    self.flags.set(Flags::CARRY, (self.a & 1) > 0);
                    self.a >>= 1;
                    let a = self.a;
                    self.set_zn_flag(a);
                } else {
                    let mut v = bus.read(address);
                    self.flags.set(Flags::CARRY, (v & 1) > 0);
                    v >>= 1;
                    bus.write(address, v);
                    self.set_zn_flag(v);
                }
            }

            // ROR - Rotate Right
            0x66 | 0x6A | 0x6E | 0x76 | 0x7E => {
                let c: u8 = if self.flags.intersects(Flags::CARRY) { 1 } else { 0 };
                if address_mode == ADDRESS_MODE_ACCUMULATOR {
                    self.flags.set(Flags::CARRY, (self.a & 1) > 0);
                    self.a = (self.a >> 1) | (c << 7);
                    let a = self.a;
                    self.set_zn_flag(a);
                } else {
                    let mut v = bus.read(address);
                    self.flags.set(Flags::CARRY, (v & 1) > 0);
                    v  = (v >> 1) | (c << 7);
                    bus.write(address, v);
                    self.set_zn_flag(v);
                }
            }

            // STX - Store X Register
            0x86 | 0x8E => {
                bus.write(address, self.x);
            }

            // TXA - Transfer X to A
            0x8A => {
                self.a = self.x;
                let a = self.a;
                self.set_zn_flag(a);
            }

            // TXS - Transfer X to Stack Pointer
            0x9A => {
                self.sp = self.x;
            }

            // LDX - Load X Register
            0xA2 | 0xA6 | 0xAE | 0x96 | 0x9E => {
                self.x = bus.read(address);
                let x = self.x;
                self.set_zn_flag(x)
            }

            // TAX - Transfer A to X
            0xAA => {
                self.x = self.a;
                let x = self.x;
                self.set_zn_flag(x);
            }

            // TSX - Transfer Stack Pointer to X
            0xBA => {
                self.x = self.sp;
                let x = self.x;
                self.set_zn_flag(x);
            }

            // DEC - Decrement Memory
            0xC6 | 0xCE | 0xD6 | 0xDE => {
                let mut v = bus.read(address);
                v = v.wrapping_sub(1);
                bus.write(address, v);
                self.set_zn_flag(v);
            }

            // DEX - Decrement X Register
            0xCA => {
                self.x = self.x.wrapping_sub(1);
                let x = self.x;
                self.set_zn_flag(x);
            }

            // INC - Increment Memory
            0xE6 | 0xEE | 0xF6 | 0xFE => {
                let mut v = bus.read(address);
                v = v.wrapping_add(1);
                bus.write(address, v);
                self.set_zn_flag(v);
            }

            // NOP
            0xEA => {}

            _ => {
                println!(
                    "Instruction {} ({:02X}) not implemented yet.",
                    INSTRUCTION_NAMES[opcode as usize], opcode
                );
            }
        }
    }
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
}

impl Bus {
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000...0x1FFF => self.ram[(address % 0x2000) as usize],
            0x2000...0x4000 => 0xCC, // TODO: self.ppu.read_register(0x2000 + address % 8)
            0x4014 => 0xCC,          // TODO: self.ppu.read_register(address)
            0x4015 => 0xCC,          // TODO: self.apu.read_register(address)
            0x4016 => 0xCC,          // TODO: self.controller1.read()
            0x4017 => 0xCC,          // TODO: self.controller2.read()
            0x4018...0x5FFF => 0xCC, // TODO: I/O registers
            0x6000...0xFFFF => self.mapper_read(address),
            _ => panic!("Invalid bus memory read at address {}", address),
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
        self.ram[(address % 2048) as usize] = value
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
}

pub struct Console {
    pub cpu: CPU,
    pub bus: Bus,
}

impl Console {
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus)
    }

    pub fn log_string(&mut self) -> String {
        self.cpu.log_string(&self.bus)
    }

    pub fn step(&mut self) {
        self.cpu.step(&mut self.bus)
    }
}

fn read_rom(path: &str) -> Result<Cartridge, io::Error> {
    let mut fp = try!(File::open(path));

    let magic = try!(fp.read_u32_be());
    if magic != 0x4e45531a {
        panic!(
            "Not an INES ROM file: magic number mismatch (got: 0x{:08X})",
            magic
        );
    }

    let prg_rom_size = try!(fp.read_u8()) as usize;
    let chr_rom_size = try!(fp.read_u8()) as usize;
    let flags_1 = try!(fp.read_u8());
    let flags_2 = try!(fp.read_u8());
    let prg_ram_size = try!(fp.read_u8());
    // Skip padding
    try!(fp.seek(SeekFrom::Current(7)));

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
        try!(fp.read(&mut trainer));
    }

    // Read PRG ROM banks
    let mut prg: Vec<u8> = Vec::new();
    prg.resize(prg_rom_size * 16384, 0);
    try!(fp.read(&mut prg));

    // Read CHR ROM banks
    let mut chr: Vec<u8> = Vec::new();
    chr.resize(chr_rom_size * 8192, 0);
    try!(fp.read(&mut chr));

    println!("prg len {}", prg.len());
    println!("chr len {}", chr.len());

    // If no CHR rom is available make some
    if chr_rom_size == 0 {
        chr.resize(8192, 0);
    }

    // let mut file = File::create("dat.rom")?;
    // file.write_all(&rom);

    // let mut file = File::create("dat.vram")?;
    // file.write_all(&vram);

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

fn main() {
    let cartridge = read_rom("testroms/nestest.nes").unwrap();

    //let mapper = new_mapper(cartridge.mapper_type, cartridge);
    let mut ram: Vec<u8> = Vec::new();
    ram.resize(2048, 0);

    let cpu = CPU::new();

    let bus = Bus { cartridge, ram };

    let mut console = Console { cpu, bus };

    console.reset();
    // This is specifically for the nestest log.
    // Setting the PC to 0xC000 will run automated tests.
    // This allows it to be compared to the nestest.log.
    // See http://www.qmtpro.com/~nes/misc/nestest.txt for more info.
    console.cpu.pc = 0xC000;

    let f = File::open("testroms/nestest.log").unwrap();
    let mut reader = BufReader::new(f);
    let mut history: Vec<String> = Vec::new();

    for i in 0..5000 {
        let mut expected = String::new();
        reader.read_line(&mut expected).unwrap();
        let expected = expected.trim_right().to_owned();
        let actual = console.log_string();
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
    }

    // let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // let mut window = Window::new(
    //     "Test - ESC to exit",
    //     WIDTH,
    //     HEIGHT,
    //     WindowOptions::default(),
    // ).unwrap_or_else(|e| {
    //     panic!("{}", e);
    // });

    // while window.is_open() && !window.is_key_down(Key::Escape) {
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
    //     window.update_with_buffer(&buffer).unwrap();
    // }
}
