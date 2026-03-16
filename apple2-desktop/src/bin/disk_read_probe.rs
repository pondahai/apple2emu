use std::path::PathBuf;

use apple2_core::machine::Apple2Machine;

const TRACE_BYTES_AFTER_PROLOGUE: usize = 24;

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
    machine.load_rom(&main_rom[main_rom.len() - 12_288..]);
    machine.mem.disk2.load_boot_rom(&disk_rom);
    machine.mem.disk2.load_disk(&disk);
    machine.power_on();

    let mut prev_motor = machine.mem.disk2.motor_on;
    let mut prev_write_mode = machine.mem.disk2.write_mode;
    let mut prev_load_mode = machine.mem.disk2.load_mode;
    let mut prev_track = machine.mem.disk2.current_track;
    let mut prev_qtr = machine.mem.disk2.current_qtr_track;
    let mut prev_index = machine.mem.disk2.byte_index;
    let mut prev_bit_phase = machine.mem.disk2.read_bit_phase;
    let mut prev_latch_high = (machine.mem.disk2.data_latch & 0x80) != 0;
    let mut prev_shift_high = (machine.mem.disk2.read_shift_register & 0x80) != 0;

    let mut event_count = 0usize;
    let max_events = 400usize;
    let mut recent_bytes = [0u8; 3];
    let mut recent_len = 0usize;
    let mut trace_bytes_remaining = 0usize;
    let mut trace_reason: Option<&'static str> = None;
    let mut trace_start_idx = 0usize;

    println!(
        "disk_read_probe start pc={:04X} qtr={} track={} idx={} latch={:02X}",
        machine.cpu.pc,
        machine.mem.disk2.current_qtr_track,
        machine.mem.disk2.current_track,
        machine.mem.disk2.byte_index,
        machine.mem.disk2.data_latch
    );

    for step in 0..500_000u32 {
        let pc_before = machine.cpu.pc;
        let total_before = machine.total_cycles;
        let cycles = machine.step();
        let disk = &machine.mem.disk2;

        let latch_high = (disk.data_latch & 0x80) != 0;
        let shift_high = (disk.read_shift_register & 0x80) != 0;
        let byte_boundary = prev_bit_phase == 7 && disk.read_bit_phase == 0;
        let changed = prev_motor != disk.motor_on
            || prev_write_mode != disk.write_mode
            || prev_load_mode != disk.load_mode
            || prev_track != disk.current_track
            || prev_qtr != disk.current_qtr_track
            || prev_index != disk.byte_index
            || (prev_latch_high != latch_high)
            || (prev_shift_high != shift_high)
            || byte_boundary;

        if byte_boundary && !disk.load_mode && !disk.write_mode {
            let completed_byte = disk.read_shift_register;
            if recent_len < recent_bytes.len() {
                recent_bytes[recent_len] = completed_byte;
                recent_len += 1;
            } else {
                recent_bytes.rotate_left(1);
                recent_bytes[recent_bytes.len() - 1] = completed_byte;
            }

            if recent_len == 3 {
                let matched = match recent_bytes {
                    [0xD5, 0xAA, 0x96] => Some("addr-prologue"),
                    [0xD5, 0xAA, 0xAD] => Some("data-prologue"),
                    _ => None,
                };

                if let Some(reason) = matched {
                    trace_bytes_remaining = TRACE_BYTES_AFTER_PROLOGUE;
                    trace_reason = Some(reason);
                    trace_start_idx = disk.byte_index.saturating_sub(1);
                    println!(
                        concat!(
                            "event={:03} step={} cyc={} +{} pc={:04X} ",
                            "PROLOGUE {} start_idx={} q6={} q7={} qtr={} track={} ",
                            "idx={} latch={:02X} shift={:02X}"
                        ),
                        event_count,
                        step,
                        total_before,
                        cycles,
                        pc_before,
                        reason,
                        trace_start_idx,
                        disk.write_mode as u8,
                        disk.load_mode as u8,
                        disk.current_qtr_track,
                        disk.current_track,
                        disk.byte_index,
                        disk.data_latch,
                        completed_byte
                    );
                    event_count += 1;
                }
            }
        }

        let tracing = trace_bytes_remaining > 0;
        if changed && tracing {
            let reason = trace_reason.unwrap_or("trace");
            let trace_offset = TRACE_BYTES_AFTER_PROLOGUE - trace_bytes_remaining;
            let event_kind = if byte_boundary { "BYTE" } else { "STATE" };
            println!(
                concat!(
                    "event={:03} step={} cyc={} +{} pc={:04X} {} {} ",
                    "motor={} q6={} q7={} qtr={} track={} idx={} ",
                    "start_idx={} off={} remain={} latch={:02X}({}) shift={:02X}({}) bit={}"
                ),
                event_count,
                step,
                total_before,
                cycles,
                pc_before,
                reason,
                event_kind,
                disk.motor_on as u8,
                disk.write_mode as u8,
                disk.load_mode as u8,
                disk.current_qtr_track,
                disk.current_track,
                disk.byte_index,
                trace_start_idx,
                trace_offset,
                trace_bytes_remaining,
                disk.data_latch,
                if latch_high { "ready" } else { "--" },
                disk.read_shift_register,
                if shift_high { "hi" } else { "--" },
                disk.read_bit_phase
            );

            if byte_boundary {
                trace_bytes_remaining -= 1;
                if trace_bytes_remaining == 0 {
                    trace_reason = None;
                }
            }

            event_count += 1;
        }

        if event_count >= max_events {
            println!(
                "stopped after {} events at pc={:04X} total_cycles={}",
                event_count, machine.cpu.pc, machine.total_cycles
            );
            return;
        }

        prev_motor = disk.motor_on;
        prev_write_mode = disk.write_mode;
        prev_load_mode = disk.load_mode;
        prev_track = disk.current_track;
        prev_qtr = disk.current_qtr_track;
        prev_index = disk.byte_index;
        prev_bit_phase = disk.read_bit_phase;
        prev_latch_high = latch_high;
        prev_shift_high = shift_high;
    }

    println!(
        "completed without hitting event limit pc={:04X} total_cycles={}",
        machine.cpu.pc, machine.total_cycles
    );
}
