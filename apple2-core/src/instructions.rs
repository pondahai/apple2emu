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
        let addr = self.get_operand_address(mem, mode);
        mem.read(addr)
    }

    // --- Loads ---
    pub(crate) fn lda<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.a = value;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ldx<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.x = value;
        self.update_zero_and_negative_flags(self.x);
    }

    pub(crate) fn ldy<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.y = value;
        self.update_zero_and_negative_flags(self.y);
    }

    // --- Stores ---
    pub(crate) fn sta<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let addr = self.get_operand_address(mem, mode);
        mem.write(addr, self.a);
    }

    pub(crate) fn stx<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let addr = self.get_operand_address(mem, mode);
        mem.write(addr, self.x);
    }

    pub(crate) fn sty<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let addr = self.get_operand_address(mem, mode);
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
        self.pc = self.get_operand_address(mem, mode);
    }

    pub(crate) fn branch(&mut self, condition: bool, offset: i8) {
        if condition {
            self.pc = self.pc.wrapping_add(offset as u16);
            // In a cycle-accurate emulator we would add cycles here
        }
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
        let value = self.read_operand(mem, mode);
        self.add_to_accumulator(value);
    }

    pub(crate) fn sbc<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        let value = self.read_operand(mem, mode);
        self.add_to_accumulator(!value); // SBC is ADC with inverted operand
    }

    fn add_to_accumulator(&mut self, data: u8) {
        let sum = self.a as u16 
            + data as u16 
            + (if self.status.c { 1 } else { 0 }) as u16;
        
        self.status.c = sum > 0xFF;
        let result = sum as u8;
        
        // Overflow occurs if signs of inputs are same, but sign of result is different
        // (A ^ result) & (data ^ result) & 0x80 != 0
        self.status.v = (self.a ^ result) & (data ^ result) & 0x80 != 0;
        
        self.a = result;
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn and<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        self.a &= self.read_operand(mem, mode);
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn eor<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        self.a ^= self.read_operand(mem, mode);
        self.update_zero_and_negative_flags(self.a);
    }

    pub(crate) fn ora<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) {
        self.a |= self.read_operand(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let addr = self.get_operand_address(mem, mode);
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
        let mut flags = self.status.clone();
        flags.b = true;
        self.stack_push(mem, flags.to_byte());
    }

    pub(crate) fn plp<M: Memory>(&mut self, mem: &mut M) {
        let flags = self.stack_pop(mem);
        self.status.from_byte(flags);
        self.status.u = true; // u is always 1
        self.status.b = false; // plp ignores b flag
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
