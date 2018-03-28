pub struct Cartridge {
    pub prg: Vec<u8>,
    pub chr: Vec<u8>,
    pub sram: Vec<u8>,
    pub mapper_type: u8,
    pub mirror_mode: u8,
    pub battery_present: bool,
}
