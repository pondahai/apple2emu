use std::io::Read;
use std::path::PathBuf;

use apple2_core::machine::Apple2Machine;
use flate2::read::GzDecoder;

fn decode_disk_image(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let raw_data =
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let is_gz_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);
    let is_gz_magic = raw_data.len() >= 2 && raw_data[0] == 0x1F && raw_data[1] == 0x8B;

    let disk = if is_gz_ext || is_gz_magic {
        let mut decoder = GzDecoder::new(raw_data.as_slice());
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| format!("Failed to decompress {}: {}", path.display(), e))?;
        out
    } else {
        raw_data
    };

    if disk.len() != 143_360 {
        return Err(format!(
            "Disk size mismatch for {}: {} (expected 143360 bytes)",
            path.display(),
            disk.len()
        ));
    }

    Ok(disk)
}

fn text_row_addr(row: usize) -> usize {
    let base = 0x0400usize;
    let block = row / 8;
    let offset = row % 8;
    base + offset * 128 + block * 40
}

fn dump_screen(machine: &Apple2Machine) {
    for row in 0..24 {
        let mut line = String::with_capacity(40);
        let addr = text_row_addr(row);
        for col in 0..40 {
            let ch = machine.mem.ram[addr + col] & 0x7F;
            if (0x20..=0x7E).contains(&ch) {
                line.push(ch as char);
            } else {
                line.push('.');
            }
        }
        if line.chars().any(|c| c != '.') {
            println!("Row {:2}: {}", row, line);
        }
    }
}

fn dump_ram(machine: &Apple2Machine, start: usize, len: usize) {
    print!("{start:04X}:");
    for i in 0..len {
        print!(" {:02X}", machine.mem.ram[start + i]);
    }
    println!();
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let roms_dir = root.join("roms");
    let disk_path =
        PathBuf::from(r"C:\Users\pondahai\Downloads\AppleWin1.26.1.1\ac\goonies.dsk.gz");

    let main_rom = std::fs::read(roms_dir.join("APPLE2PLUS.ROM")).expect("read APPLE2PLUS.ROM");
    let disk_rom = std::fs::read(roms_dir.join("DISK2.ROM")).expect("read DISK2.ROM");
    let disk = decode_disk_image(&disk_path).expect("decode goonies");

    let mut machine = Apple2Machine::new();
    machine.load_rom(&main_rom[main_rom.len() - 12_288..]);
    machine.mem.disk2.load_boot_rom(&disk_rom);
    machine.mem.disk2.load_disk(&disk);
    machine.power_on();

    let mut last_pc = machine.cpu.pc;
    let mut same_pc_count = 0u32;
    let mut dumped_loader_state = false;
    let mut pc_045f_hits = 0u32;
    let mut pc_0460_hits = 0u32;
    let mut pc_051f_hits = 0u32;
    let mut pc_0520_hits = 0u32;

    for step in 0..2_000_000u32 {
        let cycles = machine.step();

        if machine.cpu.pc == last_pc {
            same_pc_count += 1;
        } else {
            same_pc_count = 0;
            last_pc = machine.cpu.pc;
        }

        if step % 50_000 == 0 {
            println!(
                "step={} cycles={} pc={:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} qtr={} track={} idx={} latch={:02X}",
                step,
                machine.total_cycles,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.sp,
                machine.cpu.status.to_byte(),
                machine.mem.disk2.current_qtr_track,
                machine.mem.disk2.current_track,
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
        }

        match machine.cpu.pc {
            0x045F => pc_045f_hits += 1,
            0x0460 => pc_0460_hits += 1,
            0x051F => pc_051f_hits += 1,
            0x0520 => pc_0520_hits += 1,
            _ => {}
        }

        if !dumped_loader_state && (0x0450..=0x0530).contains(&machine.cpu.pc) {
            dumped_loader_state = true;
            println!("entered loader region at cycle {} pc={:04X}", machine.total_cycles, machine.cpu.pc);
            dump_ram(&machine, 0x0020, 0x30);
            dump_ram(&machine, 0x0260, 0x30);
            dump_ram(&machine, 0x03D0, 0x20);
            dump_ram(&machine, 0x0380, 0x40);
            dump_ram(&machine, 0x0450, 0x40);
            dump_ram(&machine, 0x0500, 0x40);
            dump_ram(&machine, 0x0800, 0x40);
        }

        if same_pc_count >= 100_000 {
            println!(
                "stuck loop detected after {} cycles at pc={:04X} last_step_cycles={}",
                machine.total_cycles,
                machine.cpu.pc,
                cycles
            );
            break;
        }
    }

    println!(
        "final pc={:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} qtr={} track={} idx={} latch={:02X}",
        machine.cpu.pc,
        machine.cpu.a,
        machine.cpu.x,
        machine.cpu.y,
        machine.cpu.sp,
        machine.cpu.status.to_byte(),
        machine.mem.disk2.current_qtr_track,
        machine.mem.disk2.current_track,
        machine.mem.disk2.byte_index,
        machine.mem.disk2.data_latch
    );
    dump_ram(&machine, 0x0260, 0x30);
    dump_ram(&machine, 0x0380, 0x40);
    dump_ram(&machine, 0x0450, 0x40);
    dump_ram(&machine, 0x0500, 0x40);
    println!(
        "pc hits: 045F={} 0460={} 051F={} 0520={}",
        pc_045f_hits, pc_0460_hits, pc_051f_hits, pc_0520_hits
    );
    dump_screen(&machine);
}
