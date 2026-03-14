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
}
