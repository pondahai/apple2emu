#[cfg(test)]
mod tests {
    use crate::disk2::Disk2;

    #[test]
    fn write_path_commits_latch_while_q7_is_on() {
        let mut disk = Disk2::new();
        disk.is_disk_loaded = true;
        disk.current_track = 0;
        disk.tracks[0].length = 1;
        disk.tracks[0].raw_bytes[0] = 0x11;

        disk.write_io(0xC0E9, 0); // Motor on
        disk.write_io(0xC0EF, 0); // Q7 = 1
        disk.write_io(0xC0ED, 0xAA); // Q6 = 1 (write-load), latch = 0xAA
        disk.write_io(0xC0EC, 0); // Q6 = 0 (write-shift)
        disk.tick(32);
        assert_eq!(disk.tracks[0].raw_bytes[0], 0xAA);
    }

    #[test]
    fn write_protect_probe_reads_are_compatible_on_q6_on_and_q7_off() {
        let mut disk = Disk2::new();
        disk.write_io(0xC0E9, 0); // Motor on

        assert_eq!(disk.read_io(0xC0ED), 0x00); // Q6_ON probe path
        assert_eq!(disk.read_io(0xC0EE), 0x00); // Default non-sensed read value
    }

    #[test]
    fn only_q6_on_write_loads_latch() {
        let mut disk = Disk2::new();
        disk.write_io(0xC0EF, 0); // Q7 = 1
        disk.write_io(0xC0ED, 0xAA); // Q6 = 1 and load
        assert_eq!(disk.data_latch, 0xAA);

        disk.write_io(0xC0EF, 0x55); // Q7 write should not replace latch payload
        assert_eq!(disk.data_latch, 0xAA);
    }

    #[test]
    fn adjacent_dual_phase_state_lands_on_half_step() {
        let mut disk = Disk2::new();
        disk.current_qtr_track = 92;
        disk.current_track = 23;

        disk.write_io(0xC0E3, 0); // phase 1 on
        disk.write_io(0xC0E5, 0); // phase 2 on

        assert_eq!(disk.current_qtr_track, 91);
        assert_eq!(disk.current_track, 22);
    }

    #[test]
    fn dropping_to_single_phase_returns_to_even_quarter_track() {
        let mut disk = Disk2::new();
        disk.current_qtr_track = 91;
        disk.current_track = 22;
        disk.phases[1] = true;
        disk.phases[2] = true;

        disk.write_io(0xC0E2, 0); // phase 1 off, leave phase 2 on

        assert_eq!(disk.current_qtr_track, 92);
        assert_eq!(disk.current_track, 23);
    }

    #[test]
    fn read_sequencer_advances_one_bit_every_four_cycles() {
        let mut disk = Disk2::new();
        disk.is_disk_loaded = true;
        disk.current_track = 0;
        disk.tracks[0].length = 1;
        disk.tracks[0].raw_bytes[0] = 0xD5;

        disk.write_io(0xC0E9, 0); // Motor on
        disk.write_io(0xC0EC, 0); // Q6 = 0
        disk.write_io(0xC0EE, 0); // Q7 = 0 (read mode)

        disk.tick(4);
        assert_eq!(disk.read_bit_phase, 1);
        assert_eq!(disk.read_shift_register, 0x01);

        disk.tick(28);
        assert_eq!(disk.read_bit_phase, 0);
        assert_eq!(disk.byte_index, 0);
        assert_eq!(disk.data_latch, 0xD5);
    }

    #[test]
    fn read_sequencer_keeps_bit_level_state_but_publishes_on_byte_boundary() {
        let mut disk = Disk2::new();
        disk.is_disk_loaded = true;
        disk.current_track = 0;
        disk.tracks[0].length = 1;
        disk.tracks[0].raw_bytes[0] = 0xFF;

        disk.write_io(0xC0E9, 0); // Motor on
        disk.write_io(0xC0EC, 0); // Q6 = 0
        disk.write_io(0xC0EE, 0); // Q7 = 0 (read mode)

        disk.tick(32);
        assert_eq!(disk.read_io(0xC0EC), 0xFF);
        assert_eq!(disk.data_latch, 0x7F);

        disk.tick(4);
        assert_eq!(disk.read_bit_phase, 1);
        assert_eq!(disk.data_latch, 0x7F);

        disk.tick(28);
        assert_eq!(disk.data_latch, 0xFF);
    }
}
