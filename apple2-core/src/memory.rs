extern crate alloc;

use crate::disk2::Disk2;
use alloc::vec::Vec;

/// The 6502 CPU has a 16-bit address bus (64KB addressable space)
/// and an 8-bit data bus.
pub trait Memory {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);

    fn read_word(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }
}

pub struct Apple2Memory {
    pub ram: [u8; 49152], // 48KB (0x0000 - 0xBFFF)
    pub rom: [u8; 12288], // 12KB (0xD000 - 0xFFFF)

    // Video Soft Switches
    pub text_mode: bool,
    pub mixed_mode: bool,
    pub page2: bool,
    pub hires_mode: bool,

    // Key presses from the desktop window
    pub keyboard_latch: u8,

    // Disk II controller in Slot 6
    pub disk2: alloc::boxed::Box<Disk2>,

    // Speaker State
    pub speaker: bool,
    pub speaker_toggle_cycles: Vec<u64>,
    pub cpu_step_cycle_base: u64,
    pub cpu_step_cycle_cursor: u32,
    pub cpu_step_audio_active: bool,

    // Language Card (16K RAM at $D000-$FFFF)
    pub lc_ram: [u8; 16384],
    pub lc_read_enable: bool,
    pub lc_write_enable: bool,
    pub lc_bank2: bool,
    pub lc_pre_write_switch: u16,
}

impl Apple2Memory {
    pub fn new() -> Self {
        Self {
            ram: [0; 49152],
            rom: [0; 12288],
            text_mode: true,
            mixed_mode: false,
            page2: false,
            hires_mode: false,
            keyboard_latch: 0,
            disk2: alloc::boxed::Box::new(Disk2::new()),
            speaker: false,
            speaker_toggle_cycles: Vec::new(),
            cpu_step_cycle_base: 0,
            cpu_step_cycle_cursor: 0,
            cpu_step_audio_active: false,
            lc_ram: [0; 16384],
            lc_read_enable: false,
            lc_write_enable: false,
            lc_bank2: true,
            lc_pre_write_switch: 0,
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        let copy_len = data.len().min(self.rom.len());
        self.rom[..copy_len].copy_from_slice(&data[..copy_len]);
    }

    pub fn power_on_reset(&mut self) {
        self.ram.fill(0); // Clear RAM to initial state
        self.text_mode = true;
        self.mixed_mode = false;
        self.page2 = false;
        self.hires_mode = false;
        self.keyboard_latch = 0;
        self.speaker = false;
        self.speaker_toggle_cycles.clear();
        self.cpu_step_cycle_base = 0;
        self.cpu_step_cycle_cursor = 0;
        self.cpu_step_audio_active = false;
        self.disk2.reset();

        self.lc_read_enable = false;
        self.lc_write_enable = false;
        self.lc_bank2 = true;
        self.lc_pre_write_switch = 0;

        // Ensure Apple II ROM performs a cold boot by clearing the signature
        self.ram[0x03F4] = 0;
    }

    pub fn begin_cpu_step(&mut self, cycle_base: u64) {
        self.cpu_step_cycle_base = cycle_base;
        self.cpu_step_cycle_cursor = 0;
        self.cpu_step_audio_active = true;
    }

    pub fn end_cpu_step(&mut self) {
        self.cpu_step_audio_active = false;
    }

    pub fn finalize_cpu_step_cycles(&mut self, total_cycles: u32) {
        let accounted = self.cpu_step_cycle_cursor.min(total_cycles);
        let remaining = total_cycles - accounted;
        if remaining > 0 {
            self.disk2.tick(remaining);
            self.cpu_step_cycle_cursor = total_cycles;
        }
    }

    pub fn take_speaker_toggle_cycles(&mut self) -> Vec<u64> {
        core::mem::take(&mut self.speaker_toggle_cycles)
    }

    fn record_bus_access_cycle(&mut self) -> Option<u64> {
        if !self.cpu_step_audio_active {
            return None;
        }

        let cycle = self.cpu_step_cycle_base + self.cpu_step_cycle_cursor as u64;
        self.cpu_step_cycle_cursor = self.cpu_step_cycle_cursor.saturating_add(1);
        self.disk2.tick(1);
        Some(cycle)
    }

    fn toggle_speaker(&mut self, cycle: Option<u64>) {
        self.speaker = !self.speaker;
        if let Some(c) = cycle {
            self.speaker_toggle_cycles.push(c);
        }
    }
}

// Memory map implementation specific for Apple II
impl Memory for Apple2Memory {
    fn read(&mut self, addr: u16) -> u8 {
        let access_cycle = self.record_bus_access_cycle();
        let mut clear_pre_write = true;
        let val = match addr {
            // Main RAM (48K)
            0x0000..=0xBFFF => self.ram[addr as usize],

            // Hardware I/O Space (Soft Switches)
            0xC000..=0xCFFF => {
                match addr {
                    // Keyboard Data (mirrored $C000-$C00F)
                    0xC000..=0xC00F => self.keyboard_latch,
                    // Keyboard Clear Strobe (mirrored $C010-$C01F)
                    0xC010..=0xC01F => {
                        let val = self.keyboard_latch;
                        self.keyboard_latch &= 0x7F; // Clear highest bit
                        val // Return the value BEFORE clearing (some routines check it)
                    }
                    // Language Card Soft Switches
                    0xC080..=0xC08F => {
                        let bank2 = (addr & 0x08) == 0;
                        let read_ram = (addr & 0x03) == 0x00 || (addr & 0x03) == 0x03;
                        let is_write_en_switch = (addr & 0x01) != 0;

                        self.lc_bank2 = bank2;
                        self.lc_read_enable = read_ram;

                        if is_write_en_switch {
                            if self.lc_pre_write_switch == addr {
                                self.lc_write_enable = true;
                            }
                            self.lc_pre_write_switch = addr;
                            clear_pre_write = false;
                        } else {
                            self.lc_write_enable = false;
                        }

                        0 // Normally this floats, return 0
                    }
                    // Disk II Controller (Slot 6)
                    0xC0E0..=0xC0EF => self.disk2.read_io(addr),
                    // Video Soft Switches ($C050 - $C057)
                    0xC050 => {
                        self.text_mode = false;
                        0
                    } // Graphics Mode
                    0xC051 => {
                        self.text_mode = true;
                        0
                    } // Text Mode
                    0xC052 => {
                        self.mixed_mode = false;
                        0
                    } // Full Screen
                    0xC053 => {
                        self.mixed_mode = true;
                        0
                    } // Mixed Mode
                    0xC054 => {
                        self.page2 = false;
                        0
                    } // Page 1
                    0xC055 => {
                        self.page2 = true;
                        0
                    } // Page 2
                    0xC056 => {
                        self.hires_mode = false;
                        0
                    } // Lo-Res
                    0xC057 => {
                        self.hires_mode = true;
                        0
                    } // Hi-Res

                    // Speaker toggle ($C030)
                    0xC030 => {
                        self.toggle_speaker(access_cycle);
                        0
                    }

                    // Slot 6 ROM
                    0xC600..=0xC6FF => self.disk2.rom[(addr - 0xC600) as usize],

                    // Pushbuttons / Joystick / Paddles
                    // $C061 (Pushbutton 0), $C062 (Pushbutton 1) -> 0x00 (Not pressed)
                    // $C064-$C067 (Analog Paddles) -> For simplicity, return 0x00 (Timeout immediately)
                    // Wait, returning 0x00 immediately for paddles might crash calibration loops.
                    // Let's return 0x00 for now, but if it crashes we might need a proper timer.
                    0xC061..=0xC067 => 0x00,

                    // For now, other I/O returns 0 (Video switches, Disk II, etc.)
                    _ => 0,
                }
            }

            // Standard System ROM or Language Card RAM (12K / 16K)
            0xD000..=0xFFFF => {
                if self.lc_read_enable {
                    if addr < 0xE000 {
                        if self.lc_bank2 {
                            self.lc_ram[(addr - 0xD000 + 0x1000) as usize]
                        } else {
                            self.lc_ram[(addr - 0xD000) as usize]
                        }
                    } else {
                        self.lc_ram[(addr - 0xE000 + 0x2000) as usize]
                    }
                } else {
                    self.rom[(addr - 0xD000) as usize]
                }
            }
        };

        if clear_pre_write {
            self.lc_pre_write_switch = 0;
        }

        val
    }

    fn write(&mut self, addr: u16, data: u8) {
        let access_cycle = self.record_bus_access_cycle();
        self.lc_pre_write_switch = 0;

        match addr {
            // Main RAM (48K)
            0x0000..=0xBFFF => {
                self.ram[addr as usize] = data;
            }

            // Hardware I/O Space (Soft Switches)
            0xC000..=0xCFFF => {
                match addr {
                    // Keyboard Clear Strobe (also on read, but write triggers too)
                    0xC010 => {
                        self.keyboard_latch &= 0x7F; // Clear highest bit
                    }
                    // Language Card Soft Switches
                    0xC080..=0xC08F => {
                        let bank2 = (addr & 0x08) == 0;
                        let read_ram = (addr & 0x03) == 0x00 || (addr & 0x03) == 0x03;

                        self.lc_bank2 = bank2;
                        self.lc_read_enable = read_ram;
                        self.lc_write_enable = false; // Writes to LC switches always write-protect
                    }
                    // Disk II Controller (Slot 6)
                    0xC0E0..=0xC0EF => {
                        self.disk2.write_io(addr, data);
                    }
                    // Video Soft Switches ($C050 - $C057)
                    0xC050 => {
                        self.text_mode = false;
                    } // Graphics Mode
                    0xC051 => {
                        self.text_mode = true;
                    } // Text Mode
                    0xC052 => {
                        self.mixed_mode = false;
                    } // Full Screen
                    0xC053 => {
                        self.mixed_mode = true;
                    } // Mixed Mode
                    0xC054 => {
                        self.page2 = false;
                    } // Page 1
                    0xC055 => {
                        self.page2 = true;
                    } // Page 2
                    0xC056 => {
                        self.hires_mode = false;
                    } // Lo-Res
                    0xC057 => {
                        self.hires_mode = true;
                    } // Hi-Res

                    // Speaker toggle ($C030)
                    0xC030 => {
                        self.toggle_speaker(access_cycle);
                    }

                    _ => {}
                }
            }

            // Language Card RAM (12K / 16K)
            0xD000..=0xFFFF => {
                if self.lc_write_enable {
                    if addr < 0xE000 {
                        if self.lc_bank2 {
                            self.lc_ram[(addr - 0xD000 + 0x1000) as usize] = data;
                        } else {
                            self.lc_ram[(addr - 0xD000) as usize] = data;
                        }
                    } else {
                        self.lc_ram[(addr - 0xE000 + 0x2000) as usize] = data;
                    }
                }
            }
        }
    }
}
