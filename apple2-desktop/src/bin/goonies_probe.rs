use std::io::Read;
use std::path::PathBuf;

use apple2_core::machine::Apple2Machine;
use apple2_core::nibble::TrackData;
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

fn dump_track_prologues(track: &TrackData, track_num: usize, limit: usize) {
    let mut found = 0usize;
    println!("track {} prologues:", track_num);
    for i in 0..track.length {
        if track.raw_bytes[i] == 0xD5
            && track.raw_bytes[(i + 1) % track.length] == 0xAA
            && track.raw_bytes[(i + 2) % track.length] == 0x96
        {
            println!(
                "  idx={:04} prev={:02X} next={:02X}",
                i,
                track.raw_bytes[(i + track.length - 1) % track.length],
                track.raw_bytes[(i + 3) % track.length]
            );
            found += 1;
            if found >= limit {
                break;
            }
        }
    }
    println!("track {} prologues listed={}", track_num, found);
}

fn nearest_prologue_distance(track: &TrackData, idx: usize) -> usize {
    let mut best = track.length;
    for i in 0..track.length {
        if track.raw_bytes[i] == 0xD5
            && track.raw_bytes[(i + 1) % track.length] == 0xAA
            && track.raw_bytes[(i + 2) % track.length] == 0x96
        {
            let forward = if i >= idx {
                i - idx
            } else {
                track.length - idx + i
            };
            let backward = if idx >= i {
                idx - i
            } else {
                idx + track.length - i
            };
            best = best.min(forward.min(backward));
        }
    }
    best
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
    dump_track_prologues(&machine.mem.disk2.tracks[23], 23, 16);

    let mut last_pc = machine.cpu.pc;
    let mut same_pc_count = 0u32;
    let mut dumped_loader_state = false;
    let mut pc_045f_hits = 0u32;
    let mut pc_0460_hits = 0u32;
    let mut pc_051f_hits = 0u32;
    let mut pc_0520_hits = 0u32;
    let mut trace_c08c_reads = 0u32;
    let mut pc_038b_hits = 0u32;
    let mut pc_0395_hits = 0u32;
    let mut pc_03a0_hits = 0u32;
    let mut pc_03ad_hits = 0u32;
    let mut pc_03b5_hits = 0u32;
    let mut rts_0380_logs = 0u32;
    let mut ret_0318_logs = 0u32;
    let mut path_059x_logs = 0u32;
    let mut path_0400_logs = 0u32;
    let mut post_seek_logs = 0u32;
    let mut dumped_0400_state = false;
    let mut stepper_logs = 0u32;

    for step in 0..2_000_000u32 {
        let pc_before = machine.cpu.pc;
        let cycles_before = machine.total_cycles;
        let track_before = machine.mem.disk2.current_track;
        let idx_before = machine.mem.disk2.byte_index;
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

        match pc_before {
            0x038B => pc_038b_hits += 1,
            0x0395 => pc_0395_hits += 1,
            0x03A0 => pc_03a0_hits += 1,
            0x03AD => pc_03ad_hits += 1,
            0x03B5 => pc_03b5_hits += 1,
            _ => {}
        }

        if matches!(pc_before, 0x038B | 0x0395 | 0x03A0 | 0x03AD | 0x03B5)
            && track_before == 23
            && trace_c08c_reads < 160
            && nearest_prologue_distance(&machine.mem.disk2.tracks[23], idx_before) <= 8
        {
            println!(
                "c08c_read pc={:04X} cycles={} -> {} a={:02X} p={:02X} idx={}=>{} latch={:02X}",
                pc_before,
                cycles_before,
                cycles,
                machine.cpu.a,
                machine.cpu.status.to_byte(),
                idx_before,
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
            trace_c08c_reads += 1;
        }

        if pc_before == 0x03DB && track_before == 23 && rts_0380_logs < 80 {
            let returned_to = machine.cpu.pc;
            let decoded_checksum = machine.mem.ram[0x00E6];
            let decoded_sector = machine.mem.ram[0x00E7];
            let decoded_track = machine.mem.ram[0x00E8];
            let decoded_volume = machine.mem.ram[0x00E9];
            let expected_track = machine.mem.ram[0x028E];
            let expected_volume = machine.mem.ram[0x026C];
            let sector_index = machine.mem.ram[0x026E] as usize;
            let expected_sector = machine.mem.ram[0x027E + sector_index];
            println!(
                "rts0380 cycles={} ret={:04X} carry={} vol={:02X} trk={:02X} sec={:02X} chk={:02X} expect_vol={:02X} expect_trk={:02X} expect_sec={:02X} sec_idx={:02X} idx={} qtr={}",
                cycles_before,
                returned_to,
                machine.cpu.status.c as u8,
                decoded_volume,
                decoded_track,
                decoded_sector,
                decoded_checksum,
                expected_volume,
                expected_track,
                expected_sector,
                machine.mem.ram[0x026E],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.current_qtr_track
            );
            rts_0380_logs += 1;
        }

        if matches!(
            pc_before,
            0x0596 | 0x05AA | 0x05B2 | 0x05BA | 0x05CD | 0x05D4
        ) && track_before == 23
            && path_059x_logs < 120
        {
            println!(
                "path059x pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} p={:02X} e4={:02X} e7={:02X} e8={:02X} e9={:02X} 028E={:02X} 026E={:02X} idx={}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x028E],
                machine.mem.ram[0x026E],
                machine.mem.disk2.byte_index
            );
            path_059x_logs += 1;
        }

        if pc_before == 0x03CB && track_before == 23 && ret_0318_logs < 80 {
            println!(
                "ret0318 next={:04X} carry={} a={:02X} x={:02X} y={:02X} p={:02X} e4={:02X} e6={:02X} e7={:02X} e8={:02X} e9={:02X} 028E={:02X} 028Ebit={:02X} idx={} latch={:02X}",
                machine.cpu.pc,
                machine.cpu.status.c as u8,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E6],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x028E],
                machine.mem.ram[0x028E] & 1,
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
            ret_0318_logs += 1;
        }

        if matches!(
            pc_before,
            0x0400
                | 0x0404
                | 0x0411
                | 0x0420
                | 0x0432
                | 0x0444
                | 0x045D
                | 0x0486
                | 0x04A0
                | 0x04B2
                | 0x04C8
                | 0x04E0
                | 0x0500
                | 0x0512
                | 0x051C
                | 0x052A
        ) && track_before == 23
            && path_0400_logs < 200
        {
            println!(
                "path0400 pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} p={:02X} e0={:02X} e1={:02X} e4={:02X} e6={:02X} e7={:02X} e8={:02X} e9={:02X} fe={:02X} ff={:02X} 028e={:02X} idx={}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E0],
                machine.mem.ram[0x00E1],
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E6],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x00FE],
                machine.mem.ram[0x00FF],
                machine.mem.ram[0x028E],
                machine.mem.disk2.byte_index
            );
            path_0400_logs += 1;
        }

        if matches!(
            pc_before,
            0x0500
                | 0x0504
                | 0x050A
                | 0x0512
                | 0x0515
                | 0x0519
                | 0x051C
                | 0x051F
                | 0x0520
                | 0x0524
                | 0x052A
        ) && track_before == 23
            && post_seek_logs < 240
        {
            println!(
                "postseek pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} e4={:02X} e5={:02X} e7={:02X} e8={:02X} e9={:02X} fe={:02X} ff={:02X} 028e={:02X} 026e={:02X} 0269={:02X} 05e4={:02X} 05ec={:02X} 05ed={:02X} idx={} latch={:02X} qtr={} phases={}{}{}{}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.sp,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E5],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x00FE],
                machine.mem.ram[0x00FF],
                machine.mem.ram[0x028E],
                machine.mem.ram[0x026E],
                machine.mem.ram[0x0269],
                machine.mem.ram[0x05E4],
                machine.mem.ram[0x05EC],
                machine.mem.ram[0x05ED],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch,
                machine.mem.disk2.current_qtr_track,
                if machine.mem.disk2.phases[0] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[1] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[2] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[3] {
                    '1'
                } else {
                    '0'
                },
            );
            post_seek_logs += 1;
        }

        if matches!(
            pc_before,
            0x044D | 0x0450 | 0x0457 | 0x045D | 0x0460 | 0x0467
        ) && track_before == 23
            && stepper_logs < 200
        {
            println!(
                "stepper pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} p={:02X} e5={:02X} fe={:02X} ff={:02X} qtr={} track={} idx={} latch={:02X} phases={}{}{}{}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E5],
                machine.mem.ram[0x00FE],
                machine.mem.ram[0x00FF],
                machine.mem.disk2.current_qtr_track,
                machine.mem.disk2.current_track,
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch,
                if machine.mem.disk2.phases[0] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[1] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[2] {
                    '1'
                } else {
                    '0'
                },
                if machine.mem.disk2.phases[3] {
                    '1'
                } else {
                    '0'
                },
            );
            stepper_logs += 1;
        }

        if !dumped_0400_state && pc_before == 0x0400 && track_before == 23 {
            dumped_0400_state = true;
            println!("entered 0400 consumer at cycle {}", cycles_before);
            dump_ram(&machine, 0x00E0, 0x10);
            dump_ram(&machine, 0x0200, 0x40);
            dump_ram(&machine, 0x0280, 0x40);
            dump_ram(&machine, 0x0300, 0x100);
            dump_ram(&machine, 0x0400, 0x80);
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
            println!(
                "entered loader region at cycle {} pc={:04X}",
                machine.total_cycles, machine.cpu.pc
            );
            dump_ram(&machine, 0x0020, 0x30);
            dump_ram(&machine, 0x0260, 0x30);
            dump_ram(&machine, 0x0318, 0x40);
            dump_ram(&machine, 0x0380, 0x80);
            dump_ram(&machine, 0x0450, 0x40);
            dump_ram(&machine, 0x0500, 0x100);
            dump_ram(&machine, 0x0800, 0x40);
        }

        if same_pc_count >= 100_000 {
            println!(
                "stuck loop detected after {} cycles at pc={:04X} last_step_cycles={}",
                machine.total_cycles, machine.cpu.pc, cycles
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
    dump_ram(&machine, 0x0318, 0x40);
    dump_ram(&machine, 0x0380, 0x80);
    dump_ram(&machine, 0x0450, 0x40);
    dump_ram(&machine, 0x0500, 0x100);
    println!(
        "pc hits: 045F={} 0460={} 051F={} 0520={}",
        pc_045f_hits, pc_0460_hits, pc_051f_hits, pc_0520_hits
    );
    println!(
        "loader read pcs: 038B={} 0395={} 03A0={} 03AD={} 03B5={}",
        pc_038b_hits, pc_0395_hits, pc_03a0_hits, pc_03ad_hits, pc_03b5_hits
    );
    dump_screen(&machine);
}
