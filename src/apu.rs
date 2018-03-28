use Bus;

const AUDIO_BUFFER_SIZE: u32 = 50 * 1024;

// TODO(m): This should be part of the platform layer and is used in main.rs but the
// square wave test code below also needs this.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

pub struct APU {
    // Cycle counter
    pub cycle: u32,
}

impl APU {
    pub fn new() -> APU {
        APU {
            cycle: 0,
        }
    }

    pub fn tick(&mut self) {
        self.cycle += 1;
    }

    pub fn square_wave(&mut self, bus: &mut Bus, tone_hz: u32) {
        let period = AUDIO_SAMPLE_RATE / tone_hz;
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
        bus.apu_buffer = samples;
    }

    pub fn step(&mut self, bus: &mut Bus) {
        self.tick();

        // NOTE(m): Comment this out to silence output!
        self.square_wave(bus, 440);

        // println!("APU registers: {:?}", bus.apu_registers);
    }
}
