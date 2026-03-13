extern crate alloc;
use alloc::vec::Vec;
use crate::nibble::{nibblize_dsk, TrackData};

pub struct Disk2 {
    pub rom: [u8; 256],
    pub motor_on: bool,
    pub drive_select: u8,
    pub write_mode: bool,
    pub load_mode: bool,
    pub current_track: usize, 
    pub tracks: Vec<TrackData>,
    pub is_disk_loaded: bool,
    pub phases: [bool; 4],
    pub current_qtr_track: i32,

    // Stable Emulation: Byte-level Sync (~32 cycles per byte)
    pub byte_index: usize,
    pub cycles_accumulator: u32,
    pub data_latch: u8,
}

impl Disk2 {
    pub fn new() -> Self {
        let mut tracks = Vec::with_capacity(35);
        for _ in 0..35 { tracks.push(TrackData::new()); }
        Self {
            rom: [0; 256], motor_on: false, drive_select: 1, write_mode: false, load_mode: false,
            current_track: 0, tracks, is_disk_loaded: false, phases: [false; 4],
            current_qtr_track: 0,
            byte_index: 0, cycles_accumulator: 0,
            data_latch: 0,
        }
    }

    pub fn load_boot_rom(&mut self, rom_data: &[u8]) {
        let len = rom_data.len().min(256);
        self.rom[..len].copy_from_slice(&rom_data[..len]);
    }

    pub fn load_disk(&mut self, disk_data: &[u8]) {
        if disk_data.len() == 143360 {
            self.tracks = nibblize_dsk(disk_data).into();
            self.is_disk_loaded = true;
        }
    }

    pub fn read_io(&mut self, addr: u16) -> u8 {
        if addr != 0xC0EC {
            self.handle_io(addr);
        } else {
            self.write_mode = false;
        }

        let switch = addr & 0x0F;

        if self.motor_on {
            if switch == 0x0C {
                // $C0EC (Q6_OFF): Read Data
                if !self.is_disk_loaded { return 0x00; }
                if self.load_mode {
                    // Shifting data out in write mode; reading shouldn't destroy latch
                    return 0x00;
                }
                let val = self.data_latch;
                self.data_latch &= 0x7F; // Destructive read
                return val;
            } else if switch == 0x0E {
                // $C0EE (Q7_OFF): Sense WP
                // Return 0x00 if not write-protected, 0x80 if protected.
                // It's typically polled after $C08D (Q6_ON).
                if self.write_mode {
                    return 0x00; // Not write-protected
                }
                return 0x00;
            }
        }
        0x00
    }

    pub fn write_io(&mut self, addr: u16, data: u8) { 
        self.handle_io(addr); 
        
        // When Q7=1 (load_mode) and Q6=1 (write_mode), we are in Write Load state.
        // The data bus is loaded into the controller's data register.
        if self.load_mode && self.write_mode {
            self.data_latch = data;
        }
    }

    fn handle_io(&mut self, addr: u16) {
        let switch = (addr & 0x0F) as usize;
        match switch {
            0x00..=0x07 => { 
                let phase = switch >> 1;
                let on = (switch & 1) != 0;
                if on != self.phases[phase] {
                    self.phases[phase] = on; 
                    self.step_motor(); 
                }
            }
            0x08 => self.motor_on = false,
            0x09 => self.motor_on = true,
            0x0A => self.drive_select = 1,
            0x0B => self.drive_select = 2,
            0x0C => self.write_mode = false,
            0x0D => self.write_mode = true,
            0x0E => self.load_mode = false,
            0x0F => self.load_mode = true,
            _ => {}
        }
    }
    
    fn step_motor(&mut self) {
        let mut target_qtr = self.current_qtr_track;
        for p in 0..4 {
            if self.phases[p] {
                let p_pos = (p as i32) * 2;
                let mut diff = p_pos - (target_qtr % 8);
                if diff > 4 { diff -= 8; }
                if diff < -4 { diff += 8; }
                target_qtr += diff;
            }
        }

        if target_qtr != self.current_qtr_track {
            self.current_qtr_track = target_qtr;
            if self.current_qtr_track < 0 { self.current_qtr_track = 0; }
            if self.current_qtr_track > 34 * 4 { self.current_qtr_track = 34 * 4; }
            let nt = (self.current_qtr_track / 4) as usize;
            if self.current_track != nt {
                self.current_track = nt;
                self.byte_index = 0; 
            }
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        if self.motor_on && self.is_disk_loaded {
            self.cycles_accumulator += cycles;
            while self.cycles_accumulator >= 32 {
                self.cycles_accumulator -= 32;
                let track = &mut self.tracks[self.current_track];
                if track.length > 0 {
                    if self.write_mode && self.load_mode {
                        // Q6=1, Q7=1: Write Data to disk surface
                        track.raw_bytes[self.byte_index] = self.data_latch;
                    } else if !self.write_mode {
                        // Q6=0: Read mode
                        self.data_latch = track.raw_bytes[self.byte_index];
                    }
                    // Note: If Q6=1 and Q7=0 (Sense WP), do neither read nor write to surface.
                    self.byte_index = (self.byte_index + 1) % track.length;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.motor_on = false; self.current_track = 0; self.byte_index = 0;
        self.cycles_accumulator = 0; self.data_latch = 0;
        self.phases = [false; 4]; self.current_qtr_track = 0;
    }
}