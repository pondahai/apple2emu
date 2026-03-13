use std::collections::VecDeque;
use std::path::PathBuf;

use apple2_core::machine::Apple2Machine;

fn text_row_addr(row: usize) -> usize {
    let base = 0x0400usize;
    let block = row / 8;
    let offset = row % 8;
    base + offset * 128 + block * 40
}

fn screen_lines(machine: &Apple2Machine) -> Vec<String> {
    (0..24)
        .map(|row| {
            let mut line = String::with_capacity(40);
            let addr = text_row_addr(row);
            for col in 0..40 {
                let ch = machine.mem.ram[addr + col] & 0x7F;
                if (0x20..=0x7E).contains(&ch) {
                    line.push(ch as char);
                } else {
                    line.push(' ');
                }
            }
            line
        })
        .collect()
}

fn pump(machine: &mut Apple2Machine, queue: &mut VecDeque<u8>, target_cycles: u64) {
    let mut ran = 0u64;
    while ran < target_cycles {
        if (machine.mem.keyboard_latch & 0x80) == 0 {
            if let Some(ch) = queue.pop_front() {
                machine.mem.keyboard_latch = 0x80 | ch;
            }
        }
        ran += machine.step() as u64;
    }
}

fn enqueue_line(queue: &mut VecDeque<u8>, line: &str) {
    for b in line.bytes() {
        queue.push_back(if b.is_ascii_lowercase() { b.to_ascii_uppercase() } else { b });
    }
    queue.push_back(0x0D);
}

fn resolve_roms_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    root.join("roms")
}

fn main() {
    let roms = resolve_roms_dir();
    let main_rom = std::fs::read(roms.join("APPLE2PLUS.ROM")).expect("read APPLE2PLUS.ROM");
    let disk_rom = std::fs::read(roms.join("DISK2.ROM")).expect("read DISK2.ROM");
    let disk = std::fs::read(roms.join("MASTER.DSK")).expect("read MASTER.DSK");

    let mut machine = Apple2Machine::new();
    machine.load_rom(&main_rom[main_rom.len() - 12288..]);
    machine.mem.disk2.load_boot_rom(&disk_rom);
    machine.mem.disk2.load_disk(&disk);
    let before_tracks = machine.mem.disk2.tracks.iter().map(|t| t.raw_bytes).collect::<Vec<_>>();
    machine.power_on();

    let mut q = VecDeque::new();

    // Boot to DOS/BASIC prompt.
    pump(&mut machine, &mut q, 16_000_000);
    enqueue_line(&mut q, "CATALOG");
    pump(&mut machine, &mut q, 8_000_000);
    enqueue_line(&mut q, "NEW");
    pump(&mut machine, &mut q, 4_000_000);
    enqueue_line(&mut q, "10 PRINT \"HELLO\"");
    pump(&mut machine, &mut q, 4_000_000);

    enqueue_line(&mut q, "SAVE TEST");
    pump(&mut machine, &mut q, 16_000_000);

    enqueue_line(&mut q, "CATALOG");
    pump(&mut machine, &mut q, 12_000_000);

    let lines = screen_lines(&machine);
    let screen = lines.join("\n");
    let mut changed = 0usize;
    let has_error = screen.contains("ERROR #8");
    for (i, t) in machine.mem.disk2.tracks.iter().enumerate() {
        if t.raw_bytes != before_tracks[i] {
            changed += 1;
            if has_error {
                let mut first = None;
                let mut last = 0usize;
                let mut diffs = 0usize;
                for j in 0..t.raw_bytes.len() {
                    if t.raw_bytes[j] != before_tracks[i][j] {
                        diffs += 1;
                        if first.is_none() {
                            first = Some(j);
                        }
                        last = j;
                    }
                }
                if let Some(f) = first {
                    println!("Track {i} changed: bytes={diffs}, span={f}..{last}");
                    let end = (f + 16).min(t.raw_bytes.len());
                    print!("  before:");
                    for b in &before_tracks[i][f..end] {
                        print!(" {:02X}", b);
                    }
                    println!();
                    print!("  after :");
                    for b in &t.raw_bytes[f..end] {
                        print!(" {:02X}", b);
                    }
                    println!();
                }
            }
        }
    }
    println!("Tracks changed after SAVE flow: {changed}");

    println!("=== Screen Snapshot ===");
    for l in &lines {
        if l.trim().is_empty() {
            continue;
        }
        println!("{l}");
    }

    if has_error {
        eprintln!("FAIL: Found ERROR #8 on screen");
        std::process::exit(1);
    }

    if screen.contains("TEST") {
        println!("PASS: SAVE/CATALOG flow shows TEST entry");
    } else {
        println!("WARN: No TEST entry found; but no ERROR #8 observed");
    }
}
