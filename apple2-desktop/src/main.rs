use apple2_core::machine::Apple2Machine;
use apple2_core::memory::Memory;
use apple2_core::video::{SCREEN_HEIGHT, SCREEN_WIDTH, Video};
use flate2::read::GzDecoder;
use minifb::{Key, Window, WindowOptions};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::io::Read;
use std::time::Instant;

mod config;
use config::EmulatorConfig;

struct AudioMixerState {
    next_sample_cycle: f64,
    last_mix_cycle: f64,
    speaker_on: bool,
    // PolyBLEP residuals for current and next sample
    blep_correction_current: f32,
    blep_correction_next: f32,
}

impl AudioMixerState {
    fn new(initial_speaker_on: bool) -> Self {
        Self {
            next_sample_cycle: 0.0,
            last_mix_cycle: 0.0,
            speaker_on: initial_speaker_on,
            blep_correction_current: 0.0,
            blep_correction_next: 0.0,
        }
    }

    fn reset_at(&mut self, cycle: f64, cycles_per_sample: f64, speaker_on: bool) {
        self.next_sample_cycle = cycle + cycles_per_sample;
        self.last_mix_cycle = cycle;
        self.speaker_on = speaker_on;
        self.blep_correction_current = 0.0;
        self.blep_correction_next = 0.0;
    }

    /// Adds a Band-Limited Step correction to the audio buffer.
    /// t is the fractional offset [0, 1) within the sample where the toggle occurred.
    /// direction: -1.0 for 1->0 (falling), 1.0 for 0->1 (rising)
    fn apply_blep(&mut self, t: f64, direction: f32) {
        let t = t as f32;
        // PolyBLEP residual formulas:
        // Current sample: -(t - t^2/2 - 0.5)
        // Next sample: -(t^2/2)
        let c0 = (t - (t * t / 2.0) - 0.5) * direction;
        let c1 = (t * t / 2.0) * direction;

        self.blep_correction_current -= c0;
        self.blep_correction_next -= c1;
    }

    fn mix_until(
        &mut self,
        target_cycle: f64,
        cycles_per_sample: f64,
        dc_filter_x1: &mut f32,
        dc_filter_y1: &mut f32,
        audio_samples: &mut Vec<f32>,
    ) {
        let first_sample_cycle = if self.next_sample_cycle > 0.0 {
            self.next_sample_cycle - cycles_per_sample
        } else {
            0.0
        };

        while self.next_sample_cycle <= target_cycle {
            // The value for this sample is simply the current speaker state (0 or 1)
            // converted to a signal (-0.15 or 0.15), plus any BLEP corrections.
            let raw_val = if self.speaker_on { 0.15 } else { -0.15 };
            let sample_val = raw_val + self.blep_correction_current;

            // DC Offset Filter (High-pass)
            let filtered_val = sample_val - *dc_filter_x1 + 0.995 * *dc_filter_y1;
            *dc_filter_x1 = sample_val;
            *dc_filter_y1 = filtered_val;
            audio_samples.push(filtered_val);

            // Carry over the next correction to become the current one
            self.blep_correction_current = self.blep_correction_next;
            self.blep_correction_next = 0.0;

            self.last_mix_cycle = self.next_sample_cycle;
            self.next_sample_cycle += cycles_per_sample;
        }
    }
}

fn decode_disk_image(path: &std::path::Path, raw_data: Vec<u8>) -> Result<Vec<u8>, String> {
    const EXPECTED_DSK_BYTES: usize = 143_360;
    const MAX_TRAILING_BYTES: usize = 4_096;

    let is_gz_magic = raw_data.len() >= 2 && raw_data[0] == 0x1F && raw_data[1] == 0x8B;

    let disk = if is_gz_magic {
        let mut decoder = GzDecoder::new(raw_data.as_slice());
        let mut out = Vec::new();
        match decoder.read_to_end(&mut out) {
            Ok(_) => out,
            Err(e) => {
                // If decompression fails, warn but try to use the raw data anyway
                println!("WARNING: Failed to decompress {} despite gzip magic: {}. Falling back to raw data.", path.display(), e);
                raw_data
            }
        }
    } else {
        raw_data
    };

    if disk.len() < EXPECTED_DSK_BYTES {
        return Err(format!(
            "Disk size mismatch for {}: {} (expected at least 143360 bytes)",
            path.display(),
            disk.len()
        ));
    }

    if disk.len() > EXPECTED_DSK_BYTES {
        let trailing = disk.len() - EXPECTED_DSK_BYTES;
        if trailing > MAX_TRAILING_BYTES {
            return Err(format!(
                "Disk size mismatch for {}: {} (expected 143360 bytes, trailing {} too large to auto-trim)",
                path.display(),
                disk.len(),
                trailing
            ));
        }

        println!(
            "WARNING: Trimming {} trailing bytes from {}",
            trailing,
            path.display()
        );
    }

    Ok(disk[..EXPECTED_DSK_BYTES].to_vec())
}

fn save_disk_image(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let is_gz_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);

    if is_gz_ext {
        let file = std::fs::File::create(path)?;
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(data)?;
        encoder.finish()?;
    } else {
        std::fs::write(path, data)?;
    }
    Ok(())
}

fn rebuild_sink(audio_handle: Option<&OutputStreamHandle>) -> Option<Sink> {
    audio_handle.and_then(|handle| Sink::try_new(handle).ok())
}

fn joystick_axis(negative_pressed: bool, positive_pressed: bool) -> u8 {
    match (negative_pressed, positive_pressed) {
        (true, false) => 0,
        (false, true) => 255,
        _ => 127,
    }
}

#[cfg(target_os = "windows")]
fn alt_buttons() -> (bool, bool) {
    use winapi::um::winuser::{GetAsyncKeyState, VK_LMENU, VK_RMENU};

    unsafe {
        let left = (GetAsyncKeyState(VK_LMENU) as u16 & 0x8000) != 0;
        let right = (GetAsyncKeyState(VK_RMENU) as u16 & 0x8000) != 0;
        (left, right)
    }
}

#[cfg(not(target_os = "windows"))]
fn alt_buttons() -> (bool, bool) {
    (false, false)
}

#[cfg(test)]
mod tests {
    use super::AudioMixerState;

    #[test]
    fn reset_at_aligns_mixer_to_current_cycle() {
        let mut mixer = AudioMixerState::new(false);
        mixer.reset_at(12_345.0, 23.0, true);

        assert_eq!(mixer.last_mix_cycle, 12_345.0);
        assert_eq!(mixer.next_sample_cycle, 12_368.0);
        assert_eq!(mixer.blep_correction_current, 0.0);
        assert!(mixer.speaker_on);
    }
}

fn update_window_title(window: &mut Window, speed_multiplier: u32, auto_disk_turbo_active: bool) {
    let title = if auto_disk_turbo_active {
        "Apple II Emulator (Rust no_std core) [AUTO TURBO UNTHROTTLED]".to_string()
    } else if speed_multiplier > 1 {
        format!(
            "Apple II Emulator (Rust no_std core) [MANUAL TURBO x{}]",
            speed_multiplier
        )
    } else {
        "Apple II Emulator (Rust no_std core)".to_string()
    };
    window.set_title(&title);
}

fn main() {
    const BASE_FRAME_CYCLES: u32 = 17_050;
    const MAX_SPEED_MULTIPLIER: u32 = 5;

    println!("Starting Apple II Emulator targeting Windows (minifb) and core no_std...");

    let mut config = EmulatorConfig::load();

    // Create the Windows window
    let mut window = Window::new(
        "Apple II Emulator (Rust no_std core)",
        SCREEN_WIDTH * 2, // Scale up 2x for visibility
        SCREEN_HEIGHT * 2,
        WindowOptions::default(),
    )
    .unwrap();

    // Limit to ~60 FPS
    window.set_target_fps(60);

    // Initialize the emulator core
    let mut machine = Apple2Machine::new();
    let mut video = Video::new();

    // Setup an audio stream
    let audio_device = OutputStream::try_default();
    let (mut _stream, mut audio_handle, mut sink) = (None, None, None);
    if let Ok((s, sh)) = audio_device {
        _stream = Some(s);
        audio_handle = Some(sh.clone());
        sink = rebuild_sink(audio_handle.as_ref());
    } else {
        println!("Warning: Could not initialize audio output.");
    }

    // Resolve the roms/ directory.
    let roms_dir = if let Ok(mut p) = std::env::current_exe() {
        p.pop(); // exe
        p.pop(); // debug/release
        p.pop(); // target
        let d = p.join("roms");
        if d.exists() {
            d
        } else {
            let d = std::path::PathBuf::from("roms");
            if d.exists() {
                d
            } else {
                std::path::PathBuf::from("../roms")
            }
        }
    } else {
        std::path::PathBuf::from("roms")
    };
    println!(
        ">>> ROMs directory resolved to: {:?}",
        std::fs::canonicalize(&roms_dir).unwrap_or(roms_dir.clone())
    );

    let mut cached_main_rom = Vec::new();
    let mut cached_disk_rom = Vec::new();
    let mut cached_disk_image: Option<Vec<u8>> = None;

    // Load Main ROM
    let main_rom_path = roms_dir.join("APPLE2PLUS.ROM");
    match std::fs::read(&main_rom_path) {
        Ok(rom_file) => {
            println!("Loaded Apple II+ ROM: {} bytes", rom_file.len());
            if rom_file.len() >= 12288 {
                let start = rom_file.len() - 12288;
                cached_main_rom = rom_file[start..].to_vec();
                machine.load_rom(&cached_main_rom);
            }
        }
        Err(e) => println!("ERROR: Could not open {}: {}", main_rom_path.display(), e),
    }

    // Load Disk II Boot ROM
    let disk_rom_path = roms_dir.join("DISK2.ROM");
    match std::fs::read(&disk_rom_path) {
        Ok(disk_rom) => {
            if disk_rom.len() == 256 {
                cached_disk_rom = disk_rom.clone();
                machine.mem.disk2.load_boot_rom(&cached_disk_rom);
                println!("Loaded Disk II Boot ROM (Slot 6): 256 bytes");
            }
        }
        Err(e) => println!("ERROR: Could not open {}: {}", disk_rom_path.display(), e),
    }

    // Load Disk Image
    let mut dsk_path = roms_dir.join("MASTER.DSK");
    if let Some(ref last_path) = config.last_disk_path {
        if last_path.exists() {
            dsk_path = last_path.clone();
        }
    }

    match std::fs::read(&dsk_path) {
        Ok(raw_data) => match decode_disk_image(&dsk_path, raw_data) {
            Ok(disk_image) => {
                cached_disk_image = Some(disk_image.clone());
                machine.mem.disk2.load_disk(&disk_image);
                println!("Loaded floppy image from {}: 140KB", dsk_path.display());
            }
            Err(e) => println!("WARNING: {}", e),
        },
        Err(e) => println!("ERROR: Could not open {}: {}", dsk_path.display(), e),
    }

    // Load Character ROM
    let mut char_rom = [0x55u8; 2048]; // Checkerboard fallback
    let char_rom_path = roms_dir.join("Apple II plus Video ROM - 341-0036 - Rev. 7.bin");
    if let Ok(char_file) = std::fs::read(&char_rom_path) {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            println!("Loaded Character ROM (341-0036): 2048 bytes");
        }
    }

    machine.reset();
    println!("CPU Reset Vector: {:04X} (Normal boot)", machine.cpu.pc);

    let mut last_cycle = Instant::now();
    let mut key_queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
    let mut last_keys: Vec<Key> = Vec::new();

    let mut last_f2_down = false;
    let mut last_f3_down = false;
    let mut last_f4_down = false;
    let mut speed_multiplier: u32 = 1;
    let mut dc_filter_x1: f32 = 0.0;
    let mut dc_filter_y1: f32 = 0.0;
    let mut audio_mixer = AudioMixerState::new(machine.mem.speaker);
    let sample_rate: u32 = 44_100;
    let cycles_per_sample = 1_023_000.0_f64 / sample_rate as f64;
    update_window_title(&mut window, speed_multiplier, false);
    let mut current_target_fps: usize = 60;
    let mut last_title_speed_multiplier = speed_multiplier;
    let mut last_title_auto_disk_turbo = false;

    while window.is_open() && !window.is_key_down(Key::F10) {
        // Handle Input
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
        let joystick_x = joystick_axis(
            window.is_key_down(Key::Left),
            window.is_key_down(Key::Right),
        );
        let joystick_y = joystick_axis(
            window.is_key_down(Key::Up),
            window.is_key_down(Key::Down),
        );
        let (left_alt_down, right_alt_down) = alt_buttons();
        let joystick_button_0 = right_alt_down || window.is_key_down(Key::RightAlt);
        let joystick_button_1 = left_alt_down || window.is_key_down(Key::LeftAlt);
        machine
            .mem
            .set_joystick_state(joystick_x, joystick_y, joystick_button_0, joystick_button_1);

        for &key in keys.iter() {
            let ascii = match key {
                Key::A => {
                    if ctrl_down {
                        0x01
                    } else {
                        b'A'
                    }
                }
                Key::B => {
                    if ctrl_down {
                        0x02
                    } else {
                        b'B'
                    }
                }
                Key::C => {
                    if ctrl_down {
                        0x03
                    } else {
                        b'C'
                    }
                }
                Key::D => {
                    if ctrl_down {
                        0x04
                    } else {
                        b'D'
                    }
                }
                Key::E => {
                    if ctrl_down {
                        0x05
                    } else {
                        b'E'
                    }
                }
                Key::F => {
                    if ctrl_down {
                        0x06
                    } else {
                        b'F'
                    }
                }
                Key::G => {
                    if ctrl_down {
                        0x07
                    } else {
                        b'G'
                    }
                }
                Key::H => {
                    if ctrl_down {
                        0x08
                    } else {
                        b'H'
                    }
                }
                Key::I => {
                    if ctrl_down {
                        0x09
                    } else {
                        b'I'
                    }
                }
                Key::J => {
                    if ctrl_down {
                        0x0A
                    } else {
                        b'J'
                    }
                }
                Key::K => {
                    if ctrl_down {
                        0x0B
                    } else {
                        b'K'
                    }
                }
                Key::L => {
                    if ctrl_down {
                        0x0C
                    } else {
                        b'L'
                    }
                }
                Key::M => {
                    if ctrl_down {
                        0x0D
                    } else {
                        b'M'
                    }
                }
                Key::N => {
                    if ctrl_down {
                        0x0E
                    } else {
                        b'N'
                    }
                }
                Key::O => {
                    if ctrl_down {
                        0x0F
                    } else {
                        b'O'
                    }
                }
                Key::P => {
                    if ctrl_down {
                        0x10
                    } else {
                        b'P'
                    }
                }
                Key::Q => {
                    if ctrl_down {
                        0x11
                    } else {
                        b'Q'
                    }
                }
                Key::R => {
                    if ctrl_down {
                        0x12
                    } else {
                        b'R'
                    }
                }
                Key::S => {
                    if ctrl_down {
                        0x13
                    } else {
                        b'S'
                    }
                }
                Key::T => {
                    if ctrl_down {
                        0x14
                    } else {
                        b'T'
                    }
                }
                Key::U => {
                    if ctrl_down {
                        0x15
                    } else {
                        b'U'
                    }
                }
                Key::V => {
                    if ctrl_down {
                        0x16
                    } else {
                        b'V'
                    }
                }
                Key::W => {
                    if ctrl_down {
                        0x17
                    } else {
                        b'W'
                    }
                }
                Key::X => {
                    if ctrl_down {
                        0x18
                    } else {
                        b'X'
                    }
                }
                Key::Y => {
                    if ctrl_down {
                        0x19
                    } else {
                        b'Y'
                    }
                }
                Key::Z => {
                    if ctrl_down {
                        0x1A
                    } else {
                        b'Z'
                    }
                }

                // Numbers and Symbols
                Key::Key0 => {
                    if shift_down {
                        b')'
                    } else {
                        b'0'
                    }
                }
                Key::Key1 => {
                    if shift_down {
                        b'!'
                    } else {
                        b'1'
                    }
                }
                Key::Key2 => {
                    if shift_down {
                        b'@'
                    } else {
                        b'2'
                    }
                }
                Key::Key3 => {
                    if shift_down {
                        b'#'
                    } else {
                        b'3'
                    }
                }
                Key::Key4 => {
                    if shift_down {
                        b'$'
                    } else {
                        b'4'
                    }
                }
                Key::Key5 => {
                    if shift_down {
                        b'%'
                    } else {
                        b'5'
                    }
                }
                Key::Key6 => {
                    if shift_down {
                        b'^'
                    } else {
                        b'6'
                    }
                }
                Key::Key7 => {
                    if shift_down {
                        b'&'
                    } else {
                        b'7'
                    }
                }
                Key::Key8 => {
                    if shift_down {
                        b'*'
                    } else {
                        b'8'
                    }
                }
                Key::Key9 => {
                    if shift_down {
                        b'('
                    } else {
                        b'9'
                    }
                }

                Key::Minus => {
                    if shift_down {
                        b'_'
                    } else {
                        b'-'
                    }
                }
                Key::Equal => {
                    if shift_down {
                        b'+'
                    } else {
                        b'='
                    }
                }
                Key::Comma => {
                    if shift_down {
                        b'<'
                    } else {
                        b','
                    }
                }
                Key::Period => {
                    if shift_down {
                        b'>'
                    } else {
                        b'.'
                    }
                }
                Key::Slash => {
                    if shift_down {
                        b'?'
                    } else {
                        b'/'
                    }
                }
                Key::Semicolon => {
                    if shift_down {
                        b':'
                    } else {
                        b';'
                    }
                }
                Key::Apostrophe => {
                    if shift_down {
                        b'"'
                    } else {
                        b'\''
                    }
                }

                // Control Keys
                Key::Space => b' ',
                Key::Enter => 0x0D,
                Key::Backspace => 0x08,
                Key::Escape => 0x1B,
                _ => 0,
            };
            if ascii != 0 {
                key_queue.push_back(ascii);
            }
        }

        if window.is_key_down(Key::F2) && !last_f2_down {
            if ctrl_down {
                println!(">>> SYSTEM RESET (Warm Boot)");
                machine.reset();
            } else {
                println!(">>> REBOOT (Cold Boot)");
                // Save changes before cold boot
                if machine.mem.disk2.is_dirty {
                    let path_to_save = config.last_disk_path.as_ref().unwrap_or(&dsk_path);
                    if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
                        if let Err(e) = save_disk_image(path_to_save, &new_disk) {
                            println!("ERROR: Failed to save changes to current disk {:?}: {}", path_to_save.file_name().unwrap_or_default(), e);
                        } else {
                            println!("Saved changes to disk before reboot: {:?}", path_to_save.file_name().unwrap_or_default());
                            // IMPORTANT: Update our memory cache with the new data so the reboot uses the updated disk!
                            cached_disk_image = Some(new_disk);
                        }
                    }
                }

                machine = Apple2Machine::new();
                if !cached_main_rom.is_empty() {
                    machine.load_rom(&cached_main_rom);
                }
                if !cached_disk_rom.is_empty() {
                    machine.mem.disk2.load_boot_rom(&cached_disk_rom);
                }
                if let Some(ref disk) = cached_disk_image {
                    machine.mem.disk2.load_disk(disk);
                }
                machine.reset();
            }
            sink = rebuild_sink(audio_handle.as_ref());
            audio_mixer.reset_at(machine.total_cycles as f64, cycles_per_sample, machine.mem.speaker);
            dc_filter_x1 = 0.0;
            dc_filter_y1 = 0.0;
        }
        last_f2_down = window.is_key_down(Key::F2);

        let f3_down = window.is_key_down(Key::F3);
        if f3_down && !last_f3_down {
            let file = rfd::FileDialog::new()
                .add_filter("Apple II Disk Image", &["dsk", "do", "po", "gz"])
                .pick_file();
            if let Some(path) = file {
                if let Ok(raw_data) = std::fs::read(&path) {
                    match decode_disk_image(&path, raw_data) {
                        Ok(disk_image) => {
                            // Check if current disk is dirty and save it before loading new one
                            if machine.mem.disk2.is_dirty {
                                if let Some(ref current_path) = config.last_disk_path {
                                    if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
                                        if let Err(e) = save_disk_image(current_path, &new_disk) {
                                            println!("ERROR: Failed to save changes to current disk {:?}: {}", current_path.file_name().unwrap_or_default(), e);
                                        } else {
                                            println!("Saved changes to disk: {:?}", current_path.file_name().unwrap_or_default());
                                        }
                                    }
                                }
                            }
                            
                            cached_disk_image = Some(disk_image.clone());
                            machine.mem.disk2.load_disk(&disk_image);
                            sink = rebuild_sink(audio_handle.as_ref());
                            audio_mixer.reset_at(
                                machine.total_cycles as f64,
                                cycles_per_sample,
                                machine.mem.speaker,
                            );
                            dc_filter_x1 = 0.0;
                            dc_filter_y1 = 0.0;
                            println!(
                                "Successfully loaded disk: {:?}",
                                path.file_name().unwrap_or_default()
                            );
                            config.last_disk_path = Some(path);
                            config.save();
                        }
                        Err(e) => println!("ERROR: {}", e),
                    }
                }
            }
        }
        last_f3_down = f3_down;

        let f4_down = window.is_key_down(Key::F4);
        if f4_down && !last_f4_down {
            speed_multiplier = if speed_multiplier >= MAX_SPEED_MULTIPLIER {
                1
            } else {
                speed_multiplier + 1
            };
            let turbo_mode = speed_multiplier > 1;
            let desired_fps = if turbo_mode { 120 } else { 60 };
            if current_target_fps != desired_fps {
                window.set_target_fps(desired_fps);
                current_target_fps = desired_fps;
            }
            println!(">>> Speed Mode: CPU x{}", speed_multiplier);
        }
        last_f4_down = f4_down;

        if (machine.mem.keyboard_latch & 0x80) == 0 {
            if let Some(ascii) = key_queue.pop_front() {
                machine.mem.keyboard_latch = 0x80 | ascii;
            }
        }

        // Emulate CPU execution for one Frame
        let mut frame_cycles = 0;
        let mut audio_samples: Vec<f32> = Vec::with_capacity(1500);
        let auto_disk_turbo_active = machine.mem.disk2.motor_on;
        let effective_speed_multiplier = if auto_disk_turbo_active {
            MAX_SPEED_MULTIPLIER
        } else {
            speed_multiplier
        };
        let turbo_mode = auto_disk_turbo_active || effective_speed_multiplier > 1;
        let desired_fps = if auto_disk_turbo_active {
            0
        } else if turbo_mode {
            120
        } else {
            60
        };
        if current_target_fps != desired_fps {
            window.set_target_fps(desired_fps);
            current_target_fps = desired_fps;
        }
        if last_title_speed_multiplier != speed_multiplier
            || last_title_auto_disk_turbo != auto_disk_turbo_active
        {
            update_window_title(&mut window, speed_multiplier, auto_disk_turbo_active);
            last_title_speed_multiplier = speed_multiplier;
            last_title_auto_disk_turbo = auto_disk_turbo_active;
        }
        let target_cycles = if auto_disk_turbo_active {
            BASE_FRAME_CYCLES * MAX_SPEED_MULTIPLIER
        } else {
            BASE_FRAME_CYCLES * effective_speed_multiplier
        };

        while frame_cycles < target_cycles {
            let cycles = machine.step();
            frame_cycles += cycles;

            for edge_cycle in machine.mem.take_speaker_toggle_cycles() {
                let edge_f = edge_cycle as f64;
                audio_mixer.mix_until(
                    edge_f,
                    cycles_per_sample,
                    &mut dc_filter_x1,
                    &mut dc_filter_y1,
                    &mut audio_samples,
                );

                // Apply PolyBLEP correction at the toggle point.
                // t is the fractional distance into the *current* sample period [0, 1).
                let t = 1.0 - ((audio_mixer.next_sample_cycle - edge_f) / cycles_per_sample);
                let direction = if audio_mixer.speaker_on { -1.0 } else { 1.0 };
                audio_mixer.apply_blep(t, direction);

                audio_mixer.speaker_on = !audio_mixer.speaker_on;
            }

            audio_mixer.mix_until(
                machine.total_cycles as f64,
                cycles_per_sample,
                &mut dc_filter_x1,
                &mut dc_filter_y1,
                &mut audio_samples,
            );
        }

        // Audio Frame append
        if let Some(s) = &sink {
            if !audio_samples.is_empty() {
                // To prevent chopped audio, we want to maintain a healthy backlog ahead of the soundcard.
                let buf_len = s.len();
                let max_buf = if turbo_mode { 30 } else { 15 };

                // Avoid hard-clearing the queue (can cause audible dropouts). If backlog is high,
                // skip appending this frame and let the device catch up naturally.
                if buf_len <= max_buf {
                    // If the queue is running dry (under 2 frames), we inject a tiny bit of silence
                    // to give the emulator a moment to catch up, preventing hard clipping.
                    if buf_len == 0 {
                        let mut padding = vec![0.0; (sample_rate / 60) as usize];
                        // smoothly transition the padding into silence
                        for x in padding.iter_mut() {
                            *x = dc_filter_y1;
                            dc_filter_y1 *= 0.995;
                        }
                        let pad_source = rodio::buffer::SamplesBuffer::new(1, sample_rate, padding);
                        s.append(pad_source);
                    }

                    let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, audio_samples);
                    s.append(source);
                }
            }
        }

        // Periodic Debug
        #[cfg(debug_assertions)]
        {
            static mut FRAME_COUNT: u32 = 0;
            if !turbo_mode {
                unsafe {
                    FRAME_COUNT += 1;
                    if FRAME_COUNT % 60 == 0 {
                        let mut row_data = String::new();
                        for i in 0..32 {
                            row_data.push_str(&format!("{:02X} ", machine.mem.read(0x0800 + i)));
                        }
                        let mut vec_data = String::new();
                        for i in 0..32 {
                            vec_data.push_str(&format!("{:02X} ", machine.mem.read(0x03D0 + i)));
                        }
                        let mut buf_data = String::new();
                        for i in 0..16 {
                            buf_data.push_str(&format!("{:02X} ", machine.mem.read(0x0200 + i)));
                        }

                        println!(
                            "Disk: T{} Index={} Latch={:02X}",
                            machine.mem.disk2.current_track,
                            machine.mem.disk2.byte_index,
                            machine.mem.disk2.data_latch
                        );
                        println!("Memory at $0800: {}", row_data);
                        println!("Vectors at $03D0: {}", vec_data);
                        println!("Buffer at $0200: {}", buf_data);
                        println!(
                            "CPU PC: {:04X} A:{:02X} X:{:02X} Y:{:02X} S:{:02X} P:{:02X}",
                            machine.cpu.pc,
                            machine.cpu.a,
                            machine.cpu.x,
                            machine.cpu.y,
                            machine.cpu.sp,
                            machine.cpu.status.to_byte()
                        );
                    }
                }
            }
        }

        // Render the Screen
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

        #[cfg(debug_assertions)]
        if !turbo_mode && last_cycle.elapsed().as_secs() >= 1 {
            // Print screen rows
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
                if line.chars().any(|c| c != '.') {
                    println!("Row {:2}: {}", row, line);
                }
            }
            last_cycle = Instant::now();
        }
    }

    // Save changes when closing the emulator
    if machine.mem.disk2.is_dirty {
        let path_to_save = config.last_disk_path.as_ref().unwrap_or(&dsk_path);
        if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
            if let Err(e) = save_disk_image(path_to_save, &new_disk) {
                println!("ERROR: Failed to save changes to current disk {:?}: {}", path_to_save.file_name().unwrap_or_default(), e);
            } else {
                println!("Saved changes to disk on exit: {:?}", path_to_save.file_name().unwrap_or_default());
            }
        }
    }
}
