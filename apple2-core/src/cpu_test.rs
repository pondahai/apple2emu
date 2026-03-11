#[cfg(test)]
mod tests {
    use crate::cpu::CPU;
    use crate::memory::Memory;

    struct TestMemory {
        ram: [u8; 65536],
    }

    impl TestMemory {
        fn new() -> Self {
            Self { ram: [0; 65536] }
        }
    }

    impl Memory for TestMemory {
        fn read(&mut self, addr: u16) -> u8 { self.ram[addr as usize] }
        fn write(&mut self, addr: u16, data: u8) { self.ram[addr as usize] = data; }
    }

    #[test]
    fn test_adc_binary() {
        let mut cpu = CPU::new();
        let mut mem = TestMemory::new();
        
        // ADC #$10 (0xA9 0x10)
        cpu.a = 0x20;
        cpu.status.c = false;
        mem.ram[0] = 0x69;
        mem.ram[1] = 0x10;
        cpu.pc = 0;
        
        cpu.step(&mut mem);
        assert_eq!(cpu.a, 0x30);
        assert_eq!(cpu.status.c, false);
        assert_eq!(cpu.status.z, false);
    }

    #[test]
    fn test_adc_carry() {
        let mut cpu = CPU::new();
        let mut mem = TestMemory::new();
        
        cpu.a = 0xFF;
        cpu.status.c = false;
        mem.ram[0] = 0x69;
        mem.ram[1] = 0x01;
        cpu.pc = 0;
        
        cpu.step(&mut mem);
        assert_eq!(cpu.a, 0x00);
        assert_eq!(cpu.status.c, true);
        assert_eq!(cpu.status.z, true);
    }

    #[test]
    fn test_sbc_binary() {
        let mut cpu = CPU::new();
        let mut mem = TestMemory::new();
        
        // SBC #$01 (0xE9 0x01)
        cpu.a = 0x10;
        cpu.status.c = true; // No borrow
        mem.ram[0] = 0xE9;
        mem.ram[1] = 0x01;
        cpu.pc = 0;
        
        cpu.step(&mut mem);
        assert_eq!(cpu.a, 0x0F);
        assert_eq!(cpu.status.c, true); // No borrow occurred
    }

    #[test]
    fn test_brk_pc_offset() {
        let mut cpu = CPU::new();
        let mut mem = TestMemory::new();
        
        // BRK at $1000, signature byte $AA at $1001
        cpu.pc = 0x1000;
        mem.ram[0x1000] = 0x00;
        mem.ram[0x1001] = 0xAA;
        
        // Interrupt vector at $FFFE
        mem.ram[0xFFFE] = 0xAD;
        mem.ram[0xFFFF] = 0xDE;
        
        cpu.sp = 0xFF;
        cpu.step(&mut mem);
        
        // PC should be at the vector
        assert_eq!(cpu.pc, 0xDEAD);
        
        // Stack should have pushed PC+2 ($1002)
        // SP starts at 0xFF. Pushes hi ($10) to 0x01FF, then lo ($02) to 0x01FE.
        let hi = mem.ram[0x01FF];
        let lo = mem.ram[0x01FE];
        let pushed_pc = ((hi as u16) << 8) | lo as u16;
        assert_eq!(pushed_pc, 0x1002);
        
        // Status should have B flag set on stack
        let pushed_status = mem.ram[0x01FD];
        assert_eq!(pushed_status & 0x10, 0x10);
        assert_eq!(pushed_status & 0x20, 0x20); // Unused flag
    }
}
