extern crate alloc;
use crate::nibble::{TrackData, nibblize_dsk};
use alloc::vec::Vec;

pub struct Disk2 {
    pub rom: [u8; 256],
    pub motor_on: bool, // This is the software-controlled flip-flop ($C0E8/$C0E9)
    pub motor_timer_cycles: u32, // Hardware 1-second timer re-triggered by any I/O access
    pub spin_up_cycles: u32,     // 150ms delay before data is valid
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
    pub read_shift_register: u8,
    pub read_bit_phase: u8,
    pub read_rotation_accumulator: usize,
    pub bitstream_read_mode: bool,
    pub read_stream_position: usize,
    pub defer_read_latch_update: bool,
    pub prologue_sync_tweak: bool,
    pub prologue_sync_bytes_remaining: u8,
    pub recent_published_bytes: [u8; 3],
    pub pending_read_latch: Option<u8>,
    pub write_ready: bool,
    pub write_bit_phase: u8,
    pub is_dirty: bool,
}

impl Disk2 {
    pub fn new() -> Self {
        let mut tracks = Vec::with_capacity(40);
        for _ in 0..40 {
            tracks.push(TrackData::new());
        }
        Self {
            rom: [0; 256],
            motor_on: false,
            motor_timer_cycles: 0,
            spin_up_cycles: 0,
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
            read_shift_register: 0,
            read_bit_phase: 0,
            read_rotation_accumulator: 0,
            bitstream_read_mode: false,
            read_stream_position: 0,
            defer_read_latch_update: false,
            prologue_sync_tweak: false,
            prologue_sync_bytes_remaining: 0,
            recent_published_bytes: [0; 3],
            pending_read_latch: None,
            write_ready: false,
            write_bit_phase: 0,
            is_dirty: false,
        }
    }

    pub fn set_defer_read_latch_update(&mut self, enabled: bool) {
        self.defer_read_latch_update = enabled;
        self.pending_read_latch = None;
    }

    pub fn set_prologue_sync_tweak(&mut self, enabled: bool) {
        self.prologue_sync_tweak = enabled;
        self.prologue_sync_bytes_remaining = 0;
        self.recent_published_bytes = [0; 3];
    }

    pub fn set_bitstream_read_mode(&mut self, enabled: bool) {
        self.bitstream_read_mode = enabled;
        self.read_stream_position = 0;
        self.byte_index = 0;
    }

    fn note_published_byte(&mut self, value: u8) {
        self.recent_published_bytes[0] = self.recent_published_bytes[1];
        self.recent_published_bytes[1] = self.recent_published_bytes[2];
        self.recent_published_bytes[2] = value;

        if self.prologue_sync_tweak && self.recent_published_bytes == [0xD5, 0xAA, 0x96] {
            self.prologue_sync_bytes_remaining = 8;
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
            self.reset_rotation_state();
        }
    }

    pub fn read_io(&mut self, addr: u16) -> u8 {
        // Soft switches update state on both read and write
        self.handle_io(addr);

        let switch = addr & 0x0F;

        // Drive motor is active if the flip-flop is on OR the 1-second timer is running
        if self.motor_on || self.motor_timer_cycles > 0 {
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
        let switch = (addr & 0x0F) as usize;

        // Any access to $C0E0-$C0EF re-triggers the 1-second motor-on timer
        self.motor_timer_cycles = 1_023_000; // ~1 second @ 1.023 MHz

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
            0x09 => {
                // If motor was completely off, start shorter spin-up delay
                if !self.motor_on && self.motor_timer_cycles == 0 {
                    self.spin_up_cycles = 51_150; // 50ms @ 1.023 MHz
                }
                self.motor_on = true;
            }
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

        // Expanded to 40 tracks (0-39)
        self.current_qtr_track = target_qtr.clamp(0, 39 * 4);
        let next_track = (self.current_qtr_track / 4) as usize;
        if next_track != self.current_track {
            self.current_track = next_track;
            self.reset_rotation_state();
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        // Decrement timers
        self.motor_timer_cycles = self.motor_timer_cycles.saturating_sub(cycles);
        self.spin_up_cycles = self.spin_up_cycles.saturating_sub(cycles);

        let motor_is_spinning = self.motor_on || self.motor_timer_cycles > 0;

        if motor_is_spinning && self.is_disk_loaded {
            self.cycles_accumulator += cycles;

            // If we are still spinning up, no data is transferred, but disk rotates
            let can_read_write = self.spin_up_cycles == 0;

            if can_read_write && self.load_mode && !self.write_mode {
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
                    self.is_dirty = true;
                    self.write_bit_phase += 1;
                    if self.write_bit_phase >= 8 {
                        self.write_bit_phase = 0;
                        self.write_ready = true;
                        self.byte_index = (self.byte_index + 1) % track.length;
                    }
                }
            } else if can_read_write && !self.load_mode && !self.write_mode {
                while self.cycles_accumulator >= 4 {
                    self.cycles_accumulator -= 4;
                    let track = &self.tracks[self.current_track];
                    if track.length == 0 {
                        continue;
                    }

                    let track_length = track.length;
                    let read_length = track.read_length;

                    let bit = if self.bitstream_read_mode {
                        let effective_bits = Self::effective_read_bit_length(track_length, read_length);
                        let source_bit_index =
                            Self::source_bit_index_for_stream(self.read_stream_position, track_length, effective_bits);
                        let source_byte_index = source_bit_index / 8;
                        let source_bit_phase = source_bit_index % 8;
                        self.byte_index = source_byte_index;
                        let bit = (track.raw_bytes[source_byte_index] >> (7 - source_bit_phase)) & 1;
                        self.advance_read_stream_position(effective_bits);
                        bit
                    } else {
                        let bit_pos = 7 - self.read_bit_phase;
                        let bit = (track.raw_bytes[self.byte_index] >> bit_pos) & 1;
                        bit
                    };
                    self.read_shift_register = (self.read_shift_register << 1) | bit;

                    self.read_bit_phase += 1;
                    if self.read_bit_phase >= 8 {
                        self.read_bit_phase = 0;
                        let published_byte = self.read_shift_register;
                        if self.defer_read_latch_update {
                            if let Some(pending) = self.pending_read_latch.take() {
                                self.data_latch = pending;
                            }
                            if (published_byte & 0x80) != 0 {
                                self.pending_read_latch = Some(published_byte);
                            }
                        } else if (published_byte & 0x80) != 0 {
                            self.data_latch = published_byte;
                        }

                        if (published_byte & 0x80) != 0 {
                            self.note_published_byte(published_byte);
                        }
                        if !self.bitstream_read_mode {
                            self.advance_read_rotation(track_length, read_length);
                        }
                    }
                }
            } else {
                // Spinning but no data (either spin-up or other state)
                let step_cycles = if self.bitstream_read_mode { 4 } else { 32 };
                while self.cycles_accumulator >= step_cycles {
                    self.cycles_accumulator -= step_cycles;
                    let track = &self.tracks[self.current_track];
                    if track.length > 0 {
                        if self.bitstream_read_mode {
                            let effective_bits =
                                Self::effective_read_bit_length(track.length, track.read_length);
                            self.advance_read_stream_position(effective_bits);
                        } else {
                            let track_length = track.length;
                            let read_length = track.read_length;
                            self.advance_read_rotation(track_length, read_length);
                        }
                    }
                }
            }
        }
    }

    fn effective_read_bit_length(actual_length: usize, read_length: usize) -> usize {
        let effective_read_length = if read_length == 0 { actual_length } else { read_length };
        effective_read_length.saturating_mul(8).max(1)
    }

    fn source_bit_index_for_stream(
        read_stream_position: usize,
        actual_length: usize,
        effective_bits: usize,
    ) -> usize {
        let actual_bits = actual_length.saturating_mul(8).max(1);
        ((read_stream_position % effective_bits) * actual_bits / effective_bits) % actual_bits
    }

    fn advance_read_stream_position(&mut self, effective_bits: usize) {
        self.read_stream_position = (self.read_stream_position + 1) % effective_bits;
    }

    fn advance_read_rotation(&mut self, actual_length: usize, read_length: usize) {
        if actual_length == 0 {
            return;
        }

        if self.prologue_sync_bytes_remaining > 0 {
            self.prologue_sync_bytes_remaining -= 1;
            self.byte_index = (self.byte_index + 1) % actual_length;
            return;
        }

        let effective_read_length = if read_length == 0 { actual_length } else { read_length };
        if effective_read_length <= actual_length {
            self.byte_index = (self.byte_index + 1) % actual_length;
            return;
        }

        self.read_rotation_accumulator += actual_length;
        if self.read_rotation_accumulator >= effective_read_length {
            self.read_rotation_accumulator -= effective_read_length;
            self.byte_index = (self.byte_index + 1) % actual_length;
        }
    }

    fn reset_rotation_state(&mut self) {
        self.byte_index = 0;
        self.read_shift_register = 0;
        self.read_bit_phase = 0;
        self.read_rotation_accumulator = 0;
        self.read_stream_position = 0;
        self.prologue_sync_bytes_remaining = 0;
        self.recent_published_bytes = [0; 3];
        self.pending_read_latch = None;
        self.write_bit_phase = 0;
    }

    pub fn reset(&mut self) {
        self.motor_on = false;
        self.current_track = 0;
        self.cycles_accumulator = 0;
        self.data_latch = 0;
        self.write_ready = false;
        self.phases = [false; 4];
        self.current_qtr_track = 0;
        self.reset_rotation_state();
    }
}
