use apple2_core::machine::Apple2Machine;
use apple2_core::memory::Memory;
use apple2_core::video::{SCREEN_HEIGHT, SCREEN_WIDTH, Video};
use flate2::read::GzDecoder;
use minifb::{InputCallback, Key, Window, WindowOptions};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Read;
use std::rc::Rc;
use std::time::Instant;

/// Raw key-down edges as they actually happen, independent of how often the
/// main loop gets around to polling. minifb's `is_key_down`/`get_keys_pressed`
/// only reflect a snapshot at poll time, so a key pressed and released
/// entirely between two polls (easy to hit once the loop is wall-clock/turbo
/// paced instead of a fixed 60Hz) would otherwise be silently missed.
///
/// Records both down AND up transitions: Windows itself resends WM_KEYDOWN
/// repeatedly while a key is held (its own OS-level auto-repeat), with no
/// WM_KEYUP in between - so an up event is the only reliable signal that
/// distinguishes "still holding the same key" (should be rate-limited) from
/// "a genuine new press" (should register immediately), including a real
/// quick double-tap of the same key.
struct KeyEventRecorder {
    events: Rc<RefCell<VecDeque<(Key, bool)>>>,
}

impl InputCallback for KeyEventRecorder {
    fn add_char(&mut self, _uni_char: u32) {}

    fn set_key_state(&mut self, key: Key, state: bool) {
        self.events.borrow_mut().push_back((key, state));
    }
}

/// Shared ASCII mapping used both for the guaranteed-delivery key-down event
/// queue and for re-checking whether a held key is still down.
fn key_to_ascii(key: Key, ctrl_down: bool, shift_down: bool) -> u8 {
    match key {
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
        Key::V => if ctrl_down { 0x16 } else { b'V' },
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
        Key::Space => b' ',
        Key::Enter => 0x0D,
        Key::Backspace => 0x08,
        Key::Escape => 0x1B,
        _ => 0,
    }
}

mod config;
use config::EmulatorConfig;

struct AudioMixerState {
    next_sample_cycle: f64,
    last_mix_cycle: f64,
    speaker_on: bool,
    // Accumulates total "on" time (in cycles) for the current sample window
    high_time_accumulator: f64,
    // Analog LPF state
    lpf_state: f32,
}

impl AudioMixerState {
    fn new(initial_speaker_on: bool) -> Self {
        Self {
            next_sample_cycle: 0.0,
            last_mix_cycle: 0.0,
            speaker_on: initial_speaker_on,
            high_time_accumulator: 0.0,
            lpf_state: 0.0,
        }
    }

    fn reset_at(&mut self, cycle: f64, cycles_per_sample: f64, speaker_on: bool) {
        self.next_sample_cycle = cycle + cycles_per_sample;
        self.last_mix_cycle = cycle;
        self.speaker_on = speaker_on;
        self.high_time_accumulator = 0.0;
        self.lpf_state = 0.0;
    }

    fn mix_until(
        &mut self,
        target_cycle: f64,
        cycles_per_sample: f64,
        dc_filter_x1: &mut f32,
        dc_filter_y1: &mut f32,
        audio_samples: &mut Vec<f32>,
    ) {
        let mut current_cycle = self.last_mix_cycle;

        while self.next_sample_cycle <= target_cycle {
            // How much of the remaining sample window is covered until the target or sample boundary?
            let segment_end = self.next_sample_cycle;
            let segment_duration = segment_end - current_cycle;

            if self.speaker_on {
                self.high_time_accumulator += segment_duration;
            }

            // --- Window complete: Produce a sample ---
            // Calculate duty cycle (0.0 to 1.0)
            let duty_cycle = self.high_time_accumulator / cycles_per_sample;
            // Convert to signal range (-0.4 to 0.4)
            let raw_val = (duty_cycle as f32 * 0.8) - 0.4;

            // Apply analog low-pass filter at the sample rate (approx 3.8kHz cutoff)
            let alpha = 0.35; 
            self.lpf_state += alpha * (raw_val - self.lpf_state);
            let final_val = self.lpf_state;

            // DC Offset Filter (High-pass)
            let filtered_val = final_val - *dc_filter_x1 + 0.995 * *dc_filter_y1;
            *dc_filter_x1 = final_val;
            *dc_filter_y1 = filtered_val;
            audio_samples.push(filtered_val);

            // Reset for next sample window
            current_cycle = self.next_sample_cycle;
            self.next_sample_cycle += cycles_per_sample;
            self.high_time_accumulator = 0.0;
        }

        // Handle the remaining partial segment after the last sample boundary
        if current_cycle < target_cycle {
            if self.speaker_on {
                self.high_time_accumulator += target_cycle - current_cycle;
            }
            self.last_mix_cycle = target_cycle;
        } else {
            self.last_mix_cycle = current_cycle;
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

fn rebuild_sink(audio_handle: Option<&OutputStreamHandle>, volume: f32) -> Option<Sink> {
    audio_handle.and_then(|handle| {
        Sink::try_new(handle).ok().map(|s| {
            s.set_volume(volume);
            s
        })
    })
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
        assert!(mixer.speaker_on);
    }
}

fn update_window_title(
    window: &mut Window,
    speed_multiplier: f32,
    auto_disk_turbo_active: bool,
    volume: f32,
) {
    let mut title = if auto_disk_turbo_active || speed_multiplier == 0.0 {
        "Apple II Emulator (Rust no_std core) [TURBO FULL UNTHROTTLED]".to_string()
    } else if speed_multiplier > 1.0 {
        format!(
            "Apple II Emulator (Rust no_std core) [MANUAL TURBO x{}]",
            speed_multiplier
        )
    } else {
        "Apple II Emulator (Rust no_std core)".to_string()
    };
    if volume <= 0.0 {
        title.push_str(" [MUTED]");
    } else if (volume - 1.0).abs() > f32::EPSILON {
        title.push_str(&format!(" [VOL {:.0}%]", volume * 100.0));
    }
    window.set_title(&title);
}

fn main() {
    const BASE_FRAME_CYCLES: u32 = 17_050;
    const CPU_HZ: f64 = 1_023_000.0;
    // Cap how much wall-clock time a single iteration can "owe" cycles for,
    // so a debugger pause, window drag, or other stall doesn't cause a huge
    // catch-up burst of emulated time (and audio) all at once afterwards.
    const MAX_OWED_SECS: f64 = 0.1;
    let speed_steps: [f32; 6] = [1.0, 1.2, 1.5, 2.0, 5.0, 0.0];

    println!("Starting Apple II Emulator targeting Windows (minifb) and core no_std...");

    let mut config = EmulatorConfig::load();
    config.volume = config.volume.clamp(0.0, 1.0);
    let mut speed_index: usize = if config.speed_index < speed_steps.len() { config.speed_index } else { 0 };

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

    // Guaranteed-delivery key events (see KeyEventRecorder doc comment).
    let key_events: Rc<RefCell<VecDeque<(Key, bool)>>> = Rc::new(RefCell::new(VecDeque::new()));
    window.set_input_callback(Box::new(KeyEventRecorder { events: key_events.clone() }));

    // Initialize the emulator core
    let mut machine = Apple2Machine::new();
    let mut video = Video::new();
    video.mono = config.mono;

    // Setup an audio stream
    let audio_device = OutputStream::try_default();
    let (mut _stream, mut audio_handle, mut sink) = (None, None, None);
    if let Ok((s, sh)) = audio_device {
        _stream = Some(s);
        audio_handle = Some(sh.clone());
        sink = rebuild_sink(audio_handle.as_ref(), config.volume);
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
    // Human-readable list of non-fatal missing/invalid ROM files, shown in
    // one warning dialog after the load phase.
    let mut rom_warnings: Vec<String> = Vec::new();

    // Load Main ROM
    let main_rom_path = roms_dir.join("APPLE2PLUS.ROM");
    match std::fs::read(&main_rom_path) {
        Ok(rom_file) => {
            println!("Loaded Apple II+ ROM: {} bytes", rom_file.len());
            if rom_file.len() >= 12288 {
                let start = rom_file.len() - 12288;
                cached_main_rom = rom_file[start..].to_vec();
                machine.load_rom(&cached_main_rom);
            } else {
                println!("ERROR: {} is too small ({} bytes, need 12288)", main_rom_path.display(), rom_file.len());
            }
        }
        Err(e) => println!("ERROR: Could not open {}: {}", main_rom_path.display(), e),
    }
    if cached_main_rom.is_empty() {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Missing ROM file")
            .set_description(format!(
                "Could not load the Apple II+ system ROM:\n\n{}\n\nPlace APPLE2PLUS.ROM (12KB, $D000-$FFFF) in the roms folder and restart.\nSee roms/PUT_ROMS_HERE.txt for the full file list.",
                main_rom_path.display()
            ))
            .show();
        return;
    }

    // Load Disk II Boot ROM
    let disk_rom_path = roms_dir.join("DISK2.ROM");
    match std::fs::read(&disk_rom_path) {
        Ok(disk_rom) => {
            if disk_rom.len() == 256 {
                cached_disk_rom = disk_rom.clone();
                machine.mem.disk2.load_boot_rom(&cached_disk_rom);
                println!("Loaded Disk II Boot ROM (Slot 6): 256 bytes");
            } else {
                rom_warnings.push(format!("DISK2.ROM has wrong size ({} bytes, need 256) - floppy disks cannot boot", disk_rom.len()));
            }
        }
        Err(e) => {
            println!("ERROR: Could not open {}: {}", disk_rom_path.display(), e);
            rom_warnings.push("DISK2.ROM not found - floppy disks cannot boot".to_string());
        }
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
            Err(e) => {
                println!("WARNING: {}", e);
                rom_warnings.push(format!("Disk image could not be decoded ({}) - press F3 to load another disk", e));
            }
        },
        Err(e) => {
            println!("ERROR: Could not open {}: {}", dsk_path.display(), e);
            rom_warnings.push("No boot disk found (MASTER.DSK) - press F3 to load a disk".to_string());
        }
    }

    // Load Character ROM
    let mut char_rom = [0x55u8; 2048]; // Checkerboard fallback
    let char_rom_path = roms_dir.join("Apple II plus Video ROM - 341-0036 - Rev. 7.bin");
    let mut char_rom_loaded = false;
    if let Ok(char_file) = std::fs::read(&char_rom_path) {
        if char_file.len() == 2048 {
            char_rom.copy_from_slice(&char_file);
            char_rom_loaded = true;
            println!("Loaded Character ROM (341-0036): 2048 bytes");
        }
    }
    if !char_rom_loaded {
        rom_warnings.push("Character ROM (Apple II plus Video ROM - 341-0036 - Rev. 7.bin) not found - text will render as a checkerboard".to_string());
    }

    if !rom_warnings.is_empty() {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Missing ROM files")
            .set_description(format!(
                "Some files are missing from the roms folder:\n\n- {}\n\nroms folder: {}\nSee roms/PUT_ROMS_HERE.txt for details. The emulator will start anyway.",
                rom_warnings.join("\n- "),
                roms_dir.display()
            ))
            .show();
    }

    machine.reset();
    println!("CPU Reset Vector: {:04X} (Normal boot)", machine.cpu.pc);

    let mut last_cycle = Instant::now();
    // Paste queue: clipboard text is fed to the latch one char at a time,
    // paced by the emulated program consuming the keyboard strobe.
    let mut key_queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
    // Live-typing auto-repeat: holding a key down must re-strobe the latch at
    // a fixed real-world rate, not once per main-loop iteration - the loop
    // now iterates far faster than that during turbo/auto disk-turbo, which
    // would otherwise auto-repeat unrealistically fast (or even look like
    // held-down key spam if the CPU polls fast enough to see each rewrite).
    const KEY_REPEAT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(67); // ~15 cps
    let mut last_repeat_key: Option<Key> = None;
    let mut last_repeat_time = Instant::now();

    let mut last_f1_down = false;
    let mut last_f2_down = false;
    let mut last_f3_down = false;
    let mut last_f5_down = false;
    let mut last_f7_down = false;
    let mut last_f8_down = false;
    let mut last_f9_down = false;
    let mut last_right_mouse_down = false;
    let mut clipboard = arboard::Clipboard::new().ok();
    
    let mut speed_multiplier: f32 = speed_steps[speed_index];
    let mut dc_filter_x1: f32 = 0.0;
    let mut dc_filter_y1: f32 = 0.0;
    let mut audio_mixer = AudioMixerState::new(machine.mem.speaker);
    let sample_rate: u32 = 44_100;
    let cycles_per_sample = 1_023_000.0_f64 / sample_rate as f64;
    update_window_title(&mut window, speed_multiplier, false, config.volume);
    let mut current_target_fps: usize = 60;
    let mut last_title_speed_multiplier = speed_multiplier;
    let mut last_title_auto_disk_turbo = false;

    while window.is_open() && !window.is_key_down(Key::F10) {
        // Handle Input
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

        // Guaranteed-delivery edges: these came from minifb's raw OS message
        // callback (KeyEventRecorder), so unlike polling is_key_down, a key
        // pressed and released entirely between two loop iterations is still
        // captured here - it was recorded the instant Windows delivered the
        // keydown/keyup message, not sampled from "what's down right now".
        //
        // Windows resends a down event repeatedly while a key is held (its
        // own OS-level auto-repeat) with no up event in between, so "is this
        // key still the one we're already tracking" is what distinguishes a
        // held-key repeat (rate-limit it to KEY_REPEAT_INTERVAL, like real
        // Apple II keyboard hardware) from a genuine new press or a real
        // quick double-tap (register immediately - the up event in between
        // cleared last_repeat_key).
        for (key, is_down) in key_events.borrow_mut().drain(..) {
            if !is_down {
                if last_repeat_key == Some(key) {
                    last_repeat_key = None;
                }
                continue;
            }
            let ascii = key_to_ascii(key, ctrl_down, shift_down);
            if ascii == 0 {
                continue;
            }
            let now = Instant::now();
            let is_new_key = Some(key) != last_repeat_key;
            if is_new_key || now.duration_since(last_repeat_time) >= KEY_REPEAT_INTERVAL {
                // Live typing writes the latch directly, like real hardware:
                // a new keypress overwrites the previous one even if unread.
                machine.mem.keyboard_latch = 0x80 | ascii;
                last_repeat_key = Some(key);
                last_repeat_time = now;
            }
        }

        if window.is_key_down(Key::F1) && !last_f1_down {
            println!(">>> SYSTEM RESET (Ctrl-Reset / Warm Boot)");
            machine.reset();
        }
        last_f1_down = window.is_key_down(Key::F1);

        if window.is_key_down(Key::F2) && !last_f2_down {
            if ctrl_down {
                println!(">>> SYSTEM RESET (Warm Boot)");
                machine.reset();
            } else {
                println!(">>> REBOOT (Cold Boot)");
                if machine.mem.disk2.is_dirty {
                    let path_to_save = config.last_disk_path.as_ref().unwrap_or(&dsk_path);
                    if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
                        let _ = save_disk_image(path_to_save, &new_disk);
                        cached_disk_image = Some(new_disk);
                    }
                }
                machine = Apple2Machine::new();
                if !cached_main_rom.is_empty() { machine.load_rom(&cached_main_rom); }
                if !cached_disk_rom.is_empty() { machine.mem.disk2.load_boot_rom(&cached_disk_rom); }
                if let Some(ref disk) = cached_disk_image { machine.mem.disk2.load_disk(disk); }
                machine.reset();
                key_queue.clear();
            }
            sink = rebuild_sink(audio_handle.as_ref(), config.volume);
            audio_mixer.reset_at(machine.total_cycles as f64, cycles_per_sample, machine.mem.speaker);
            dc_filter_x1 = 0.0; dc_filter_y1 = 0.0;
        }
        last_f2_down = window.is_key_down(Key::F2);

        let f3_down = window.is_key_down(Key::F3);
        if f3_down && !last_f3_down {
            if let Some(path) = rfd::FileDialog::new().add_filter("Apple II Disk", &["dsk","gz"]).pick_file() {
                if let Ok(raw_data) = std::fs::read(&path) {
                    if let Ok(disk_image) = decode_disk_image(&path, raw_data) {
                        if machine.mem.disk2.is_dirty {
                            if let Some(ref current_path) = config.last_disk_path {
                                if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
                                    let _ = save_disk_image(current_path, &new_disk);
                                }
                            }
                        }
                        cached_disk_image = Some(disk_image.clone());
                        machine.mem.disk2.load_disk(&disk_image);
                        sink = rebuild_sink(audio_handle.as_ref(), config.volume);
                        audio_mixer.reset_at(machine.total_cycles as f64, cycles_per_sample, machine.mem.speaker);
                        dc_filter_x1 = 0.0; dc_filter_y1 = 0.0;
                        config.last_disk_path = Some(path);
                        config.save();
                    }
                }
            }
        }
        last_f3_down = f3_down;

        let f5_down = window.is_key_down(Key::F5);
        if f5_down && !last_f5_down {
            speed_index = (speed_index + 1) % speed_steps.len();
            speed_multiplier = speed_steps[speed_index];
            config.speed_index = speed_index;
            config.save();
        }
        last_f5_down = f5_down;

        let f7_down = window.is_key_down(Key::F7);
        if f7_down && !last_f7_down {
            video.toggle_mono();
            println!(">>> Screen mode: {}", if video.mono { "Green (mono)" } else { "Color" });
            config.mono = video.mono;
            config.save();
        }
        last_f7_down = f7_down;

        // F8 / F9: volume down / up in 10% steps.
        let f8_down = window.is_key_down(Key::F8);
        let f9_down = window.is_key_down(Key::F9);
        let volume_step = if f8_down && !last_f8_down {
            -0.1
        } else if f9_down && !last_f9_down {
            0.1
        } else {
            0.0
        };
        if volume_step != 0.0 {
            config.volume = ((config.volume + volume_step) * 10.0).round().clamp(0.0, 10.0) / 10.0;
            if let Some(s) = &sink {
                s.set_volume(config.volume);
            }
            config.save();
            update_window_title(&mut window, speed_multiplier, machine.mem.disk2.motor_on, config.volume);
        }
        last_f8_down = f8_down;
        last_f9_down = f9_down;

        let right_mouse_down = window.get_mouse_down(minifb::MouseButton::Right);
        if right_mouse_down && !last_right_mouse_down {
            if let Some(ref mut cb) = clipboard {
                if let Ok(text) = cb.get_text() {
                    for c in text.chars() {
                        let mut ascii = c as u32;
                        if ascii == 10 { ascii = 13; }
                        if ascii >= 97 && ascii <= 122 { ascii -= 32; }
                        if ascii < 128 { key_queue.push_back(ascii as u8); }
                    }
                }
            }
        }
        last_right_mouse_down = right_mouse_down;

        if (machine.mem.keyboard_latch & 0x80) == 0 {
            if let Some(ascii) = key_queue.pop_front() {
                machine.mem.keyboard_latch = 0x80 | ascii;
            }
        }

        let mut frame_cycles = 0;
        let mut audio_samples: Vec<f32> = Vec::with_capacity(1500);
        let auto_disk_turbo_active = machine.mem.disk2.motor_on;
        
        // If disk is on, we force FULL speed (unthrottled) for the best loading experience.
        // Otherwise use the manual speed_multiplier set by F4.
        let effective_speed_multiplier = if auto_disk_turbo_active {
            0.0
        } else {
            speed_multiplier
        };
        
        let is_full_speed = effective_speed_multiplier == 0.0;
        let desired_fps = if is_full_speed {
            0 // Unthrottled
        } else if effective_speed_multiplier > 1.0 {
            (60.0 * effective_speed_multiplier) as usize
        } else {
            60
        };

        if current_target_fps != desired_fps {
            window.set_target_fps(desired_fps);
            current_target_fps = desired_fps;
        }

        if (last_title_speed_multiplier - speed_multiplier).abs() > 0.001 || last_title_auto_disk_turbo != auto_disk_turbo_active {
            update_window_title(&mut window, speed_multiplier, auto_disk_turbo_active, config.volume);
            last_title_speed_multiplier = speed_multiplier;
            last_title_auto_disk_turbo = auto_disk_turbo_active;
        }

        // Wall-clock-driven pacing: run exactly as many emulated cycles as
        // real time has actually elapsed for (scaled by the speed multiplier),
        // instead of assuming a fixed cycle count per host frame. That fixed
        // assumption only held as long as minifb's FPS limiter was hitting
        // its target exactly - any drift, or auto disk-turbo skipping the
        // limiter (desired_fps=0) entirely, meant cycles (and therefore
        // audio) could race far ahead of real time.
        // Full/unthrottled speed is deliberately exempt: there "as fast as
        // the host can go" is the point, so it still just runs one base
        // frame's worth of cycles per loop iteration with no throttling.
        let now = Instant::now();
        let owed_secs = now.duration_since(last_cycle).as_secs_f64().min(MAX_OWED_SECS);
        last_cycle = now;
        let target_cycles = if is_full_speed {
            BASE_FRAME_CYCLES
        } else {
            (CPU_HZ * effective_speed_multiplier as f64 * owed_secs) as u32
        };
        while frame_cycles < target_cycles {
            let cycles = machine.step();
            frame_cycles += cycles;

            for edge_cycle in machine.mem.take_speaker_toggle_cycles() {
                audio_mixer.mix_until(edge_cycle as f64, cycles_per_sample, &mut dc_filter_x1, &mut dc_filter_y1, &mut audio_samples);
                audio_mixer.speaker_on = !audio_mixer.speaker_on;
            }
            audio_mixer.mix_until(machine.total_cycles as f64, cycles_per_sample, &mut dc_filter_x1, &mut dc_filter_y1, &mut audio_samples);
        }

        // Audio must represent "now", not a queue to work through. If the
        // sink has more than a couple of chunks backlogged (turbo burst,
        // disk-motor auto-turbo, a slow host frame, whatever the cause),
        // catching up later or playing it back pitch-shifted still means
        // what you hear lags behind what's on screen. Instead, once the
        // backlog passes a small slack threshold, drop it entirely by
        // rebuilding the sink and only append this frame's fresh samples -
        // a brief audible skip during a burst, but audio is never stale.
        //
        // At a steady manual speed step (1.2x/1.5x/2x/5x) cycles - and
        // therefore audio samples - are generated faster than real time on
        // purpose and *continuously*, not as an occasional burst. Playing
        // that queue back at normal speed would mean it structurally never
        // drains, hitting the drop-threshold over and over and clicking
        // constantly. So play it back pitch-shifted at that same multiplier
        // instead, keeping generation and playback in lockstep; the drop
        // fallback below then only has to catch genuine spikes (disk-motor
        // auto-turbo, a stalled host frame), not steady-state turbo.
        if let Some(s) = &sink {
            s.set_speed(if is_full_speed { 1.0 } else { effective_speed_multiplier });
        }
        const MAX_QUEUED_CHUNKS: usize = 2;
        if let Some(s) = &sink {
            if s.len() > MAX_QUEUED_CHUNKS {
                sink = rebuild_sink(audio_handle.as_ref(), config.volume);
                if let Some(s) = &sink {
                    s.set_speed(if is_full_speed { 1.0 } else { effective_speed_multiplier });
                }
                audio_mixer.reset_at(machine.total_cycles as f64, cycles_per_sample, machine.mem.speaker);
                dc_filter_x1 = 0.0; dc_filter_y1 = 0.0;
            }
        }
        if let Some(s) = &sink {
            if !audio_samples.is_empty() {
                s.append(rodio::buffer::SamplesBuffer::new(1, sample_rate, audio_samples));
            }
        }

        if machine.mem.text_mode { video.render_text_frame(&machine.mem, &char_rom); }
        else if machine.mem.hires_mode { video.render_hires_frame(&machine.mem, &char_rom); }
        else { video.render_lores_frame(&machine.mem, &char_rom); }

        window.update_with_buffer(&video.frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT).unwrap();
    }

    if machine.mem.disk2.is_dirty {
        let path_to_save = config.last_disk_path.as_ref().unwrap_or(&dsk_path);
        if let Ok(new_disk) = apple2_core::nibble::denibblize_dsk(&machine.mem.disk2.tracks) {
            let _ = save_disk_image(path_to_save, &new_disk);
        }
    }
}
