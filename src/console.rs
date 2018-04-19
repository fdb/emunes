use cpu::{CPU, CPU_FREQUENCY};
use ppu::PPU;
use bus::Bus;
use apu::APU;

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
        for _ in 0..cpu_cycles {
            self.apu.step(&mut self.bus);
        }
        cpu_cycles
    }

    pub fn step_seconds(&mut self, seconds: f64) {
        let mut cycles = (CPU_FREQUENCY as f64 * seconds) as i32;
        while cycles > 0 {
            cycles = cycles - self.step() as i32;
        }
    }
}
