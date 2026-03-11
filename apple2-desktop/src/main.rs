use std::time::Instant;
use minifb::{Key, Window, WindowOptions};
use apple2_core::machine::Apple2Machine;
use apple2_core::video::{Video, SCREEN_WIDTH, SCREEN_HEIGHT};
use apple2_core::memory::Memory;
use rodio::{OutputStream, Sink};
use std::io::Read;
use flate2::read::GzDecoder;

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

    // Resolve the roms/ directory.
    // 1. Try relative to the executable (e.g., target/debug/../..)
    // 2. Try ./roms (relative to current working directory)
    // 3. Fallback to ../roms (if run from a subdirectory)
    let roms_dir = if let Ok(mut p) = std::env::current_exe() {
        p.pop(); // exe
        p.pop(); // debug/release
        p.pop(); // target
        let d = p.join("roms");
        if d.exists() { d } else {
            let d = std::path::PathBuf::from("roms");
            if d.exists() { d } else {
                std::path::PathBuf::from("../roms")
            }
        }
    } else {
        std::path::PathBuf::from("roms")
    };
    println!(">>> Using ROMs directory: {:?}", roms_dir);

    // The downloaded ROM is ~20KB. Typical layout for these dumps:
    // Load the correct Apple II+ ROM set (341-0011 through 341-0020)
    // Downloaded from mirrors.apple2.org.za, merged into a single 12KB file
    let mut char_rom = [0x55u8; 2048]; // Checkerboard fallback
    let main_rom_path = roms_dir.join("APPLE2PLUS.ROM");
    if let Ok(rom_file) = std::fs::read(&main_rom_path) {
        println!("Loaded Apple II+ ROM: {} bytes", rom_file.len());
        if rom_file.len() >= 12288 {
            let start = rom_file.len() - 12288;
            machine.load_rom(&rom_file[start..]);
        }
    } else {
        println!("Warning: Could not open {}. Using empty memory.", main_rom_path.display());
    }
    
    // Apple II+ Character ROM (341-0036 Rev. 7)
    let char_rom_path = roms_dir.join("Apple II plus Video ROM - 341-0036 - Rev. 7.bin");
    let fallback_char_path = roms_dir.join("extracted_2048_152.bin");
    if let Ok(char_file) = std::fs::read(&char_rom_path) {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            println!("Loaded Character ROM (341-0036): 2048 bytes");
        }
    } else if let Ok(char_file) = std::fs::read(&fallback_char_path) {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            println!("Loaded Character ROM (fallback): 2048 bytes");
        }
    }
    
    // Load Disk II Boot ROM (P5A / 341-0027) into Slot 6
    let disk_rom_path = roms_dir.join("DISK2.ROM");
    if let Ok(disk_rom) = std::fs::read(&disk_rom_path) {
        if disk_rom.len() == 256 {
            machine.mem.disk2.load_boot_rom(&disk_rom);
            println!("Loaded Disk II Boot ROM (Slot 6): 256 bytes");
        }
    } else {
        println!("Warning: Could not open Disk II Boot ROM at {}", disk_rom_path.display());
    }

    // Load DOS 3.3 MASTER.DSK (place in roms/ folder - see SETUP.md)
    let dsk_path = roms_dir.join("MASTER.DSK");
    if let Ok(disk_image) = std::fs::read(&dsk_path) {
        if disk_image.len() == 143360 {
            machine.mem.disk2.load_disk(&disk_image);
            println!("Loaded MASTER.DSK floppy image: 140KB");
        } else {
            println!("Warning: MASTER.DSK size mismatch: {}", disk_image.len());
        }
    } else {
        println!("Warning: Could not open MASTER.DSK at {}", dsk_path.display());
    }

    // Initialize Memory mapped states and then Reset CPU
    machine.reset();

    // Normal boot: let the Autostart ROM initialize the system
    // This will display the "APPLE ][" logo and scan slots.
    println!("CPU Reset Vector: {:04X} (Normal boot)", machine.cpu.pc);

    let mut last_cycle = Instant::now();
    let mut key_queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
    let mut last_keys: Vec<Key> = Vec::new();
    let mut clipboard = arboard::Clipboard::new().ok();
    
    // Track F3 key to prevent multiple dialogs
    let mut last_f3_down = false;
    let mut last_f2_down = false;
    let mut last_delete_down = false;
    let start_time = Instant::now();
    // Audio Phase Tracking
    let mut unprocessed_cycles: f32 = 0.0;
    
    // Simple DC Blocker filter state
    let mut dc_filter_x1: f32 = 0.0;
    let mut dc_filter_y1: f32 = 0.0;

    while window.is_open() && !window.is_key_down(Key::Escape) {

        // 1. Handle Input
        // ... (input handling remains same)
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
                                    if ascii == b'\n' { ascii = 0x0D; }
                                    if (ascii >= 0x20 && ascii <= 0x7F) || ascii == 0x0D || ascii == 0x08 {
                                        key_queue.push_back(ascii);
                                    }
                                }
                            }
                        }
                        0
                    } else { b'V' }
                },
                Key::W => if ctrl_down { 0x17 } else { b'W' },
                Key::X => if ctrl_down { 0x18 } else { b'X' },
                Key::Y => if ctrl_down { 0x19 } else { b'Y' },
                Key::Z => if ctrl_down { 0x1A } else { b'Z' },
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
                Key::Space => b' ', Key::Enter => 0x0D, Key::Escape => 0x1B, Key::Backspace => 0x08,
                _ => 0,
            };
            if ascii != 0 { key_queue.push_back(ascii); }
        }

        let f2_down = window.is_key_down(Key::F2);
        if f2_down && !last_f2_down {
            println!(">>> REBOOT");
            machine.power_on();
        }
        last_f2_down = f2_down;

        let delete_down = window.is_key_down(Key::Delete);
        if delete_down && !last_delete_down {
            let ctrl_down = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
            if ctrl_down {
                println!(">>> SYSTEM RESET (Warm Boot)");
                machine.reset();
            }
        }
        last_delete_down = delete_down;

        let f3_down = window.is_key_down(Key::F3);
        if f3_down && !last_f3_down {
            let file = rfd::FileDialog::new()
                .add_filter("Apple II Disk Image", &["dsk", "do", "po", "gz"])
                .pick_file();
            if let Some(path) = file {
                if let Ok(raw_data) = std::fs::read(&path) {
                    machine.mem.disk2.load_disk(&raw_data);
                    println!("Successfully loaded disk: {:?}", path.file_name().unwrap_or_default());
                }
            }
        }
        last_f3_down = f3_down;

        if (machine.mem.keyboard_latch & 0x80) == 0 {
            if let Some(ascii) = key_queue.pop_front() {
                machine.mem.keyboard_latch = 0x80 | ascii;
            }
        }

        // 2. Emulate CPU execution for one Frame
        let mut frame_cycles = 0;
        let mut audio_samples: Vec<f32> = Vec::with_capacity(750);
        let sample_rate = 22050.0;
        let cycles_per_sample = 1_023_000.0 / sample_rate;

        while frame_cycles < 17_050 {
            let cycles = machine.step();
            frame_cycles += cycles;

            unprocessed_cycles += cycles as f32;
            while unprocessed_cycles >= cycles_per_sample {
                let raw_sample_val = if machine.mem.speaker { 0.1 } else { -0.1 };
                let r = 0.995;
                let filtered_val = raw_sample_val - dc_filter_x1 + r * dc_filter_y1;
                dc_filter_x1 = raw_sample_val;
                dc_filter_y1 = filtered_val;
                audio_samples.push(filtered_val);
                unprocessed_cycles -= cycles_per_sample;
            }
        }

        // Output audio frame
        if let Some(s) = &sink {
            if !audio_samples.is_empty() {
                // To prevent chopped audio, we want to maintain a healthy backlog ahead of the soundcard.
                let buf_len = s.len();
                
                // If queue is absurdly long (emulator dragging/paused), clear and resync
                if buf_len > 15 {
                    s.clear();
                }
                
                // If the queue is running dry (under 2 frames), we inject a tiny bit of silence
                // to give the emulator a moment to catch up, preventing hard clipping.
                if buf_len == 0 {
                    let mut padding = vec![0.0; (sample_rate / 60.0) as usize];
                    // smoothly transition the padding into silence
                    for x in padding.iter_mut() {
                        *x = dc_filter_y1;
                        dc_filter_y1 *= 0.995;
                    }
                    let pad_source = rodio::buffer::SamplesBuffer::new(1, sample_rate as u32, padding);
                    s.append(pad_source);
                }

                let source = rodio::buffer::SamplesBuffer::new(1, sample_rate as u32, audio_samples);
                s.append(source);
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
            let _ = last_cycle;
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
