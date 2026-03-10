use std::time::{Duration, Instant};
use minifb::{Key, Window, WindowOptions};
use apple2_core::machine::Apple2Machine;
use apple2_core::video::{Video, SCREEN_WIDTH, SCREEN_HEIGHT};
use apple2_core::memory::Memory;
use rodio::{OutputStream, Sink};

fn main() {
    println!("Starting Apple II Emulator targeting Windows (minifb) and core no_std...");

    // Create the Windows window
    let mut window = Window::new(
        "Apple II Emulator (Rust no_std core)",
        SCREEN_WIDTH * 2,  // Scale up 2x for visibility
        SCREEN_HEIGHT * 2,
        WindowOptions::default(),
    ).unwrap();

    // Limit to ~60 FPS
    window.set_target_fps(60);

    // Initialize the emulator core
    let mut machine = Apple2Machine::new();
    let mut video = Video::new();

    // Setup an audio stream
    let audio_device = OutputStream::try_default();
    let (mut _stream, mut sink) = (None, None);
    if let Ok((s, sh)) = audio_device {
        _stream = Some(s);
        if let Ok(sk) = Sink::try_new(&sh) {
            sink = Some(sk);
        }
    } else {
        println!("Warning: Could not initialize audio output.");
    }

    // The downloaded ROM is ~20KB. Typical layout for these dumps:
    // Load the correct Apple II+ ROM set (341-0011 through 341-0020)
    // Downloaded from mirrors.apple2.org.za, merged into a single 12KB file
    let mut char_rom = [0x55u8; 2048]; // Checkerboard fallback
    if let Ok(rom_file) = std::fs::read("../roms/APPLE2PLUS.ROM") {
        println!("Loaded Apple II+ ROM: {} bytes", rom_file.len());
        if rom_file.len() >= 12288 {
            let start = rom_file.len() - 12288;
            machine.load_rom(&rom_file[start..]);
        }
    } else {
        println!("Warning: Could not open ../roms/APPLE2PLUS.ROM. Using empty memory.");
    }
    
    // Apple II+ Character ROM (341-0036 Rev. 7)
    if let Ok(char_file) = std::fs::read("../roms/341-0036.bin") {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            println!("Loaded Character ROM (341-0036): 2048 bytes");
        }
    } else if let Ok(char_file) = std::fs::read("../roms/extracted_2048_152.bin") {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            println!("Loaded Character ROM (fallback): 2048 bytes");
        }
    }
    
    // Load Disk II Boot ROM into Slot 6
    if let Ok(disk_rom) = std::fs::read("../roms/extracted_256_150.bin") {
        if disk_rom.len() == 256 {
            machine.mem.disk2.load_boot_rom(&disk_rom);
            println!("Loaded Disk II Boot ROM (Slot 6): 256 bytes");
        }
    } else {
        println!("Warning: Could not open Disk II Boot ROM");
    }

    // Load DOS 3.3 MASTER.DSK
    let dsk_path = r"C:\Users\Dell\Downloads\AppleWin1.30.18.0\MASTER.DSK";
    if let Ok(disk_image) = std::fs::read(dsk_path) {
        if disk_image.len() == 143360 {
            machine.mem.disk2.load_disk(&disk_image);
            println!("Loaded MASTER.DSK floppy image: 140KB");
        } else {
            println!("Warning: MASTER.DSK size mismatch: {}", disk_image.len());
        }
    } else {
        println!("Warning: Could not open MASTER.DSK at {}", dsk_path);
    }

    // Initialize Memory mapped states and then Reset CPU
    machine.reset();

    // Normal boot: let the Autostart ROM initialize the system
    // User can type PR#6 to boot the Disk II
    // machine.cpu.pc = 0xC600;

    println!("CPU Reset Vector: {:04X} (Normal boot)", machine.cpu.pc);

    let mut last_cycle = Instant::now();
    let mut key_queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
    let mut last_keys: Vec<Key> = Vec::new();
    let mut clipboard = arboard::Clipboard::new().ok();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        
        // 1. Handle Input
        // Extract all currently pressed keys
        let current_keys = window.get_keys();
        let mut keys = Vec::new();
        for &k in &current_keys {
            if !last_keys.contains(&k) {
                keys.push(k);
            }
        }
        last_keys = current_keys;
        let ctrl_down = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        let shift_down = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        
        for &key in keys.iter() {
            let ascii = match key {
                // Letters are always uppercase on Apple II/II+ unless we add lower case mod, but Shift+Letter is still uppercase
                Key::A => if ctrl_down { 0x01 } else { b'A' },
                Key::B => if ctrl_down { 0x02 } else { b'B' },
                Key::C => if ctrl_down { 0x03 } else { b'C' },
                Key::D => if ctrl_down { 0x04 } else { b'D' },
                Key::E => if ctrl_down { 0x05 } else { b'E' },
                Key::F => if ctrl_down { 0x06 } else { b'F' },
                Key::G => if ctrl_down { 0x07 } else { b'G' },
                Key::H => if ctrl_down { 0x08 } else { b'H' },
                Key::I => if ctrl_down { 0x09 } else { b'I' },
                Key::J => if ctrl_down { 0x0A } else { b'J' },
                Key::K => if ctrl_down { 0x0B } else { b'K' },
                Key::L => if ctrl_down { 0x0C } else { b'L' },
                Key::M => if ctrl_down { 0x0D } else { b'M' },
                Key::N => if ctrl_down { 0x0E } else { b'N' },
                Key::O => if ctrl_down { 0x0F } else { b'O' },
                Key::P => if ctrl_down { 0x10 } else { b'P' },
                Key::Q => if ctrl_down { 0x11 } else { b'Q' },
                Key::R => if ctrl_down { 0x12 } else { b'R' },
                Key::S => if ctrl_down { 0x13 } else { b'S' },
                Key::T => if ctrl_down { 0x14 } else { b'T' },
                Key::U => if ctrl_down { 0x15 } else { b'U' },
                Key::V => {
                    if ctrl_down {
                        if let Some(cb) = &mut clipboard {
                            if let Ok(text) = cb.get_text() {
                                for c in text.chars() {
                                    if c == '\r' { continue; }
                                    let mut ascii = c.to_ascii_uppercase() as u8;
                                    if ascii == b'\n' {
                                        ascii = 0x0D; // Apple II Return
                                    }
                                    // Make sure it's valid printable ASCII or Return/Backspace
                                    if (ascii >= 0x20 && ascii <= 0x7F) || ascii == 0x0D || ascii == 0x08 {
                                        key_queue.push_back(ascii);
                                    }
                                }
                            }
                        }
                        0 // Return 0 since characters are already queued
                    } else {
                        b'V'
                    }
                },
                Key::W => if ctrl_down { 0x17 } else { b'W' },
                Key::X => if ctrl_down { 0x18 } else { b'X' },
                Key::Y => if ctrl_down { 0x19 } else { b'Y' },
                Key::Z => if ctrl_down { 0x1A } else { b'Z' },
                
                // Numbers and Symbols
                Key::Key0 => if shift_down { b')' } else { b'0' }, 
                Key::Key1 => if shift_down { b'!' } else { b'1' }, 
                Key::Key2 => if shift_down { b'@' } else { b'2' }, 
                Key::Key3 => if shift_down { b'#' } else { b'3' }, 
                Key::Key4 => if shift_down { b'$' } else { b'4' }, 
                Key::Key5 => if shift_down { b'%' } else { b'5' },
                Key::Key6 => if shift_down { b'^' } else { b'6' }, 
                Key::Key7 => if shift_down { b'&' } else { b'7' }, 
                Key::Key8 => if shift_down { b'*' } else { b'8' }, 
                Key::Key9 => if shift_down { b'(' } else { b'9' },
                
                Key::Minus => if shift_down { b'_' } else { b'-' }, 
                Key::Equal => if shift_down { b'+' } else { b'=' }, 
                Key::Comma => if shift_down { b'<' } else { b',' }, 
                Key::Period => if shift_down { b'>' } else { b'.' }, 
                Key::Slash => if shift_down { b'?' } else { b'/' }, 
                Key::Semicolon => if shift_down { b':' } else { b';' },
                Key::Apostrophe => if shift_down { b'"' } else { b'\'' },
                
                // Control Keys
                Key::Space => b' ', Key::Enter => 0x0D, Key::Escape => 0x1B, Key::Backspace => 0x08,
                
                _ => 0,
            };
            if ascii != 0 {
                key_queue.push_back(ascii);
            }
        }
        // Feed next key from queue only if the Apple II has consumed the previous one
        // (keyboard_latch bit 7 is clear = consumed)
        if (machine.mem.keyboard_latch & 0x80) == 0 {
            if let Some(ascii) = key_queue.pop_front() {
                machine.mem.keyboard_latch = 0x80 | ascii;
            }
        }

        // 2. Emulate CPU execution for one Frame (~17,050 1MHz cycles per 1/60th sec)
        let mut frame_cycles = 0;
        let mut audio_samples: Vec<f32> = Vec::with_capacity(750);
        let mut unprocessed_cycles = 0.0;
        let cycles_per_sample = 1_023_000.0 / 44100.0;

        while frame_cycles < 17_050 {
            let pc_before = machine.cpu.pc;
            let cycles = machine.step();
            frame_cycles += cycles;

            // Generate audio samples based on CPU cycles passed
            unprocessed_cycles += cycles as f32;
            while unprocessed_cycles >= cycles_per_sample {
                let sample_val = if machine.mem.speaker { 0.1 } else { -0.1 };
                audio_samples.push(sample_val);
                unprocessed_cycles -= cycles_per_sample;
            }

            // Detect jumps TO or FROM the C600 range, or unusual PC values
            if machine.cpu.pc >= 0xC600 && machine.cpu.pc <= 0xC6FF && pc_before < 0xC600 {
                println!(">>> ENTERED C600 boot ROM from PC={:04X}", pc_before);
            }
            if pc_before >= 0xC600 && pc_before <= 0xC6FF && machine.cpu.pc < 0xC600 {
                println!(">>> LEFT C600 boot ROM to PC={:04X}", machine.cpu.pc);
            }
        }

        // Output audio frame
        if let Some(s) = &sink {
            if !audio_samples.is_empty() {
                let source = rodio::buffer::SamplesBuffer::new(1, 44100, audio_samples);
                s.append(source);
                // Keep the queue from lagging behind realtime
                if s.len() > 3 {
                    s.clear();
                }
            }
        }

        // Detect JMP to $0801 (end of Boot ROM)
        // Need to capture last PC after the loop
        let cpu_pc = machine.cpu.pc;
        if cpu_pc == 0x0801 {
            println!(">>> JUMPED to DOS Boot Sector code at $0801");
        }

        // Periodic Debug
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT % 60 == 0 {
                println!("Disk Status: Track={}, ByteIndex={}, CyclesSinceByte={}",
                    machine.mem.disk2.current_track,
                    machine.mem.disk2.byte_index,
                    machine.mem.disk2.cycles_since_last_byte);
                println!("Memory at $0800: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                    machine.mem.read(0x0800), machine.mem.read(0x0801), machine.mem.read(0x0802), machine.mem.read(0x0803),
                    machine.mem.read(0x0804), machine.mem.read(0x0805), machine.mem.read(0x0806), machine.mem.read(0x0807));
                println!("CPU PC: {:04X} A:{:02X} X:{:02X} Y:{:02X} S:{:02X} P:{:02X} Code: {:02X} {:02X} {:02X}", 
                    machine.cpu.pc, machine.cpu.a, machine.cpu.x, machine.cpu.y, machine.cpu.sp, 
                    machine.cpu.status.to_byte(),
                    machine.mem.read(machine.cpu.pc),
                    machine.mem.read(machine.cpu.pc.wrapping_add(1)),
                    machine.mem.read(machine.cpu.pc.wrapping_add(2)));
            }
        }

        // 3. Render the Screen
        if machine.mem.text_mode {
            video.render_text_frame(&machine.mem, &char_rom);
        } else if machine.mem.hires_mode {
            video.render_hires_frame(&machine.mem, &char_rom);
        } else {
            video.render_lores_frame(&machine.mem, &char_rom);
        }

        window
            .update_with_buffer(&video.frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT)
            .unwrap();

        // Debug: Print the 1st line of the screen to console to see if it booted!
        if last_cycle.elapsed().as_secs() >= 1 {
            last_cycle = Instant::now();
            // Always print $0800 to verify boot sector decoding
            println!("Memory at $0800: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}", 
                     machine.mem.ram[0x0800], machine.mem.ram[0x0801], 
                     machine.mem.ram[0x0802], machine.mem.ram[0x0803],
                     machine.mem.ram[0x0804], machine.mem.ram[0x0805],
                     machine.mem.ram[0x0806], machine.mem.ram[0x0807]);
            
            // Print current CPU PC
            println!("CPU PC: {:04X}", machine.cpu.pc);
            
            // Print all 24 text rows (using our row address function)
            for row in 0..24 {
                let base: usize = 0x0400;
                let block = row / 8;
                let offset = row % 8;
                let row_addr = base + offset * 128 + block * 40;
                
                let mut line = String::new();
                for col in 0..40 {
                    let char_code = machine.mem.ram[row_addr + col];
                    let ascii = char_code & 0x7F;
                    if ascii >= 0x20 && ascii <= 0x7E {
                        line.push(ascii as char);
                    } else {
                        line.push('.');
                    }
                }
                // Only print non-empty rows
                if line.chars().any(|c| c != '.') {
                    println!("Row {:2}: {}", row, line);
                }
            }
            
            last_cycle = Instant::now();
        }

        // Update the Windows screen buffer
        window.update_with_buffer(&video.frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT)
            .unwrap();
    }
}
