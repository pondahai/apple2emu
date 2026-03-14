#[cfg(test)]
mod tests {
    use crate::memory::{Apple2Memory, Memory};

    #[test]
    fn speaker_toggle_records_bus_cycle() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(100);
        let _ = mem.read(0xC030);
        mem.end_cpu_step();

        assert!(mem.speaker);
        assert_eq!(mem.take_speaker_toggle_cycles(), vec![100]);
    }

    #[test]
    fn speaker_toggle_cycle_advances_with_each_bus_access() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(42);
        let _ = mem.read(0x0000);
        let _ = mem.read(0xC030);
        mem.write(0x0001, 0xAA);
        mem.write(0xC030, 0x00);
        mem.end_cpu_step();

        assert!(!mem.speaker);
        assert_eq!(mem.take_speaker_toggle_cycles(), vec![43, 45]);
    }

    #[test]
    fn pushbutton_reads_set_bit7_when_pressed() {
        let mut mem = Apple2Memory::new();
        mem.set_joystick_state(127, 127, true, false);

        mem.begin_cpu_step(0);
        let pb0 = mem.read(0xC061);
        let pb1 = mem.read(0xC062);
        mem.end_cpu_step();

        assert_eq!(pb0, 0x80);
        assert_eq!(pb1, 0x00);
    }

    #[test]
    fn paddle_reads_stay_high_until_timeout_after_strobe() {
        let mut mem = Apple2Memory::new();
        mem.set_joystick_state(255, 0, false, false);

        mem.begin_cpu_step(100);
        let _ = mem.read(0xC070);
        let early = mem.read(0xC064);
        mem.end_cpu_step();

        mem.begin_cpu_step(3_000);
        let late = mem.read(0xC064);
        mem.end_cpu_step();

        assert_eq!(early, 0x80);
        assert_eq!(late, 0x00);
    }
}
