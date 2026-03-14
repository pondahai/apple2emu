extern crate alloc;
use crate::nibble::{TrackData, nibblize_dsk};
use alloc::vec::Vec;

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
    pub write_ready: bool,
    pub write_bit_phase: u8,
    pub io_access_count: u64,
}

impl Disk2 {
    pub fn new() -> Self {
        let mut tracks = Vec::with_capacity(35);
        for _ in 0..35 {
            tracks.push(TrackData::new());
        }
        Self {
            rom: [0; 256],
            motor_on: false,
            drive_select: 1,
            write_mode: false,
            load_mode: false,
            current_track: 0,
            tracks,
            is_disk_loaded: false,
            phases: [false; 4],
            current_qtr_track: 0,
            byte_index: 0,
            cycles_accumulator: 0,
            data_latch: 0,
            write_ready: false,
            write_bit_phase: 0,
            io_access_count: 0,
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
        // Soft switches update state on both read and write
        self.handle_io(addr);

        let switch = addr & 0x0F;

        if self.motor_on {
            if switch == 0x0C {
                // $C0EC (Q6_OFF): Read Data
                if !self.is_disk_loaded {
                    return 0x00;
                }

                // If Q7 is ON (load_mode), we are shifting data out,
                // do NOT destructively read the latch.
                if self.load_mode {
                    let ready = self.write_ready;
                    self.write_ready = false;
                    return if ready { 0x80 } else { 0x00 };
                }

                let val = self.data_latch;
                // Reading data when Q6=0 and Q7=0 clears the MSB (destructive read)
                self.data_latch &= 0x7F;
                return val;
            } else if switch == 0x0D {
                // Q7=0,Q6=1 write-protect sense path.
                // Current emulator defaults to writable media.
                return 0x00;
            }
        }

        // Default return for other switches / motor off
        // Usually returning random bus noise, but 0x00 is safe.
        0x00
    }

    pub fn write_io(&mut self, addr: u16, data: u8) {
        self.handle_io(addr);
        let switch = addr & 0x0F;

        // When Q7=1 (load_mode) and Q6=1 (write_mode), we are in Write Load state.
        // The data bus is loaded into the controller's data register.
        // Latch loads on Q6_ON ($C0ED) writes.
        if self.load_mode && self.write_mode && switch == 0x0D {
            self.data_latch = data;
        }
    }

    fn handle_io(&mut self, addr: u16) {
        self.io_access_count = self.io_access_count.wrapping_add(1);
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
            0x0C => {
                if self.load_mode && self.write_mode {
                    self.write_bit_phase = 0;
                }
                self.write_mode = false;
            }
            0x0D => self.write_mode = true,
            0x0E => self.load_mode = false,
            0x0F => self.load_mode = true,
            _ => {}
        }
    }

    fn step_motor(&mut self) {
        let phase_mask = (self.phases[0] as u8)
            | ((self.phases[1] as u8) << 1)
            | ((self.phases[2] as u8) << 2)
            | ((self.phases[3] as u8) << 3);

        // Canonical Disk II head positions across one 8-quarter-track cycle:
        // single-coil states land on even positions, adjacent dual-coil states on odd positions.
        let target_mod = match phase_mask {
            0b0001 => Some(0),
            0b0011 => Some(1),
            0b0010 => Some(2),
            0b0110 => Some(3),
            0b0100 => Some(4),
            0b1100 => Some(5),
            0b1000 => Some(6),
            0b1001 => Some(7),
            _ => None,
        };

        let Some(target_mod) = target_mod else {
            return;
        };

        let base = self.current_qtr_track.div_euclid(8) * 8;
        let candidates = [
            base + target_mod,
            base + target_mod - 8,
            base + target_mod + 8,
        ];
        let mut target_qtr = self.current_qtr_track;
        let mut best_diff = i32::MAX;

        for candidate in candidates {
            let diff = (candidate - self.current_qtr_track).abs();
            if diff < best_diff {
                best_diff = diff;
                target_qtr = candidate;
            }
        }

        self.current_qtr_track = target_qtr.clamp(0, 34 * 4);
        self.current_track = (self.current_qtr_track / 4) as usize;
    }

    pub fn tick(&mut self, cycles: u32) {
        if self.motor_on && self.is_disk_loaded {
            self.cycles_accumulator += cycles;

            if self.load_mode && !self.write_mode {
                // Q7=1,Q6=0: shift write stream at bit granularity (4 cycles/bit).
                while self.cycles_accumulator >= 4 {
                    self.cycles_accumulator -= 4;
                    let track = &mut self.tracks[self.current_track];
                    if track.length == 0 {
                        continue;
                    }
                    let bit_pos = 7 - self.write_bit_phase;
                    let bit = (self.data_latch >> bit_pos) & 1;
                    if bit != 0 {
                        track.raw_bytes[self.byte_index] |= 1 << bit_pos;
                    } else {
                        track.raw_bytes[self.byte_index] &= !(1 << bit_pos);
                    }
                    self.write_bit_phase += 1;
                    if self.write_bit_phase >= 8 {
                        self.write_bit_phase = 0;
                        self.write_ready = true;
                        self.byte_index = (self.byte_index + 1) % track.length;
                    }
                }
            } else {
                while self.cycles_accumulator >= 32 {
                    self.cycles_accumulator -= 32;
                    let track = &mut self.tracks[self.current_track];
                    if track.length > 0 {
                        if !self.write_mode {
                            // Q7 = 0, Q6 = 0: Read Mode
                            self.data_latch = track.raw_bytes[self.byte_index];
                        }
                        // Other states only advance rotational position.
                        self.byte_index = (self.byte_index + 1) % track.length;
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.motor_on = false;
        self.current_track = 0;
        self.byte_index = 0;
        self.cycles_accumulator = 0;
        self.data_latch = 0;
        self.write_ready = false;
        self.write_bit_phase = 0;
        self.phases = [false; 4];
        self.current_qtr_track = 0;
        self.io_access_count = 0;
    }
}
