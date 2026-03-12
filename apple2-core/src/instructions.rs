use crate::memory::Memory;
use crate::cpu::{CPU, AddressingMode};

impl CPU {
    pub(crate) fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.z = result == 0;
        self.status.n = (result & 0b1000_0000) != 0;
    }

    /// Read data using the given addressing mode
    pub(crate) fn read_operand<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) -> u8 {
        if let AddressingMode::Immediate = mode {
            let val = mem.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            return val;
        }
        let (addr, _) = self.get_operand_address(mem, mode);
        mem.read(addr)
    }

    // --- Loads ---
    pub(crate) fn lda<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn lda_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.a = mem.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ldx<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.x = value;
        self.update_zero_and_negative_flags(self.x);
    }

    pub(crate) fn ldx_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.x = mem.read(addr);
        self.update_zero_and_negative_flags(self.x);
    }

    pub(crate) fn ldy<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.y = value;
        self.update_zero_and_negative_flags(self.y);
    }

    pub(crate) fn ldy_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.y = mem.read(addr);
        self.update_zero_and_negative_flags(self.y);
    }

    // --- Stores ---
    pub(crate) fn sta<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        mem.write(addr, self.a);
    }

    pub(crate) fn stx<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        mem.write(addr, self.x);
    }

    pub(crate) fn sty<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        mem.write(addr, self.y);
    }

    // --- Transfers ---
    pub(crate) fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }
    pub(crate) fn tay(&mut self) {
        self.y = self.a;
        self.update_zero_and_negative_flags(self.y);
    }
    pub(crate) fn txa(&mut self) {
        self.a = self.x;
        self.update_zero_and_negative_flags(self.a);
    }
    pub(crate) fn tya(&mut self) {
        self.a = self.y;
        self.update_zero_and_negative_flags(self.a);
    }
    pub(crate) fn tsx(&mut self) {
        self.x = self.sp;
        self.update_zero_and_negative_flags(self.x);
    }
    pub(crate) fn txs(&mut self) {
        self.sp = self.x; // does not affect flags
    }

    // --- Jumps / Branches ---
    pub(crate) fn jmp<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        self.pc = addr;
    }

    pub(crate) fn branch(&mut self, condition: bool, offset: i8) -> u32 {
        if condition {
            let old_pc = self.pc;
            self.pc = self.pc.wrapping_add(offset as u16);
            if (old_pc & 0xFF00) != (self.pc & 0xFF00) {
                return 2; // +1 for branch taken, +1 for page cross
            }
            return 1; // +1 for branch taken
        }
        0
    }

    // --- Stack Operations ---
    pub(crate) fn stack_push<M: Memory>(&mut self, mem: &mut M, data: u8) {
        mem.write(0x0100 + self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    pub(crate) fn stack_pop<M: Memory>(&mut self, mem: &mut M) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        mem.read(0x0100 + self.sp as u16)
    }

    // --- Math & Logical (ALU) ---
    pub(crate) fn adc<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let data = self.read_operand(mem, mode);
        self.adc_internal(data);
    }

    pub(crate) fn adc_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        let data = mem.read(addr);
        self.adc_internal(data);
    }

    fn adc_internal(&mut self, data: u8) {
        if self.status.d {
            // Decimal Mode ADC
            let mut lower = (self.a & 0x0F) + (data & 0x0F) + (if self.status.c { 1 } else { 0 });
            let mut upper = (self.a >> 4) + (data >> 4) + (if lower > 0x09 { 1 } else { 0 });
            
            // Set Zero flag before decimal adjustment
            self.status.z = self.a.wrapping_add(data).wrapping_add(if self.status.c { 1 } else { 0 }) == 0;
            self.status.n = (upper & 0x08) != 0;
            
            self.status.v = (((self.a ^ data) & 0x80) == 0) && (((self.a ^ (upper << 4)) & 0x80) != 0);

            if lower > 0x09 { lower += 0x06; }
            if upper > 0x09 { upper += 0x06; }
            
            self.status.c = upper > 0x0F;
            self.a = ((upper << 4) | (lower & 0x0F)) as u8;
        } else {
            // Binary Mode ADC
            let sum = self.a as u16 + data as u16 + (if self.status.c { 1 } else { 0 }) as u16;
            self.status.c = sum > 0xFF;
            let result = sum as u8;
            self.status.v = (self.a ^ result) & (data ^ result) & 0x80 != 0;
            self.a = result;
            self.update_zero_and_negative_flags(self.a);
        }
    }

    pub(crate) fn sbc<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let data = self.read_operand(mem, mode);
        self.sbc_internal(data);
    }

    pub(crate) fn sbc_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        let data = mem.read(addr);
        self.sbc_internal(data);
    }

    fn sbc_internal(&mut self, data: u8) {
        if self.status.d {
            // Decimal Mode SBC
            let diff = self.a as i32 - data as i32 - (if self.status.c { 0 } else { 1 }) as i32;
            let mut lower = (self.a & 0x0F) as i32 - (data & 0x0F) as i32 - (if self.status.c { 0 } else { 1 }) as i32;
            let mut upper = (self.a >> 4) as i32 - (data >> 4) as i32;
            if lower < 0 {
                lower -= 0x06;
                upper -= 1;
            }
            if upper < 0 {
                upper -= 0x06;
            }
            
            let sum = self.a as u16 + (!data) as u16 + (if self.status.c { 1 } else { 0 }) as u16;
            self.status.c = diff >= 0;
            self.status.v = (self.a ^ sum as u8) & ((!data) ^ sum as u8) & 0x80 != 0;
            self.status.z = (diff & 0xFF) == 0;
            self.status.n = (diff & 0x80) != 0;

            self.a = ((upper << 4) | (lower & 0x0F)) as u8;
        } else {
            // Binary Mode SBC
            let sum = self.a as u16 + (!data) as u16 + (if self.status.c { 1 } else { 0 }) as u16;
            self.status.c = sum > 0xFF;
            let result = sum as u8;
            self.status.v = (self.a ^ result) & ((!data) ^ result) & 0x80 != 0;
            self.a = result;
            self.update_zero_and_negative_flags(self.a);
        }
    }

    pub(crate) fn and<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let data = self.read_operand(mem, mode);
        self.a &= data;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn and_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.a &= mem.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn eor<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let data = self.read_operand(mem, mode);
        self.a ^= data;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn eor_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.a ^= mem.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ora<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let data = self.read_operand(mem, mode);
        self.a |= data;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ora_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        self.a |= mem.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    // --- Comparisons ---
    fn compare(&mut self, a: u8, b: u8) {
        let diff = a as i16 - b as i16;
        self.status.c = diff >= 0;
        self.update_zero_and_negative_flags(diff as u8);
    }

    pub(crate) fn cmp<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.compare(self.a, value);
    }

    pub(crate) fn cmp_with_addr<M: Memory>(&mut self, mem: &mut M, addr: u16) {
        let value = mem.read(addr);
        self.compare(self.a, value);
    }

    pub(crate) fn cpx<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.compare(self.x, value);
    }

    pub(crate) fn cpy<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.compare(self.y, value);
    }

    // --- Increments & Decrements ---
    pub(crate) fn inc<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut value = mem.read(addr);
        value = value.wrapping_add(1);
        mem.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    pub(crate) fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    }

    pub(crate) fn dec<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut value = mem.read(addr);
        value = value.wrapping_sub(1);
        mem.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.x);
    }

    pub(crate) fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.y);
    }

    // --- Shifts & Rotates ---
    pub(crate) fn asl_acc(&mut self) {
        self.status.c = (self.a >> 7) == 1;
        self.a <<= 1;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn asl<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut data = mem.read(addr);
        self.status.c = (data >> 7) == 1;
        data <<= 1;
        mem.write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    pub(crate) fn lsr_acc(&mut self) {
        self.status.c = (self.a & 1) == 1;
        self.a >>= 1;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn lsr<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut data = mem.read(addr);
        self.status.c = (data & 1) == 1;
        data >>= 1;
        mem.write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    // --- ROL ---
    pub(crate) fn rol_acc(&mut self) {
        let carry_in = if self.status.c { 1 } else { 0 };
        self.status.c = (self.a >> 7) == 1;
        self.a = (self.a << 1) | carry_in;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn rol<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut data = mem.read(addr);
        let carry_in = if self.status.c { 1 } else { 0 };
        self.status.c = (data >> 7) == 1;
        data = (data << 1) | carry_in;
        mem.write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    // --- ROR ---
    pub(crate) fn ror_acc(&mut self) {
        let carry_in = if self.status.c { 0x80 } else { 0 };
        self.status.c = (self.a & 1) == 1;
        self.a = (self.a >> 1) | carry_in;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ror<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let (addr, _) = self.get_operand_address(mem, mode);
        let mut data = mem.read(addr);
        let carry_in = if self.status.c { 0x80 } else { 0 };
        self.status.c = (data & 1) == 1;
        data = (data >> 1) | carry_in;
        mem.write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    pub(crate) fn bit<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        let result = self.a & value;
        self.status.z = result == 0;
        self.status.n = (value & 0b1000_0000) != 0;
        self.status.v = (value & 0b0100_0000) != 0;
    }

    // --- Subroutines & Interrupts ---
    pub(crate) fn jsr<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        // JSR fetches the absolute address, but pushes PC + 2 to stack
        let (addr, _) = self.get_operand_address(mem, mode);
        // At this point, PC points to the next instruction because get_operand_address(Absolute) increments PC by 2
        // JSR pushes PC - 1
        let ret_addr = self.pc.wrapping_sub(1);
        self.stack_push(mem, (ret_addr >> 8) as u8);
        self.stack_push(mem, (ret_addr & 0xFF) as u8);
        self.pc = addr;
    }

    pub(crate) fn rts<M: Memory>(&mut self, mem: &mut M) {
        let lo = self.stack_pop(mem) as u16;
        let hi = self.stack_pop(mem) as u16;
        self.pc = ((hi << 8) | lo).wrapping_add(1);
    }

    pub(crate) fn rti<M: Memory>(&mut self, mem: &mut M) {
        let flags = self.stack_pop(mem);
        self.status.from_byte(flags);
        let lo = self.stack_pop(mem) as u16;
        let hi = self.stack_pop(mem) as u16;
        self.pc = (hi << 8) | lo;
    }

    pub(crate) fn php<M: Memory>(&mut self, mem: &mut M) {
        // PHP always pushes the status byte with both Bit 4 (B) and Bit 5 (Unused) set to 1.
        self.stack_push(mem, self.status.to_byte() | 0x10 | 0x20);
    }

    pub(crate) fn plp<M: Memory>(&mut self, mem: &mut M) {
        let flags = self.stack_pop(mem);
        // PLP pulls the status but the B flag (Bit 4) and Unused (Bit 5) are NOT affected
        // by the value on the stack in real 6502 hardware.
        let b_before = self.status.b;
        self.status.from_byte(flags);
        self.status.b = b_before;
        self.status.u = true; // Always 1
    }

    pub(crate) fn pha<M: Memory>(&mut self, mem: &mut M) {
        self.stack_push(mem, self.a);
    }

    pub(crate) fn pla<M: Memory>(&mut self, mem: &mut M) {
        self.a = self.stack_pop(mem);
        self.update_zero_and_negative_flags(self.a);
    }

    // --- Flag Manipulations ---
    pub(crate) fn clc(&mut self) { self.status.c = false; }
    pub(crate) fn cld(&mut self) { self.status.d = false; }
    pub(crate) fn cli(&mut self) { self.status.i = false; }
    pub(crate) fn clv(&mut self) { self.status.v = false; }
    pub(crate) fn sec(&mut self) { self.status.c = true; }
    pub(crate) fn sed(&mut self) { self.status.d = true; }
    pub(crate) fn sei(&mut self) { self.status.i = true; }

    pub(crate) fn nop(&mut self) {}
}
