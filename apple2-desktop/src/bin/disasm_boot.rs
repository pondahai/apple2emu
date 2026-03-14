use std::fs::File;
use std::io::Read;

// Simple 6502 disassembler for the boot ROM
fn disasm(bytes: &[u8], base: u16) {
    let mut pc = 0usize;
    while pc < bytes.len() {
        let opcode = bytes[pc];
        let (mnemonic, mode, len) = decode_6502(opcode);
        print!("${:04X}: ", base as usize + pc);
        match len {
            1 => println!("{:02X}       {} {}", opcode, mnemonic, mode),
            2 => {
                let op1 = bytes.get(pc + 1).copied().unwrap_or(0);
                println!(
                    "{:02X} {:02X}    {} {}",
                    opcode,
                    op1,
                    mnemonic,
                    format_operand(mode, op1, 0, base as usize + pc)
                );
            }
            3 => {
                let op1 = bytes.get(pc + 1).copied().unwrap_or(0);
                let op2 = bytes.get(pc + 2).copied().unwrap_or(0);
                println!(
                    "{:02X} {:02X} {:02X} {} {}",
                    opcode,
                    op1,
                    op2,
                    mnemonic,
                    format_operand(mode, op1, op2, base as usize + pc)
                );
            }
            _ => println!("{:02X}       ???", opcode),
        }
        pc += len;
    }
}

fn format_operand(mode: &str, op1: u8, op2: u8, pc: usize) -> String {
    match mode {
        "impl" | "A" => String::new(),
        "#" => format!("#${:02X}", op1),
        "zpg" => format!("${:02X}", op1),
        "zpg,X" => format!("${:02X},X", op1),
        "zpg,Y" => format!("${:02X},Y", op1),
        "abs" => format!("${:04X}", op1 as u16 | (op2 as u16) << 8),
        "abs,X" => format!("${:04X},X", op1 as u16 | (op2 as u16) << 8),
        "abs,Y" => format!("${:04X},Y", op1 as u16 | (op2 as u16) << 8),
        "(ind,X)" => format!("(${:02X},X)", op1),
        "(ind),Y" => format!("(${:02X}),Y", op1),
        "rel" => format!("${:04X}", (pc as i32 + 2 + op1 as i8 as i32) as u16),
        "(ind)" => format!("(${:04X})", op1 as u16 | (op2 as u16) << 8),
        _ => format!("???"),
    }
}

fn decode_6502(opcode: u8) -> (&'static str, &'static str, usize) {
    match opcode {
        0x00 => ("BRK", "impl", 1),
        0x01 => ("ORA", "(ind,X)", 2),
        0x05 => ("ORA", "zpg", 2),
        0x06 => ("ASL", "zpg", 2),
        0x08 => ("PHP", "impl", 1),
        0x09 => ("ORA", "#", 2),
        0x0A => ("ASL", "A", 1),
        0x0D => ("ORA", "abs", 3),
        0x10 => ("BPL", "rel", 2),
        0x18 => ("CLC", "impl", 1),
        0x20 => ("JSR", "abs", 3),
        0x24 => ("BIT", "zpg", 2),
        0x25 => ("AND", "zpg", 2),
        0x26 => ("ROL", "zpg", 2),
        0x28 => ("PLP", "impl", 1),
        0x29 => ("AND", "#", 2),
        0x2A => ("ROL", "A", 1),
        0x2C => ("BIT", "abs", 3),
        0x30 => ("BMI", "rel", 2),
        0x38 => ("SEC", "impl", 1),
        0x45 => ("EOR", "zpg", 2),
        0x46 => ("LSR", "zpg", 2),
        0x48 => ("PHA", "impl", 1),
        0x49 => ("EOR", "#", 2),
        0x4A => ("LSR", "A", 1),
        0x4C => ("JMP", "abs", 3),
        0x4E => ("LSR", "abs", 3),
        0x50 => ("BVC", "rel", 2),
        0x60 => ("RTS", "impl", 1),
        0x65 => ("ADC", "zpg", 2),
        0x66 => ("ROR", "zpg", 2),
        0x68 => ("PLA", "impl", 1),
        0x69 => ("ADC", "#", 2),
        0x6C => ("JMP", "(ind)", 3),
        0x70 => ("BVS", "rel", 2),
        0x78 => ("SEI", "impl", 1),
        0x81 => ("STA", "(ind,X)", 2),
        0x84 => ("STY", "zpg", 2),
        0x85 => ("STA", "zpg", 2),
        0x86 => ("STX", "zpg", 2),
        0x88 => ("DEY", "impl", 1),
        0x8A => ("TXA", "impl", 1),
        0x8C => ("STY", "abs", 3),
        0x8D => ("STA", "abs", 3),
        0x8E => ("STX", "abs", 3),
        0x90 => ("BCC", "rel", 2),
        0x91 => ("STA", "(ind),Y", 2),
        0x94 => ("STY", "zpg,X", 2),
        0x95 => ("STA", "zpg,X", 2),
        0x98 => ("TYA", "impl", 1),
        0x99 => ("STA", "abs,Y", 3),
        0x9A => ("TXS", "impl", 1),
        0x9D => ("STA", "abs,X", 3),
        0xA0 => ("LDY", "#", 2),
        0xA2 => ("LDX", "#", 2),
        0xA4 => ("LDY", "zpg", 2),
        0xA5 => ("LDA", "zpg", 2),
        0xA6 => ("LDX", "zpg", 2),
        0xA8 => ("TAY", "impl", 1),
        0xA9 => ("LDA", "#", 2),
        0xAA => ("TAX", "impl", 1),
        0xAC => ("LDY", "abs", 3),
        0xAD => ("LDA", "abs", 3),
        0xAE => ("LDX", "abs", 3),
        0xB0 => ("BCS", "rel", 2),
        0xB1 => ("LDA", "(ind),Y", 2),
        0xB4 => ("LDY", "zpg,X", 2),
        0xB5 => ("LDA", "zpg,X", 2),
        0xB9 => ("LDA", "abs,Y", 3),
        0xBD => ("LDA", "abs,X", 3),
        0xC0 => ("CPY", "#", 2),
        0xC4 => ("CPY", "zpg", 2),
        0xC5 => ("CMP", "zpg", 2),
        0xC6 => ("DEC", "zpg", 2),
        0xC8 => ("INY", "impl", 1),
        0xC9 => ("CMP", "#", 2),
        0xCA => ("DEX", "impl", 1),
        0xCC => ("CPY", "abs", 3),
        0xCD => ("CMP", "abs", 3),
        0xD0 => ("BNE", "rel", 2),
        0xD5 => ("CMP", "zpg,X", 2),
        0xD8 => ("CLD", "impl", 1),
        0xE0 => ("CPX", "#", 2),
        0xE4 => ("CPX", "zpg", 2),
        0xE5 => ("SBC", "zpg", 2),
        0xE6 => ("INC", "zpg", 2),
        0xE8 => ("INX", "impl", 1),
        0xE9 => ("SBC", "#", 2),
        0xEA => ("NOP", "impl", 1),
        0xF0 => ("BEQ", "rel", 2),
        0xF5 => ("SBC", "zpg,X", 2),
        0xF6 => ("INC", "zpg,X", 2),
        _ => ("???", "impl", 1),
    }
}

fn main() {
    let mut f = File::open("roms/DISK2.ROM").unwrap();
    let mut rom = [0u8; 256];
    f.read_exact(&mut rom).unwrap();
    disasm(&rom, 0xC600);
}
