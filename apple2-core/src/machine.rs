use crate::cpu::CPU;
use crate::memory::{Apple2Memory, Memory};

pub struct Apple2Machine {
    pub cpu: CPU,
    pub mem: Box<Apple2Memory>,
}

impl Apple2Machine {
    pub fn new() -> Self {
        Self {
            cpu: CPU::new(),
            mem: Box::new(Apple2Memory::new()),
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.mem.load_rom(rom_data);
    }

    pub fn reset(&mut self) {
        self.cpu.reset(&mut *self.mem);
    }

    pub fn step(&mut self) -> u32 {
        let cycles = self.cpu.step(&mut *self.mem);
        self.mem.disk2.tick(cycles);
        cycles
    }

    pub fn tick_disk(&mut self, cycles: u32) {
        self.mem.disk2.tick(cycles);
    }

    pub fn power_on(&mut self) {
        self.mem.power_on_reset();
        self.cpu.reset(&mut *self.mem);
    }
}
