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
    pub current_qtr_track: i32, // Precise mechanical position of the head
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
            current_qtr_track: 0,
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

            // Real Disk II hardware: The shift register holds the byte with MSB=1
            // until the next 8 bits are shifted in (approx 32 cycles).
            // We don't clear latch_valid on read; it only changes when the next byte arrives.
            if self.latch_valid {
                self.bytes_read += 1;
                return self.data_latch;
            } else {
                // If the drive just started or is between bytes
                return self.data_latch & 0x7F;
            }
        }
        
        0x00 // Floating bus
    }

    /// Write data to the Disk II I/O registers ($C0E0 - $C0EF)
    pub fn write_io(&mut self, addr: u16, _data: u8) {
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
            0x00 => { self.phases[0] = false; self.step_motor(); },
            0x01 => { self.phases[0] = true;  self.step_motor(); },
            0x02 => { self.phases[1] = false; self.step_motor(); },
            0x03 => { self.phases[1] = true;  self.step_motor(); },
            0x04 => { self.phases[2] = false; self.step_motor(); },
            0x05 => { self.phases[2] = true;  self.step_motor(); },
            0x06 => { self.phases[3] = false; self.step_motor(); },
            0x07 => { self.phases[3] = true;  self.step_motor(); },
            
            // $C0E8 / $C0E9: Motor Off / On
            0x08 => {
                if self.motor_on {
                    println!("[Disk] Motor OFF (track={})", self.current_track);
                }
                self.motor_on = false;
            },
            0x09 => {
                if !self.motor_on {
                    println!("[Disk] Motor ON");
                    self.bytes_read = 0; // reset read counter
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
        // Disk II head is attracted to all active phases.
        // We calculate the target position based on the center of all active magnets.
        // 1 track = 4 units in current_qtr_track.
        // Phase 0 -> Track 0, 2, 4... (unit 0, 8, 16...)
        // Phase 1 -> Track 0.5, 2.5... (unit 2, 10, 18...)
        // Phase 2 -> Track 1, 3, 5... (unit 4, 12, 20...)
        // Phase 3 -> Track 1.5, 3.5... (unit 6, 14, 22...)
        
        let mut sum_target = 0;
        let mut active_count = 0;
        
        for i in 0..4 {
            if self.phases[i] {
                // Find the target for this phase nearest to the current position
                let mut phase_target = (i as i32) * 2;
                
                // Adjust phase_target to be in the [current-4, current+4] range
                while phase_target - self.current_qtr_track > 4 {
                    phase_target -= 8;
                }
                while self.current_qtr_track - phase_target > 4 {
                    phase_target += 8;
                }
                
                sum_target += phase_target;
                active_count += 1;
            }
        }

        if active_count > 0 {
            let target = sum_target / active_count;
            if self.current_qtr_track != target {
                self.current_qtr_track = target;
                
                // Ensure bounds (0 to 34 tracks)
                if self.current_qtr_track < 0 { self.current_qtr_track = 0; }
                if self.current_qtr_track > 34 * 4 { self.current_qtr_track = 34 * 4; }

                let new_track = (self.current_qtr_track / 4) as usize;
                if self.current_track != new_track {
                    println!("[Disk] Track step: {} -> {} (qtr={})",
                        self.current_track, new_track, self.current_qtr_track);
                    self.current_track = new_track;
                }
            }
        }
    }
    pub fn tick(&mut self, cycles: u32) {
        if self.motor_on {
            self.cycles_since_last_byte += cycles;

            // A new nibble window arrives approx every 32 cycles.
            // However, real hardware is bit-based. If we are in read mode,
            // we skip 0-bits at the start of a byte to align with the next 1-bit.
            while self.cycles_since_last_byte >= 32 {
                self.cycles_since_last_byte -= 32;

                if self.is_disk_loaded {
                    let track = &self.tracks[self.current_track];
                    if track.length > 0 {
                        // Return the byte at the current index.
                        // In a real Disk II, if Bit 7 isn't 1, the sequencer waits (slips bits).
                        // Since our nibblized track is already byte-aligned with valid nibbles (MSB=1),
                        // we just ensure the index advances. 
                        // The key fix here is making sure latch_valid stays true for the whole window.
                        self.data_latch = track.raw_bytes[self.byte_index];
                        self.latch_valid = true; 
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
        self.phases = [false; 4];
        self.phase_index = 0;
    }
}
