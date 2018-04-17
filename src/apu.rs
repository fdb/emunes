use Bus;

const AUDIO_BUFFER_SIZE: u32 = 50 * 1024;
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

pub struct APU {
    // Cycle counter
    pub cycle: u32,
    pub sample_rate: u32
}

impl APU {
    pub fn new(sample_rate: u32) -> APU {
        APU {
            cycle: 0,
            sample_rate: sample_rate
        }
    }

    pub fn tick(&mut self) {
        self.cycle += 1;
    }

    pub fn square_wave(&mut self, bus: &mut Bus, tone_hz: u32) {
        let period = self.sample_rate / tone_hz;
        let mut samples = Vec::new();

        for x in 0..AUDIO_BUFFER_SIZE as u32 {
            samples.push(
                if (x / period) % 2 == 0 {
                    1_000
                } else {
                    -1_000
                }
            );
        }

        // Push generated samples to the bus
        // Note(m): Use a separate thread for audio and use Rust channels in the
        // future.
        bus.apu_buffer = samples;
    }

    pub fn step(&mut self, bus: &mut Bus) {
        self.tick();

        // NOTE(m): Comment this out to silence output!
        // self.square_wave(bus, 440);

        println!("APU registers: {:?}", bus.apu_registers);
    }
}

pub struct Pulse {
    pub enabled: bool,
    pub channel: u8,
    pub length_enabled: bool,
    pub length_value: u8,
    pub timer_period: u16,
    pub timer_value: u16,
    pub duty_mode: u8,
    pub duty_value: u8,
    pub sweep_reload: bool,
    pub sweep_enabled: bool,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_period: u8,
    pub sweep_value: u8,
    pub envelope_enabled: bool,
    pub envelope_loop: bool,
    pub envelope_start: bool,
    pub envelope_period: u8,
    pub envelope_value: u8,
    pub envelope_volume: u8,
    pub constant_volume: u8
}

impl Pulse {
    pub fn write_control(&mut self, value: u8) {
        self.duty_mode = (value >> 6) & 3;
        self.length_enabled = (value >> 5) & 1 == 0;
        self.envelope_loop = (value >> 5) & 1 == 1;
        self.envelope_enabled = (value >> 4) & 1 == 0;
        self.envelope_period = value & 15;
        self.constant_volume = value & 15;
        self.envelope_start = true;
    }

    pub fn write_sweep(&mut self, value: u8) {
        self.sweep_enabled = (value >> 7) & 1 == 1;
        self.sweep_period = (value >> 4) & 7 + 1;
        self.sweep_negate = (value >> 3) & 1 == 1;
        self.sweep_shift = value & 7;
        self.sweep_reload = true;
    }

    pub fn write_timer_low(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0xFF00) | value as u16;
    }

    pub fn write_timer_high(&mut self, value: u8) {
        self.length_value = LENGTH_TABLE[(value >> 3) as usize];
        self.timer_period = (self.timer_period & 0x00FF) | (((value & 7) as u16) << 8);
        self.envelope_start = true;
        self.duty_value = 0;
    }

    pub fn step_timer(&mut self) {
        if self.timer_value  == 0 {
            self.timer_value = self.timer_period;
            self.duty_value = (self.duty_value + 1) % 8;
        } else {
            self.timer_value = self.timer_value - 1;
        }
    }

    pub fn step_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_volume = 15;
            self.envelope_value = self.envelope_period;
            self.envelope_start = false;
        } else if self.envelope_value > 0 {
            self.envelope_value = self.envelope_value - 1;
        } else {
            if self.envelope_volume > 0 {
                self.envelope_volume = self.envelope_volume - 1;
            } else if self.envelope_loop {
                self.envelope_volume = 15;
            }
            self.envelope_value = self.envelope_period;
        }
    }

    pub fn step_sweep(&mut self) {
        if self.sweep_reload {
            if self.sweep_enabled && self.sweep_value == 0 {
                self.sweep();
            }
            self.sweep_value = self.sweep_period;
            self.sweep_reload = false;
        } else if self.sweep_value > 0 {
            self.sweep_value = self.sweep_value - 1;
        } else {
            if self.sweep_enabled {
                self.sweep();
            }
            self.sweep_value = self.sweep_period;
        }
    }

    pub fn step_length(&mut self) {
        if self.length_enabled && self.length_value > 0 {
            self.length_value = self.length_value - 1;
        }
    }

    pub fn sweep(&mut self) {
        let delta = self.timer_period >> self.sweep_shift;
        if self.sweep_negate {
            self.timer_period = self.timer_period - delta;
            if self.channel == 1 {
                self.timer_period = self.timer_period - 1;
            }
        } else {
            self.timer_period = self.timer_period + delta;
        }
    }

    pub fn output(&mut self) -> u8 {
        if !self.enabled {
            return 0;
        }
        if self.length_value == 0 {
            return 0;
        }
        if DUTY_TABLE[self.duty_mode as usize][self.duty_value as usize] == 0 {
            return 0;
        }
        if self.timer_period < 8 || self.timer_period > 0x7FF {
            return 0;
        }
        if self.envelope_enabled {
            self.envelope_volume
        } else {
            self.constant_volume
        }
    }
}
