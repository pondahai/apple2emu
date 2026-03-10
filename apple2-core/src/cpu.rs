use crate::memory::Memory;

/// Status Flags for the 6502 CPU (P Register)
#[derive(Clone, Copy, Debug)]
pub struct StatusFlags {
    pub c: bool, // Carry
    pub z: bool, // Zero
    pub i: bool, // Interrupt Disable
    pub d: bool, // Decimal Mode
    pub b: bool, // Break Command
    pub u: bool, // Unused (always 1)
    pub v: bool, // Overflow
    pub n: bool, // Negative
}

impl Default for StatusFlags {
    fn default() -> Self {
        Self {
            c: false,
            z: false,
            i: true, // Interrupt disabled by default on reset
            d: false,
            b: false,
            u: true, // Always true
            v: false,
            n: false,
        }
    }
}

impl StatusFlags {
    pub fn to_byte(&self) -> u8 {
        (if self.c { 1 } else { 0 })
            | (if self.z { 1 << 1 } else { 0 })
            | (if self.i { 1 << 2 } else { 0 })
            | (if self.d { 1 << 3 } else { 0 })
            | (if self.b { 1 << 4 } else { 0 })
            | (if self.u { 1 << 5 } else { 0 })
            | (if self.v { 1 << 6 } else { 0 })
            | (if self.n { 1 << 7 } else { 0 })
    }

    pub fn from_byte(&mut self, b: u8) {
        self.c = (b & 1) != 0;
        self.z = (b & (1 << 1)) != 0;
        self.i = (b & (1 << 2)) != 0;
        self.d = (b & (1 << 3)) != 0;
        self.b = (b & (1 << 4)) != 0;
        self.u = true; // Always 1
        self.v = (b & (1 << 6)) != 0;
        self.n = (b & (1 << 7)) != 0;
    }
}

pub struct CPU {
    pub a: u8,      // Accumulator
    pub x: u8,      // X Register
    pub y: u8,      // Y Register
    pub pc: u16,    // Program Counter
    pub sp: u8,     // Stack Pointer
    pub status: StatusFlags, // P Register (Processor Status)
}

impl CPU {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            status: StatusFlags::default(),
        }
    }

    pub fn reset<M: Memory>(&mut self, mem: &mut M) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = StatusFlags::default();
        // Load Reset Vector from $FFFC / $FFFD
        self.pc = mem.read_word(0xFFFC);
    }

    /// Read a byte from memory and increment PC
    fn fetch_byte<M: Memory>(&mut self, mem: &mut M) -> u8 {
        let result = mem.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        result
    }

    /// Read a word from memory and increment PC by 2
    fn fetch_word<M: Memory>(&mut self, mem: &mut M) -> u16 {
        let result = mem.read_word(self.pc);
        self.pc = self.pc.wrapping_add(2);
        result
    }

    /// Calculate the target address based on addressing mode
    pub(crate) fn get_operand_address<M: Memory>(&mut self, mem: &mut M, mode: AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => {
                let addr = self.pc;
                self.pc = self.pc.wrapping_add(1);
                addr
            }
            AddressingMode::ZeroPage => self.fetch_byte(mem) as u16,
            AddressingMode::ZeroPageX => {
                let base = self.fetch_byte(mem);
                base.wrapping_add(self.x) as u16
            }
            AddressingMode::ZeroPageY => {
                let base = self.fetch_byte(mem);
                base.wrapping_add(self.y) as u16
            }
            AddressingMode::Absolute => self.fetch_word(mem),
            AddressingMode::AbsoluteX => {
                let base = self.fetch_word(mem);
                base.wrapping_add(self.x as u16)
            }
            AddressingMode::AbsoluteY => {
                let base = self.fetch_word(mem);
                base.wrapping_add(self.y as u16)
            }
            AddressingMode::Indirect => {
                let ptr = self.fetch_word(mem);
                // 6502 bug: if ptr ends in 0xFF, it wraps around the page
                let lo = mem.read(ptr) as u16;
                let hi_ptr = if ptr & 0x00FF == 0x00FF { ptr & 0xFF00 } else { ptr + 1 };
                let hi = mem.read(hi_ptr) as u16;
                (hi << 8) | lo
            }
            AddressingMode::IndirectX => {
                let base = self.fetch_byte(mem);
                let ptr = base.wrapping_add(self.x);
                let lo = mem.read(ptr as u16) as u16;
                let hi = mem.read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo
            }
            AddressingMode::IndirectY => {
                let base = self.fetch_byte(mem);
                let lo = mem.read(base as u16) as u16;
                let hi = mem.read(base.wrapping_add(1) as u16) as u16;
                let deref_base = (hi << 8) | lo;
                deref_base.wrapping_add(self.y as u16)
            }
            AddressingMode::NoneAddressing => {
                0
            }
            AddressingMode::Relative => {
                // Relative fetch handled differently by branch instructions directly for speed
                0
            }
        }
    }

    /// Step one instruction
    /// Returns number of cycles consumed (dummy value for now)
    pub fn step<M: Memory>(&mut self, mem: &mut M) -> u32 {
        let opcode = self.fetch_byte(mem);
        match opcode {
            // BRK
            0x00 => {
                let pc = self.pc.wrapping_add(1);
                self.stack_push(mem, (pc >> 8) as u8);
                self.stack_push(mem, (pc & 0xFF) as u8);
                self.stack_push(mem, self.status.to_byte() | 0x10); // B flag set
                self.status.i = true;
                self.pc = mem.read_word(0xFFFE);
                return 7;
            }

            // LDA
            0xA9 => self.lda(mem, AddressingMode::Immediate),
            0xA5 => self.lda(mem, AddressingMode::ZeroPage),
            0xB5 => self.lda(mem, AddressingMode::ZeroPageX),
            0xAD => self.lda(mem, AddressingMode::Absolute),
            0xBD => self.lda(mem, AddressingMode::AbsoluteX),
            0xB9 => self.lda(mem, AddressingMode::AbsoluteY),
            0xA1 => self.lda(mem, AddressingMode::IndirectX),
            0xB1 => self.lda(mem, AddressingMode::IndirectY),

            // LDX
            0xA2 => self.ldx(mem, AddressingMode::Immediate),
            0xA6 => self.ldx(mem, AddressingMode::ZeroPage),
            0xB6 => self.ldx(mem, AddressingMode::ZeroPageY),
            0xAE => self.ldx(mem, AddressingMode::Absolute),
            0xBE => self.ldx(mem, AddressingMode::AbsoluteY),

            // LDY
            0xA0 => self.ldy(mem, AddressingMode::Immediate),
            0xA4 => self.ldy(mem, AddressingMode::ZeroPage),
            0xB4 => self.ldy(mem, AddressingMode::ZeroPageX),
            0xAC => self.ldy(mem, AddressingMode::Absolute),
            0xBC => self.ldy(mem, AddressingMode::AbsoluteX),

            // STA
            0x85 => self.sta(mem, AddressingMode::ZeroPage),
            0x95 => self.sta(mem, AddressingMode::ZeroPageX),
            0x8D => self.sta(mem, AddressingMode::Absolute),
            0x9D => self.sta(mem, AddressingMode::AbsoluteX),
            0x99 => self.sta(mem, AddressingMode::AbsoluteY),
            0x81 => self.sta(mem, AddressingMode::IndirectX),
            0x91 => self.sta(mem, AddressingMode::IndirectY),

            // STX
            0x86 => self.stx(mem, AddressingMode::ZeroPage),
            0x96 => self.stx(mem, AddressingMode::ZeroPageY),
            0x8E => self.stx(mem, AddressingMode::Absolute),

            // STY
            0x84 => self.sty(mem, AddressingMode::ZeroPage),
            0x94 => self.sty(mem, AddressingMode::ZeroPageX),
            0x8C => self.sty(mem, AddressingMode::Absolute),

            // JMP
            0x4C => self.jmp(mem, AddressingMode::Absolute),
            0x6C => self.jmp(mem, AddressingMode::Indirect),

            // JSR, RTS, RTI
            0x20 => self.jsr(mem, AddressingMode::Absolute),
            0x60 => self.rts(mem),
            0x40 => self.rti(mem),

            // Branches
            0x90 => { let off = self.fetch_byte(mem) as i8; self.branch(!self.status.c, off); }, // BCC
            0xB0 => { let off = self.fetch_byte(mem) as i8; self.branch(self.status.c, off); },  // BCS
            0xF0 => { let off = self.fetch_byte(mem) as i8; self.branch(self.status.z, off); },  // BEQ
            0xD0 => { let off = self.fetch_byte(mem) as i8; self.branch(!self.status.z, off); }, // BNE
            0x10 => { let off = self.fetch_byte(mem) as i8; self.branch(!self.status.n, off); }, // BPL
            0x30 => { let off = self.fetch_byte(mem) as i8; self.branch(self.status.n, off); },  // BMI
            0x50 => { let off = self.fetch_byte(mem) as i8; self.branch(!self.status.v, off); }, // BVC
            0x70 => { let off = self.fetch_byte(mem) as i8; self.branch(self.status.v, off); },  // BVS

            // NOP
            0xEA => self.nop(),

            // Math: ADC
            0x69 => self.adc(mem, AddressingMode::Immediate),
            0x65 => self.adc(mem, AddressingMode::ZeroPage),
            0x75 => self.adc(mem, AddressingMode::ZeroPageX),
            0x6D => self.adc(mem, AddressingMode::Absolute),
            0x7D => self.adc(mem, AddressingMode::AbsoluteX),
            0x79 => self.adc(mem, AddressingMode::AbsoluteY),
            0x61 => self.adc(mem, AddressingMode::IndirectX),
            0x71 => self.adc(mem, AddressingMode::IndirectY),

            // Math: SBC
            0xE9 => self.sbc(mem, AddressingMode::Immediate),
            0xE5 => self.sbc(mem, AddressingMode::ZeroPage),
            0xF5 => self.sbc(mem, AddressingMode::ZeroPageX),
            0xED => self.sbc(mem, AddressingMode::Absolute),
            0xFD => self.sbc(mem, AddressingMode::AbsoluteX),
            0xF9 => self.sbc(mem, AddressingMode::AbsoluteY),
            0xE1 => self.sbc(mem, AddressingMode::IndirectX),
            0xF1 => self.sbc(mem, AddressingMode::IndirectY),

            // Logic: AND
            0x29 => self.and(mem, AddressingMode::Immediate),
            0x25 => self.and(mem, AddressingMode::ZeroPage),
            0x35 => self.and(mem, AddressingMode::ZeroPageX),
            0x2D => self.and(mem, AddressingMode::Absolute),
            0x3D => self.and(mem, AddressingMode::AbsoluteX),
            0x39 => self.and(mem, AddressingMode::AbsoluteY),
            0x21 => self.and(mem, AddressingMode::IndirectX),
            0x31 => self.and(mem, AddressingMode::IndirectY),

            // Logic: ORA
            0x09 => self.ora(mem, AddressingMode::Immediate),
            0x05 => self.ora(mem, AddressingMode::ZeroPage),
            0x15 => self.ora(mem, AddressingMode::ZeroPageX),
            0x0D => self.ora(mem, AddressingMode::Absolute),
            0x1D => self.ora(mem, AddressingMode::AbsoluteX),
            0x19 => self.ora(mem, AddressingMode::AbsoluteY),
            0x01 => self.ora(mem, AddressingMode::IndirectX),
            0x11 => self.ora(mem, AddressingMode::IndirectY),

            // Logic: EOR
            0x49 => self.eor(mem, AddressingMode::Immediate),
            0x45 => self.eor(mem, AddressingMode::ZeroPage),
            0x55 => self.eor(mem, AddressingMode::ZeroPageX),
            0x4D => self.eor(mem, AddressingMode::Absolute),
            0x5D => self.eor(mem, AddressingMode::AbsoluteX),
            0x59 => self.eor(mem, AddressingMode::AbsoluteY),
            0x41 => self.eor(mem, AddressingMode::IndirectX),
            0x51 => self.eor(mem, AddressingMode::IndirectY),

            // BIT
            0x24 => self.bit(mem, AddressingMode::ZeroPage),
            0x2C => self.bit(mem, AddressingMode::Absolute),

            // Compares: CMP
            0xC9 => self.cmp(mem, AddressingMode::Immediate),
            0xC5 => self.cmp(mem, AddressingMode::ZeroPage),
            0xD5 => self.cmp(mem, AddressingMode::ZeroPageX),
            0xCD => self.cmp(mem, AddressingMode::Absolute),
            0xDD => self.cmp(mem, AddressingMode::AbsoluteX),
            0xD9 => self.cmp(mem, AddressingMode::AbsoluteY),
            0xC1 => self.cmp(mem, AddressingMode::IndirectX),
            0xD1 => self.cmp(mem, AddressingMode::IndirectY),

            // Compares: CPX
            0xE0 => self.cpx(mem, AddressingMode::Immediate),
            0xE4 => self.cpx(mem, AddressingMode::ZeroPage),
            0xEC => self.cpx(mem, AddressingMode::Absolute),

            // Compares: CPY
            0xC0 => self.cpy(mem, AddressingMode::Immediate),
            0xC4 => self.cpy(mem, AddressingMode::ZeroPage),
            0xCC => self.cpy(mem, AddressingMode::Absolute),

            // Increments/Decrements
            0xE6 => self.inc(mem, AddressingMode::ZeroPage),
            0xF6 => self.inc(mem, AddressingMode::ZeroPageX),
            0xEE => self.inc(mem, AddressingMode::Absolute),
            0xFE => self.inc(mem, AddressingMode::AbsoluteX),

            0xC6 => self.dec(mem, AddressingMode::ZeroPage),
            0xD6 => self.dec(mem, AddressingMode::ZeroPageX),
            0xCE => self.dec(mem, AddressingMode::Absolute),
            0xDE => self.dec(mem, AddressingMode::AbsoluteX),

            0xE8 => self.inx(),
            0xC8 => self.iny(),
            0xCA => self.dex(),
            0x88 => self.dey(),

            // Shifts/Rotates: ASL
            0x0A => self.asl_acc(),
            0x06 => self.asl(mem, AddressingMode::ZeroPage),
            0x16 => self.asl(mem, AddressingMode::ZeroPageX),
            0x0E => self.asl(mem, AddressingMode::Absolute),
            0x1E => self.asl(mem, AddressingMode::AbsoluteX),

            // Shifts/Rotates: LSR
            0x4A => self.lsr_acc(),
            0x46 => self.lsr(mem, AddressingMode::ZeroPage),
            0x56 => self.lsr(mem, AddressingMode::ZeroPageX),
            0x4E => self.lsr(mem, AddressingMode::Absolute),
            0x5E => self.lsr(mem, AddressingMode::AbsoluteX),

            // Shifts/Rotates: ROL
            0x2A => self.rol_acc(),
            0x26 => self.rol(mem, AddressingMode::ZeroPage),
            0x36 => self.rol(mem, AddressingMode::ZeroPageX),
            0x2E => self.rol(mem, AddressingMode::Absolute),
            0x3E => self.rol(mem, AddressingMode::AbsoluteX),

            // Shifts/Rotates: ROR
            0x6A => self.ror_acc(),
            0x66 => self.ror(mem, AddressingMode::ZeroPage),
            0x76 => self.ror(mem, AddressingMode::ZeroPageX),
            0x6E => self.ror(mem, AddressingMode::Absolute),
            0x7E => self.ror(mem, AddressingMode::AbsoluteX),

            // Transfers
            0xAA => self.tax(),
            0xA8 => self.tay(),
            0x8A => self.txa(),
            0x98 => self.tya(),
            0xBA => self.tsx(),
            0x9A => self.txs(),

            // Stack
            0x48 => self.pha(mem),
            0x68 => self.pla(mem),
            0x08 => self.php(mem),
            0x28 => self.plp(mem),

            // Flags
            0x18 => self.clc(),
            0x38 => self.sec(),
            0x58 => self.cli(),
            0x78 => self.sei(),
            0xB8 => self.clv(),
            0xD8 => self.cld(),
            0xF8 => self.sed(),

            _ => {
                println!("CPU: Unimplemented opcode {:02X} at {:04X}", opcode, self.pc.wrapping_sub(1));
            }
        }
        4
    }
}

pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    NoneAddressing,
    Relative,
}

