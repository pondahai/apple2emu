extern crate alloc;
use alloc::vec::Vec;
use crate::nibble::{nibblize_dsk, TrackData};

pub struct Disk2 {
    // Hardware ROM (Slot 6)
    pub rom: [u8; 256],

    // Soft switches state
    pub motor_on: bool,
    pub drive_select: u8, // 1 or 2
    pub write_mode: bool, // Q6
    pub load_mode: bool,  // Q7

    // Physical drive state (Simplified)
    pub current_track: usize, 
    pub current_sector: usize,
    pub byte_index: usize,
    pub bytes_read: u64,
    pub read_delay: u8,

    pub tracks: Vec<TrackData>,
    pub is_disk_loaded: bool,

    // Phase stepper motors (0-3) magnetic states
    pub phases: [bool; 4],
    pub phase_index: u8, // 0..3 representing the current magnetic alignment

    // Timing
    pub cycles_since_last_byte: u32,
    pub data_latch: u8,
    pub latch_valid: bool,
    pub latch_valid_cycles: i32, // Cycles remaining where the current latch bit 7 is set
}

impl Disk2 {
    pub fn new() -> Self {
        let mut tracks = Vec::with_capacity(35);
        for _ in 0..35 { tracks.push(TrackData::new()); }
        
        Self {
            rom: [0; 256],
            motor_on: false,
            drive_select: 1,
            write_mode: false,
            load_mode: false,
            current_track: 0,
            current_sector: 0,
            byte_index: 0,
            bytes_read: 0,
            read_delay: 0,
            tracks,
            is_disk_loaded: false,
            phases: [false; 4],
            phase_index: 0,
            cycles_since_last_byte: 0,
            data_latch: 0,
            latch_valid: false,
            latch_valid_cycles: 0,
        }
    }

    pub fn load_boot_rom(&mut self, rom_data: &[u8]) {
        let len = rom_data.len().min(256);
        self.rom[..len].copy_from_slice(&rom_data[..len]);
    }

    pub fn load_disk(&mut self, disk_data: &[u8]) {
        if disk_data.len() == 143360 {
            // Convert standard 140KB block array to Apple II expected physical track data
            self.tracks = nibblize_dsk(disk_data).into();
            self.is_disk_loaded = true;
        }
    }

    pub fn read_io(&mut self, addr: u16) -> u8 {
        self.handle_io(addr);
        
        // When reading from the data register ($C0EC), return the current disk byte
        if self.motor_on && addr == 0xC0EC {
            if !self.is_disk_loaded { return 0x00; }
            
            let track_len = self.tracks[self.current_track].length;
            if track_len == 0 { return 0x00; }

            // Return the latched byte. 
            // In Disk II hardware, bit 7 is set when a full nibble is shifted in. 
            // We clear our valid window immediately on read so the CPU doesn't see 
            // the same byte twice and get confused expecting the *next* byte.
            if self.latch_valid_cycles > 0 {
                self.latch_valid_cycles = 0; // Clear immediately on read!
                return self.data_latch; // Bit 7 is already set in GCR bytes
            } else {
                return self.data_latch & 0x7F; // Clear bit 7 to signal "not ready yet"
            }
        }
        
        0x00 // Floating bus
    }

    /// Write data to the Disk II I/O registers ($C0E0 - $C0EF)
    pub fn write_io(&mut self, addr: u16, data: u8) {
        self.handle_io(addr);
        
        if self.motor_on && self.write_mode {
            if addr == 0xC0ED {
                // TODO: Write GCR encoded Nibble sequence to the current track
            }
        }
    }

    fn handle_io(&mut self, addr: u16) {
        let switch = addr & 0x0F; // 0..15
        
        match switch {
            // $C0E0 - $C0E7: Stepper Motors. Even = OFF, Odd = ON
            0x00 => self.phases[0] = false,
            0x01 => { self.phases[0] = true; self.step_motor(); },
            0x02 => self.phases[1] = false,
            0x03 => { self.phases[1] = true; self.step_motor(); },
            0x04 => self.phases[2] = false,
            0x05 => { self.phases[2] = true; self.step_motor(); },
            0x06 => self.phases[3] = false,
            0x07 => { self.phases[3] = true; self.step_motor(); },
            
            // $C0E8 / $C0E9: Motor Off / On
            0x08 => {
                if self.motor_on {
                    // println!("Disk II: Motor turned OFF");
                }
                self.motor_on = false;
            },
            0x09 => {
                if !self.motor_on {
                    // println!("Disk II: Motor turned ON");
                }
                self.motor_on = true;
            },
            
            // $C0EA / $C0EB: Drive Select 1 / 2
            0x0A => self.drive_select = 1,
            0x0B => self.drive_select = 2,
            
            // $C0EC / $C0ED: Q6 (Read/Write Mode)
            0x0C => self.write_mode = false,
            0x0D => self.write_mode = true,
            
            // $C0EE / $C0EF: Q7 (Register setup)
            0x0E => self.load_mode = false,
            0x0F => self.load_mode = true,
            
            _ => {}
        }
    }
    
    fn step_motor(&mut self) {
        // Find which phase is active.
        let mut active_phase = 0;
        for i in 0..4 {
            if self.phases[i] {
                active_phase = i as u8;
                break;
            }
        }

        let diff = (active_phase as i8) - (self.phase_index as i8);
        self.phase_index = active_phase;

        let mut current_qtr_track = (self.current_track * 2) as i32;

        if diff == 1 || diff == -3 {
            // Step in (towards higher track)
            current_qtr_track += 1;
        } else if diff == -1 || diff == 3 {
            // Step out (towards track 0)
            current_qtr_track -= 1;
        }

        if current_qtr_track < 0 {
            current_qtr_track = 0;
        }
        if current_qtr_track > 35 * 2 {
            current_qtr_track = 35 * 2;
        }

        let new_track = (current_qtr_track / 2) as usize;
        
        // Debug
        // if self.current_track != new_track {
        // }
        // We probably don't need to spam step traces, but let's record the movement
        self.current_track = new_track;
    }
    pub fn tick(&mut self, cycles: u32) {
        if self.motor_on {
            self.cycles_since_last_byte += cycles;
            if self.latch_valid_cycles > 0 {
                self.latch_valid_cycles -= cycles as i32;
            }

            while self.cycles_since_last_byte >= 32 {
                self.cycles_since_last_byte -= 32;

                if self.is_disk_loaded {
                    let track = &self.tracks[self.current_track];
                    if track.length > 0 {
                        self.data_latch = track.raw_bytes[self.byte_index];
                        self.latch_valid_cycles = 32; // Full 32 cycles window
                        self.byte_index = (self.byte_index + 1) % track.length;
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.motor_on = false;
        self.drive_select = 1;
        self.write_mode = false;
        self.load_mode = false;
        self.current_track = 0;
        self.byte_index = 0;
        self.cycles_since_last_byte = 0;
        self.data_latch = 0;
        self.latch_valid = false;
        self.latch_valid_cycles = 0;
        self.phases = [false; 4];
        self.phase_index = 0;
    }
}
