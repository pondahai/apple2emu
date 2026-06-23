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

    // Joystick / Paddle input
    pub pushbuttons: [bool; 2],
    pub paddles: [u8; 4],
    pub paddle_latch_cycle: u64,

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
            pushbuttons: [false; 2],
            paddles: [127; 4],
            paddle_latch_cycle: 0,
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
        self.pushbuttons = [false; 2];
        self.paddles = [127; 4];
        self.paddle_latch_cycle = 0;
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

    pub fn set_joystick_state(&mut self, x: u8, y: u8, button0: bool, button1: bool) {
        self.paddles[0] = x;
        self.paddles[1] = y;
        self.pushbuttons[0] = button0;
        self.pushbuttons[1] = button1;
    }

    fn canonical_lc_switch(addr: u16) -> u16 {
        0xC080 | ((addr - 0xC080) & 0x000B)
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

    fn current_io_cycle(&self, access_cycle: Option<u64>) -> u64 {
        access_cycle.unwrap_or(self.cpu_step_cycle_base + self.cpu_step_cycle_cursor as u64)
    }

    fn latch_paddles(&mut self, access_cycle: Option<u64>) {
        self.paddle_latch_cycle = self.current_io_cycle(access_cycle);
    }

    fn read_pushbutton(&self, index: usize) -> u8 {
        if self.pushbuttons[index] {
            0x80
        } else {
            0x00
        }
    }

    fn paddle_timeout_cycles(value: u8) -> u64 {
        8 + (value as u64 * 11)
    }

    fn read_paddle(&self, index: usize, access_cycle: Option<u64>) -> u8 {
        let elapsed = self.current_io_cycle(access_cycle).saturating_sub(self.paddle_latch_cycle);
        if elapsed < Self::paddle_timeout_cycles(self.paddles[index]) {
            0x80
        } else {
            0x00
        }
    }

    /// Address the video scanner is fetching from RAM at the given CPU cycle.
    ///
    /// Ported from AppleWin's `VideoGetScannerAddress`, which implements Jim
    /// Sather's model (Understanding the Apple IIe, ch.5). NTSC timing: 65
    /// horizontal clocks per scan line, 262 scan lines per frame. The scanner
    /// keeps generating addresses during HBL/VBL, which is exactly why the
    /// floating bus is useful as a randomness source — its value tracks the
    /// beam position and so changes from one read to the next.
    fn video_scanner_address(&self, cycle: u64) -> u16 {
        const H_CLOCKS: u64 = 65;
        const H_PE_CLOCK: u64 = 40; // last HBL clock
        const H_PRESET_CLOCK: u64 = 41; // visible region begins
        const H_CLOCK0_STATE: i32 = 0x18;
        const V_LINE0_STATE: i32 = 0x100;
        const V_PRESET_LINE: u64 = 256;
        const SCAN_LINES: u64 = 262; // NTSC
        const SCAN_CYCLES: u64 = SCAN_LINES * H_CLOCKS;

        let n_cycles = cycle % SCAN_CYCLES;

        // Horizontal counter state (h_0..h_5).
        let n_hclock = (n_cycles + H_PE_CLOCK) % H_CLOCKS;
        let mut n_hstate = H_CLOCK0_STATE + n_hclock as i32;
        if n_hclock >= H_PRESET_CLOCK {
            n_hstate -= 1;
        }
        let h_0 = (n_hstate >> 0) & 1;
        let h_1 = (n_hstate >> 1) & 1;
        let h_2 = (n_hstate >> 2) & 1;
        let h_3 = (n_hstate >> 3) & 1;
        let h_4 = (n_hstate >> 4) & 1;
        let h_5 = (n_hstate >> 5) & 1;

        // Vertical counter state (v_a..v_5).
        let n_vline = n_cycles / H_CLOCKS;
        let mut n_vstate = V_LINE0_STATE + n_vline as i32;
        if n_vline >= V_PRESET_LINE {
            n_vstate -= SCAN_LINES as i32;
        }
        let v_a = (n_vstate >> 0) & 1;
        let v_b = (n_vstate >> 1) & 1;
        let v_c = (n_vstate >> 2) & 1;
        let v_0 = (n_vstate >> 3) & 1;
        let v_1 = (n_vstate >> 4) & 1;
        let v_2 = (n_vstate >> 5) & 1;
        let v_3 = (n_vstate >> 6) & 1;
        let v_4 = (n_vstate >> 7) & 1;

        let mut hires = self.hires_mode && !self.text_mode;
        let page2 = self.page2;
        // 80STORE is an Apple //e feature; on the II/II+ it is always off.

        // In mixed mode the bottom four text rows fetch from the text page.
        if hires && self.mixed_mode && v_4 != 0 && v_2 != 0 {
            hires = false;
        }

        // Sather's 4-bit "sum" that forms address bits A3..A6.
        let addend0 = 0x0D;
        let addend1 = (h_5 << 2) | (h_4 << 1) | (h_3 << 0);
        let addend2 = (v_4 << 3) | (v_3 << 2) | (v_4 << 1) | (v_3 << 0);
        let sum = (addend0 + addend1 + addend2) & 0x0F;

        let mut addr_h: u16 = 0;
        addr_h |= (h_0 as u16) << 0;
        addr_h |= (h_1 as u16) << 1;
        addr_h |= (h_2 as u16) << 2;
        addr_h |= (sum as u16) << 3;

        if !hires {
            // Apple II/II+: during HBL the text/lores scanner addresses the
            // $1000/$1800 region (no display there, but the bus still carries it).
            if h_5 == 0 && (h_4 == 0 || h_3 == 0) {
                addr_h |= 1 << 12;
            }
        }

        let mut addr_v: u16 = 0;
        addr_v |= (v_0 as u16) << 7;
        addr_v |= (v_1 as u16) << 8;
        addr_v |= (v_2 as u16) << 9;

        // With 80STORE off: p2a selects page 1, p2b selects page 2.
        let p2a = if !page2 { 1u16 } else { 0 };
        let p2b = if page2 { 1u16 } else { 0 };

        let mut addr_p: u16 = 0;
        if hires {
            addr_v |= (v_a as u16) << 10;
            addr_v |= (v_b as u16) << 11;
            addr_v |= (v_c as u16) << 12;
            addr_p |= p2a << 13; // $2000
            addr_p |= p2b << 14; // $4000
        } else {
            addr_p |= p2a << 10; // $0400
            addr_p |= p2b << 11; // $0800
        }

        addr_p | addr_v | addr_h
    }

    /// Value seen when reading an undriven `$C0xx` location: the byte the video
    /// scanner is fetching this cycle. Games (e.g. Castle Wolfenstein) use this
    /// as their only hardware randomness source — for explosion white-noise and
    /// for combat hit/penetration rolls. Returning a constant here collapses the
    /// noise to a single tone and makes every roll land the same way.
    fn floating_bus(&self, access_cycle: Option<u64>) -> u8 {
        let cycle = self.current_io_cycle(access_cycle);
        let addr = self.video_scanner_address(cycle) as usize;
        // Every scanner address falls within the 48K main RAM; guard anyway.
        *self.ram.get(addr).unwrap_or(&0)
    }
}

// Memory map implementation specific for Apple II
impl Memory for Apple2Memory {
    fn read(&mut self, addr: u16) -> u8 {
        let access_cycle = self.record_bus_access_cycle();
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
                        let canonical = Self::canonical_lc_switch(addr);

                        self.lc_bank2 = bank2;
                        self.lc_read_enable = read_ram;

                        if is_write_en_switch {
                            // Two consecutive reads of an odd $C08x address enable LC
                            // write. The pre-write flip-flop is hardware that only
                            // responds to $C08x accesses — intervening RAM/ROM reads
                            // (including the CPU's own opcode/operand fetches between
                            // the two `LDA $C083` instructions) must NOT clear it.
                            // The previous `clear_pre_write` reset-on-every-read broke
                            // this, so the LC write-enable never latched and software
                            // RAM probes saw a 48K machine instead of 64K (Rescue
                            // Raiders bailed to its title as a result). Matches the
                            // PicoApple2 model, verified on real hardware.
                            if self.lc_pre_write_switch == canonical {
                                self.lc_write_enable = true;
                            }
                            self.lc_pre_write_switch = canonical;
                        } else {
                            self.lc_write_enable = false;
                        }

                        self.floating_bus(access_cycle) // read floats
                    }
                    // Disk II Controller (Slot 6)
                    0xC0E0..=0xC0EF => self.disk2.read_io(addr),
                    // Video Soft Switches ($C050 - $C057)
                    0xC050 => {
                        self.text_mode = false;
                        self.floating_bus(access_cycle)
                    } // Graphics Mode
                    0xC051 => {
                        self.text_mode = true;
                        self.floating_bus(access_cycle)
                    } // Text Mode
                    0xC052 => {
                        self.mixed_mode = false;
                        self.floating_bus(access_cycle)
                    } // Full Screen
                    0xC053 => {
                        self.mixed_mode = true;
                        self.floating_bus(access_cycle)
                    } // Mixed Mode
                    0xC054 => {
                        self.page2 = false;
                        self.floating_bus(access_cycle)
                    } // Page 1
                    0xC055 => {
                        self.page2 = true;
                        self.floating_bus(access_cycle)
                    } // Page 2
                    0xC056 => {
                        self.hires_mode = false;
                        self.floating_bus(access_cycle)
                    } // Lo-Res
                    0xC057 => {
                        self.hires_mode = true;
                        self.floating_bus(access_cycle)
                    } // Hi-Res

                    // Speaker toggle ($C030-$C03F). The read both clicks the
                    // speaker and returns the floating bus; noise routines read
                    // here in a timing loop and use the value as randomness.
                    0xC030..=0xC03F => {
                        self.toggle_speaker(access_cycle);
                        self.floating_bus(access_cycle)
                    }

                    // Slot 6 ROM
                    0xC600..=0xC6FF => self.disk2.rom[(addr - 0xC600) as usize],

                    // Pushbuttons / Joystick / Paddles
                    0xC061 => self.read_pushbutton(0),
                    0xC062 => self.read_pushbutton(1),
                    0xC063 => 0x00,
                    0xC064..=0xC067 => self.read_paddle((addr - 0xC064) as usize, access_cycle),
                    0xC070 => {
                        self.latch_paddles(access_cycle);
                        0x00
                    }

                    // Any other undriven I/O location reads the floating bus.
                    _ => self.floating_bus(access_cycle),
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

        val
    }

    fn write(&mut self, addr: u16, data: u8) {
        let access_cycle = self.record_bus_access_cycle();
        // Note: a plain write does NOT clear the LC pre-write flip-flop. Only an
        // even-$C08x access does (handled below). Resetting it on every write here
        // would break unlock sequences that store to RAM between the two reads.

        match addr {
            // Main RAM (48K)
            0x0000..=0xBFFF => {
                self.ram[addr as usize] = data;
            }

            // Hardware I/O Space (Soft Switches)
            0xC000..=0xCFFF => {
                match addr {
                    // Any write to $C010-$C01F clears the keyboard strobe
                    0xC010..=0xC01F => {
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

                    // Speaker toggle ($C030-$C03F)
                    0xC030..=0xC03F => {
                        self.toggle_speaker(access_cycle);
                    }
                    0xC070 => {
                        self.latch_paddles(access_cycle);
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
