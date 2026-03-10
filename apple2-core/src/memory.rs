extern crate alloc;
use crate::disk2::Disk2;

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
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        let copy_len = data.len().min(self.rom.len());
        self.rom[..copy_len].copy_from_slice(&data[..copy_len]);
    }
}

// Memory map implementation specific for Apple II
impl Memory for Apple2Memory {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // Main RAM (48K)
            0x0000..=0xBFFF => self.ram[addr as usize],

            // Hardware I/O Space (Soft Switches)
            0xC000..=0xCFFF => {
                match addr {
                    // Keyboard Data (mirrored $C000-$C00F)
                    0xC000..=0xC00F => {
                        self.keyboard_latch
                    }
                    // Keyboard Clear Strobe (mirrored $C010-$C01F)
                    0xC010..=0xC01F => {
                        let val = self.keyboard_latch;
                        self.keyboard_latch &= 0x7F; // Clear highest bit
                        val // Return the value BEFORE clearing (some routines check it)
                    }
                    // Disk II Controller (Slot 6)
                    0xC0E0..=0xC0EF => {
                        self.disk2.read_io(addr)
                    }
                    // Video Soft Switches ($C050 - $C057)
                    0xC050 => { self.text_mode = false; 0 } // Graphics Mode
                    0xC051 => { self.text_mode = true; 0 }  // Text Mode
                    0xC052 => { self.mixed_mode = false; 0 } // Full Screen
                    0xC053 => { self.mixed_mode = true; 0 }  // Mixed Mode
                    0xC054 => { self.page2 = false; 0 }      // Page 1
                    0xC055 => { self.page2 = true; 0 }       // Page 2
                    0xC056 => { self.hires_mode = false; 0 } // Lo-Res
                    0xC057 => { self.hires_mode = true; 0 }  // Hi-Res
                    
                    // Slot 6 ROM
                    0xC600..=0xC6FF => {
                        self.disk2.rom[(addr - 0xC600) as usize]
                    }
                    // For now, other I/O returns 0 (Video switches, Disk II, etc.)
                    _ => 0,
                }
            }

            // Standard System ROM (12K)
            0xD000..=0xFFFF => self.rom[(addr - 0xD000) as usize],
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
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
                    // Disk II Controller (Slot 6)
                    0xC0E0..=0xC0EF => {
                        self.disk2.write_io(addr, data);
                    }
                    // Video Soft Switches ($C050 - $C057)
                    0xC050 => { self.text_mode = false; } // Graphics Mode
                    0xC051 => { self.text_mode = true; }  // Text Mode
                    0xC052 => { self.mixed_mode = false; } // Full Screen
                    0xC053 => { self.mixed_mode = true; }  // Mixed Mode
                    0xC054 => { self.page2 = false; }      // Page 1
                    0xC055 => { self.page2 = true; }       // Page 2
                    0xC056 => { self.hires_mode = false; } // Lo-Res
                    0xC057 => { self.hires_mode = true; }  // Hi-Res
                    
                    // TODO: Speaker toggle
                    _ => {}
                }
            }

            // Try to write to ROM -> Ignore
            0xD000..=0xFFFF => {
                // ROM is Read-Only
            }
        }
    }
}

