use std::io::Read;
use std::path::PathBuf;
use std::{env, fmt::Write as _};

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

fn peek_byte(machine: &Apple2Machine, addr: u16) -> u8 {
    match addr {
        0x0000..=0xBFFF => machine.mem.ram[addr as usize],
        0xD000..=0xFFFF => machine.mem.rom[(addr - 0xD000) as usize],
        _ => 0,
    }
}

fn peek_word(machine: &Apple2Machine, addr: u16) -> u16 {
    let lo = peek_byte(machine, addr) as u16;
    let hi = peek_byte(machine, addr.wrapping_add(1)) as u16;
    (hi << 8) | lo
}

fn classify_control_transfer(machine: &Apple2Machine, from: u16, to: u16) -> &'static str {
    let opcode = peek_byte(machine, from);
    let brk_vector = peek_word(machine, 0xFFFE);
    let reset_vector = peek_word(machine, 0xFFFC);

    if opcode == 0x00 && to == brk_vector {
        "brk_vector"
    } else if opcode == 0x4C {
        "jmp_abs"
    } else if opcode == 0x6C {
        "jmp_ind"
    } else if to == reset_vector {
        "reset_vector"
    } else {
        "other"
    }
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

fn encode_4x4(val: u8) -> (u8, u8) {
    ((val >> 1) | 0xAA, val | 0xAA)
}

fn patch_address_field_volume(track: &mut TrackData, volume: u8) {
    for i in 0..track.length {
        if track.raw_bytes[i] == 0xD5
            && track.raw_bytes[(i + 1) % track.length] == 0xAA
            && track.raw_bytes[(i + 2) % track.length] == 0x96
        {
            let trk = ((track.raw_bytes[(i + 5) % track.length] << 1) | 1)
                & track.raw_bytes[(i + 6) % track.length];
            let sec = ((track.raw_bytes[(i + 7) % track.length] << 1) | 1)
                & track.raw_bytes[(i + 8) % track.length];
            let chk = volume ^ trk ^ sec;
            let (v1, v2) = encode_4x4(volume);
            let (c1, c2) = encode_4x4(chk);
            track.raw_bytes[(i + 3) % track.length] = v1;
            track.raw_bytes[(i + 4) % track.length] = v2;
            track.raw_bytes[(i + 9) % track.length] = c1;
            track.raw_bytes[(i + 10) % track.length] = c2;
        }
    }
}

fn env_flag(name: &str) -> bool {
    match env::var(name) {
        Ok(val) => matches!(val.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

fn env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|val| val.parse::<u32>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str) -> Option<usize> {
    env::var(name).ok().and_then(|val| val.parse::<usize>().ok())
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
    let patch_volume_zero = env_flag("GOONIES_PATCH_VOLUME_ZERO");
    let stretch_track23_read_len = env_flag("GOONIES_STRETCH_TRACK23_READLEN");
    let max_steps = env_u32("GOONIES_MAX_STEPS", 2_000_000);
    let track23_read_len_override = env_usize("GOONIES_TRACK23_READLEN");

    if let Some(track) = machine.mem.disk2.tracks.get_mut(23) {
        if let Some(read_len) = track23_read_len_override {
            track.read_length = read_len;
        } else if stretch_track23_read_len {
            track.read_length = track.raw_bytes.len();
        }
    }
    if patch_volume_zero {
        for track in &mut machine.mem.disk2.tracks {
            patch_address_field_volume(track, 0x00);
        }
    }
    println!(
        "probe options: patch_volume_zero={} stretch_track23_read_len={} track23_read_len_override={:?} max_steps={}",
        patch_volume_zero,
        stretch_track23_read_len,
        track23_read_len_override,
        max_steps
    );
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
    let mut decision_logs = 0u32;
    let mut retry_logs = 0u32;
    let mut read0318_logs = 0u32;
    let mut patch028e_logs = 0u32;
    let mut enter0400_logs = 0u32;
    let mut fail0318_logs = 0u32;
    let mut enter_faxx_logged = false;
    let mut high_jump_logs = 0u32;
    let mut seen_0400_consumer = false;
    let mut watch_4c_logs = 0u32;
    let mut call0300_logs = 0u32;
    let mut path05a0_logs = 0u32;
    let mut prev_4c_window = [0u8; 0x80];

    prev_4c_window.copy_from_slice(&machine.mem.ram[0x4C00..0x4C80]);

    for step in 0..max_steps {
        let pc_before = machine.cpu.pc;
        let cycles_before = machine.total_cycles;
        let track_before = machine.mem.disk2.current_track;
        let idx_before = machine.mem.disk2.byte_index;
        let cycles = machine.step();

        if seen_0400_consumer && watch_4c_logs < 32 {
            let current_4c = &machine.mem.ram[0x4C00..0x4C80];
            let mut changed_offsets = String::new();
            let mut change_count = 0usize;
            let mut any_nonzero = false;

            for (idx, (&before, &after)) in prev_4c_window.iter().zip(current_4c.iter()).enumerate() {
                if after != 0 {
                    any_nonzero = true;
                }
                if before != after && change_count < 8 {
                    if !changed_offsets.is_empty() {
                        changed_offsets.push(' ');
                    }
                    let _ = write!(changed_offsets, "{:02X}:{:02X}->{:02X}", idx, before, after);
                    change_count += 1;
                }
            }

            if change_count > 0 {
                println!(
                    "write_4c pc={:04X}->{:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} nonzero={} qtr={} track={} idx={} latch={:02X}",
                    pc_before,
                    machine.cpu.pc,
                    machine.cpu.a,
                    machine.cpu.x,
                    machine.cpu.y,
                    machine.cpu.sp,
                    machine.cpu.status.to_byte(),
                    any_nonzero,
                    machine.mem.disk2.current_qtr_track,
                    machine.mem.disk2.current_track,
                    machine.mem.disk2.byte_index,
                    machine.mem.disk2.data_latch
                );
                println!("write_4c_changes {}", changed_offsets);
                dump_ram(&machine, 0x4C00, 0x80);
                watch_4c_logs += 1;
            }

            prev_4c_window.copy_from_slice(current_4c);
        }

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

        if matches!(
            pc_before,
            0x0535
                | 0x0538
                | 0x053B
                | 0x0540
                | 0x0547
                | 0x0553
                | 0x055C
                | 0x058B
                | 0x0591
                | 0x0596
                | 0x059A
                | 0x05A0
                | 0x05AA
                | 0x05B2
                | 0x05BA
                | 0x05CD
                | 0x05D4
        ) && track_before == 23
            && decision_logs < 220
        {
            let sector_index = machine.mem.ram[0x026E] as usize;
            let expected_sector = machine.mem.ram[0x027E + sector_index];
            println!(
                "decision pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} p={:02X} c={} e4={:02X} e5={:02X} e6={:02X} e7={:02X} e8={:02X} e9={:02X} 0269={:02X} 026c={:02X} 026e={:02X} exp_sec={:02X} 028e={:02X} idx={} latch={:02X}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.cpu.status.c as u8,
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E5],
                machine.mem.ram[0x00E6],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x0269],
                machine.mem.ram[0x026C],
                machine.mem.ram[0x026E],
                expected_sector,
                machine.mem.ram[0x028E],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
            decision_logs += 1;
        }

        if pc_before == 0x059A && track_before == 23 && retry_logs < 80 {
            let sector_index = machine.mem.ram[0x026E] as usize;
            let expected_sector = machine.mem.ram[0x027E + sector_index];
            println!(
                "retry059A next={:04X} c={} decoded(vol/trk/sec/chk)={:02X}/{:02X}/{:02X}/{:02X} expected(vol/trk/sec)={:02X}/{:02X}/{:02X} 0269={:02X} 026e={:02X} 028e={:02X} idx={} qtr={}",
                machine.cpu.pc,
                machine.cpu.status.c as u8,
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E6],
                machine.mem.ram[0x026C],
                machine.mem.ram[0x028E],
                expected_sector,
                machine.mem.ram[0x0269],
                machine.mem.ram[0x026E],
                machine.mem.ram[0x028E],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.current_qtr_track
            );
            retry_logs += 1;
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

        if pc_before == 0x03CB && track_before == 23 && read0318_logs < 80 {
            let mut sector_head = String::new();
            for i in 0..16usize {
                if i != 0 {
                    sector_head.push(' ');
                }
                let _ = write!(sector_head, "{:02X}", machine.mem.ram[0x0200 + i]);
            }
            let mut zp_head = String::new();
            for addr in [0x00E0usize, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7, 0x00E8, 0x00E9] {
                if !zp_head.is_empty() {
                    zp_head.push(' ');
                }
                let _ = write!(zp_head, "{:02X}", machine.mem.ram[addr]);
            }
            println!(
                "read0318 next={:04X} carry={} sector[0200..020F]={} zp[e0..e9]={} idx={} qtr={}",
                machine.cpu.pc,
                machine.cpu.status.c as u8,
                sector_head,
                zp_head,
                machine.mem.disk2.byte_index,
                machine.mem.disk2.current_qtr_track
            );
            read0318_logs += 1;
        }

        if pc_before == 0x0300 && call0300_logs < 80 {
            let dest = machine.mem.ram[0x00F8] as u16 | ((machine.mem.ram[0x00F9] as u16) << 8);
            println!(
                "call0300 next={:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} f8={:02X} f9={:02X} dest={:04X} b8[0..15]={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.sp,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00F8],
                machine.mem.ram[0x00F9],
                dest,
                machine.mem.ram[0xB800],
                machine.mem.ram[0xB801],
                machine.mem.ram[0xB802],
                machine.mem.ram[0xB803],
                machine.mem.ram[0xB804],
                machine.mem.ram[0xB805],
                machine.mem.ram[0xB806],
                machine.mem.ram[0xB807],
                machine.mem.ram[0xB808],
                machine.mem.ram[0xB809],
                machine.mem.ram[0xB80A],
                machine.mem.ram[0xB80B],
                machine.mem.ram[0xB80C],
                machine.mem.ram[0xB80D],
                machine.mem.ram[0xB80E],
                machine.mem.ram[0xB80F]
            );
            call0300_logs += 1;
        }

        if matches!(pc_before, 0x059D | 0x059E | 0x05A0 | 0x05A2 | 0x05A5)
            && track_before == 23
            && path05a0_logs < 120
        {
            println!(
                "path05a0 pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} e0={:02X} f8={:02X} f9={:02X} 028e={:02X} idx={} latch={:02X}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.sp,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E0],
                machine.mem.ram[0x00F8],
                machine.mem.ram[0x00F9],
                machine.mem.ram[0x028E],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
            path05a0_logs += 1;
        }

        if matches!(pc_before, 0x0366 | 0x0370 | 0x037A) && track_before == 23 && fail0318_logs < 120 {
            let reason = match pc_before {
                0x0366 => "checksum_cmp",
                0x0370 => "epilogue_de",
                0x037A => "epilogue_aa",
                _ => unreachable!(),
            };
            println!(
                "fail0318 kind={} next={:04X} a={:02X} x={:02X} y={:02X} p={:02X} cmp_target={:02X} e0={:02X} e4={:02X} e6={:02X} e7={:02X} e8={:02X} e9={:02X} idx={} latch={:02X}",
                reason,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x0200 + machine.cpu.y as usize],
                machine.mem.ram[0x00E0],
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E6],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.disk2.byte_index,
                machine.mem.disk2.data_latch
            );
            fail0318_logs += 1;
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

        if matches!(pc_before, 0x05BA | 0x05BC | 0x05BF | 0x05C2 | 0x05C5 | 0x05C7)
            && track_before == 23
            && patch028e_logs < 120
        {
            let mut table = String::new();
            for i in 0..16usize {
                if i != 0 {
                    table.push(' ');
                }
                let _ = write!(table, "{:02X}", machine.mem.ram[0x028E + i]);
            }
            println!(
                "patch028e pc={:04X} next={:04X} a={:02X} x={:02X} y={:02X} e4={:02X} 028e..029d={}",
                pc_before,
                machine.cpu.pc,
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.mem.ram[0x00E4],
                table
            );
            patch028e_logs += 1;
        }

        if pc_before == 0x0400 && track_before == 23 && enter0400_logs < 80 {
            seen_0400_consumer = true;
            let mut sector_head = String::new();
            for i in 0..32usize {
                if i != 0 {
                    sector_head.push(' ');
                }
                let _ = write!(sector_head, "{:02X}", machine.mem.ram[0x0200 + i]);
            }
            let mut table = String::new();
            for i in 0..16usize {
                if i != 0 {
                    table.push(' ');
                }
                let _ = write!(table, "{:02X}", machine.mem.ram[0x028E + i]);
            }
            println!(
                "enter0400 a={:02X} x={:02X} y={:02X} p={:02X} e0={:02X} e4={:02X} e7={:02X} e8={:02X} e9={:02X} 0269={:02X} 026e={:02X} 028e..029d={} sector[0200..021f]={}",
                machine.cpu.a,
                machine.cpu.x,
                machine.cpu.y,
                machine.cpu.status.to_byte(),
                machine.mem.ram[0x00E0],
                machine.mem.ram[0x00E4],
                machine.mem.ram[0x00E7],
                machine.mem.ram[0x00E8],
                machine.mem.ram[0x00E9],
                machine.mem.ram[0x0269],
                machine.mem.ram[0x026E],
                table,
                sector_head
            );
            enter0400_logs += 1;
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

        if !enter_faxx_logged && (0xFA00..=0xFAFF).contains(&machine.cpu.pc) {
            enter_faxx_logged = true;
            let opcode = peek_byte(&machine, pc_before);
            let op1 = peek_byte(&machine, pc_before.wrapping_add(1));
            let op2 = peek_byte(&machine, pc_before.wrapping_add(2));
            println!(
                "enter_faxx from={:04X} to={:04X} kind={} opcode={:02X} ops={:02X} {:02X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} qtr={} track={} idx={} latch={:02X}",
                pc_before,
                machine.cpu.pc,
                classify_control_transfer(&machine, pc_before, machine.cpu.pc),
                opcode,
                op1,
                op2,
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
            let stack_base = 0x0100usize + machine.cpu.sp as usize;
            let start = stack_base.saturating_sub(8).max(0x0100);
            let len = (0x01FFusize - start).min(24);
            dump_ram(&machine, start, len);
            dump_ram(&machine, 0x00E0, 0x20);
            dump_ram(&machine, 0x0200, 0x40);
            dump_ram(&machine, 0x0280, 0x40);
        }

        if seen_0400_consumer
            && high_jump_logs < 64
            && !matches!(pc_before, 0xFA00..=0xFFFF)
            && matches!(machine.cpu.pc, 0xF800..=0xFFFF)
        {
            let opcode = peek_byte(&machine, pc_before);
            let op1 = peek_byte(&machine, pc_before.wrapping_add(1));
            let op2 = peek_byte(&machine, pc_before.wrapping_add(2));
            println!(
                "high_jump from={:04X} to={:04X} kind={} opcode={:02X} ops={:02X} {:02X} a={:02X} x={:02X} y={:02X} sp={:02X} p={:02X} qtr={} track={} idx={} latch={:02X}",
                pc_before,
                machine.cpu.pc,
                classify_control_transfer(&machine, pc_before, machine.cpu.pc),
                opcode,
                op1,
                op2,
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
            dump_ram(&machine, 0x00E0, 0x20);
            dump_ram(&machine, 0x0200, 0x40);
            dump_ram(&machine, 0x0280, 0x40);
            dump_ram(&machine, 0x0400, 0x80);
            if pc_before >= 0x0200 {
                let region_start = pc_before.saturating_sub(0x20) as usize;
                dump_ram(&machine, region_start, 0x60);
            }
            high_jump_logs += 1;
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
