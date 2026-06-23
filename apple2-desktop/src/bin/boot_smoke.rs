// Host-side boot smoke harness.
//
// Boots a .dsk/.gz image headlessly through the native Apple2Machine API and
// reports the disk seek sequence, CPU hot spots, video mode, motor status and
// the text screen (BOTH page 1 and page 2) so disk/timing/video regressions can
// be reproduced offline without the windowed frontend.
//
// This is the desktop counterpart of PicoApple2's `boot_test.rs`. Because
// apple2emu's `load_disk` nibblizes the whole image into memory up front, there
// is no per-track reload loop: stepping the machine rotates the disk on its own.
//
// Usage:
//   cargo run --release --bin boot_smoke                  # boots roms/MASTER.DSK for 15s
//   cargo run --release --bin boot_smoke -- path/to.dsk   # boot a specific image
//   BOOT_DSK=path BOOT_SECS=60 cargo run --bin boot_smoke
//
// Environment knobs:
//   BOOT_DSK=<path>       disk image (overrides the positional arg)
//   BOOT_SECS=<n>         emulated seconds to run (default 15)
//   BOOT_PRESS_BTN=<sec>  from this second on, pulse pushbutton 0 and sweep the
//                         paddles through the four corners (Goonies-style
//                         joystick calibration). 0 disables (default).
//   BOOT_TYPE=<string>    after boot, type this string into the keyboard latch,
//                         one char at a time, waiting for the program to consume
//                         each strobe. Use \r for Return, ^G for Ctrl-G, etc.

use std::io::Read as _;
use std::path::PathBuf;
use std::{env, process};

use apple2_core::machine::Apple2Machine;

const CPU_HZ: f64 = 1_020_500.0; // emulated 6502 clock used for the cycle->seconds scale

fn decode_disk_image(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let raw = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let is_gz = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false)
        || (raw.len() >= 2 && raw[0] == 0x1F && raw[1] == 0x8B);

    let mut disk = if is_gz {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(raw.as_slice())
            .read_to_end(&mut out)
            .map_err(|e| format!("gunzip {}: {e}", path.display()))?;
        out
    } else {
        raw
    };

    // Match the desktop frontend: accept images with up to 4096 bytes of
    // trailing padding (some .dsk dumps have it) by trimming to 143360.
    const EXPECTED: usize = 143_360;
    if disk.len() < EXPECTED {
        return Err(format!("{}: size {} (expected at least {EXPECTED})", path.display(), disk.len()));
    }
    if disk.len() > EXPECTED {
        let trailing = disk.len() - EXPECTED;
        if trailing > 4096 {
            return Err(format!("{}: size {} (trailing {trailing} too large to trim)", path.display(), disk.len()));
        }
        println!("note: trimming {trailing} trailing byte(s) to {EXPECTED}");
        disk.truncate(EXPECTED);
    }
    Ok(disk)
}

/// First-byte address of a text/lores row for the given display page.
/// `page_base` is $0400 (page 1) or $0800 (page 2); the interleave is identical.
fn text_row_addr(page_base: usize, row: usize) -> usize {
    page_base + (row % 8) * 128 + (row / 8) * 40
}

/// Print a text page if it holds any printable, non-blank content.
fn dump_text_page(machine: &Apple2Machine, page_base: usize, label: &str) {
    let mut rows = Vec::new();
    for row in 0..24 {
        let addr = text_row_addr(page_base, row);
        let mut line = String::with_capacity(40);
        for col in 0..40 {
            let ch = machine.mem.ram[addr + col] & 0x7F;
            line.push(if (0x20..0x7F).contains(&ch) { ch as char } else { '.' });
        }
        rows.push(line);
    }
    if rows.iter().any(|l| l.chars().any(|c| c != '.' && c != ' ')) {
        println!("--- text {label} (${page_base:04X}) ---");
        for (row, line) in rows.iter().enumerate() {
            if line.chars().any(|c| c != '.' && c != ' ') {
                println!("  row {row:2}: {line}");
            }
        }
    } else {
        println!("--- text {label} (${page_base:04X}): blank ---");
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Expand \r \n \t and ^X (Ctrl-X) escapes in BOOT_TYPE into raw key codes.
fn parse_type_string(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('r') => out.push(0x0D),
                Some('n') => out.push(0x0D),
                Some('t') => out.push(0x09),
                Some('\\') => out.push(b'\\'),
                Some(other) => out.push(other as u8),
                None => {}
            },
            '^' => {
                if let Some(k) = chars.next() {
                    out.push((k.to_ascii_uppercase() as u8) & 0x1F);
                }
            }
            _ => out.push(c as u8),
        }
    }
    out
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let roms_dir = root.join("roms");

    let disk_path = env::var("BOOT_DSK")
        .ok()
        .or_else(|| env::args().nth(1))
        .map(PathBuf::from)
        .unwrap_or_else(|| roms_dir.join("MASTER.DSK"));

    let seconds = env_u64("BOOT_SECS", 15);
    let press_btn_at = match env_u64("BOOT_PRESS_BTN", 0) {
        0 => None,
        s => Some(s),
    };
    let type_keys = env::var("BOOT_TYPE").ok().map(|s| parse_type_string(&s)).unwrap_or_default();
    let type_at_cycle = env_u64("BOOT_TYPE_AT", 0) * CPU_HZ as u64;

    let main_rom = std::fs::read(roms_dir.join("APPLE2PLUS.ROM"))
        .unwrap_or_else(|e| { eprintln!("read APPLE2PLUS.ROM: {e}"); process::exit(1); });
    let disk_rom = std::fs::read(roms_dir.join("DISK2.ROM"))
        .unwrap_or_else(|e| { eprintln!("read DISK2.ROM: {e}"); process::exit(1); });
    let disk = decode_disk_image(&disk_path)
        .unwrap_or_else(|e| { eprintln!("disk: {e}"); process::exit(1); });

    println!("==================================================");
    println!("BOOT: {}  ({seconds}s)", disk_path.display());
    if let Some(s) = press_btn_at {
        println!("  joystick calibration: pulse button 0 + sweep paddles from {s}s");
    }
    if !type_keys.is_empty() {
        println!("  typing {} key(s) after boot", type_keys.len());
    }

    let mut machine = Apple2Machine::new();
    machine.load_rom(&main_rom[main_rom.len() - 12_288..]);
    machine.mem.disk2.load_boot_rom(&disk_rom);
    machine.mem.disk2.load_disk(&disk);
    machine.power_on();

    let target = seconds * CPU_HZ as u64;
    let mut cycles: u64 = 0;
    let mut last_track: i32 = -1;
    let mut seek_seq: Vec<(u8, f64)> = Vec::new();
    let mut pc_hist: std::collections::HashMap<u16, u32> = std::collections::HashMap::new();
    let mut type_idx = 0usize;
    // BOOT_TRACE=N: once typing starts, record up to N run-length-deduped PCs to
    // reveal where control flow goes after a key is accepted.
    let trace_cap = env_u64("BOOT_TRACE", 0) as usize;
    let mut trace: Vec<u16> = Vec::new();
    // BOOT_WATCH_PC=HEX: capture the A register every time this PC executes
    // (after BOOT_TYPE_AT), revealing the byte stream a read loop consumes.
    let watch_pc = env::var("BOOT_WATCH_PC").ok().and_then(|s| u16::from_str_radix(s.trim(), 16).ok());
    let mut watch_samples: Vec<u8> = Vec::new();
    let wall_start = std::time::Instant::now();

    while cycles < target {
        let pc_before = machine.cpu.pc;
        cycles += machine.step() as u64;

        if trace_cap > 0 && cycles >= type_at_cycle && trace.len() < trace_cap {
            if trace.last() != Some(&pc_before) {
                trace.push(pc_before);
            }
        }

        if let Some(wpc) = watch_pc {
            if pc_before == wpc && cycles >= type_at_cycle && watch_samples.len() < 4096 {
                watch_samples.push(machine.cpu.a);
            }
        }

        // Track seek sequence (record each whole-track change).
        let track = machine.mem.disk2.current_track as i32;
        if track != last_track {
            last_track = track;
            if seek_seq.len() < 512 {
                seek_seq.push((track as u8, cycles as f64 / CPU_HZ));
            }
        }

        // PC hotspot sampling over the final ~2s window.
        if cycles > target.saturating_sub(2 * CPU_HZ as u64) {
            *pc_hist.entry(machine.cpu.pc).or_insert(0) += 1;
        }

        // Goonies-style joystick calibration: pulse button 0 for ~0.5s every 3s
        // and rotate the paddles through the four extreme corners.
        if let Some(at) = press_btn_at {
            let t0 = at * CPU_HZ as u64;
            if cycles >= t0 {
                let phase = (cycles - t0) % 3_061_500;
                let step = ((cycles - t0) / 3_061_500) % 4;
                let (px, py) = match step { 0 => (0, 0), 1 => (255, 0), 2 => (255, 255), _ => (0, 255) };
                machine.mem.set_joystick_state(px, py, phase < 510_000, false);
            }
        }

        // Type keys (after BOOT_TYPE_AT) once the latch strobe has been consumed.
        if cycles >= type_at_cycle && type_idx < type_keys.len() && (machine.mem.keyboard_latch & 0x80) == 0 {
            machine.mem.keyboard_latch = (type_keys[type_idx] & 0x7F) | 0x80;
            type_idx += 1;
        }
    }

    let wall = wall_start.elapsed().as_secs_f64();
    let emulated_mhz = cycles as f64 / wall / 1_000_000.0;

    // --- report ---
    println!();
    let seq: Vec<String> = seek_seq.iter().map(|(t, s)| format!("T{t}@{s:.1}s")).collect();
    println!("seek sequence ({}): {}", seek_seq.len(), seq.join(" "));

    let motor_spinning = machine.mem.disk2.motor_on || machine.mem.disk2.motor_timer_cycles > 0;
    let mode = if machine.mem.text_mode {
        "TEXT"
    } else if machine.mem.hires_mode {
        "HIRES"
    } else {
        "LORES"
    };
    println!(
        "video: {mode}{}{}  motor: {}  final pc: {:04X}  track: {}",
        if machine.mem.mixed_mode { "+MIXED" } else { "" },
        if machine.mem.page2 { "+PAGE2" } else { "" },
        if motor_spinning { "spinning" } else { "stopped" },
        machine.cpu.pc,
        machine.mem.disk2.current_track,
    );
    println!(
        "ran {cycles} cycles in {wall:.2}s host -> {emulated_mhz:.1} emulated MHz ({:.0}x realtime)",
        emulated_mhz / 1.0205
    );

    // Scan the track the head ended on for standard address/data prologues.
    {
        let t = machine.mem.disk2.current_track;
        if let Some(track) = machine.mem.disk2.tracks.get(t) {
            let len = track.length;
            let b = &track.raw_bytes;
            let (mut addr_pro, mut data_pro) = (0u32, 0u32);
            if len >= 3 {
                for i in 0..len {
                    let x = b[i];
                    let y = b[(i + 1) % len];
                    let z = b[(i + 2) % len];
                    if x == 0xD5 && y == 0xAA && z == 0x96 { addr_pro += 1; }
                    if x == 0xD5 && y == 0xAA && z == 0xAD { data_pro += 1; }
                }
            }
            println!("track {t} (len {len}): D5AA96 addr-prologues={addr_pro}  D5AAAD data-prologues={data_pro}");
        }
    }

    let mut hot: Vec<(u16, u32)> = pc_hist.into_iter().collect();
    hot.sort_by(|a, b| b.1.cmp(&a.1));
    let hot_str: Vec<String> = hot.iter().take(8).map(|(pc, n)| format!("{pc:04X}x{n}")).collect();
    println!("hot PCs (last 2s): {}", hot_str.join(" "));
    if hot.len() <= 2 {
        println!("  (note: very few distinct PCs -> likely a tight wait/idle loop, e.g. BASIC ready)");
    }

    // BOOT_DUMP=HEXADDR,HEXLEN: hex-dump an arbitrary RAM range (for inspecting
    // the code/data the machine is sitting in).
    if let Ok(spec) = env::var("BOOT_DUMP") {
        let parts: Vec<usize> = spec.split(',').filter_map(|s| usize::from_str_radix(s.trim(), 16).ok()).collect();
        if let [start, len] = parts[..] {
            println!();
            for i in 0..len {
                if i % 16 == 0 {
                    print!("\n  {:04X}:", start + i);
                }
                print!(" {:02X}", machine.mem.ram[(start + i) & 0xFFFF]);
            }
            println!();
        }
    }

    if !trace.is_empty() {
        let t: Vec<String> = trace.iter().map(|pc| format!("{pc:04X}")).collect();
        println!("\nPC trace after key ({}): {}", trace.len(), t.join(" "));
    }

    if let Some(wpc) = watch_pc {
        let mut hist = [0u32; 256];
        for &b in &watch_samples {
            hist[b as usize] += 1;
        }
        let mut top: Vec<(usize, u32)> = hist.iter().enumerate().filter(|&(_, &n)| n > 0).map(|(b, &n)| (b, n)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        let top_str: Vec<String> = top.iter().take(12).map(|(b, n)| format!("{b:02X}x{n}")).collect();
        let first: Vec<String> = watch_samples.iter().take(48).map(|b| format!("{b:02X}")).collect();
        println!("\nA at {wpc:04X} ({} samples)\n  byte histogram: {}\n  first 48: {}",
            watch_samples.len(), top_str.join(" "), first.join(" "));
    }

    println!();
    dump_text_page(&machine, 0x0400, "page 1");
    dump_text_page(&machine, 0x0800, "page 2");
}
