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
    fn speaker_toggle_handles_mirrored_addresses() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(100);
        let _ = mem.read(0xC031); // Toggle via $C031
        let _ = mem.write(0xC03F, 0x00); // Toggle via $C03F
        mem.end_cpu_step();

        assert!(!mem.speaker); // Two toggles result in original state (false)
        assert_eq!(mem.take_speaker_toggle_cycles(), vec![100, 101]);
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
    fn undriven_io_reads_track_the_floating_bus() {
        // Mark each RAM cell with its own low address byte so a floating-bus
        // read returns the low byte of whatever address the video scanner is
        // fetching this cycle.
        let mut mem = Apple2Memory::new();
        for (i, cell) in mem.ram.iter_mut().enumerate() {
            *cell = i as u8;
        }

        // Read an undriven soft switch ($C055 = Page 2 select) once per cycle as
        // the bus cycle advances. A constant-0 stub would return the same value
        // every time; the floating bus must vary as the scanner moves.
        mem.begin_cpu_step(0);
        let mut values = vec![];
        for _ in 0..200 {
            values.push(mem.read(0xC055));
        }
        mem.end_cpu_step();

        let first = values[0];
        assert!(
            values.iter().any(|&v| v != first),
            "floating bus returned a constant ({:#04x}); randomness source is dead",
            first
        );
        // And it must be reading real RAM, never a hardcoded sentinel.
        assert!(values.iter().any(|&v| v != 0));
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

        // Full deflection now runs into the saturation region (~3191 cycles from
        // the strobe), so the pulse has to be sampled past that to read low.
        mem.begin_cpu_step(4_000);
        let late = mem.read(0xC064);
        mem.end_cpu_step();

        assert_eq!(early, 0x80);
        assert_eq!(late, 0x00);
    }

    #[test]
    fn language_card_requires_double_read_on_same_canonical_switch_to_enable_writes() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(0);
        let _ = mem.read(0xC081);
        assert!(!mem.lc_write_enable);
        let _ = mem.read(0xC085);
        mem.end_cpu_step();

        assert!(mem.lc_write_enable);
    }

    #[test]
    fn language_card_bank1_and_bank2_are_distinct_in_d000_window() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(0);
        let _ = mem.read(0xC081);
        let _ = mem.read(0xC081);
        mem.write(0xD000, 0x11);

        let _ = mem.read(0xC089);
        let _ = mem.read(0xC089);
        mem.write(0xD000, 0x22);

        let _ = mem.read(0xC083);
        let bank1_val = mem.read(0xD000);
        let _ = mem.read(0xC08B);
        let bank2_val = mem.read(0xD000);
        mem.end_cpu_step();

        assert_eq!(bank1_val, 0x11);
        assert_eq!(bank2_val, 0x22);
    }

    #[test]
    fn language_card_e000_window_is_shared_between_banks() {
        let mut mem = Apple2Memory::new();

        mem.begin_cpu_step(0);
        let _ = mem.read(0xC081);
        let _ = mem.read(0xC081);
        mem.write(0xE000, 0x33);

        let _ = mem.read(0xC08B);
        let shared_val = mem.read(0xE000);
        mem.end_cpu_step();

        assert_eq!(shared_val, 0x33);
    }

    #[test]
    fn language_card_can_read_rom_while_writing_ram() {
        let mut mem = Apple2Memory::new();
        mem.rom[0] = 0xAA;

        mem.begin_cpu_step(0);
        let _ = mem.read(0xC081);
        let _ = mem.read(0xC081);
        mem.write(0xD000, 0x44);

        let _ = mem.read(0xC082);
        let rom_val = mem.read(0xD000);
        mem.end_cpu_step();

        assert_eq!(rom_val, 0xAA);
        assert_eq!(mem.lc_ram[0x1000], 0x44);
    }

    #[test]
    fn kbdstrobe_read_clears_strobe_and_returns_floating_bus() {
        let mut mem = Apple2Memory::new();
        // Give the scanner something to pick up: a per-address pattern so the
        // floating bus value actually changes as the beam moves.
        for (i, b) in mem.ram.iter_mut().enumerate() {
            *b = (i & 0xFF) as u8;
        }
        mem.keyboard_latch = 0xC1; // 'A' with strobe set

        // $C000 still reports the latch verbatim.
        mem.begin_cpu_step(0);
        let data = mem.read(0xC000);
        mem.end_cpu_step();
        assert_eq!(data, 0xC1);

        // $C010 clears the strobe...
        mem.begin_cpu_step(0);
        let first = mem.read(0xC010);
        mem.end_cpu_step();
        assert_eq!(mem.keyboard_latch, 0x41, "strobe bit must be cleared");

        // ...and what it returns tracks the video scanner, not the latch.
        mem.keyboard_latch = 0xC1;
        mem.begin_cpu_step(12_345);
        let second = mem.read(0xC010);
        mem.end_cpu_step();

        assert_ne!(
            first, second,
            "$C010 must float with the scanner, not return a constant"
        );
    }

    #[test]
    fn full_deflection_paddle_pulse_reaches_saturation_region() {
        let mut mem = Apple2Memory::new();
        mem.paddles[0] = 255;

        // Latch the paddles at cycle 0.
        mem.begin_cpu_step(0);
        let _ = mem.read(0xC070);
        mem.end_cpu_step();

        // A coarse 54-cycle read loop needs 55 iterations (~2970 cycles) to call
        // it full deflection. The old linear curve expired at 2813 and the pulse
        // read low here, so right/down never registered.
        mem.begin_cpu_step(2_970);
        let still_high = mem.read(0xC064);
        mem.end_cpu_step();
        assert_eq!(still_high, 0x80, "full deflection must survive 55 iterations");

        // It must still end, well before the ~3300-cycle real-hardware ceiling
        // turns into a hang.
        mem.begin_cpu_step(4_000);
        let expired = mem.read(0xC064);
        mem.end_cpu_step();
        assert_eq!(expired, 0x00);
    }

    #[test]
    fn centered_paddle_pulse_is_unchanged() {
        let mut mem = Apple2Memory::new();
        mem.paddles[0] = 128; // centered -- must stay byte-for-byte as before

        mem.begin_cpu_step(0);
        let _ = mem.read(0xC070);
        mem.end_cpu_step();

        // 8 + 128*11 = 1416
        mem.begin_cpu_step(1_415);
        let high = mem.read(0xC064);
        mem.end_cpu_step();
        assert_eq!(high, 0x80);

        mem.begin_cpu_step(1_416);
        let low = mem.read(0xC064);
        mem.end_cpu_step();
        assert_eq!(low, 0x00);
    }
}
